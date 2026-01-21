# Task 38: Auto-Update

## Goal

Implement automatic update checking and installation so users are notified when a new version is available and can update with one click.

## Requirements

- Check for updates on app startup (background)
- Manual "Check for Updates" button in Settings
- Show modal when update available with version + release notes
- "Update Now" downloads, installs, and restarts
- "Later" dismisses until next startup, shows subtle indicator
- Use GitHub Releases as update server
- Signed updates for security

## Technical Approach

- Tauri `tauri-plugin-updater` for update mechanism
- GitHub Releases hosts `latest.json` manifest + signed installers
- Signing keypair: private in GitHub Secrets, public in app config
- Extend existing `release.yml` CI workflow

## Status

- [x] Design complete (see `02-design.md`)
- [ ] Implementation
