# Install vibecasso on Windows.
#
#   Public repo:   irm https://raw.githubusercontent.com/CydoEntis/vibecasso/main/install.ps1 | iex
#   Private repo:  gh api -H "Accept: application/vnd.github.raw" repos/CydoEntis/vibecasso/contents/install.ps1 | Out-String | iex
#
# Settings (environment variables):
#   VIBECASSO_VERSION      a tag such as v0.1.0 (default: the latest release)
#   VIBECASSO_INSTALL_DIR  where vibecasso.exe goes (default: %LOCALAPPDATA%\Programs\vibecasso)
#   VIBECASSO_NO_PATH      set to 1 to leave your PATH alone
$ErrorActionPreference = 'Stop'

$repo = 'CydoEntis/vibecasso'
$dir = if ($env:VIBECASSO_INSTALL_DIR) { $env:VIBECASSO_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\vibecasso' }
# Windows on ARM runs the x64 build.
$target = 'x86_64-pc-windows-msvc'
$asset = "vibecasso-$target.zip"
$tmp = Join-Path ([IO.Path]::GetTempPath()) ("vibecasso-install-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force $tmp | Out-Null

function Fetch($name) {
    $gh = Get-Command gh -ErrorAction SilentlyContinue
    $signedIn = $false
    if ($gh) { gh auth status *> $null; $signedIn = ($LASTEXITCODE -eq 0) }
    if ($signedIn) {
        # A private repo: the GitHub CLI brings your sign-in.
        if ($env:VIBECASSO_VERSION) { gh release download $env:VIBECASSO_VERSION -R $repo -p $name -D $tmp --clobber }
        else { gh release download -R $repo -p $name -D $tmp --clobber }
        if ($LASTEXITCODE -ne 0) { throw "couldn't download $name with gh" }
    } else {
        $url = if ($env:VIBECASSO_VERSION) { "https://github.com/$repo/releases/download/$($env:VIBECASSO_VERSION)/$name" }
               else { "https://github.com/$repo/releases/latest/download/$name" }
        try { Invoke-WebRequest -UseBasicParsing $url -OutFile (Join-Path $tmp $name) }
        catch { throw "couldn't download $name (a private repo needs the GitHub CLI: gh auth login)" }
    }
}

try {
    Write-Host "Downloading vibecasso for $target..."
    Fetch $asset
    Fetch 'sha256sums.txt'

    $line = Get-Content (Join-Path $tmp 'sha256sums.txt') | Where-Object { $_ -match " $([regex]::Escape($asset))$" }
    if (-not $line) { throw "no checksum for $asset" }
    $expected = ($line -split '\s+')[0].ToLower()
    $actual = (Get-FileHash (Join-Path $tmp $asset) -Algorithm SHA256).Hash.ToLower()
    if ($expected -ne $actual) { throw "checksum mismatch for $asset" }

    Expand-Archive (Join-Path $tmp $asset) -DestinationPath $tmp -Force
    New-Item -ItemType Directory -Force $dir | Out-Null
    # Windows locks a running exe: stop the vibecasso being replaced (its sessions come back on
    # the next start). A vibecasso installed elsewhere keeps running.
    $installed = Join-Path $dir 'vibecasso.exe'
    $running = Get-Process -Name vibecasso -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $installed }
    if ($running) {
        Write-Host "Stopping the running vibecasso to replace it..."
        $running | Stop-Process -Force
        Start-Sleep -Milliseconds 500
    }
    Copy-Item (Join-Path $tmp "vibecasso-$target\vibecasso.exe") (Join-Path $dir 'vibecasso.exe') -Force

    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if ($env:VIBECASSO_NO_PATH -ne '1' -and -not (($userPath -split ';') -contains $dir)) {
        [Environment]::SetEnvironmentVariable('Path', ($userPath.TrimEnd(';') + ";$dir"), 'User')
        Write-Host "Added $dir to your PATH (new terminals pick it up)."
    }
    if (-not (($env:Path -split ';') -contains $dir)) { $env:Path = "$env:Path;$dir" }

    $version = & (Join-Path $dir 'vibecasso.exe') --version
    Write-Host "Installed $version to $dir\vibecasso.exe"

    # Another vibecasso.exe found first on PATH?
    $first = (Get-Command vibecasso -All -ErrorAction SilentlyContinue | Select-Object -First 1).Source
    if ($first -and ($first -ne (Join-Path $dir 'vibecasso.exe'))) {
        Write-Host ""
        Write-Host "Note: '$first' comes first on your PATH (an older install, or a different tool)."
        Write-Host "Remove it, or give this one its own name in your PowerShell profile:"
        Write-Host "  Set-Alias hy '$dir\vibecasso.exe'"
    }
    Write-Host ""
    Write-Host "Next: run 'vibecasso doctor' to check your setup, then 'vibecasso'."
    Write-Host "If vibecasso was already running, just run vibecasso: it restarts itself onto the new version (agents resume)."
} finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
