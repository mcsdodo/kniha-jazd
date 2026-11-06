#!/usr/bin/env bash
# List the commits that changed a screen after its mockup file last changed.
#
# Reads the registry table in docs/mockups/README.md. For each row it finds the last
# commit that changed the mockup file, then runs
#   git log --oneline <that commit>..HEAD -- <source paths>
# and prints the commits under the page name. No output = every mockup is current.
#
# Usage: bash scripts/mockup-drift.sh [registry-file]
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
registry="${1:-$root/docs/mockups/README.md}"
regdir="$(cd "$(dirname "$registry")" && pwd)"

if [ ! -f "$registry" ]; then
    echo "mockup-drift: no registry at $registry" >&2
    exit 1
fi

# Registry rows: | Page | File | Artifact | UUID | Source paths |
# Only rows between the markers count, so other tables in the file do not.
awk '/<!-- registry:start -->/{f=1; next} /<!-- registry:end -->/{f=0} f' "$registry" |
    grep -E '^\|' | grep -vE '^\|[[:space:]]*(Page|-)' |
    while IFS='|' read -r _ page file _ _ paths _; do
        page="$(echo "$page" | xargs)"
        # The file cell is a Markdown link: [name.html](name.html). Take the target.
        file="$(echo "$file" | sed -E 's/.*\]\(([^)]*)\).*/\1/' | xargs)"
        paths="$(echo "$paths" | tr -d '`' | xargs)"
        [ -z "$file" ] && continue
        mockup="$regdir/$file"
        if [ ! -f "$mockup" ]; then
            echo "## $page: no mockup file $file"
            continue
        fi
        last="$(git -C "$root" log -1 --format=%H -- "$mockup")"
        if [ -z "$last" ]; then
            echo "## $page: $file is not committed yet"
            continue
        fi
        # shellcheck disable=SC2086  # paths is a space-separated list on purpose
        log="$(git -C "$root" log --oneline "${last}..HEAD" -- $paths)"
        if [ -n "$log" ]; then
            echo "## $page (mockup last changed in ${last:0:7})"
            echo "$log"
        fi
    done
