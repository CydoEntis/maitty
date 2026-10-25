# Install the app on Windows.
#
#   Public repo:   irm https://raw.githubusercontent.com/<REPO>/main/install.ps1 | iex
#   Private repo:  gh api -H "Accept: application/vnd.github.raw" repos/<REPO>/contents/install.ps1 | Out-String | iex
#
# Settings (environment variables, <APP> being the app's command in capitals):
#   <APP>_VERSION      a tag such as v0.1.0 (default: the latest release)
#   <APP>_INSTALL_DIR  where <app>.exe goes (default: %LOCALAPPDATA%\Programs\<app>)
#   <APP>_NO_PATH      set to 1 to leave your PATH alone
#   <APP>_NO_INTEGRATE set to 1 to leave your agents' config alone (no `<app> integrate`)
$ErrorActionPreference = 'Stop'

# The app's command and where it is released: the only place this script names it (ADR 0019).
$app = 'maitty'
$repo = 'CydoEntis/maitty'

$envPrefix = $app.ToUpper().Replace('-', '_')
function Setting($name) { [Environment]::GetEnvironmentVariable("$($envPrefix)_$name") }
$version = Setting 'VERSION'
$dir = if (Setting 'INSTALL_DIR') { Setting 'INSTALL_DIR' } else { Join-Path $env:LOCALAPPDATA "Programs\$app" }
$exe = "$app.exe"
# Windows on ARM runs the x64 build.
$target = 'x86_64-pc-windows-msvc'
$asset = "$app-$target.zip"
$tmp = Join-Path ([IO.Path]::GetTempPath()) ("$app-install-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force $tmp | Out-Null

function Fetch($name) {
    $gh = Get-Command gh -ErrorAction SilentlyContinue
    $signedIn = $false
    if ($gh) { gh auth status *> $null; $signedIn = ($LASTEXITCODE -eq 0) }
    if ($signedIn) {
        # A private repo: the GitHub CLI brings your sign-in.
        if ($version) { gh release download $version -R $repo -p $name -D $tmp --clobber }
        else { gh release download -R $repo -p $name -D $tmp --clobber }
        if ($LASTEXITCODE -ne 0) { throw "couldn't download $name with gh" }
    } else {
        $url = if ($version) { "https://github.com/$repo/releases/download/$version/$name" }
               else { "https://github.com/$repo/releases/latest/download/$name" }
        try { Invoke-WebRequest -UseBasicParsing $url -OutFile (Join-Path $tmp $name) }
        catch { throw "couldn't download $name (a private repo needs the GitHub CLI: gh auth login)" }
    }
}

try {
    Write-Host "Downloading $app for $target..."
    Fetch $asset
    Fetch 'sha256sums.txt'

    $line = Get-Content (Join-Path $tmp 'sha256sums.txt') | Where-Object { $_ -match " $([regex]::Escape($asset))$" }
    if (-not $line) { throw "no checksum for $asset" }
    $expected = ($line -split '\s+')[0].ToLower()
    $actual = (Get-FileHash (Join-Path $tmp $asset) -Algorithm SHA256).Hash.ToLower()
    if ($expected -ne $actual) { throw "checksum mismatch for $asset" }

    Expand-Archive (Join-Path $tmp $asset) -DestinationPath $tmp -Force
    New-Item -ItemType Directory -Force $dir | Out-Null
    # Windows locks a running exe: stop the copy being replaced (its sessions come back on the
    # next start). A copy installed elsewhere keeps running.
    $installed = Join-Path $dir $exe
    $running = Get-Process -Name $app -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $installed }
    if ($running) {
        Write-Host "Stopping the running $app to replace it..."
        $running | Stop-Process -Force
        Start-Sleep -Milliseconds 500
    }
    Copy-Item (Join-Path $tmp "$app-$target\$exe") $installed -Force

    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if ((Setting 'NO_PATH') -ne '1' -and -not (($userPath -split ';') -contains $dir)) {
        [Environment]::SetEnvironmentVariable('Path', ($userPath.TrimEnd(';') + ";$dir"), 'User')
        Write-Host "Added $dir to your PATH (new terminals pick it up)."
    }
    if (-not (($env:Path -split ';') -contains $dir)) { $env:Path = "$env:Path;$dir" }

    $installedVersion = & $installed --version
    Write-Host "Installed $installedVersion to $installed"

    # Status hooks and MCP for every agent installed here; a failure must not fail the install.
    Write-Host ""
    if ((Setting 'NO_INTEGRATE') -eq '1') {
        Write-Host "Left your agents alone ($($envPrefix)_NO_INTEGRATE=1); run '$app integrate' to connect them."
    } else {
        Write-Host "Connecting your agents (set $($envPrefix)_NO_INTEGRATE=1 to skip)..."
        # 'Stop' would turn the program's own error output into a failed install.
        & { $ErrorActionPreference = 'Continue'; & $installed integrate }
        if ($LASTEXITCODE -ne 0) { Write-Host "Warning: some agents weren't connected; run '$app integrate' to retry." }
    }

    # Another program of the same name found first on PATH?
    $first = (Get-Command $app -All -ErrorAction SilentlyContinue | Select-Object -First 1).Source
    if ($first -and ($first -ne $installed)) {
        Write-Host ""
        Write-Host "Note: '$first' comes first on your PATH (an older install, or a different tool)."
        Write-Host "Remove it, or give this one its own name in your PowerShell profile:"
        Write-Host "  Set-Alias $($app)2 '$installed'"
    }
    Write-Host ""
    Write-Host "Next: run '$app'. Restart agents that were already running so they load the hooks; '$app doctor' checks your setup."
    Write-Host "If $app was already running, just run ${app}: it restarts itself onto the new version (agents resume)."
} finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
