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
- **Odometer check** - A warning on a row whose odometer does not match the recorded kilometres, and on trips that share a date and time
- **Compensation trip suggestions** - How to get back under the limit
- **Fill-up suggestions** - Automatic calculation of liters needed for optimal consumption
- **Route memory** - Frequent routes auto-complete
- **Yearly overviews** - Each year = separate logbook
- **Column visibility** - Customize the trip grid by hiding/showing columns
- **Backup and restore** - Automatic backup before database migrations, backup management
- **Export** - HTML preview with print-to-PDF (Ctrl+P), respects hidden columns
- **Invoices (Paperless-ngx)** - Invoices are pulled from your Paperless-ngx and assigned to trips; Paperless-ngx is the only invoice source
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

### 5. Invoices (Paperless-ngx)

Invoices come from [Paperless-ngx](https://docs.paperless-ngx.com/). Paperless-ngx is
the only invoice source: it OCRs the document on its own server, and the app fetches it,
shows it, and assigns it to a trip.

#### Setup

1. In Paperless-ngx, tag documents with `fuel` (fill-ups) and `car` (other costs), and
   create the custom fields `total_amount` (EUR), `litres` (fuel only), and
   `receipt_datetime` (ISO-8601 date and time).
2. In the app, open Settings -> Paperless-ngx and enter the instance URL and API token.

   > **Alternative:** the `PAPERLESS_URL`, `PAPERLESS_API_TOKEN` and `PAPERLESS_ENABLED`
   > environment variables on the container take precedence over the stored setting.

3. The Doklady page loads the invoices for the selected vehicle and year.
4. Use "Priradiť k jazde" on a row to assign the invoice to a trip.

The "Otvoriť v Paperless" button opens the document in a new browser tab.

> **Important before you upgrade:** this release removes the local receipt scanning and
> the upgrade drops the `receipts` table. If the database still holds local receipts you
> need, run `scripts/migrate_local_to_paperless.py` BEFORE upgrading.

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

**Paperless invoices not showing?**
1. Check the Paperless URL and API token (the `PAPERLESS_URL`, `PAPERLESS_API_TOKEN`,
   `PAPERLESS_ENABLED` env vars or `local.settings.json`)
2. Verify the documents carry the `fuel` or `car` tag and the `total_amount`, `litres`,
   `receipt_datetime` custom fields
3. Check the connection status under Settings -> Paperless-ngx

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

All data stays on your server. The server has no authentication and is meant for a trusted local network only (CORS allows private IP ranges). Do not expose it to the internet. The app now talks only to your Paperless-ngx and your Home Assistant, both of which you host yourself.

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
