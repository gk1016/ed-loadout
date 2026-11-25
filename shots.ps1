# Capture screens from the PACKAGED exe, after restaging qml\ into dist
# (QML-only iteration; ship.ps1 rebuilds properly). Logs QML warnings.
#
# -Survey <tag>: the UI survey for /carbon-ui. home (no profile yet), home,
# profiles, log, settings, the APPLY confirm and home with the game running
# (the debug-core twin), each in deepfield and daylight at 1320 wide, as JPG <= 400 KB in _shots\uipass\survey-<tag>\ with
# an index.md; plus the HOME capture of the installed exe when ship.ps1's
# installer check left one. Everything runs against a scratch folder in
# %TEMP%: a copy of fixtures\devmachine02 as the Options dir and two profiles
# captured from it with the packaged ed-loadout.exe. --options / --profiles /
# --select / --theme are never saved, so the box's real Elite Options and
# LOADOUT.ini are not touched; the INI's hash is checked before and after.
# The log ends "=== SURVEY OK ===" when every capture exists and it held.
param([string]$Tabs = 'home,profiles,log,settings', [string]$Theme = '', [string]$Survey = '')
$TabList = $Tabs -split ','
$root = $PSScriptRoot
$dist = Join-Path $root 'dist\LOADOUT'
$transfer = Join-Path (Split-Path $root -Parent) 'transfer'
$out = Join-Path $transfer 'loshots'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$log = Join-Path $out 'shots.log'
"=== $((Get-Date).ToUniversalTime().ToString('u'))" | Set-Content $log
# Never Stop-Process an app someone is using.
# (Add-Content, not Tee-Object: in PowerShell 5.1 Tee appends UTF-16, which a literal match on the log then misses.)
function Note($m) { $m | Add-Content $log; Write-Output $m }
if (Get-Process loadout -ErrorAction SilentlyContinue) { Note "REFUSED: LOADOUT is running - close it and rerun"; exit 1 }
if (-not (Test-Path (Join-Path $dist 'loadout.exe'))) { Note "REFUSED: no $dist\loadout.exe - run ship.ps1 -NoRelease first"; exit 1 }
Copy-Item (Join-Path $root 'qml\*') (Join-Path $dist 'qml') -Recurse -Force

# The scratch Options dir and profiles folder every capture reads.
$scratch = Join-Path $env:TEMP ("loadout-shots-" + $PID)
$opts = Join-Path $scratch 'Options'
$profiles = Join-Path $scratch 'profiles'
$none = Join-Path $scratch 'no-profiles-yet'
Remove-Item $scratch -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $opts | Out-Null
Copy-Item (Join-Path $root 'fixtures\devmachine02\Graphics') $opts -Recurse
Copy-Item (Join-Path $root 'fixtures\devmachine02\Bindings') $opts -Recurse
$cli = Join-Path $dist 'ed-loadout.exe'
& $cli capture --options $opts --out (Join-Path $profiles 'devmachine02') --machine devmachine02 2>&1 | Select-Object -Last 1 | Add-Content $log
& $cli capture --options $opts --out (Join-Path $profiles 'fieldlaptop') --machine fieldlaptop 2>&1 | Select-Object -Last 1 | Add-Content $log
# One key set for this machine, so WILL APPLY shows both sources: derived and [machine.<host>].
Add-Content -Encoding ascii (Join-Path $profiles 'devmachine02\profile.toml') ("`n[machine." + $env:COMPUTERNAME + "]`nmax_fps = 60`n")

