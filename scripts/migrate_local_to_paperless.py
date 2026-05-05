#!/usr/bin/env python3
"""
Migrate local receipt assignments to Paperless-ngx document links.

For each trip with a local receipt assigned, finds the matching Paperless
document by comparing liters, price, and datetime.  Flags discrepancies.
Inserts paperless_trip_links rows without touching the existing receipt link.

Usage:
    python scripts/migrate_local_to_paperless.py              # dry run (safe)
    python scripts/migrate_local_to_paperless.py --apply      # write to DB
    python scripts/migrate_local_to_paperless.py --env prod   # use prod settings
    python scripts/migrate_local_to_paperless.py --db-path PATH --paperless-url URL --paperless-token TOKEN
"""

import argparse
import io
import json
import os
import sqlite3
import sys
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

# Force UTF-8 output on Windows (avoids cp1252 encoding errors from Slovak/Czech chars)
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")
sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding="utf-8", errors="replace")

APPDATA = Path(os.environ["APPDATA"])
DEV_APP_DIR  = APPDATA / "com.notavailable.kniha-jazd.dev"
PROD_APP_DIR = APPDATA / "com.notavailable.kniha-jazd"

TOLERANCE = 0.01  # EUR / litre tolerance for value matching


# ---------------------------------------------------------------------------
# Settings
# ---------------------------------------------------------------------------

def load_settings(app_dir: Path) -> dict:
    p = app_dir / "local.settings.json"
    return json.loads(p.read_text(encoding="utf-8")) if p.exists() else {}


# ---------------------------------------------------------------------------
# Paperless API helpers
# ---------------------------------------------------------------------------

def _get(url: str, token: str) -> dict:
    req = urllib.request.Request(url, headers={"Authorization": f"Token {token}"})
    with urllib.request.urlopen(req, timeout=15) as r:
        return json.loads(r.read())


def resolve_tag_id(base: str, token: str, name: str) -> int:
    url = f"{base}/api/tags/?name__iexact={urllib.parse.quote(name)}"
    results = _get(url, token).get("results", [])
    if not results:
        raise ValueError(f"Tag '{name}' not found in Paperless")
    return results[0]["id"]


def resolve_field_ids(base: str, token: str, names: dict) -> dict:
    url = f"{base}/api/custom_fields/?page_size=200"
    all_fields = {f["name"]: f["id"] for f in _get(url, token).get("results", [])}
    missing = [n for n in names.values() if n not in all_fields]
    if missing:
        raise ValueError(f"Custom fields not found in Paperless: {missing}")
    return {
        "datetime_id": all_fields[names["datetime"]],
        "liters_id":   all_fields[names["liters"]],
        "total_id":    all_fields[names["total"]],
    }


def fetch_all_docs(base: str, token: str, fuel_id: int, car_id: int, fmap: dict) -> list[dict]:
    url: Optional[str] = (
        f"{base}/api/documents/"
        f"?tags__id__in={fuel_id},{car_id}&page_size=100"
    )
    docs = []
    while url:
        page = _get(url, token)
        for r in page["results"]:
            total = litres = dt = None
            for cf in r.get("custom_fields", []):
                val = cf["value"]
                if val is None:
                    continue
                fid = cf["field"]
                if fid == fmap["total_id"]:
                    total = float(val)
                elif fid == fmap["liters_id"]:
                    litres = float(val)
                elif fid == fmap["datetime_id"]:
                    for fmt in ("%Y-%m-%dT%H:%M:%S", "%Y-%m-%d"):
                        try:
                            dt = datetime.strptime(str(val), fmt); break
                        except ValueError:
                            pass

            created = datetime.strptime(r["created"][:10], "%Y-%m-%d").date()
            docs.append({
                "id": r["id"], "title": r["title"],
                "tag_ids": r["tags"], "created": created,
                "total_amount": total, "litres": litres,
                "receipt_datetime": dt,
            })
        url = page.get("next")
    return docs


# ---------------------------------------------------------------------------
# Matching logic
# ---------------------------------------------------------------------------

def _parse_dt(s: Optional[str]) -> Optional[datetime]:
    if not s:
        return None
    for fmt in ("%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M:%S", "%Y-%m-%d"):
        try:
            return datetime.strptime(s, fmt)
        except ValueError:
            pass
    return None


def _val_match(a: Optional[float], b: Optional[float]) -> bool:
    return a is not None and b is not None and abs(a - b) <= TOLERANCE


