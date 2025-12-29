#!/usr/bin/env python3
"""Compare imported data with Excel source for validation."""

import sqlite3
import os
import sys
import io
import pandas as pd

# Fix Windows console encoding
if sys.platform == 'win32':
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')

DB_PATH = os.path.join(os.environ['APPDATA'], 'com.tauri.dev', 'kniha-jazd.db')
EXCEL_PATH = os.path.join(os.path.dirname(__file__), '..', '_tasks', '01-init', 'kniha_jazd.xlsx')

SHEETS = [
    ('Kniha - 2025 - MB', '2025'),
    ('Kniha - 2024 - MB', '2024'),
    ('Kniha - 2023 - MB', '2023'),
]


def parse_date(date_val) -> str:
    """Convert Excel date to ISO format (YYYY-MM-DD)."""
    if isinstance(date_val, str):
        parts = date_val.split('.')
        if len(parts) == 3:
            return f"{parts[2]}-{parts[1].zfill(2)}-{parts[0].zfill(2)}"
    elif hasattr(date_val, 'strftime'):
        return date_val.strftime('%Y-%m-%d')
    return str(date_val)


def get_excel_data(sheet_name: str) -> list[dict]:
    """Read Excel sheet and return list of trip dicts."""
    df = pd.read_excel(EXCEL_PATH, sheet_name=sheet_name, header=None)
    trips = []

    for i in range(3, len(df)):
        row = df.iloc[i].tolist()
        date_val = row[0]
        if pd.isna(date_val) or str(date_val).lower() == 'total':
            continue

        trips.append({
            'date': parse_date(date_val),
            'origin': str(row[1]) if not pd.isna(row[1]) else '',
            'destination': str(row[2]) if not pd.isna(row[2]) else '',
            'odometer': float(row[3]) if not pd.isna(row[3]) else 0.0,
            'purpose': str(row[4]) if not pd.isna(row[4]) else '',
            'fuel_liters': float(row[6]) if not pd.isna(row[6]) else None,
            'fuel_cost_eur': float(row[7]) if not pd.isna(row[7]) else None,
            'excel_spotreba': float(row[8]) if not pd.isna(row[8]) else None,  # Fuel consumed on trip (L)
            'excel_zostatok': float(row[11]) if not pd.isna(row[11]) else None,  # Remaining fuel (L)
            'distance_km': float(row[10]) if not pd.isna(row[10]) else 0.0,
            'excel_rate': float(row[14]) if not pd.isna(row[14]) else None,  # l/100km rate used
        })

    return trips


def get_db_data(year: str) -> list[dict]:
    """Get trips from database for a specific year."""
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()

    # Get vehicle ID
    cursor.execute("SELECT id FROM vehicles LIMIT 1")
    vehicle_id = cursor.fetchone()[0]

    # Get trips for year, ordered chronologically (oldest first to match Excel)
    cursor.execute("""
        SELECT date, origin, destination, odometer, purpose,
               fuel_liters, fuel_cost_eur, distance_km, id
        FROM trips
        WHERE vehicle_id = ? AND strftime('%Y', date) = ?
        ORDER BY date ASC, odometer ASC
    """, (vehicle_id, year))

    trips = []
    for row in cursor.fetchall():
        trips.append({
            'date': row[0],
            'origin': row[1],
            'destination': row[2],
            'odometer': row[3],
            'purpose': row[4],
            'fuel_liters': row[5],
            'fuel_cost_eur': row[6],
            'distance_km': row[7],
            'id': row[8],
        })

    conn.close()
    return trips


def get_calculated_values(year: str) -> dict:
    """Get calculated values from the app's grid data endpoint."""
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()

    # We can't call Tauri commands, so we'll calculate manually using the same logic
    cursor.execute("SELECT id, tp_consumption, tank_size_liters FROM vehicles LIMIT 1")
    vehicle = cursor.fetchone()
    vehicle_id, tp_consumption, tank_size = vehicle

    # Get trips chronologically
    cursor.execute("""
        SELECT id, date, distance_km, fuel_liters, full_tank, odometer
        FROM trips
        WHERE vehicle_id = ? AND strftime('%Y', date) = ?
        ORDER BY date ASC, odometer ASC
    """, (vehicle_id, year))

    trips = cursor.fetchall()
    conn.close()

    # Calculate rates using the same algorithm as backend
    rates = {}
    current_trip_ids = []
    km_in_period = 0.0
    fuel_in_period = 0.0

    for trip_id, date, distance_km, fuel_liters, full_tank, odometer in trips:
        current_trip_ids.append(trip_id)
        km_in_period += distance_km

        if fuel_liters and fuel_liters > 0:
            fuel_in_period += fuel_liters

            if full_tank and km_in_period > 0:
                rate = (fuel_in_period / km_in_period) * 100.0
                for tid in current_trip_ids:
                    rates[tid] = rate
                current_trip_ids = []
                km_in_period = 0.0
                fuel_in_period = 0.0

    # Remaining trips use TP rate
    for tid in current_trip_ids:
        rates[tid] = tp_consumption

    # Calculate zostatok and spotreba
    zostatok_map = {}
    spotreba_map = {}
    zostatok = tank_size  # Start with full tank

    for trip_id, date, distance_km, fuel_liters, full_tank, odometer in trips:
        rate = rates.get(trip_id, tp_consumption)
        spotreba = (distance_km * rate) / 100.0
        spotreba_map[trip_id] = spotreba
        zostatok -= spotreba

        if fuel_liters and fuel_liters > 0:
            if full_tank:
                zostatok = tank_size
            else:
                zostatok = min(zostatok + fuel_liters, tank_size)

        zostatok = max(0, min(zostatok, tank_size))
        zostatok_map[trip_id] = zostatok

    return {'rates': rates, 'zostatok': zostatok_map, 'spotreba': spotreba_map}


