# Task: Multi-Currency Receipt Support

## Problem

Receipts from Czech Republic, Hungary, and Poland are scanned with amounts in local currency (CZK, HUF, PLN). The OCR currently assumes EUR, causing:
- Incorrect amounts (100 CZK shown as 100 EUR)
- Receipt-to-trip matching failures
- No way for user to correct the currency

## Goal

Allow receipts to store original currency + amount, with user-provided EUR conversion for accounting.

## Acceptance Criteria

- [ ] OCR detects currency symbols (€, Kč, Ft, zł) and returns currency code
- [ ] Receipt stores `original_amount`, `original_currency`, and `total_price_eur` separately
- [ ] EUR receipts: `total_price_eur` auto-populated from `original_amount`
- [ ] Foreign currency receipts: `total_price_eur` = None until user converts
- [ ] Doklady view shows "100 CZK → 3,95 €" format for converted receipts
- [ ] Receipt edit modal allows setting currency and EUR amount
- [ ] Matching logic unchanged (uses `total_price_eur`)

## Supported Currencies

- EUR (Euro)
- CZK (Czech koruna)
- HUF (Hungarian forint)
- PLN (Polish złoty)

## References

- Design: `01-design.md`
- Plan: `02-plan.md`
