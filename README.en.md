**English** | [Slovensky](README.md)

[![Tests](https://github.com/mcsdodo/kniha-jazd/actions/workflows/test.yml/badge.svg)](https://github.com/mcsdodo/kniha-jazd/actions/workflows/test.yml)

# Kniha Jázd (Vehicle Logbook)

Application for tracking business vehicle trips for Slovak sole proprietors and small businesses.
Automatically calculates fuel consumption, monitors the legal 20% over-consumption limit, and helps with tax records.

It runs as a Docker container on your own server (NAS, Raspberry Pi, homelab) and you
use it from a browser on any device on the local network. The desktop build is
discontinued — existing installs keep working but receive no further updates.

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
- **Backup and restore** - Automatic backup before database migrations, backup management
- **Export** - HTML preview with print-to-PDF (Ctrl+P), respects hidden columns
- **Receipts (AI OCR)** - Automatic recognition of gas station receipts with multi-currency support (EUR, CZK, HUF, PLN)
- **Home Assistant integration** - Display ODO and fuel level from HA, push suggested fill-up to HA sensor
- **Browser access** - Phone, tablet and desktop all reach the same instance over the local network
- **Docker deployment** - One container, one `/data` volume, for always-on devices (NAS, Raspberry Pi). See [docs/features/server-mode.md](docs/features/server-mode.md) for details.

## Installation

The app is distributed as a Docker image. No installers are published.

```bash
mkdir -p data
docker run -d --name kniha-jazd \
  -p 3456:3456 \
  -v "$PWD/data:/data" \
  --restart unless-stopped \
  ghcr.io/mcsdodo/kniha-jazd-web:latest
```

The app is then at `http://<server-ip>:3456`.

To build the image from source instead, [docker-compose.web.yml](docker-compose.web.yml)
builds it from [Dockerfile.web](Dockerfile.web):

```bash
docker compose -f docker-compose.web.yml up -d
```

### Image channels

| Tag | What it is |
|-----|------------|
| `:latest` | Last released version — use this by default |
| `:vX.Y.Z` | A specific release, never moves |
| `:main` | Tip of the `main` branch, updated after every green build |
| `:main-<sha>` | One specific commit from `main`, never moves |

`:main` is for trying changes before they are released — everything on it passed the
full test suite, but it is not a release. If something breaks, fall back to `:latest`
or to a specific `:main-<sha>`.

Updating = pull a newer tag and restart the container. The database in `/data` stays
put; migrations run automatically on start.

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

   > **Alternative:** the `GEMINI_API_KEY` environment variable on the container (it
   > takes precedence over the stored setting), or manual configuration in
   > `local.settings.json` inside the data directory (`/data/local.settings.json` in
   > the container):
   > ```json
   > {
   >   "gemini_api_key": "AIza...",
   >   "receipts_folder_path": "/data/receipts"
   > }
   > ```

   The receipts folder is a path **on the server** (inside the container), not on your
   own machine — type it in, e.g. `/data/receipts`, and mount it into the container.

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
In a SQLite database on the container's `/data` volume — in a typical deployment that
is the `./data` folder on the host:
- Database: `/data/kniha-jazd.db`
- Backups: `/data/backups/`
- Settings: `/data/local.settings.json`

**Fuel remaining shows negative value?**
Remaining fuel is calculated from filled liters minus consumption. If negative, check:
- Whether you entered correct km
- Whether you recorded all fill-ups

**Receipt recognition not working?**
1. Verify your Gemini API key (`GEMINI_API_KEY` env var or `local.settings.json`)
2. Check that the receipts folder exists **inside the container**
3. Supported formats: JPG, PNG, WebP, PDF

**How to move data to another server?**

*Via the folder:* stop the container and copy the whole `./data` directory. It holds
the database, the backups and the settings.

*Via backup:*
1. Create a backup in Settings
2. Copy the `.backup` file into `data/backups/` on the new server
3. Restore from backup in Settings

Exactly one instance opens the database — do not point two containers at the same
`/data` directory.

## Privacy

All data stays on your server. The server has no authentication and is meant for a trusted local network only (CORS allows private IP ranges) — do not expose it to the internet. The only external connection is when using AI receipt recognition - receipt images are sent to the Gemini API (Google). This feature is optional.

## For Developers

### Tech Stack

- **Frontend:** SvelteKit + TypeScript (static SPA)
- **Backend:** Rust — `kniha-jazd-core` (logic) + `kniha-jazd-web` (Axum HTTP server)
- **Database:** SQLite
- **Deployment:** Docker image `ghcr.io/mcsdodo/kniha-jazd-web`

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

Two processes in two terminals:

```bash
# Install dependencies
npm install

# 1) backend on port 3456 (leave STATIC_DIR unset - vite serves the SPA)
cargo run --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web

# 2) frontend on port 5173, proxying /api to localhost:3456
npm run dev
```

### Running Tests

```bash
npm run test:backend      # Rust tests (whole workspace)

# Integration tests need the built SPA and the debug binary
npm run build
cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
npm run test:integration
```

### Building

```bash
docker build -f Dockerfile.web -t kniha-jazd-web:local .
```

## License

[GPL-3.0](LICENSE)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
