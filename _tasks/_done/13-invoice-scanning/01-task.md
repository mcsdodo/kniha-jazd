# Task: Invoice/Receipt Scanning Feature

## Summary

Add ability to scan fuel receipts (bločky) from a watched folder, parse them using Gemini API, and assign parsed data to trips instead of manual entry.

## User Requirements

- Scan paper receipts (bločky) from up to 5 gas stations
- Extract: liters, total price, station address (for cross-checking)
- Smart queue UX: when adding fill-up trip, show relevant unassigned receipts filtered by date
- Gemini 2.5 Flash Lite for OCR/extraction (user provides API key)
- "Doklady" page for full receipt management + floating indicator
- Handle parsing errors gracefully: best effort + flag uncertain fields + mark for review
- Per-station fine-tuning with learned patterns
- E2E testing with Playwright

## Development Setup

- Test receipts folder: `C:\_dev\_tmp\doklady`
- Local settings override file (gitignored) for API key and folder path
- Override file takes priority over DB settings
