# Smoke test local de l'application complète (backend Rust + UI WebAssembly + CSP), à lancer avant
# chaque push destiné à produire un APK. Construit l'app de bureau avec les assets embarqués (même
# configuration que l'APK), la lance avec le débogage WebView2 et vérifie le rendu et la console.
param(
    [string]$Expected = 'Liquid Compagnon',
    [int]$Port = 9222
)
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')

Write-Host '==> Build (assets embarqués)' -ForegroundColor Magenta
cargo tauri build --debug --no-bundle
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host '==> Lancement et inspection' -ForegroundColor Magenta
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$Port"
$app = Start-Process -FilePath 'target\debug\liquid-compagnon.exe' -PassThru
try {
    Start-Sleep -Seconds 6
    node scripts/smoke-inspect.mjs $Port $Expected
    $code = $LASTEXITCODE
}
finally {
    Stop-Process -Id $app.Id -Force -ErrorAction SilentlyContinue
    Remove-Item Env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS -ErrorAction SilentlyContinue
}
exit $code
