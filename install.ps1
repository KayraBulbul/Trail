# Installs the latest Trail release on Windows.
#
#   irm https://raw.githubusercontent.com/KayraBulbul/Trail/main/install.ps1 | iex
#
# $env:TRAIL_INSTALL_DIR overrides the install directory (default: %LOCALAPPDATA%\Programs\trail).
# $env:TRAIL_VERSION installs a specific release tag, e.g. v0.6.0 (default: latest).

# Wrapped in a function so `irm | iex` doesn't leak variables into the user's session,
# and errors use `throw` because `exit` would close their terminal.
function Install-Trail {
    $ErrorActionPreference = 'Stop'
    $ProgressPreference = 'SilentlyContinue'
    # Windows PowerShell 5.1 doesn't enable TLS 1.2 by default, which GitHub requires.
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

    $repo = 'KayraBulbul/Trail'

    # ARM64 Windows runs the x64 build through emulation.
    if ($env:PROCESSOR_ARCHITECTURE -notin @('AMD64', 'ARM64')) {
        throw "Unsupported architecture: $env:PROCESSOR_ARCHITECTURE"
    }

    $asset = 'trail-windows-x86_64.zip'
    $version = if ($env:TRAIL_VERSION) { $env:TRAIL_VERSION } else { 'latest' }
    $baseUrl = if ($version -eq 'latest') {
        "https://github.com/$repo/releases/latest/download"
    } else {
        "https://github.com/$repo/releases/download/$version"
    }
    $installDir = if ($env:TRAIL_INSTALL_DIR) {
        $env:TRAIL_INSTALL_DIR
    } else {
        Join-Path $env:LOCALAPPDATA 'Programs\trail'
    }

    $tmpDir = Join-Path ([IO.Path]::GetTempPath()) "trail-install-$([Guid]::NewGuid())"
    New-Item -ItemType Directory -Path $tmpDir | Out-Null

    try {
        Write-Host "Downloading $asset ($version)..."
        $archive = Join-Path $tmpDir $asset
        $sums = Join-Path $tmpDir 'SHA256SUMS.txt'
        Invoke-WebRequest -UseBasicParsing -Uri "$baseUrl/$asset" -OutFile $archive
        Invoke-WebRequest -UseBasicParsing -Uri "$baseUrl/SHA256SUMS.txt" -OutFile $sums

        $expected = Get-Content $sums |
            ForEach-Object { $hash, $name = $_ -split '\s+', 2; if ($name.TrimStart('*') -eq $asset) { $hash } } |
            Select-Object -First 1
        if (-not $expected) {
            throw "SHA256SUMS.txt has no entry for $asset"
        }
        $actual = (Get-FileHash -Algorithm SHA256 -Path $archive).Hash
        if ($actual -ne $expected) {
            throw "Checksum mismatch for $asset, aborting"
        }

        Expand-Archive -Path $archive -DestinationPath $tmpDir -Force
        $binary = Join-Path $tmpDir 'trail.exe'
        if (-not (Test-Path $binary)) {
            throw "Release archive doesn't contain trail.exe"
        }

        New-Item -ItemType Directory -Path $installDir -Force | Out-Null
        Copy-Item -Path $binary -Destination (Join-Path $installDir 'trail.exe') -Force
    } finally {
        Remove-Item -Recurse -Force $tmpDir -ErrorAction SilentlyContinue
    }

    Write-Host "Installed Trail to $(Join-Path $installDir 'trail.exe')"

    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $onPath = ($userPath -split ';') -contains $installDir
    if (-not $onPath) {
        $newPath = if ($userPath) { "$userPath;$installDir" } else { $installDir }
        [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
        $env:Path = "$env:Path;$installDir"
        Write-Host "Added $installDir to your PATH. Open a new terminal if 'trail' isn't found."
    } else {
        Write-Host "Run 'trail' to get started."
    }
}

Install-Trail
