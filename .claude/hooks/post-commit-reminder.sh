#!/bin/bash
# Post-commit hook: changelog reminder + temp file cleanup

input=$(cat)
[ -z "$input" ] && exit 0

# Clean up tmpclaude-* temp files (always, on any bash command).
# Anchor to the project dir: the hook cwd can be any subfolder.
find "${CLAUDE_PROJECT_DIR:-.}" -name "tmpclaude-*" -delete 2>/dev/null

# Only show reminder for git commit commands
if ! echo "$input" | grep -q '"command"[^}]*git commit'; then
    exit 0
fi

msg=""

# Mockup reminder: the commit at HEAD changed a screen (a page or a component) and
# no mockup file in docs/mockups/. It only reminds: an invisible change needs no
# mockup. See .claude/skills/mockup-skill/SKILL.md.
head_files=$(git -C "${CLAUDE_PROJECT_DIR:-.}" diff-tree --no-commit-id --name-only -r HEAD 2>/dev/null)
ui_files=$(echo "$head_files" | grep -E '^src/(routes|lib/components)/.*\.svelte$' | head -5)
if [ -n "$ui_files" ] && ! echo "$head_files" | grep -qE '^docs/mockups/.*\.html$'; then
    msg="REMINDER: This commit changed a screen ($(echo "$ui_files" | tr '\n' ' ')). If the change is visible, update the mockup file of the page in docs/mockups/ and publish it (mockup skill)."
fi

# Changelog reminder, except for a changelog commit
if ! echo "$input" | grep -qi '"command"[^}]*changelog'; then
    msg="${msg:+$msg
}REMINDER: If this was a user-visible change, run /changelog. Skip for internal docs (CLAUDE.md, _tasks/, etc.)"
fi

[ -z "$msg" ] && exit 0

# Exit code 2 with stderr = blocking message shown to Claude
echo "$msg" >&2
exit 2
