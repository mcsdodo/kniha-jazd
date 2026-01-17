# Test Release Script
# Builds a TEST release with bumped version for auto-update testing
# Automatically reverts version after build - does NOT modify git state

param(
    [string]$BumpType = "minor"  # minor (0.17.2 -> 0.18.0) or patch (0.17.2 -> 0.17.3)
)

Write-Host "Building test release..." -ForegroundColor Cyan

# Read current version
$tauriConfPath = "src-tauri\tauri.conf.json"
$packageJsonPath = "package.json"

$tauriConf = Get-Content $tauriConfPath -Raw | ConvertFrom-Json
$packageJson = Get-Content $packageJsonPath -Raw | ConvertFrom-Json

$currentVersion = $tauriConf.version
$versionParts = $currentVersion.Split('.')
$major = [int]$versionParts[0]
$minor = [int]$versionParts[1]
$patch = [int]$versionParts[2]

# Calculate test version
if ($BumpType -eq "patch") {
    $testVersion = "$major.$minor.$($patch + 1)"
} else {
    $testVersion = "$major.$($minor + 1).0"
}

Write-Host "Current version: $currentVersion" -ForegroundColor Gray
Write-Host "Test version:    $testVersion" -ForegroundColor Yellow

# Temporarily bump version in both config files
Write-Host ""
Write-Host "Temporarily bumping version..." -ForegroundColor Cyan

$tauriConf.version = $testVersion
$tauriConf | ConvertTo-Json -Depth 10 | Set-Content $tauriConfPath

$packageJson.version = $testVersion
$packageJson | ConvertTo-Json -Depth 10 | Set-Content $packageJsonPath

try {
    # Build the release
    Write-Host ""
    Write-Host "Building..." -ForegroundColor Cyan
    $ErrorActionPreference = "Continue"
    npm run tauri build
    $ErrorActionPreference = "Stop"

    # Check if artifacts were created
    $bundleDir = "src-tauri\target\release\bundle"
    $nsis = Get-ChildItem "$bundleDir\nsis\*$testVersion*-setup.exe" -ErrorAction SilentlyContinue
    $sig = Get-ChildItem "$bundleDir\nsis\*$testVersion*-setup.exe.sig" -ErrorAction SilentlyContinue

    if (-not $nsis) {
        Write-Host "Build failed - no NSIS installer found!" -ForegroundColor Red
        exit 1
    }

    # Create output directory
    $outputDir = "_test-releases\releases\v$testVersion"
    if (Test-Path $outputDir) {
        Remove-Item $outputDir -Recurse -Force
    }
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null

    Write-Host ""
    Write-Host "Copying artifacts to $outputDir..." -ForegroundColor Cyan

    # Copy NSIS installer
    Copy-Item $nsis.FullName $outputDir
    Write-Host "  Copied: $($nsis.Name)" -ForegroundColor Green

    # Update latest.json
    $exeName = $nsis.Name -replace ' ', '%20'
    $signature = ""

    # Copy signature if exists
    if ($sig) {
        Copy-Item $sig.FullName $outputDir
        Write-Host "  Copied: $($sig.Name)" -ForegroundColor Green
        $signature = (Get-Content $sig.FullName -Raw).Trim()
    } else {
        Write-Host ""
        Write-Host "  WARNING: No .sig file - set TAURI_SIGNING_PRIVATE_KEY to enable signing" -ForegroundColor Yellow
        Write-Host "  Without signature, auto-update won't work (but you can test the installer manually)" -ForegroundColor Yellow
    }

    $latestJson = @{
        version = $testVersion
        notes = "Test release v$testVersion"
        pub_date = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
        platforms = @{
            "windows-x86_64" = @{
                url = "http://localhost:3456/releases/v$testVersion/$exeName"
                signature = $signature
            }
        }
    }

    $latestJson | ConvertTo-Json -Depth 4 | Set-Content "_test-releases\latest.json"
    Write-Host "  Updated: latest.json" -ForegroundColor Green

    Write-Host ""
    Write-Host "Test release ready!" -ForegroundColor Green
    Write-Host ""

    # List what was created
    Get-ChildItem $outputDir | ForEach-Object {
        $size = "{0:N2} MB" -f ($_.Length / 1MB)
        Write-Host "  $($_.Name) ($size)"
    }

    Write-Host ""
    Write-Host "To test auto-update:" -ForegroundColor Cyan
    Write-Host "  1. node _test-releases/serve.js"
    Write-Host "  2. set TAURI_UPDATER_ENDPOINT=http://localhost:3456/latest.json && npm run tauri dev"
    Write-Host ""
    Write-Host "App will run at v$currentVersion and detect v$testVersion as available update." -ForegroundColor Gray

} finally {
    # Always revert version
    Write-Host ""
    Write-Host "Reverting version to $currentVersion..." -ForegroundColor Cyan

    $tauriConf.version = $currentVersion
    $tauriConf | ConvertTo-Json -Depth 10 | Set-Content $tauriConfPath

    $packageJson.version = $currentVersion
    $packageJson | ConvertTo-Json -Depth 10 | Set-Content $packageJsonPath

    Write-Host "  Done" -ForegroundColor Green
}
