# Review: export-system.md

## Convention Compliance

The document follows the `docs/CLAUDE.md` template structure well:
- User Flow section present
- Technical Implementation section present
- Key Files table present
- Design Decisions section present (excellent rationale explanations)

**Overall:** Mostly compliant, with two code embedding violations.

## Issues Found

### Issue 1: Embedded Rust Struct Definition (Lines 25-38)

**Location:** "Export Totals Calculation" section

**Problem:** The document embeds a Rust-like struct definition:
```rust
ExportTotals {
    total_km,           // Sum of all trip distances
    total_fuel_liters,  // Sum of fuel fillups
    total_fuel_cost,    // Sum of fuel costs
    ...
}
```

**Convention violated:** "Reference code with `file.rs:L###` pointers, don't embed full implementations"

**Why it's a problem:** If `ExportTotals` fields change (e.g., adding PHEV fields, renaming fields), the doc becomes stale. The actual struct is at `export.rs:L71-84`.

**Recommended fix:** Replace with:
```markdown
The `ExportTotals::calculate()` function (`export.rs:L96`) processes trip data to produce footer statistics. The struct is defined at `export.rs:L71-84`.

Key fields:
- **Fuel totals:** `total_km`, `total_fuel_liters`, `total_fuel_cost`, `avg_consumption`, `deviation_percent`
- **Energy totals (BEV/PHEV):** `total_energy_kwh`, `total_energy_cost`, `avg_energy_rate`, `energy_deviation_percent`
- **Common:** `total_other_costs`
```

### Issue 2: Embedded TypeScript Translation Example (Lines 110-118)

**Location:** "Internationalization" section

**Problem:** The document embeds TypeScript code:
```typescript
export: {
    pageTitle: 'KNIHA JAZD',
    headerCompany: 'Firma:',
    footerDeviation: 'Odchylka od TP',
    printHint: 'Pre export do PDF pouzite Ctrl+P -> Ulozit ako PDF',
    // ...
}
```

**Convention violated:** "Avoid documentation drift by not duplicating code"

**Why it's a problem:** Translation strings are frequently updated. The exact values here may already be stale.

**Recommended fix:** Replace with:
```markdown
**Translation example:** See Slovak translations in `src/lib/i18n/sk/index.ts` (search for `export:` key). Labels cover page title, headers, column names, footer labels, and print hints.
```

## Compliant Sections (Good Examples)

These sections correctly avoid code embedding:

1. **Key behavior bullets (L42-45)** - Describes behavior in prose, not code
2. **Vehicle-Type Templates table (L78-82)** - Documents what columns appear per type
3. **Export Command Flow (L61-73)** - Lists steps without implementation details
4. **Key Files table (L120-130)** - References files with descriptions, no code
5. **Design Decisions (L132-175)** - Explains "why" with rationale, no code

## Recommendations

| Priority | Section | Action |
|----------|---------|--------|
| Medium | Export Totals Calculation (L25-38) | Replace struct with prose + `export.rs:L71-84` reference |
| Low | Internationalization (L110-118) | Replace TypeScript with file reference |

**Severity:** Low-Medium. The embedded snippets are small and somewhat stable, but create maintenance burden. The struct fields are unlikely to change often, but translation strings do change.

## Summary

The document is well-written and follows most conventions. The two code snippets should be converted to file references to prevent documentation drift. The Design Decisions section is particularly good - it explains rationale without code duplication.
