**Date:** 2025-12-28
**Subject:** Public release design
**Status:** Approved

## Decisions Made

| Decision | Choice |
|----------|--------|
| Target audience | Both: Slovak business users + developer community |
| Platforms | Windows + macOS (Intel & Apple Silicon) |
| Distribution | GitHub Releases only |
| Documentation | Minimal structure, complete feature coverage |
| Screenshots | Single hero image with dummy data |
| License | GPL-3.0 |
| Repo name | kniha-jazd |
| Default language | Slovak (README.md), English secondary (README.en.md) |

## Repository Structure

```
kniha-jazd/
├── README.md              # Slovak (primary)
├── README.en.md           # English
├── LICENSE                # GPL v3
├── CHANGELOG.md           # Version history
├── CONTRIBUTING.md        # How to contribute (English)
├── .github/
│   └── workflows/
│       └── release.yml    # Tauri build for Win/Mac
├── docs/
│   └── screenshots/
│       └── hero.png       # Main app screenshot
└── (existing code...)
```

## README Structure (Slovak)

```markdown
🌐 [English](README.en.md) | **Slovensky**

# Kniha Jázd

Desktopová aplikácia na evidenciu jázd služobných vozidiel pre SZČO a malé firmy.
Automaticky počíta spotrebu, sleduje 20% limit nadpotreby a pomáha s daňovou evidenciou.

![Kniha Jázd](docs/screenshots/hero.png)

## Funkcie

- Evidencia jázd s automatickým výpočtom spotreby
- Sledovanie tankovania a zostatku paliva
- Upozornenie pri prekročení 20% limitu nadpotreby
- Návrhy kompenzačných jázd pre dodržanie limitu
- Ročné prehľady (každý rok = samostatná kniha jázd)
- Zálohovanie a obnova databázy
- Export do PDF

## Inštalácia

Download table with links to GitHub Releases for each platform.

## Použitie

Basic usage instructions (add vehicle, record trips, fill-ups, monitor margin).

## Pre vývojárov

Links to English docs, tech stack summary, local dev instructions.

## Licencia

GPL-3.0
```

## GitHub Actions Workflow

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    strategy:
      matrix:
        include:
          - platform: windows-latest
            args: ''
          - platform: macos-latest
            args: '--target aarch64-apple-darwin'
          - platform: macos-latest
            args: '--target x86_64-apple-darwin'

    runs-on: ${{ matrix.platform }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
      - uses: dtolnay/rust-toolchain@stable
      - run: npm install
      - uses: tauri-apps/tauri-action@v0
        with:
          tagName: ${{ github.ref_name }}
          releaseName: 'Kniha Jázd ${{ github.ref_name }}'
          releaseBody: 'See CHANGELOG.md for details.'
          args: ${{ matrix.args }}
```

**Release process:**
1. Update version in `tauri.conf.json` and `Cargo.toml`
2. Update `CHANGELOG.md`
3. Commit and tag: `git tag v0.1.0 && git push --tags`
4. GitHub Actions builds all platforms and creates Release automatically

## CLAUDE.md Addition

Add documentation mandate section requiring:
- Update README.md (Slovak) when features change
- Update README.en.md (English) to mirror
- Update CHANGELOG.md with version notes
- Update screenshots if UI changes significantly

## Task Breakdown

| # | Task | Notes |
|---|------|-------|
| 1 | Create dummy showcase database | Realistic data for screenshot |
| 2 | Take hero screenshot | After dummy DB ready |
| 3 | Write README.md (Slovak) | Main content |
| 4 | Write README.en.md (English) | Translation |
| 5 | Add LICENSE (GPL-3.0) | Standard file |
| 6 | Create CHANGELOG.md | Start with v0.1.0 |
| 7 | Create CONTRIBUTING.md | Basic contribution guide |
| 8 | Add GitHub Actions workflow | release.yml |
| 9 | Update CLAUDE.md | Documentation mandate |
| 10 | Update Cargo.toml metadata | license, repository, description |
| 11 | Update tauri.conf.json | identifier, metadata |
