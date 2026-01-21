# Integration Test Data

Test fixtures for receipt/invoice integration tests.

## Directory Structure

```
data/
├── invoices/       # Receipt files to be scanned (PDF, JPG, PNG)
│   └── invoice.pdf # Real Slovnaft fuel receipt
├── mocks/          # Mock Gemini API responses (JSON)
│   └── invoice.json
└── README.md
```

## How Mock Mode Works

When `KNIHA_JAZD_MOCK_GEMINI_DIR` is set, the Gemini client loads mock JSON
instead of calling the API:

1. Receipt file: `invoices/invoice.pdf`
2. Mock file: `mocks/invoice.json` (matched by filename stem)
3. Result: `load_mock_extraction()` returns the JSON data

## Mock JSON Schema

Must match `ExtractedReceipt` struct in `src-tauri/src/gemini.rs`:

```json
{
    "liters": 63.68,
    "total_price_eur": 91.32,
    "receipt_date": "2026-01-20",
    "station_name": "Slovnaft, a.s.",
    "station_address": "Prístavná ulica, Bratislava",
    "vendor_name": null,
    "cost_description": null,
    "raw_text": null,
    "confidence": {
        "liters": "high",
        "total_price": "high",
        "date": "high"
    }
}
```

## Adding New Test Fixtures

1. Add receipt file to `invoices/` (e.g., `car-wash.jpg`)
2. Add mock JSON to `mocks/` with same stem (e.g., `car-wash.json`)
3. Set confidence levels appropriately for the test scenario

## Test Scenarios

| File | Purpose | Key Values |
|------|---------|------------|
| `invoice.pdf` + `invoice.json` | Standard fuel receipt | 63.68L, €91.32, 2026-01-20 |
