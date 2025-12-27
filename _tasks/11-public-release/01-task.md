**Date:** 2025-12-28
**Subject:** Publish app to public GitHub repository
**Status:** Planning

## Goal

Prepare the app for public release on GitHub with proper documentation, cross-platform builds, and distribution via GitHub Releases.

## Requirements

### Documentation
- README.md in Slovak (primary) with all features described
- README.en.md in English (for developers/international)
- Language switcher at top of each README
- CHANGELOG.md for version history
- CONTRIBUTING.md for contributors
- GPL-3.0 LICENSE file

### Distribution
- GitHub Releases as download source
- Cross-platform builds: Windows (.msi), macOS Intel (.dmg), macOS Apple Silicon (.dmg)
- GitHub Actions workflow triggered by version tags

### Screenshots
- Single hero screenshot showing main trip grid with data
- Dummy showcase database with realistic test data (separate sub-task)

### Process Changes
- Update CLAUDE.md to mandate documentation updates with feature changes
- Update Cargo.toml and tauri.conf.json metadata

## Out of Scope (Future)
- Landing page / website
- Auto-updates (tauri-plugin-updater)
- Linux builds