def _date_match(
    receipt_dt: Optional[datetime],
    doc_dt: Optional[datetime],
    trip_start: Optional[datetime],
    trip_end: Optional[datetime],
) -> bool:
    if doc_dt is None:
        return False
    if receipt_dt is not None and doc_dt.date() == receipt_dt.date():
        return True
    if trip_start and trip_end and trip_start <= doc_dt <= trip_end:
        return True
    if trip_start and doc_dt.date() == trip_start.date():
        return True
    return False


def find_match(row: dict, docs: list[dict]) -> tuple:
    """
    Returns (best_doc | None, quality, note, alt_candidates).
    quality: "exact" | "partial" | "other_ok" | None
    """
    atype      = (row["assignment_type"] or "Fuel").strip()
    trip_start = _parse_dt(row["start_datetime"])
    trip_end   = _parse_dt(row["end_datetime"])
    receipt_dt = _parse_dt(row["receipt_datetime"])

    scored = []
    for doc in docs:
        doc_dt = doc["receipt_datetime"]
        if atype == "Fuel":
            lm = _val_match(row["fuel_liters"], doc["litres"])
            pm = _val_match(row["fuel_cost_eur"], doc["total_amount"])
            dm = _date_match(receipt_dt, doc_dt, trip_start, trip_end)
            scored.append((lm + pm + dm, doc, lm, pm, dm))
        else:
            pm = _val_match(row["other_costs_eur"], doc["total_amount"])
            dm = _date_match(receipt_dt, doc_dt, trip_start, trip_end)
            scored.append((pm + dm, doc, False, pm, dm))

    scored.sort(key=lambda x: x[0], reverse=True)
    if not scored or scored[0][0] < 2:
        top3 = [s[1] for s in scored[:3]]
        return None, None, "no 2-field match found", top3

    best_score, best_doc, lm, pm, dm = scored[0]
    alts = [s[1] for s in scored[1:] if s[0] == best_score]

    if atype == "Fuel":
        discrepancies = []
        if not lm:
            discrepancies.append(
                f"liters trip={row['fuel_liters']} doc={best_doc['litres']}"
            )
        if not pm:
            discrepancies.append(
                f"price trip={row['fuel_cost_eur']} doc={best_doc['total_amount']}"
            )
        if not dm:
            rd = receipt_dt.date() if receipt_dt else "?"
            dd = best_doc["receipt_datetime"].date() if best_doc["receipt_datetime"] else "?"
            discrepancies.append(f"date receipt={rd} doc={dd}")
        quality = "exact" if best_score == 3 else "partial"
        note = "; ".join(discrepancies)
    else:
        quality = "other_ok"
        note = ""

    return best_doc, quality, note, alts


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    ap = argparse.ArgumentParser(
        description="Migrate local receipt assignments → Paperless-ngx document links"
    )
    ap.add_argument("--apply", action="store_true",
                    help="Write to DB (default: dry run)")
    ap.add_argument("--env", choices=["dev", "prod"], default="dev",
                    help="App environment (default: dev)")
    ap.add_argument("--db-path",        help="Override SQLite DB path")
    ap.add_argument("--paperless-url",  help="Override Paperless base URL")
    ap.add_argument("--paperless-token",help="Override Paperless API token")
    args = ap.parse_args()

    app_dir  = PROD_APP_DIR if args.env == "prod" else DEV_APP_DIR
    settings = load_settings(app_dir)

    db_path   = args.db_path         or settings.get("custom_db_path") or str(app_dir / "kniha-jazd.db")
    p_url     = (args.paperless_url  or settings.get("paperless_url", "")).rstrip("/")
    p_token   = args.paperless_token or settings.get("paperless_api_token", "")
    f_names   = {
        "datetime": settings.get("paperless_field_name_datetime") or "receipt_datetime",
        "liters":   settings.get("paperless_field_name_liters")   or "liters",
        "total":    settings.get("paperless_field_name_total")     or "total_price_eur",
    }

    if not p_url or not p_token:
        sys.exit("ERROR: Paperless URL or token not configured")

    mode = "LIVE — DB WILL BE MODIFIED" if args.apply else "DRY RUN — no DB changes"
    print(f"\nMIGRATION: local receipts -> Paperless links  [{mode}]")
    print(f"DB:        {db_path}")
    print(f"Paperless: {p_url}")
    print(f"Fields:    datetime={f_names['datetime']}  liters={f_names['liters']}  total={f_names['total']}")
    print("=" * 72)

    # --- DB: load assigned trips -------------------------------------------
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    cur = conn.cursor()
    cur.execute("""
        SELECT
            t.id            AS trip_id,
            t.start_datetime,
            t.end_datetime,
            t.fuel_liters,
            t.fuel_cost_eur,
            t.other_costs_eur,
            t.other_costs_note,
            r.id            AS receipt_id,
            r.liters        AS receipt_liters,
            r.total_price_eur AS receipt_price,
            r.receipt_datetime,
            r.assignment_type,
            r.vendor_name,
            r.cost_description,
            (SELECT ptl.paperless_document_id
               FROM paperless_trip_links ptl
              WHERE ptl.trip_id = t.id)  AS existing_doc_id
        FROM trips t
        INNER JOIN receipts r ON r.trip_id = t.id
        ORDER BY t.start_datetime
    """)
    rows = [dict(r) for r in cur.fetchall()]
    print(f"\nTrips with local receipt: {len(rows)}")

    # --- Paperless: fetch all docs -----------------------------------------
    print("Fetching Paperless documents …")
    try:
        fuel_id = resolve_tag_id(p_url, p_token, "fuel")
        car_id  = resolve_tag_id(p_url, p_token, "car")
        fmap    = resolve_field_ids(p_url, p_token, f_names)
        docs    = fetch_all_docs(p_url, p_token, fuel_id, car_id, fmap)
    except Exception as exc:
        sys.exit(f"ERROR fetching Paperless: {exc}")
    print(f"Paperless documents:      {len(docs)}\n")

    # --- Match & report ---------------------------------------------------
    counts = {"exact": 0, "partial": 0, "other_ok": 0,
              "unresolved": 0, "skipped": 0, "ambiguous": 0}
    to_insert: list[tuple] = []

    for row in rows:
        trip_date = (_parse_dt(row["start_datetime"]) or datetime.min).strftime("%Y-%m-%d")
        tid_short = row["trip_id"][:8]

        if row["existing_doc_id"]:
            print(f"[SKIP]        {trip_date}  trip={tid_short}  already → Paperless #{row['existing_doc_id']}")
            counts["skipped"] += 1
            continue

        doc, quality, note, alts = find_match(row, docs)

        if quality is None:
            atype = (row["assignment_type"] or "Fuel").strip()
            if atype == "Fuel":
                receipt_vals = f"liters={row['fuel_liters']}  price={row['fuel_cost_eur']}  date={row['receipt_datetime']}"
            else:
                receipt_vals = f"price={row['other_costs_eur']}  date={row['receipt_datetime']}"
            print(f"[UNRESOLVED]  {trip_date}  trip={tid_short}  type={atype}")
            print(f"              receipt:  {receipt_vals}")
            for c in alts:  # alts holds top-3 candidates here
                cdt = c["receipt_datetime"].strftime("%Y-%m-%dT%H:%M:%S") if c["receipt_datetime"] else "?"
                print(f"              closest:  #{c['id']}  liters={c['litres']}  price={c['total_amount']}  date={cdt}  \"{c['title'][:50]}\"")
            counts["unresolved"] += 1
            continue

        label = {
            "exact":    "[OK]         ",
            "partial":  "[DISCREPANCY]",
            "other_ok": "[OTHER]      ",
        }[quality]

        disc  = f"  ({note})"          if note else ""
        ambig = f"  also: {[c['id'] for c in alts]}" if alts else ""
        print(f"{label} {trip_date}  trip={tid_short}  -> #{doc['id']}  \"{doc['title'][:42]}\"{disc}{ambig}")

        counts[quality] += 1
        if alts:
            counts["ambiguous"] += 1
        to_insert.append((row["trip_id"], doc["id"]))

    # --- Summary -----------------------------------------------------------
    matched = counts["exact"] + counts["partial"] + counts["other_ok"]
    print(f"\n{'=' * 72}")
    print(
        f"SUMMARY  {len(rows)} trips | "
        f"{matched} matched ({counts['exact']} exact, {counts['partial']} discrepancy, {counts['other_ok']} other) | "
        f"{counts['ambiguous']} ambiguous | "
        f"{counts['unresolved']} unresolved | "
        f"{counts['skipped']} already linked"
    )

    # --- Write (live mode only) -------------------------------------------
    if args.apply:
        if to_insert:
            now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S")
            written = 0
            for trip_id, doc_id in to_insert:
                try:
                    cur.execute(
                        "INSERT OR IGNORE INTO paperless_trip_links "
                        "(trip_id, paperless_document_id, created_at, updated_at) "
                        "VALUES (?, ?, ?, ?)",
                        (trip_id, doc_id, now, now),
                    )
                    if cur.rowcount:
                        written += 1
                except Exception as exc:
                    print(f"  ERROR trip={trip_id}: {exc}", file=sys.stderr)
            conn.commit()
            print(f"\nInserted {written} paperless_trip_links rows.")
        else:
            print("\nNothing to insert.")
    else:
        print(f"\n→ Run with --apply to insert {len(to_insert)} rows.")

    conn.close()


if __name__ == "__main__":
    main()
