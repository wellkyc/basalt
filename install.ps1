& {
    $ErrorActionPreference = 'Stop'

    $repo = 'wellkyc/basalt'
    $dir = if ($env:BASALT_INSTALL_DIR) { $env:BASALT_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'basalt\bin' }
    $url = "https://github.com/$repo/releases/latest/download/basalt-windows-x86_64.zip"
    $tmp = Join-Path ([IO.Path]::GetTempPath()) ('basalt-' + [guid]::NewGuid())

    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
    New-Item -ItemType Directory -Force $tmp | Out-Null
    try {
        Write-Host "Downloading $url"
        Invoke-WebRequest $url -OutFile (Join-Path $tmp 'basalt.zip') -UseBasicParsing
        Expand-Archive (Join-Path $tmp 'basalt.zip') $tmp -Force
        New-Item -ItemType Directory -Force $dir | Out-Null
        Copy-Item (Join-Path $tmp 'basalt.exe') $dir -Force
    }
    finally {
        Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }
    Write-Host "Installed basalt to $dir\basalt.exe"

    if (-not $env:BASALT_NO_MODIFY_PATH) {
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        if (-not $userPath) { $userPath = '' }
        if (($userPath -split ';') -notcontains $dir) {
            $newPath = (($userPath.TrimEnd(';'), $dir) | Where-Object { $_ }) -join ';'
            [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
            Write-Host "Added $dir to your PATH. Open a new terminal and run: basalt"
        }
    }
}
