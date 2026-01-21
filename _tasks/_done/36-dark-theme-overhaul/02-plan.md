# Dark Theme Styling Overhaul - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix dark theme styling issues by implementing muted/subtle color approach for all accent elements

**Architecture:** Add new CSS variables for vehicle type badges and update components to use variables instead of hardcoded colors

---

## Task 1: Add Vehicle Type Badge CSS Variables

**Files:**
- Modify: `src/lib/theme.css`

**Steps:**
1. Add new CSS variables to `:root` (light theme):
   ```css
   /* Vehicle type badges */
   --badge-ice-bg: #e3f2fd;
   --badge-ice-color: #1565c0;
   --badge-bev-bg: #e8f5e9;
   --badge-bev-color: #2e7d32;
   --badge-phev-bg: #fff3e0;
   --badge-phev-color: #e65100;
   ```

2. Add dark theme variants to `[data-theme="dark"]`:
   ```css
   /* Vehicle type badges - muted style */
   --badge-ice-bg: #1a3a5c;
   --badge-ice-color: #5dade2;
   --badge-bev-bg: #1e3a2a;
   --badge-bev-color: #58d68d;
   --badge-phev-bg: #3d3020;
   --badge-phev-color: #f5b041;
   ```

**Verification:** CSS variables appear in both `:root` and `[data-theme="dark"]` sections

---

## Task 2: Update Settings Page Badge Styles

**Files:**
- Modify: `src/routes/settings/+page.svelte`

**Steps:**
1. Find `.badge.type-ice`, `.badge.type-bev`, `.badge.type-phev` styles (lines 674-687)
2. Replace hardcoded colors with CSS variables:
   ```css
   .badge.type-ice {
       background-color: var(--badge-ice-bg);
       color: var(--badge-ice-color);
   }
   .badge.type-bev {
       background-color: var(--badge-bev-bg);
       color: var(--badge-bev-color);
   }
   .badge.type-phev {
       background-color: var(--badge-phev-bg);
       color: var(--badge-phev-color);
   }
   ```

**Verification:** Open Settings page in dark mode, vehicle type badges should show muted colors

---

## Task 3: Fix Confidence Indicator Colors

**Files:**
- Modify: `src/routes/doklady/+page.svelte` (lines 870, 874)

**Context:** Review found hardcoded colors `#f39c12` (medium) and `#e74c3c` (low) for confidence indicators. These are orange/red colors that work acceptably in both themes.

**Steps:**
1. Review `.confidence-medium` (line 870) - uses `#f39c12`
2. Review `.confidence-low` (line 874) - uses `#e74c3c`
3. **Decision:** These colors are already dark-mode compatible (not light backgrounds). Keep as-is OR optionally replace with existing CSS variables:
   - `#f39c12` → `var(--accent-warning)` or keep
   - `#e74c3c` → `var(--accent-danger)` or keep
4. No light-theme-only colors (#e3f2fd, #e8f5e9, etc.) found elsewhere

**Verification:** Confidence indicators visible and readable in both light and dark mode

---

## Task 4: Verify Editing Row Background Contrast ✅ SKIP

**Status:** Already implemented - no action needed

**Files:**
- Verify only: `src/lib/theme.css`

**Review Finding:** Current `--editing-row-bg: #1a3a4a` is already appropriate. Verified during plan review.

**Verification:** Edit a trip row in dark mode - editing state is clearly visible ✓

---

## Task 5: Visual Testing Across All Components

**Files:**
- Test: All main pages in dark mode

**Steps:**
1. Run app in dev mode: `npm run tauri dev`
2. Switch to dark theme via Settings
3. Check each page:
   - [ ] Home page (TripGrid) - row colors, buttons, editing state
   - [ ] Settings page - vehicle badges, active states, form inputs
   - [ ] Doklady (Receipts) page - if any badges exist
4. Document any remaining issues

**Verification:** All UI elements look professional in dark mode with no jarring bright colors

---

## Task 6: Cleanup Demo File

**Files:**
- Delete: `_dark-theme-demo.html`

**Steps:**
1. Remove the temporary demo file created during brainstorming:
   ```bash
   rm _dark-theme-demo.html
   ```

**Verification:** File is removed from project root

---

## Task 7: Update Changelog

**Files:**
- Modify: `CHANGELOG.md`

**Steps:**
1. Run `/changelog` skill to add entry under [Unreleased]
2. Category: Changed/Fixed
3. Description: Improved dark theme styling with muted accent colors for vehicle type badges

**Verification:** CHANGELOG.md contains entry for dark theme improvements
