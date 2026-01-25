**Date:** 2026-01-24
**Subject:** Automatic Database Backup Before Updates
**Status:** Planning

## User Story

As a user, I want the app to automatically create a database backup before installing updates, so that I can restore my data if the update causes problems.

## Requirements

1. **Automatic backup before update** — When user clicks "Aktualizovať", create backup before download starts
2. **Clear identification** — Pre-update backups should be distinguishable from manual backups (filename + UI badge)
3. **Graceful failure handling** — If backup fails, warn user and let them choose to proceed or cancel
4. **Configurable retention** — User can set how many pre-update backups to keep
5. **Manual cleanup** — "Vyčistiť teraz" button to apply retention settings immediately
6. **Auto-cleanup** — After successful update, cleanup old pre-update backups (if retention enabled)

## Acceptance Criteria

- [ ] Pre-update backups have filename suffix `-pred-v{version}`
- [ ] Backup list shows badge `[pred v0.20.0]` for pre-update backups
- [ ] Update modal shows backup step with progress indicator
- [ ] Backup failure shows warning dialog with proceed/cancel options
- [ ] Settings has retention controls (checkbox + keep count dropdown)
- [ ] "Vyčistiť teraz" button works and shows preview of what will be deleted
- [ ] Manual backups are never affected by cleanup
- [ ] Auto-cleanup runs after successful update (when version matches backup)

## Related

- Design: `02-design.md`
