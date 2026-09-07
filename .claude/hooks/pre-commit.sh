#!/bin/bash
# Pre-commit hook: block a commit if the backend tests fail.
# Exit 0 = allow, exit 2 = block (stderr is shown to Claude).

input=$(cat)
[ -z "$input" ] && exit 0

# Only gate `git commit`, anchored at the start of the command value so that
# e.g. `git log --grep "git commit"` does not trigger a full test run.
if ! echo "$input" | grep -q '"command"[[:space:]]*:[[:space:]]*"git commit'; then
    exit 0
fi

project_dir="${CLAUDE_PROJECT_DIR:-$PWD}"
manifest="$project_dir/src-tauri/Cargo.toml"

if [ ! -f "$manifest" ]; then
    echo "Warning: $manifest not found, skipping backend tests" >&2
    exit 0
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "Warning: cargo not found in PATH, skipping backend tests" >&2
    exit 0
fi

echo "=== Pre-commit: running backend tests ===" >&2

if ! cargo test --manifest-path "$manifest" --workspace >&2; then
    echo "COMMIT BLOCKED: backend tests failed." >&2
    exit 2
fi

echo "Backend tests passed. Proceeding with commit." >&2
exit 0
