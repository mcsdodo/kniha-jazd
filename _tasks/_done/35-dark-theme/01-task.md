**Date:** 2026-01-12
**Subject:** Dark theme for the application
**Status:** Planning

# Dark Theme Feature

## User Story

As a user, I want to switch between light and dark themes so I can use the app comfortably in different lighting conditions.

## Requirements

1. **Theme trigger**: Both system preference detection AND manual toggle
2. **Visual style**: Soft dark (#1e1e1e) - charcoal/VS Code style
3. **Toggle location**: Settings page only (Appearance section)
4. **Persistence**: Backend storage in `local.settings.json`

## Success Criteria

- [ ] Theme persists across app restarts
- [ ] System preference changes are detected when in "system" mode
- [ ] All UI components render correctly in both themes
- [ ] No flash of wrong theme on startup
