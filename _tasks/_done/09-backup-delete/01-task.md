**Date:** 2025-12-26
**Subject:** Add ability to delete backups
**Status:** Complete

## User Story

As a user, I want to delete old backup files from the settings page so I can manage disk space and keep only relevant backups.

## Requirements

1. Add delete button next to each backup in the list
2. Show confirmation modal before deleting (with backup date and size)
3. All backups can be deleted (no special protection)
4. After deletion, refresh the backup list

## Acceptance Criteria

- [ ] Delete button visible for each backup
- [ ] Confirmation modal shows backup details
- [ ] Backup file is removed from disk after confirmation
- [ ] Backup list updates after deletion
- [ ] Error handling for failed deletions
