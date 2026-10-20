# Vérification locale : formatage, lint, tests. S'arrête à la première étape en échec.
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')

$steps = @(
    @{ Name = 'Formatage'; Cmd = { cargo fmt --all --check } },
    @{ Name = 'Clippy';    Cmd = { cargo clippy --workspace --all-targets -- -D warnings } },
    @{ Name = 'Tests';     Cmd = { cargo test --workspace } }
)

foreach ($step in $steps) {
    Write-Host "==> $($step.Name)" -ForegroundColor Magenta
    & $step.Cmd
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Échec : $($step.Name)" -ForegroundColor Red
        exit $LASTEXITCODE
    }
}

Write-Host 'Tout est vert.' -ForegroundColor Green
