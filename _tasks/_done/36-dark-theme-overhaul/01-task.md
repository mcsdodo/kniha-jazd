**Date:** 2026-01-13
**Subject:** Dark Theme Styling Overhaul - Muted/Subtle Approach
**Status:** Planning

## Goal

Comprehensive overhaul of dark theme styling to fix light blue/green elements that don't work well on dark backgrounds. Implement a "muted/subtle" visual style with dark tinted backgrounds and bright text for all accent colors.

## Problem

Current dark theme has several elements with hardcoded light colors that look jarring:
- Vehicle type badges (ICE/BEV/PHEV) use light backgrounds (#e3f2fd, #e8f5e9, #fff3e0)
- Filter buttons don't adapt properly to dark theme
- Some button variants use light backgrounds that clash with dark surfaces

## Chosen Approach: Muted/Subtle Style

After reviewing options with user, selected **Option 1: Muted/Subtle**:
- Dark tinted backgrounds (e.g., dark blue `#1a3a5c`) with bright text (`#5dade2`)
- Professional, calm, easy on the eyes for long sessions
- Better contrast ratios for accessibility (WCAG compliant)

Demo file created: `_dark-theme-demo.html` (can be deleted after implementation)

## Requirements

1. **Add missing CSS variables** for dark theme variants of:
   - Vehicle type badges (ICE blue, BEV green, PHEV orange)
   - ~~Filter button active states~~ ✅ Already use CSS variables (`--accent-primary`)
   - Any other hardcoded light colors

2. **Update all components** to use CSS variables instead of hardcoded colors

3. **Color palette** (from research - WCAG compliant):
   | Color | Bright (text) | Muted (bg) |
   |-------|---------------|------------|
   | Blue  | #5dade2       | #1a3a5c    |
   | Green | #58d68d       | #1e3a2a    |
   | Orange| #f5b041       | #3d3020    |
   | Yellow| #f4d03f       | #3d3520    |
   | Red   | #ec7063       | #3d2020    |

4. **Maintain light theme** - changes should only affect dark mode

## Technical Notes

- All theme colors are in `src/lib/theme.css`
- Theme switching via `[data-theme="dark"]` CSS selector
- Some badge colors are hardcoded in `settings/+page.svelte` - need to move to CSS vars
- Filter buttons in layout need text color fixes