def compare_year(sheet_name: str, year: str) -> dict:
    """Compare Excel and DB data for a specific year."""
    excel_data = get_excel_data(sheet_name)
    db_data = get_db_data(year)
    calc_data = get_calculated_values(year)

    result = {
        'year': year,
        'excel_count': len(excel_data),
        'db_count': len(db_data),
        'fixed_mismatches': [],
        'calc_mismatches': [],
    }

    if len(excel_data) != len(db_data):
        result['count_mismatch'] = True
        return result

    for i, (excel, db) in enumerate(zip(excel_data, db_data)):
        row_num = i + 1

        # Compare fixed values
        fixed_fields = ['date', 'origin', 'destination', 'distance_km', 'odometer', 'purpose', 'fuel_liters', 'fuel_cost_eur']
        for field in fixed_fields:
            ev = excel.get(field)
            dv = db.get(field)

            # Handle None vs None
            if ev is None and dv is None:
                continue

            # Handle float comparison
            if isinstance(ev, float) and isinstance(dv, float):
                if abs(ev - dv) > 0.01:
                    result['fixed_mismatches'].append({
                        'row': row_num,
                        'field': field,
                        'excel': ev,
                        'db': dv,
                    })
            elif ev != dv:
                result['fixed_mismatches'].append({
                    'row': row_num,
                    'field': field,
                    'excel': ev,
                    'db': dv,
                })

        # Compare calculated values
        trip_id = db['id']

        # Consumption rate
        excel_rate = excel.get('excel_rate')
        calc_rate = calc_data['rates'].get(trip_id)
        if excel_rate is not None and calc_rate is not None:
            if abs(excel_rate - calc_rate) > 0.05:  # Allow small tolerance
                result['calc_mismatches'].append({
                    'row': row_num,
                    'field': 'consumption_rate',
                    'excel': excel_rate,
                    'calculated': calc_rate,
                    'diff': abs(excel_rate - calc_rate),
                })

        # Spotreba (fuel consumed on trip)
        excel_spotreba = excel.get('excel_spotreba')
        calc_spotreba = calc_data['spotreba'].get(trip_id)
        if excel_spotreba is not None and calc_spotreba is not None:
            if abs(excel_spotreba - calc_spotreba) > 0.1:  # Allow small tolerance
                result['calc_mismatches'].append({
                    'row': row_num,
                    'field': 'spotreba',
                    'excel': excel_spotreba,
                    'calculated': calc_spotreba,
                    'diff': abs(excel_spotreba - calc_spotreba),
                })

        # Zostatok
        excel_zostatok = excel.get('excel_zostatok')
        calc_zostatok = calc_data['zostatok'].get(trip_id)
        if excel_zostatok is not None and calc_zostatok is not None:
            if abs(excel_zostatok - calc_zostatok) > 0.5:  # Allow small tolerance
                result['calc_mismatches'].append({
                    'row': row_num,
                    'field': 'zostatok',
                    'excel': excel_zostatok,
                    'calculated': calc_zostatok,
                    'diff': abs(excel_zostatok - calc_zostatok),
                })

    return result


def main():
    print("=" * 70)
    print("IMPORT COMPARISON REPORT")
    print("=" * 70)

    for sheet_name, year in SHEETS:
        print(f"\n{'=' * 70}")
        print(f"YEAR: {year} (Sheet: {sheet_name})")
        print("=" * 70)

        # First import the data
        print(f"\nImporting {sheet_name}...")
        import subprocess
        result = subprocess.run(
            ['python', 'scripts/import_excel.py', EXCEL_PATH, sheet_name],
            capture_output=True, text=True, cwd=os.path.dirname(os.path.dirname(__file__))
        )
        if result.returncode != 0:
            print(f"  ERROR: {result.stderr}")
            continue

        # Compare
        comparison = compare_year(sheet_name, year)

        print(f"\n  Record counts: Excel={comparison['excel_count']}, DB={comparison['db_count']}")

        if comparison.get('count_mismatch'):
            print("  ERROR: Record count mismatch!")
            continue

        # Fixed value summary
        if comparison['fixed_mismatches']:
            print(f"\n  FIXED VALUE MISMATCHES: {len(comparison['fixed_mismatches'])}")
            for m in comparison['fixed_mismatches'][:5]:  # Show first 5
                print(f"    Row {m['row']}: {m['field']} - Excel: {m['excel']!r}, DB: {m['db']!r}")
            if len(comparison['fixed_mismatches']) > 5:
                print(f"    ... and {len(comparison['fixed_mismatches']) - 5} more")
        else:
            print("\n  FIXED VALUES: All match!")

        # Calculated value summary
        if comparison['calc_mismatches']:
            print(f"\n  CALCULATED VALUE MISMATCHES: {len(comparison['calc_mismatches'])}")
            for m in comparison['calc_mismatches'][:10]:  # Show first 10
                print(f"    Row {m['row']}: {m['field']} - Excel: {m['excel']:.2f}, Calc: {m['calculated']:.2f}, Diff: {m['diff']:.2f}")
            if len(comparison['calc_mismatches']) > 10:
                print(f"    ... and {len(comparison['calc_mismatches']) - 10} more")
        else:
            print("\n  CALCULATED VALUES: All match!")

    print("\n" + "=" * 70)
    print("COMPARISON COMPLETE")
    print("=" * 70)


if __name__ == '__main__':
    main()