function Launch([string[]]$argv, [string]$png, [int]$ms, [string]$exe = (Join-Path $dist 'loadout.exe')) {
    $runLog = Join-Path (Split-Path $exe -Parent) 'loadout.log'
    Remove-Item $png, $runLog -ErrorAction SilentlyContinue
    if (-not (Test-Path $exe)) { "no $exe" | Add-Content $log; return }
    $all = @('--options', "`"$opts`"", '--shot', "`"$png`"", '--quit-after', "$ms") + $argv
    $p = Start-Process -FilePath $exe -ArgumentList $all -WorkingDirectory (Split-Path $exe -Parent) -WindowStyle Normal -PassThru
    $t0 = Get-Date
    while ($p.MainWindowHandle -eq [IntPtr]::Zero -and ((Get-Date) - $t0).TotalSeconds -lt 8) { Start-Sleep -Milliseconds 200; $p.Refresh() }
    # 0x15 = NOSIZE|NOZORDER|NOACTIVATE: on screen at 40,40, at the window's own 1320.
    if ($script:Win -and $p.MainWindowHandle -ne [IntPtr]::Zero) { [LoadoutShots.Win]::SetWindowPos($p.MainWindowHandle, [IntPtr]::Zero, 40, 40, 0, 0, 0x15) | Out-Null }
    if (-not $p.WaitForExit($ms + 20000)) { $p.Kill() }
    Get-Content $runLog -ErrorAction SilentlyContinue | Where-Object { $_ -match 'WARN|CRIT|shot' } | Add-Content $log
    Remove-Item $runLog -ErrorAction SilentlyContinue
}

if ($Survey) {
    if ($Survey -notmatch '^[a-z0-9-]+$') { throw "survey tag must match [a-z0-9-]+" }
    Add-Type -AssemblyName System.Drawing
    Add-Type -Namespace LoadoutShots -Name Win -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
'@
    $script:Win = $true
    $dir = Join-Path $root "_shots\uipass\survey-$Survey"
    if (Test-Path $dir) { Get-ChildItem $dir -Force | Remove-Item -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    $jpegCodec = [System.Drawing.Imaging.ImageCodecInfo]::GetImageEncoders() | Where-Object { $_.MimeType -eq 'image/jpeg' }
    $sha = (git -C $root rev-parse --short HEAD)
    $ini = Join-Path $env:APPDATA 'Boring Tech Stuff\LOADOUT.ini'
    $iniHash = if (Test-Path $ini) { (Get-FileHash $ini).Hash } else { 'absent' }
    $rows = New-Object System.Collections.Generic.List[string]
    $missing = 0

    function Keep([string]$png, [string]$screen, [string]$theme, [string]$qml, [string]$what) {
        $name = "survey-$Survey-$screen-$theme-desk.jpg"
        "--- $name" | Add-Content $log
        if (-not (Test-Path $png)) { "MISSING $name" | Add-Content $log; $script:missing++; return }
        $img = [System.Drawing.Image]::FromFile($png)
        $px = "$($img.Width)x$($img.Height)"
        $jpg = Join-Path $dir $name
        try {
            foreach ($q in 85, 75, 65, 55) {
                $ep = New-Object System.Drawing.Imaging.EncoderParameters 1
                $ep.Param[0] = New-Object System.Drawing.Imaging.EncoderParameter ([System.Drawing.Imaging.Encoder]::Quality), ([long]$q)
                $img.Save($jpg, $jpegCodec, $ep)
                if ((Get-Item $jpg).Length -le 400KB) { break }
            }
        } finally { $img.Dispose() }
        $kb = [math]::Round((Get-Item $jpg).Length / 1KB)
        $rows.Add("| ``$name`` | $screen | $theme | desk (1320) | $px, $kb KB | $qml | $what |")
    }
    function Shoot([string]$screen, [string]$theme, [string[]]$argv, [string]$qml, [string]$what, [int]$ms = 5000, [string]$exe = (Join-Path $dist 'loadout.exe')) {
        $png = Join-Path $env:TEMP 'loadout-survey.png'
        Launch (@('--theme', $theme) + $argv) $png $ms $exe
        Keep $png $screen $theme $qml $what
        Remove-Item $png -ErrorAction SilentlyContinue
    }

    $main = "``qml/Main.qml``, ``qml/Rail.qml``, ``qml/Theme.qml``"
    $with = @('--profiles', "`"$profiles`"", '--select', 'devmachine02')
    try {
        foreach ($th in 'deepfield', 'daylight') {
            Shoot 'home-empty' $th @('--profiles', "`"$none`"", '--tab', 'home') "``qml/HomePage.qml``, ``qml/ProfilesPage.qml``, $main" 'first open: no profile folder yet'
            Shoot 'home' $th ($with + @('--tab', 'home')) "``qml/HomePage.qml``, ``qml/ProfilesPage.qml``, $main" 'devmachine02 selected: derived rows and one [machine] row'
            Shoot 'profiles' $th ($with + @('--tab', 'profiles')) "``qml/ProfilesPage.qml``, $main" 'two profiles, the files of the selected one'
            Shoot 'settings' $th ($with + @('--tab', 'settings')) "``qml/SettingsPage.qml``, ``qml/Field.qml``, $main" 'folders, theme, about'
            Shoot 'confirm' $th ($with + @('--tab', 'home', '--confirm')) "``qml/ConfirmApply.qml``, ``qml/HomePage.qml``, $main" 'the APPLY confirm: files and backup folder'
        }
        # LOG last: these two runs really apply (into the scratch Options dir), so the page holds real lines.
        foreach ($th in 'deepfield', 'daylight') {
            Shoot 'log' $th ($with + @('--tab', 'home', '--script', 'apply,log')) "``qml/LogPage.qml``, $main" 'after APPLY: the lines the CLI prints' 6000
        }
        # The installed exe's HOME, from ship.ps1's installer check (criterion 6).
        $installed = Join-Path $transfer 'loadout-installed-home.png'
        if (Test-Path $installed) { Keep $installed 'installed-home' 'deepfield' "``qml/HomePage.qml``, $main" "the INSTALLED exe (%LOCALAPPDATA%\LOADOUT), captured by ship.ps1 $((Get-Item $installed).LastWriteTimeUtc.ToString('u'))" }
    } finally {
        Remove-Item $scratch -Recurse -Force -ErrorAction SilentlyContinue
    }
    $iniNow = if (Test-Path $ini) { (Get-FileHash $ini).Hash } else { 'absent' }
    $iniOk = $iniNow -eq $iniHash
    "LOADOUT.ini untouched: $iniOk ($iniHash)" | Add-Content $log

    $md = @("# UI survey ``$Survey``", '',
            "Packaged exe built from ``$sha``, captured $((Get-Date).ToUniversalTime().ToString('u')) on $env:COMPUTERNAME. DETECTED is this machine, read live. The Options dir is a scratch copy of ``fixtures/devmachine02``; the profiles are two captures of it made with the packaged ``ed-loadout.exe`` (``devmachine02`` carries ``[machine.$env:COMPUTERNAME] max_fps = 60``). The LOG captures follow a real APPLY into the scratch dir. Desk posture, 1320 wide. Produced by ``shots.ps1 -Survey $Survey``.",
            '', '| File | Screen | Theme | Posture | Pixels, size | QML | State |', '|---|---|---|---|---|---|---|') + $rows
    $md | Set-Content -Encoding utf8 (Join-Path $dir 'index.md')
    Note "survey $Survey`: $($rows.Count) captures in $dir"
    if ($missing -eq 0 -and $iniOk) { Note "=== SURVEY OK ==="; exit 0 }
    Note "=== SURVEY FAILED ($missing missing, LOADOUT.ini untouched: $iniOk) ==="
    exit 1
}
foreach ($t in $TabList) {
    $png = Join-Path $out ("$t" + $(if ($Theme) { "-$Theme" } else { "" }) + ".png")
    "--- $t" | Add-Content $log
    $argv = @('--profiles', "`"$profiles`"", '--select', 'devmachine02', '--tab', $t)
    if ($Theme) { $argv += @('--theme', $Theme) }
    Launch $argv $png 5000
}
Remove-Item $scratch -Recurse -Force -ErrorAction SilentlyContinue
"=== done" | Add-Content $log
