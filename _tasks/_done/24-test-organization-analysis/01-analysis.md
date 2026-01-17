# Rust Test Organization Analysis

**Date:** 2026-01-06
**Subject:** Analysis of test organization patterns for growing Rust codebase
**Status:** Complete

## Executive Summary

This document analyzes best practices for organizing Rust tests when files become too long, specifically for the `kniha-jazd` codebase. The research concludes that **both approaches are valid** - inline tests are idiomatic Rust, but separating tests into companion files is an established pattern for larger codebases.

**Recommendation:** Use a **hybrid approach** - keep inline tests for smaller files, but split tests into separate files for modules where tests exceed 40-50% of the file or where test code exceeds 300-400 lines.

---

## Current Codebase Analysis

### File Statistics

| File | Total Lines | Impl Lines | Test Lines | Test % | Test Count |
|------|-------------|------------|------------|--------|------------|
| `calculations.rs` | 611 | 98 | 513 | **83%** | 28 |
| `commands.rs` | 2408 | 1690 | 718 | 29% | 25 |
| `db.rs` | 1621 | 995 | 626 | 38% | 17 |
| `suggestions.rs` | 289 | 81 | 208 | **71%** | 8 |
| `receipts.rs` | 579 | 266 | 313 | **54%** | 17 |
| `export.rs` | 556 | 436 | 120 | 21% | 7 |
| `gemini.rs` | 287 | 227 | 60 | 20% | 3 |
| `settings.rs` | 68 | 29 | 39 | 57% | 3 |

**Total: 108 tests across 8 files**

### Critical Files for Splitting

Based on test-to-implementation ratio:

1. **`calculations.rs` (83% tests)** - Most critical. Only 98 lines of implementation but 513 lines of tests. Perfect candidate for split.

2. **`suggestions.rs` (71% tests)** - 81 lines impl vs 208 lines tests. Good candidate.

3. **`receipts.rs` (54% tests)** - 266 lines impl vs 313 lines tests. Borderline.

4. **`db.rs` (38% tests)** - Large file overall (1621 lines), but implementation dominates. Could be split for navigation, but not critical.

5. **`commands.rs` (29% tests)** - Largest file (2408 lines), but mostly implementation. Test ratio is reasonable.

---

## Research Findings

### The Idiomatic Rust Approach: Inline Tests

The official Rust documentation recommends placing unit tests in the same file:

