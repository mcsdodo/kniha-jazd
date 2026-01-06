# Post-commit reminder: Prompt for changelog update

$inputText = [Console]::In.ReadToEnd()
if (-not $inputText) { exit 0 }

try {
    $json = $inputText | ConvertFrom-Json
} catch {
    exit 0
}

if ($json.tool_input.command -notmatch '^git commit') {
    exit 0
}

if ($json.tool_input.command -match 'changelog|CHANGELOG') {
    exit 0  # Skip if this is a changelog commit
}

Write-Host ""
Write-Host "=== REMINDER: Update Changelog ===" -ForegroundColor Yellow
Write-Host "Run /changelog to update CHANGELOG.md [Unreleased] section" -ForegroundColor White
Write-Host ""

exit 0
