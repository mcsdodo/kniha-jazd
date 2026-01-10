**Date:** 2026-01-10
**Subject:** Portable CSV Backup Export/Import
**Status:** Planning

# Portable CSV Backup

## Summary

Add ability to export the entire database as a human-readable ZIP of CSV files, and import it back. This enables data portability — moving data to another system, sharing with accountant, or long-term archival.

## User Story

As a user, I want to export my vehicle logbook data in a format that:
- Can be opened in Excel/Google Sheets for review
- Can be shared with my accountant
- Can be imported into a fresh app installation
- Serves as a portable backup alongside the binary SQLite backup

## Requirements

### Export

- [ ] Export all data as ZIP containing CSVs + metadata
- [ ] Single command/button in Settings
- [ ] UTF-8 with BOM for Slovak character support in Excel
- [ ] ISO 8601 date format (YYYY-MM-DD)
- [ ] Save dialog for user to choose location

### Import

- [ ] Import ZIP file, validate structure
- [ ] Full replace (wipe current DB, import all)
- [ ] Auto-backup before import (safety net)
- [ ] Clear, actionable error messages with file/row/field context
- [ ] Confirmation dialog before destructive action

### Data Scope

- All tables: vehicles, trips, routes, settings, receipts
- Full database dump (no filtering by year/vehicle)
- Receipts: metadata only (file_path won't be portable)

## Out of Scope

- Merge/update import (too complex, not needed for portability)
- Per-vehicle or per-year export filtering
- Exporting actual receipt image files
