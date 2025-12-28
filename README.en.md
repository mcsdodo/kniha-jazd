**English** | [Slovensky](README.md)

# Kniha Jázd (Vehicle Logbook)

Desktop application for tracking business vehicle trips for Slovak sole proprietors and small businesses.
Automatically calculates fuel consumption, monitors the legal 20% over-consumption limit, and helps with tax records.

![Kniha Jázd - Main Screen](docs/screenshots/hero.png)

## Features

- **Trip logging** - Record date, route, km, and purpose of each trip
- **Automatic consumption calculation** - l/100km calculated automatically on fill-up
- **Fuel remaining tracking** - Tank balance after each trip
- **20% limit monitoring** - Warning when exceeding the legal over-consumption limit
- **Compensation trip suggestions** - How to get back under the limit
- **Route memory** - Frequent routes auto-complete
- **Yearly overviews** - Each year = separate logbook
- **Backup and restore** - Simple database management
- **Export** - HTML preview with print-to-PDF (Ctrl+P)

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
- Date
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

## For Developers

### Tech Stack

- **Frontend:** SvelteKit + TypeScript
- **Backend:** Tauri (Rust)
- **Database:** SQLite

### Architecture

All business logic lives in the Rust backend (see [DECISIONS.md](DECISIONS.md) ADR-008):
- `src-tauri/src/calculations.rs` - Core consumption calculations
- `src-tauri/src/suggestions.rs` - Compensation trip logic
- Frontend is display-only, calls Tauri commands

### Local Development

```bash
# Install dependencies
npm install

# Run in development mode
npm run tauri dev
```

### Running Tests

```bash
# Rust backend tests (61 tests)
cd src-tauri && cargo test
```

### Building

```bash
npm run tauri build
```

## License

[GPL-3.0](LICENSE)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
