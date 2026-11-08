# Builds and runs the CURRENT worktree, so what you look at is never an old install.
#   .\scripts\dev.ps1 tui    build the terminal app and run it from target\debug (nothing is installed)
#   .\scripts\dev.ps1 gui    start the desktop app (Vite and the Rust host rebuild from this folder)
# Both print the commit and when the binary was built first; compare it with what you expect.
param([Parameter(Mandatory = $true)][ValidateSet('tui', 'gui')][string]$App)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
$commit = (git rev-parse --short HEAD)
$dirty = if (git status --porcelain) { '*' } else { '' }
Write-Host "worktree: $root  commit: $commit$dirty" -ForegroundColor Cyan

# Windows will not replace a program that is running, so say so instead of failing halfway.
$running = Get-Process maitty, maitty-gui -ErrorAction SilentlyContinue | Where-Object { $_.Path -like "$root*" }
if ($running) {
    Write-Host 'A build from this worktree is still running; close it first:' -ForegroundColor Yellow
    $running | ForEach-Object { Write-Host "  $($_.Name) (pid $($_.Id))" }
    exit 1
}

if ($App -eq 'tui') {
    cargo build -p maitty
    $exe = Join-Path $root 'target\debug\maitty.exe'
    Write-Host "running $exe (built $((Get-Item $exe).LastWriteTime))" -ForegroundColor Cyan
    & $exe
} else {
    Set-Location (Join-Path $root 'apps\gui')
    if (-not (Test-Path node_modules)) { pnpm install --frozen-lockfile --config.node-linker=hoisted }
    pnpm desktop:dev
}
