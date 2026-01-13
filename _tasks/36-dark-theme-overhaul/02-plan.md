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

## Task 3: Audit and Fix Any Remaining Hardcoded Colors

**Files:**
- Search: All `.svelte` files for hardcoded hex colors
- Potentially modify: Any files with hardcoded light theme colors

**Steps:**
1. Run grep for common light-theme hex colors:
   ```
   #e3f2fd, #e8f5e9, #fff3e0, #d4edda, #f8d7da, #d1ecf1
   ```
2. For each occurrence, determine if it needs a CSS variable
3. Add variables if needed and update component

**Verification:** No hardcoded light-only colors remain in dark-mode-affected areas

---

## Task 4: Improve Editing Row Background Contrast

**Files:**
- Modify: `src/lib/theme.css`

**Steps:**
1. The current `--editing-row-bg` in dark mode is `#1a3a4a` - this is good
2. Verify contrast is sufficient (should be visible but not jarring)
3. If needed, adjust to slightly brighter: `#1e4050`

**Verification:** Edit a trip row in dark mode - editing state should be clearly visible but not harsh

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
