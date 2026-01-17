**Date:** 2025-12-23
**Subject:** Database Backup/Restore + UI Navigation Redesign
**Status:** Planning

# Task: Backup/Restore & Navigation

## Requirements

### 1. Top Navigation Header
- Add navigation links: "Kniha jázd | Nastavenia"
- Always visible in header bar
- Replace bottom Settings button/link

### 2. Totals Section Redesign
- Change from single row to two rows
- Rename "Km" to "Celkovo najazdené"
- Better visual organization

### 3. Database Backup System
- **Location:** `{app_data_dir}/backups/`
- **File naming:** `kniha-jazd-backup-YYYY-MM-DD-HHmmss.db`
- **Trigger:** Manual "Zálohovať" button in Settings
- **Retention:** Keep all backups (user manages manually)

### 4. Database Restore System
- **File selection:** List available backups from backup folder
- **Confirmation dialog:** Shows date, record counts (vehicles, trips), warning about data loss
- **Action:** Complete replacement of current database

## UI Location

- Navigation: Top header bar (always visible)
- Backup/Restore: New section in Settings page

## Decisions Made

See brainstorming session for decision rationale:
- Fixed backup folder (not user-chosen) - simpler
- Manual backup only (not auto) - user-controlled
- Replace with confirmation (not merge) - cleaner for personal logbook
- Keep all backups (not auto-delete) - user manages
