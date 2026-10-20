# Parcours utilisateur de bout en bout sur l'app de bureau (même UI et même backend que l'APK),
# avec une base temporaire isolée. Les captures d'écran sont écrites dans target/e2e-captures.
param([int]$Port = 9223)
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')

Write-Host '==> Build (assets embarqués)' -ForegroundColor Magenta
cargo tauri build --debug --no-bundle
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$dataDir = Join-Path ([IO.Path]::GetTempPath()) ("lc-e2e-" + [guid]::NewGuid())
$shots = Join-Path (Get-Location) 'target\e2e-captures'
if (Test-Path $shots) { Remove-Item $shots -Recurse -Force }

Write-Host '==> Parcours' -ForegroundColor Magenta
$env:LC_DATA_DIR = $dataDir
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$Port"
$app = Start-Process -FilePath 'target\debug\liquid-compagnon.exe' -PassThru
try {
    Start-Sleep -Seconds 6
    node scripts/e2e-ui.mjs $Port $shots
    $code = $LASTEXITCODE
}
finally {
    Stop-Process -Id $app.Id -Force -ErrorAction SilentlyContinue
    Remove-Item Env:LC_DATA_DIR, Env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 1
    Remove-Item $dataDir -Recurse -Force -ErrorAction SilentlyContinue
}
exit $code
