**English** | [Slovensky](README.md)

[![Tests](https://github.com/mcsdodo/kniha-jazd/actions/workflows/test.yml/badge.svg)](https://github.com/mcsdodo/kniha-jazd/actions/workflows/test.yml)

# Kniha Jázd (Vehicle Logbook)

Desktop application for tracking business vehicle trips for Slovak sole proprietors and small businesses.
Automatically calculates fuel consumption, monitors the legal 20% over-consumption limit, and helps with tax records.

![Kniha Jázd - Main Screen](docs/screenshots/hero.png)

## Features

- **Trip logging** - Record date/time, route, km, and purpose of each trip
- **Legal compliance (from 1.1.2026)** - Trip numbering, driver name, end time, km before trip, month-end rows
- **Automatic consumption calculation** - l/100km calculated automatically on fill-up
- **Fuel remaining tracking** - Tank balance after each trip
- **20% limit monitoring** - Warning when exceeding the legal over-consumption limit
- **Compensation trip suggestions** - How to get back under the limit
- **Fill-up suggestions** - Automatic calculation of liters needed for optimal consumption
- **Route memory** - Frequent routes auto-complete
- **Yearly overviews** - Each year = separate logbook
- **Column visibility** - Customize the trip grid by hiding/showing columns
- **Backup and restore** - Automatic backup before updates, backup management
- **Database relocation** - Custom database location (Google Drive, NAS) for multi-PC access
- **Export** - HTML preview with print-to-PDF (Ctrl+P), respects hidden columns
- **Receipts (AI OCR)** - Automatic recognition of gas station receipts with multi-currency support (EUR, CZK, HUF, PLN)
- **Home Assistant integration** - Display ODO and fuel level from HA, push suggested fill-up to HA sensor

## Installation

Download the latest version for your system from [Releases](../../releases):

| Platform | File |
|----------|------|
| Windows | `Kniha-Jazd_x.x.x_x64-setup.msi` |
| macOS (Apple Silicon) | `Kniha-Jazd_x.x.x_aarch64.dmg` |
| macOS (Intel) | `Kniha-Jazd_x.x.x_x64.dmg` |

## Usage

### 1. Add a Vehicle

In settings, add a vehicle with:
- Name and license plate
- Tank size (liters)
- TP consumption (l/100km from technical passport)
- Initial odometer reading

### 2. Record a Trip

For each trip enter:
- Start/end date and time
- Origin - Destination
- Kilometers (or calculated from ODO)
- Purpose

### 3. Fill-ups

When refueling enter:
- Liters filled
- Cost (optional)
- Whether it was a full tank

The app calculates consumption automatically.

### 4. Monitor the Limit

- Margin under 20% = OK
- Margin over 20% = warning + compensation trip suggestions

### 5. Receipts (AI OCR Recognition)

The app supports automatic recognition of gas station receipts using AI (Gemini).
Supported currencies: EUR, CZK, HUF, PLN (foreign currencies require manual EUR conversion).

#### Setup

1. **Get a Gemini API key:**
   - Visit [Google AI Studio](https://aistudio.google.com/apikey)
   - Create a new API key (free tier is sufficient for typical usage)

2. **Configure in the app** under Settings → Receipt Scanning:
   - Enter your Gemini API key
   - Select the receipts folder

   > **Alternative:** Manual configuration via `local.settings.json`:
   > - Windows: `%APPDATA%\com.notavailable.kniha-jazd\local.settings.json`
   > - macOS: `~/Library/Application Support/com.notavailable.kniha-jazd/local.settings.json`
   > ```json
   > {
   >   "gemini_api_key": "AIza...",
   >   "receipts_folder_path": "C:\\Path\\To\\Receipts"
   > }
   > ```

#### Receipt Folder Structure

The app supports two ways to organize receipts:

**Flat structure** - all files directly in the folder:
```
/receipts/
  receipt1.jpg
  receipt2.png
```
→ Receipts are shown in all years

**Year-based structure** - files in year subfolders:
```
/receipts/
  2024/
    receipt1.jpg
  2025/
    receipt2.png
```
→ Receipts are filtered by selected year

**Notes:**
- Mixed structure (files + folders) shows a warning and receipts won't load
- OCR date takes priority over folder year (helps identify misfiled receipts)

#### Usage

1. Save receipt photos to the configured folder
2. Open the "Doklady" section and click "Sync"
3. AI will recognize date, liters, and total amount
4. Assign receipts to trips

## FAQ

**Where is my data stored?**
All data is stored locally in a SQLite database:
- Windows: `%APPDATA%\com.notavailable.kniha-jazd\kniha-jazd.db`
- macOS: `~/Library/Application Support/com.notavailable.kniha-jazd/kniha-jazd.db`

**Fuel remaining shows negative value?**
Remaining fuel is calculated from filled liters minus consumption. If negative, check:
- Whether you entered correct km
- Whether you recorded all fill-ups

**Receipt recognition not working?**
1. Verify your Gemini API key in `local.settings.json`
2. Check that the receipts folder exists
3. Supported formats: JPG, PNG, WebP, PDF

**How to transfer data to a new computer?**

*Via backup:*
1. Create a backup in Settings
2. Copy the `.backup` file to the new computer
3. Restore from backup in Settings

*Via shared storage:*
In Settings → Database Location, move the database to shared storage (Google Drive, NAS). A lock file prevents simultaneous access from multiple computers.

## Privacy

All data stays on your computer. The only external connection is when using AI receipt recognition - receipt images are sent to the Gemini API (Google). This feature is optional.

## For Developers

### Tech Stack

- **Frontend:** SvelteKit + TypeScript
- **Backend:** Tauri (Rust)
- **Database:** SQLite

### Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed architecture documentation.

For individual feature implementation docs, see [docs/features/](docs/features/).

**Key principle:** All business logic lives in the Rust backend (ADR-008). Frontend is display-only.

### Local Development

#### macOS: Install Rust

Before running the app locally on macOS, you need to install Rust:

```bash
# Install Rust (official method for macOS)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# After installation, restart terminal or run:
source "$HOME/.cargo/env"

# Verify it works:
cargo --version
```

#### Run the App

```bash
# Install dependencies
npm install

# Run in development mode
npm run tauri dev
```

### Running Tests

```bash
# Rust backend tests (257 tests)
cd src-tauri && cargo test

# E2E integration tests
npm run test:integration:build
```

### Building

```bash
npm run tauri build
```

## License

[GPL-3.0](LICENSE)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
