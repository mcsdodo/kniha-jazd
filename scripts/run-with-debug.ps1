# Run Tauri app with WebView2 remote debugging enabled
# This allows Playwright to connect via Chrome DevTools Protocol (CDP)

$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9222"

Write-Host "Starting Tauri app with remote debugging on port 9222..."
Write-Host "Playwright can connect to: http://localhost:9222"
Write-Host ""

# Run the dev build
Set-Location "$PSScriptRoot\.."
npm run tauri dev
