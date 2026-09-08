#!/usr/bin/env python3
"""Measure the evidence behind ADR-044, the one trip ordering.

Reads a COPY of a logbook database and reports, for each vehicle:

  * span warnings raised over the whole book under each candidate ordering.
    The rule is the shipped one: a row warns when
    |odometer - odometer_start - distance_km| >= 1.0 km, where odometer_start
    is the previous row's stored odometer in that ordering
    (`calculate_odometer_span_warnings`, statistics.rs).
  * tied `start_datetime` groups, and how many sit at 00:00:00.
  * the groups where the two candidate tie-breaks disagree, with how many
    place links each order connects.
  * how often the place chain alone settles a tied group.

A place link connects when one row's destination equals the next row's origin,
compared with the same folding the place book uses (lower case, diacritics
removed, whitespace collapsed). The window for a tied group is
"row before -> the group -> row after".

Usage:  python3 scripts/measure-trip-order.py <path-to-db-copy>

The script opens the file read-only. Never point it at a live database.
"""

import itertools
import sqlite3
import sys
import unicodedata
from collections import defaultdict

SPAN_TOLERANCE_KM = 1.0

ORDERINGS = (
    ("datetime, created_at, id", lambda t: (t["start_datetime"], t["created_at"], t["id"])),
    ("datetime, odometer, id", lambda t: (t["start_datetime"], t["odometer"], t["id"])),
    (
        "datetime, created_at, odometer, id (shipped)",
        lambda t: (t["start_datetime"], t["created_at"], t["odometer"], t["id"]),
    ),
)


def fold(text):
    """Lower case, drop diacritics, collapse whitespace.

    Close to `places::normalise` (normalise.rs), not identical: this drops the
    combining marks Unicode NFD produces, while the Rust side maps a closed table
    of accented letters. They differ on letters NFD does not decompose but the
    table still maps -- l-stroke and sharp-s. No place name in this book carries
    either, so no figure this script prints depends on the difference.
    """
    lowered = (text or "").lower()
    decomposed = unicodedata.normalize("NFD", lowered)
    stripped = "".join(c for c in decomposed if unicodedata.category(c) != "Mn")
    return " ".join(stripped.split())


def span_warnings(trips, key, initial_odometer):
    """Warnings per year under one ordering. The chain runs over the whole book."""
    per_year = defaultdict(int)
    prev_odometer = initial_odometer
    for trip in sorted(trips, key=key):
        drift = abs(trip["odometer"] - prev_odometer - trip["distance_km"])
        if drift >= SPAN_TOLERANCE_KM:
            per_year[int(trip["start_datetime"][:4])] += 1
        prev_odometer = trip["odometer"]
    return dict(sorted(per_year.items()))


def tied_groups(trips):
    groups = defaultdict(list)
    for trip in trips:
        groups[trip["start_datetime"]].append(trip)
    return {k: v for k, v in groups.items() if len(v) > 1}


def window(canonical, index, group):
    """The rows around a tied group: the one before and the one after."""
    first = min(index[t["id"]] for t in group)
    last = max(index[t["id"]] for t in group)
    before = [canonical[first - 1]] if first > 0 else []
    after = [canonical[last + 1]] if last + 1 < len(canonical) else []
    return before, after


def connected_links(sequence):
    return sum(
        1
        for i in range(len(sequence) - 1)
        if fold(sequence[i]["destination"]) == fold(sequence[i + 1]["origin"])
    )


def report_vehicle(rows, initial_odometer, label):
    print(f"vehicle {label}: {len(rows)} trips, initial odometer {initial_odometer}")

    print("  span warnings over the whole book")
    for name, key in ORDERINGS:
        per_year = span_warnings(rows, key, initial_odometer)
        print(f"    {name:44s} {sum(per_year.values()):3d}  {per_year}")

    groups = tied_groups(rows)
    at_midnight = sum(1 for k in groups if k[11:] == "00:00:00")
    rows_tied = sum(len(v) for v in groups.values())
    print(
        f"  tied start_datetime groups {len(groups)}"
        f" | rows in them {rows_tied} | groups at 00:00:00 {at_midnight}"
    )

    canonical = sorted(rows, key=ORDERINGS[2][1])
    index = {t["id"]: i for i, t in enumerate(canonical)}

    by_created = lambda t: (t["created_at"], t["odometer"], t["id"])
    by_odometer = lambda t: (t["odometer"], t["id"])
    disagreeing = [
        k
        for k, v in groups.items()
        if [t["id"] for t in sorted(v, key=by_created)]
        != [t["id"] for t in sorted(v, key=by_odometer)]
    ]
    print(f"  groups where created_at order != odometer order: {len(disagreeing)}")
    for k in disagreeing:
        before, after = window(canonical, index, groups[k])
        for name, key in (("created_at", by_created), ("odometer", by_odometer)):
            sequence = before + sorted(groups[k], key=key) + after
            print(
                f"    {k} {name:10s} connects "
                f"{connected_links(sequence)} of {len(sequence) - 1} place links"
            )

    settled_fully = settled_best = 0
    for group in groups.values():
        before, after = window(canonical, index, group)
        scores = [
            connected_links(before + list(perm) + after)
            for perm in itertools.permutations(group)
        ]
        best = max(scores)
        links = len(before) + len(group) + len(after) - 1
        if scores.count(best) == 1:
            settled_best += 1
            if best == links:
                settled_fully += 1
    print(
        f"  place chain settles: {settled_fully} of {len(groups)} groups by one fully"
        f" connected order, {settled_best} of {len(groups)} by one best-scoring order"
    )


def main(path):
    db = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    db.row_factory = sqlite3.Row
    vehicles = db.execute(
        "select id, license_plate, initial_odometer from vehicles"
    ).fetchall()
    for vehicle in vehicles:
        rows = [
            dict(r)
            for r in db.execute(
                "select id, start_datetime, created_at, odometer, distance_km,"
                " origin, destination from trips where vehicle_id = ?",
                (vehicle["id"],),
            )
        ]
        if not rows:
            continue
        report_vehicle(rows, vehicle["initial_odometer"], vehicle["license_plate"])


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    main(sys.argv[1])
