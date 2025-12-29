#!/usr/bin/env python3
"""
Import trips from Excel file into the kniha-jazd database.
Deletes existing trips for the imported year only, then imports from Excel.

Usage: python scripts/import_excel.py [excel_file] [sheet_name]
Default file: _tasks/01-init/kniha_jazd.xlsx
Default sheet: Kniha - 2023 - MB
"""

import sqlite3
import os
import sys
import io
import uuid
from datetime import datetime, timezone

import pandas as pd

# Fix Windows console encoding
if sys.platform == 'win32':
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')

# Database path
DB_PATH = os.path.join(os.environ['APPDATA'], 'com.tauri.dev', 'kniha-jazd.db')

# Defaults
DEFAULT_EXCEL = os.path.join(os.path.dirname(__file__), '..', '_tasks', '01-init', 'kniha_jazd.xlsx')
DEFAULT_SHEET = 'Kniha - 2023 - MB'


def parse_date(date_val) -> str:
    """Convert Excel date to ISO format (YYYY-MM-DD)."""
    if isinstance(date_val, str):
        # Format: DD.MM.YYYY
        parts = date_val.split('.')
        if len(parts) == 3:
            return f"{parts[2]}-{parts[1].zfill(2)}-{parts[0].zfill(2)}"
    elif hasattr(date_val, 'strftime'):
        return date_val.strftime('%Y-%m-%d')
    return str(date_val)


def parse_float(val) -> float | None:
    """Convert value to float, returning None for NaN/empty."""
    if pd.isna(val):
        return None
    try:
        return float(val)
    except (ValueError, TypeError):
        return None


def main():
    excel_file = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_EXCEL
    sheet_name = sys.argv[2] if len(sys.argv) > 2 else DEFAULT_SHEET
    excel_file = os.path.abspath(excel_file)

    if not os.path.exists(excel_file):
        print(f"Error: Excel file not found: {excel_file}")
        sys.exit(1)

    if not os.path.exists(DB_PATH):
        print(f"Error: Database not found: {DB_PATH}")
        sys.exit(1)

    print(f"Reading Excel file: {excel_file} (sheet: {sheet_name})")
    df = pd.read_excel(excel_file, sheet_name=sheet_name, header=None)

    # Connect to database
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()

    # Get vehicle ID
    cursor.execute("SELECT id FROM vehicles LIMIT 1")
    row = cursor.fetchone()
    if not row:
        print("Error: No vehicle found in database")
        sys.exit(1)
    vehicle_id = row[0]
    print(f"Using vehicle ID: {vehicle_id}")

    # Import trips from Excel
    # Row 0: Header info
    # Row 1: Column names
    # Row 2: First record (initial state - skip)
    # Rows 3-70: Trip data
    # Row 71: Total (skip)

    # First pass: determine year from data
    import_year = None
    for i in range(3, len(df)):
        row = df.iloc[i].tolist()
        date_val = row[0]
        if pd.isna(date_val) or str(date_val).lower() == 'total':
            continue
        date = parse_date(date_val)
        import_year = date[:4]  # Extract YYYY from YYYY-MM-DD
        break

    if not import_year:
        print("Error: No valid dates found in Excel data")
        sys.exit(1)

    # Delete only trips for the imported year
    cursor.execute(
        "DELETE FROM trips WHERE vehicle_id = ? AND strftime('%Y', date) = ?",
        (vehicle_id, import_year)
    )
    deleted_count = cursor.rowcount
    print(f"Deleted {deleted_count} existing trips for year {import_year}")

    now = datetime.now(timezone.utc).isoformat()
    imported = 0

    # Process rows 3 to end, skip Total row
    for i in range(3, len(df)):
        row = df.iloc[i].tolist()

        # Skip Total row and empty rows
        date_val = row[0]
        if pd.isna(date_val) or str(date_val).lower() == 'total':
            continue

        # Parse data
        # Columns: 0=date, 1=origin, 2=dest, 3=odo, 4=purpose, 6=fuel_l, 7=fuel_eur, 10=km
        date = parse_date(date_val)
        origin = str(row[1]) if not pd.isna(row[1]) else ''
        destination = str(row[2]) if not pd.isna(row[2]) else ''
        odometer = parse_float(row[3]) or 0.0
        purpose = str(row[4]) if not pd.isna(row[4]) else ''
        fuel_liters = parse_float(row[6])
        fuel_cost_eur = parse_float(row[7])
        distance_km = parse_float(row[10]) or 0.0
        other_costs_eur = parse_float(row[13])
        other_costs_note = str(row[12]) if not pd.isna(row[12]) else None

        # Generate trip ID and sort_order
        trip_id = str(uuid.uuid4())
        # sort_order: 0 = newest, higher = older
        # Since we're importing chronologically (oldest first), reverse the order
        sort_order = len(df) - 3 - imported - 1  # -3 for header rows, -1 for 0-indexing

        # Default full_tank to 1 (true) for all imported trips
        full_tank = 1

        cursor.execute("""
            INSERT INTO trips (
                id, vehicle_id, date, origin, destination, distance_km, odometer,
                purpose, fuel_liters, fuel_cost_eur, other_costs_eur, other_costs_note,
                full_tank, created_at, updated_at, sort_order
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """, (
            trip_id, vehicle_id, date, origin, destination, distance_km, odometer,
            purpose, fuel_liters, fuel_cost_eur, other_costs_eur, other_costs_note,
            full_tank, now, now, sort_order
        ))

        imported += 1
        print(f"  Imported: {date} | {origin[:30]:30} -> {destination[:30]:30} | {distance_km:6.1f} km")

    conn.commit()
    conn.close()

    print(f"\nSuccessfully imported {imported} trips")


if __name__ == '__main__':
    main()