> "You'll put unit tests in the src directory in each file with the code that they're testing. The convention is that you create a module named tests in each file to contain the test functions and to annotate the module with cfg(test)."
> — [The Rust Programming Language Book](https://doc.rust-lang.org/book/ch11-03-test-organization.html)

**Benefits:**
- Tests close to implementation code
- Private function access via `use super::*`
- Single file to understand a module
- IDE support (go-to-test, run test at cursor)

**Drawbacks:**
- Files can become unwieldy (doubling/tripling in size)
- Hard to navigate between impl and test code
- Git history mixes implementation and test changes
- Side-by-side comparison impractical

### The Split Files Approach

Several patterns exist for separating tests while maintaining private access:

#### Pattern 1: `#[path]` Attribute (Recommended)

```rust
// src/calculations.rs
pub fn calculate_consumption_rate(liters: f64, km: f64) -> f64 {
    // implementation
}

#[cfg(test)]
#[path = "calculations_tests.rs"]
mod tests;
```

```rust
// src/calculations_tests.rs
use super::*;

#[test]
fn test_consumption_rate_normal() {
    assert!((calculate_consumption_rate(50.0, 820.0) - 6.0975).abs() < 0.001);
}
```

**Key insight:** The test file is a submodule of the source file, so `use super::*` gives access to private items.

**Sources:**
- [Better Programming: How to Structure Unit Tests in Rust](https://betterprogramming.pub/how-to-structure-unit-tests-in-rust-cc4945536a32)
- [Rust Forum: Real world tips for organising unit-tests](https://users.rust-lang.org/t/real-world-tips-for-organising-unit-tests-for-larger-projects-and-files/130749)

#### Pattern 2: Tests Subdirectory

```
src/
├── calculations.rs
├── lib.rs
└── tests/
    ├── mod.rs
    └── calculations_tests.rs
```

```rust
// src/lib.rs
#[cfg(test)]
mod tests;

pub mod calculations;
```

```rust
// src/tests/mod.rs
pub mod calculations_tests;
```

**Trade-off:** More complex module structure, but cleaner separation.

**Source:** [Walk N' Squawk: Rust unit test layout](https://www.walknsqualk.com/020-rust-unit-test-layout/)

#### Pattern 3: Module Directory with Tests

```
src/
├── calculations/
│   ├── mod.rs          # Implementation
│   └── tests.rs        # Tests
└── lib.rs
```

```rust
// src/calculations/mod.rs
pub fn calculate_consumption_rate(...) { ... }

#[cfg(test)]
mod tests;
```

```rust
// src/calculations/tests.rs
use super::*;
// tests here
```

**Sources:**
- [Sling Academy: Organizing Rust Test Files](https://www.slingacademy.com/article/organizing-rust-test-files-and-modules-for-clarity-and-maintainability/)
- [LogRocket: How to organize Rust tests](https://blog.logrocket.com/how-to-organize-rust-tests/)

### Integration Tests (tests/ directory)

For public API testing, Rust uses a top-level `tests/` directory:

```
kniha-jazd/
├── src-tauri/
│   ├── src/
│   └── tests/          # Integration tests (public API only)
│       └── integration_test.rs
```

**Important:** Files in `tests/` are separate crates - they can only access **public** API, not private internals.

---

## Community Consensus

From Rust community discussions:

> "The convention is to put the unit tests in the same file as the source code in a mod to keep tests close to the code. However, for real life code (with large files and complex folder structures), it makes source files quite large doubling or tripling the lines of code and makes it hard to navigate."
> — [Rust Forum Discussion](https://users.rust-lang.org/t/should-unit-tests-really-be-put-in-the-same-file-as-the-source/62153)

> "Several experienced developers favor separating test code into distinct files... Easier navigation between code and tests during development, side-by-side viewing becomes practical."
> — [Rust Forum: Real world tips](https://users.rust-lang.org/t/real-world-tips-for-organising-unit-tests-for-larger-projects-and-files/130749)

**Consensus:** No single "best" approach exists. For larger codebases, separation into dedicated files is a valid and common practice.

---

## Recommendations for kniha-jazd

### Decision Matrix

| Criterion | Inline Tests | Separate Test Files |
|-----------|-------------|---------------------|
| Test > 50% of file | ❌ Poor | ✅ Good |
| Test < 30% of file | ✅ Good | ⚠️ Overhead |
| Total file > 500 LOC | ⚠️ Consider split | ✅ Good |
| Test helpers reused | ⚠️ Duplication | ✅ Centralized |
| Frequent test changes | ❌ Noisy git history | ✅ Isolated changes |
| IDE navigation | ✅ Same file | ⚠️ Jump between files |

### Recommended Structure

```
src-tauri/src/
├── calculations.rs           # 98 lines (impl only)
├── calculations_tests.rs     # 513 lines (tests)
├── suggestions.rs            # 81 lines (impl only)
├── suggestions_tests.rs      # 208 lines (tests)
├── receipts.rs               # 266 lines (impl only)
├── receipts_tests.rs         # 313 lines (tests)
├── commands.rs               # Keep inline (tests are 29%)
├── db.rs                     # Keep inline (tests are 38%)
├── export.rs                 # Keep inline (tests are 21%)
├── gemini.rs                 # Keep inline (tests are 20%)
├── settings.rs               # Keep inline (small file)
└── test_helpers.rs           # Optional: shared test utilities
```

### Implementation Priority

1. **High Priority:** `calculations.rs` - Core business logic, 83% tests
2. **Medium Priority:** `suggestions.rs` - 71% tests, moderate size
3. **Low Priority:** `receipts.rs` - 54% tests, could wait

### Migration Steps (Per File)

1. Create `{module}_tests.rs` file
2. Move test code from `#[cfg(test)] mod tests { ... }` to new file
3. Add `use super::*;` at top of test file
4. Replace inline test module with:
   ```rust
   #[cfg(test)]
   #[path = "{module}_tests.rs"]
   mod tests;
   ```
5. Run `cargo test` to verify
6. Commit separately for clean git history

---

## Alternative: Partial Split with Test Helpers

If full separation feels like overkill, consider:

```rust
// src/calculations.rs
pub fn calculate_consumption_rate(...) { ... }

#[cfg(test)]
mod tests {
    use super::*;

    // Core tests stay inline (10-15 most important)
    #[test]
    fn test_consumption_rate_basic() { ... }

    // Include extended tests from separate file
    #[path = "calculations_tests_extended.rs"]
    mod extended;
}
```

This keeps essential tests visible while moving extensive edge-case testing to a separate file.

---

## Test Helpers Consolidation

Currently, each test module has its own helper functions (e.g., `make_trip_with_fuel` in `commands.rs`). Consider:

```rust
// src/test_helpers.rs (new file)
#[cfg(test)]
pub mod helpers {
    use crate::models::{Trip, Receipt, Vehicle};

    pub fn make_test_trip(...) -> Trip { ... }
    pub fn make_test_receipt(...) -> Receipt { ... }
    pub fn make_test_vehicle(...) -> Vehicle { ... }
}
```

Usage in test files:
```rust
#[cfg(test)]
use crate::test_helpers::helpers::*;
```

**Benefits:**
- DRY test setup code
- Consistent test fixtures
- Easier to maintain

**Trade-off:**
- More coupling between tests
- Slightly more complex imports

---

## Decision Required

Before implementing, decide:

1. **Split now or later?** - Tests will keep growing with TDD workflow
2. **Which pattern?** - `#[path]` attribute (simplest) vs tests subdirectory
3. **Test helpers?** - Consolidate into shared module or keep per-file?
4. **Which files first?** - Start with `calculations.rs` (most extreme ratio)

---

## Sources

1. [The Rust Programming Language - Test Organization](https://doc.rust-lang.org/book/ch11-03-test-organization.html)
2. [Rust Forum: Should unit tests be in the same file?](https://users.rust-lang.org/t/should-unit-tests-really-be-put-in-the-same-file-as-the-source/62153)
3. [Rust Forum: Real world tips for organising unit-tests](https://users.rust-lang.org/t/real-world-tips-for-organising-unit-tests-for-larger-projects-and-files/130749)
4. [Sling Academy: Organizing Rust Test Files](https://www.slingacademy.com/article/organizing-rust-test-files-and-modules-for-clarity-and-maintainability/)
5. [LogRocket: How to organize Rust tests](https://blog.logrocket.com/how-to-organize-rust-tests/)
6. [Walk N' Squawk: Rust unit test layout](https://www.walknsqualk.com/020-rust-unit-test-layout/)
7. [Software Patterns Lexicon: Rust Unit Testing](https://softwarepatternslexicon.com/patterns-rust/22/2/)
8. [Zero To Mastery: Complete Guide To Testing Code In Rust](https://zerotomastery.io/blog/complete-guide-to-testing-code-in-rust/)
