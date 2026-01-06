# Pre-commit hook: Block commits if backend tests fail
# Exit 0 = allow, Exit 2 = block

$inputText = [Console]::In.ReadToEnd()
if (-not $inputText) { exit 0 }

try {
    $json = $inputText | ConvertFrom-Json
} catch {
    exit 0  # Don't block on parse errors
}

if ($json.tool_input.command -notmatch '^git commit') {
    exit 0
}

$projectDir = if ($env:CLAUDE_PROJECT_DIR) { $env:CLAUDE_PROJECT_DIR } else { (Get-Location).Path }
$srcTauri = Join-Path $projectDir "src-tauri"

if (-not (Test-Path $srcTauri)) {
    Write-Host "Warning: src-tauri not found at $srcTauri" -ForegroundColor Yellow
    exit 0
}

Write-Host "`n=== Pre-commit: Running backend tests ===" -ForegroundColor Cyan

Push-Location $srcTauri
try {
    # Check cargo availability
    $cargoPath = Get-Command cargo -ErrorAction SilentlyContinue
    if (-not $cargoPath) {
        Write-Host "Warning: cargo not found in PATH" -ForegroundColor Yellow
        exit 0
    }

    cargo test 2>&1 | Tee-Object -Variable testOutput
    if ($LASTEXITCODE -ne 0) {
        Write-Host "`nCOMMIT BLOCKED: Tests failed!" -ForegroundColor Red
        exit 2
    }
    Write-Host "Tests passed. Proceeding with commit." -ForegroundColor Green
} finally {
    Pop-Location
}
exit 0
