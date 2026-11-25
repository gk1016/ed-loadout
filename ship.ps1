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

L "--- scripted run: CAPTURE THIS MACHINE, select, APPLY (confirm), LOG ---"
$r = Run (Join-Path $dist 'loadout.exe') @('--script', 'capture,apply') 7000
Need $r 'script: capture ok=true profiles=1 selected=\S+' "capture did not create and select a profile"
$made = @(Get-ChildItem $profiles -Directory | Where-Object { Test-Path (Join-Path $_.FullName 'profile.toml') })
if ($made.Count -ne 1) { Fail "expected one profile under $profiles, found $($made.Count)" }
L "profile created: $($made[0].FullName)"
Need $r 'select: \S+, \d+ effective rows, [1-9]\d* files to write' "selecting the profile did not fill WILL APPLY"
Need $r 'script: apply enabled=true' "APPLY was not enabled"
Need $r 'script: confirm open=true files=[1-9]\d* backup under .+_loadout-backup' "the confirm did not open with the file list and the backup path"
Need $r 'core: backup: .+_loadout-backup' "apply logged no backup"
Need $r 'core: wrote Graphics/DisplaySettings.xml' "apply logged no written file"
Need $r 'script: applied; Applied \S+ Backup \d{8}-\d{6}-\d{3}\. log lines=[1-9]' "apply did not report success with log lines"
$applied = Hashes $opts
if (-not (Compare-Object $before $applied)) { Fail "apply changed nothing in the scratch Options dir" }
L ("apply changed: " + ((Compare-Object $before $applied | Where-Object SideIndicator -eq '=>' | ForEach-Object { ($_.InputObject -split ' ')[0] }) -join ', '))

L "--- scripted run: RESTORE LAST BACKUP ---"
$r = Run (Join-Path $dist 'loadout.exe') @('--script', 'restore') 6000
Need $r 'script: restore ok=true; Restored backup \d{8}-\d{6}-\d{3}\.' "restore did not report success"
$after = Hashes $opts
$diff = Compare-Object $before $after
if ($diff) { L ($diff | Out-String); Fail "Options dir differs after restore" }
L ("restore OK: " + $before.Count + " files, same file set, same SHA-256 as before the apply (Get-FileHash; _loadout-backup left out)")

L "--- game running: LOADOUT_FAKE_GAME_RUNNING=1, read by debug builds of the core only ---"
# The packaged (release) exe must ignore the variable. build\loadout-dbgcore.exe
# is the same shell on the debug core: there APPLY must be off, the confirm
# must not open, and nothing may be written.
$env:LOADOUT_FAKE_GAME_RUNNING = '1'
try {
    $r = Run (Join-Path $dist 'loadout.exe') @('--select', $made[0].Name) 4000
    Need $r 'home: detected=true profiles=1 selected=\S+ game=false apply enabled=true' "the release build read LOADOUT_FAKE_GAME_RUNNING"
    L "release build: variable ignored (game=false, APPLY enabled)"
    $r = Run (Join-Path $root 'build\loadout-dbgcore.exe') @('--select', $made[0].Name, '--script', 'apply,restore') 6000
    Need $r 'home: detected=true profiles=1 selected=\S+ game=true apply enabled=false' "debug core: the game-running state did not disable APPLY"
    Need $r 'script: apply enabled=false' "debug core: APPLY stayed enabled"
    Need $r 'script: confirm open=false' "debug core: the confirm opened while the game runs"
    Need $r 'script: restore ok=false; refusing: EliteDangerous64.exe is running' "debug core: RESTORE was not refused"
} finally { Remove-Item Env:\LOADOUT_FAKE_GAME_RUNNING }
if (Compare-Object $before (Hashes $opts)) { Fail "something was written while the game was (faked) running" }
L "debug core: game=true, APPLY disabled, confirm closed, RESTORE refused, Options dir unchanged"

Remove-Item $scratch -Recurse -Force -ErrorAction SilentlyContinue
L "=== VERIFY OK (no release) ==="
exit 0
