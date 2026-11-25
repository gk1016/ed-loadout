# One driver, one log (ship.log): build -> deploy -> package (dist\LOADOUT and
# ..\transfer\LOADOUT_Setup.exe) -> launch the PACKAGED exe and read its log ->
# the scripted run (capture, apply, restore against a scratch Options dir,
# hashed before and after) -> the game-running state on the debug-core twin
# -> the installer check (silent install, four
# asserts, a HOME capture of the installed exe, silent uninstall).
# Ends with VERIFY OK or ABORT.
#   .\ship.ps1 -NoRelease     build and verify; the only mode there is.
# LOADOUT has no engine and no Drive copy, and this script cuts no release:
# a release is .github/workflows/release.yml on a v* tag (Cowork), on Graham's
# explicit instruction. Without -NoRelease the script says so and stops.
param([switch]$NoRelease)
$root = $PSScriptRoot
$log = Join-Path $root 'ship.log'
"=== ship start $(Get-Date -Format u) ===" | Set-Content $log
function L($m) { $m | Add-Content $log }
function Fail($m) { L "ABORT: $m"; L "=== ABORTED ==="; exit 1 }
Set-Location $root

if (-not $NoRelease) { Fail "ship.ps1 only verifies (-NoRelease). A release is release.yml on a v* tag, on explicit instruction." }

$ver = (Select-String -Path 'rust\Cargo.toml' -Pattern '^version\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value
$issVer = (Select-String -Path 'loadout.iss' -Pattern '^AppVersion=(.+)$').Matches[0].Groups[1].Value.Trim()
if ($ver -ne $issVer) { Fail "version drift: Cargo.toml $ver vs loadout.iss $issVer" }
L "version $ver"
# Never Stop-Process an app someone is using: verify-win guards this too.
if (Get-Process loadout -ErrorAction SilentlyContinue) { Fail "LOADOUT is running - close it and run ship.ps1 again" }

L "--- build ---"
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $root 'build.ps1') *> $null
$b = Get-Content (Join-Path $root 'build.log') -Raw
L (($b -split "`n") | Where-Object { $_ -match '^test result|^EXE|^CLI|^===' } | Out-String)
if ($b -notmatch '=== BUILD DONE ===') { Fail "build failed - see build.log" }
L "--- deploy ---"
$d = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $root 'deploy.ps1') 2>&1
L ($d | Select-Object -Last 3 | Out-String)
if (($d -join "`n") -notmatch 'DEPLOY DONE') { Fail "deploy failed" }
L "--- package ---"
$p = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $root 'package.ps1') 2>&1
L ($p | Out-String)
if (($p -join "`n") -notmatch 'PACKAGE DONE') { Fail "package failed" }

$dist = Join-Path $root 'dist\LOADOUT'
$transfer = Join-Path (Split-Path $root -Parent) 'transfer'
# Everything the runs below touch is under one scratch folder in %TEMP%: a
# copy of fixtures\devmachine02 as the Options dir, and a profiles folder
# that does not exist yet. Never the box's real Elite Options, never the
# saved LOADOUT settings (--options / --profiles are not saved).
$scratch = Join-Path $env:TEMP ("loadout-verify-" + $PID)
$opts = Join-Path $scratch 'Options'
$profiles = Join-Path $scratch 'profiles'
Remove-Item $scratch -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $opts | Out-Null
Copy-Item (Join-Path $root 'fixtures\devmachine02\Graphics') $opts -Recurse
Copy-Item (Join-Path $root 'fixtures\devmachine02\Bindings') $opts -Recurse

# "<relative path> <sha256>" for every file under the Options dir, sorted.
# _loadout-backup is LOADOUT's own folder there and is left out.
function Hashes($dir) {
    Get-ChildItem $dir -Recurse -File | Where-Object { $_.FullName.Substring($dir.Length + 1) -notlike '_loadout-backup\*' } |
        Sort-Object FullName | ForEach-Object { $_.FullName.Substring($dir.Length + 1) + ' ' + (Get-FileHash $_.FullName -Algorithm SHA256).Hash }
}
# Launch an exe with the scratch dirs, wait for it to quit, return its log.
function Run($exe, [string[]]$more, $ms) {
    $dir = Split-Path $exe -Parent
    $runLog = Join-Path $dir 'loadout.log'
    Remove-Item $runLog -ErrorAction SilentlyContinue
    $argv = @('--options', "`"$opts`"", '--profiles', "`"$profiles`"", '--quit-after', "$ms") + $more
    $proc = Start-Process -FilePath $exe -ArgumentList $argv -WorkingDirectory $dir -WindowStyle Normal -PassThru
    if (-not $proc.WaitForExit($ms + 25000)) { $proc.Kill(); Fail "$exe did not exit" }
    $r = if (Test-Path $runLog) { Get-Content $runLog -Raw } else { '' }
    Remove-Item $runLog -ErrorAction SilentlyContinue
    L $r
    if ($r -notmatch [regex]::Escape("LOADOUT $ver")) { Fail "the exe did not log its start" }
    if ($r -match 'did not load' -or $r -match '\.qml:\d+') { Fail "QML errors" }
    return $r
}
function Need($r, $pattern, $what) { if ($r -notmatch $pattern) { Fail $what } }

L "--- run the packaged exe: first open, no profile folder ---"
$shot = Join-Path $transfer 'loadout-shot.png'
Remove-Item $shot -ErrorAction SilentlyContinue
$before = Hashes $opts
$r = Run (Join-Path $dist 'loadout.exe') @('--shot', "`"$shot`"") 6000
Need $r 'home: detected=true profiles=0 selected=none' "first open: DETECTED not filled, or a profile where none should be"
if (-not (Test-Path $shot)) { Fail "no window capture" }
if (Test-Path $profiles) { Fail "opening the app created the profiles folder" }
L "run OK, capture $shot"

Remove-Item $scratch -Recurse -Force -ErrorAction SilentlyContinue
L "=== VERIFY OK (no release) ==="
exit 0
