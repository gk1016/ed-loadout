# Cut dist\LOADOUT from build\ (loadout.exe, the CLI ed-loadout.exe, the Qt
# runtime, the MSVC runtime), then compile the installer:
# ..\transfer\LOADOUT_Setup.exe. The version is checked against the crate so a
# forgotten bump cannot ship an installer older than its binary.
$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot
$src  = Join-Path $root 'build'
$dist = Join-Path $root 'dist\LOADOUT'
$transfer = Join-Path (Split-Path $root -Parent) 'transfer'

if (Get-Process -Name 'loadout' -ErrorAction SilentlyContinue) {
    Write-Output "LOADOUT is running - close it and re-run package.ps1"
    Write-Output "=== PACKAGE SKIPPED ==="
    exit 1
}
$issVer = (Select-String -Path (Join-Path $root 'loadout.iss') -Pattern '^AppVersion=(.+)$').Matches[0].Groups[1].Value.Trim()
$cargoVer = (Select-String -Path (Join-Path $root 'rust\Cargo.toml') -Pattern '^version\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value.Trim()
if ($issVer -ne $cargoVer) { throw ("version drift: loadout.iss " + $issVer + " vs Cargo.toml " + $cargoVer) }

Remove-Item $dist -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $dist | Out-Null
Copy-Item (Join-Path $src 'loadout.exe') $dist
# The engine underneath, for scripting: same crate, same build.
Copy-Item (Join-Path $root 'rust\target\release\ed-loadout.exe') $dist
Get-ChildItem $src -Filter *.dll | Copy-Item -Destination $dist
foreach ($d in @('platforms','imageformats','iconengines','generic','styles','qml','fonts','icons')) {
    $p = Join-Path $src $d
    if (Test-Path $p) { Copy-Item $p $dist -Recurse -Force }
}

# The MSVC runtime beside the exes, so the folder runs on a box without the
# redistributable: ..\transfer\crt (FRAGO's source) when it is there, else the
# toolset's own redist folder.
$crt = Join-Path $transfer 'crt'
if (-not (Test-Path (Join-Path $crt 'vcruntime140.dll'))) {
    $vcvars = if ($env:HUMINT_VCVARS) { $env:HUMINT_VCVARS } else { Join-Path ${env:ProgramFiles} 'Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat' }
    $vc = Split-Path (Split-Path (Split-Path $vcvars -Parent) -Parent) -Parent
    $crt = Get-ChildItem (Join-Path $vc 'Redist\MSVC') -Directory -ErrorAction SilentlyContinue | Sort-Object Name -Descending |
        ForEach-Object { Get-ChildItem (Join-Path $_.FullName 'x64') -Directory -Filter 'Microsoft.VC*.CRT' -ErrorAction SilentlyContinue } |
        Select-Object -First 1 -ExpandProperty FullName
}
if (-not $crt -or -not (Test-Path (Join-Path $crt 'vcruntime140.dll'))) { throw "no MSVC runtime DLLs (looked in ..\transfer\crt and the toolset's Redist folder)" }
Get-ChildItem $crt -Filter *.dll | Copy-Item -Destination $dist -Force
Write-Output ("msvc runtime from " + $crt)

# Code signing is parked until public distribution: builds ship unsigned.
# When it returns, sign both exes HERE, before the installer packs them.

$gitSha = (& git -C $root rev-parse --short HEAD 2>$null)
$n = (Get-ChildItem $dist -Recurse -File).Count
$bytes = (Get-ChildItem $dist -Recurse -File | Measure-Object -Property Length -Sum).Sum
Write-Output ("dist: v" + $issVer + " @ " + $gitSha + ", " + $n + " files, " + [math]::Round($bytes/1MB,1) + " MB")

$iscc = if ($env:HUMINT_ISCC) { $env:HUMINT_ISCC } else { Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe' }
if (-not (Test-Path $iscc)) { throw "no ISCC.exe at $iscc (set HUMINT_ISCC)" }
New-Item -ItemType Directory -Force -Path $transfer | Out-Null
$setup = Join-Path $transfer 'LOADOUT_Setup.exe'
Remove-Item $setup -ErrorAction SilentlyContinue
& $iscc /Qp (Join-Path $root 'loadout.iss') 2>&1 | Select-Object -Last 4 | ForEach-Object { Write-Output "  $_" }
if ($LASTEXITCODE -ne 0 -or -not (Test-Path $setup)) { throw "ISCC failed: no $setup" }
Write-Output ("installer: " + $setup + ", " + (Get-Item $setup).Length + " bytes, sha256 " + (Get-FileHash $setup -Algorithm SHA256).Hash.ToLower())
Write-Output "=== PACKAGE DONE ==="
exit 0
