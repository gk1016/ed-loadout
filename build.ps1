# Build LOADOUT: cargo (build + test), cmake configure + build, restage QML.
# Same toolchain and the same hard-won rules as FRAGO's build.ps1: every exit
# code is checked, vcvars is imported here (an inherited empty LIB dies at the
# link and deletes the exe), and success prints a TIMESTAMP.
$ErrorActionPreference = 'Continue'
$root = $PSScriptRoot
$log  = Join-Path $root 'build.log'
Remove-Item $log -ErrorAction SilentlyContinue
function Say($m) { $m | Tee-Object -FilePath $log -Append }
Set-Location $root

$pyScripts = if ($env:HUMINT_PY_SCRIPTS) { $env:HUMINT_PY_SCRIPTS } else { Join-Path $env:LOCALAPPDATA 'Python\pythoncore-3.14-64\Scripts' }
$env:PATH = $pyScripts + ';' + (Join-Path $env:USERPROFILE '.cargo\bin') + ';' + $env:PATH
$vcvars = if ($env:HUMINT_VCVARS) { $env:HUMINT_VCVARS } else { Join-Path ${env:ProgramFiles} 'Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat' }
if (-not $env:LIB) {
    if (-not (Test-Path $vcvars)) { Say "NO vcvars64.bat at $vcvars"; exit 1 }
    cmd /c "`"$vcvars`" >nul 2>&1 && set" | ForEach-Object {
        if ($_ -match '^([^=]+)=(.*)$') { Set-Item -Path ("env:" + $matches[1]) -Value $matches[2] -ErrorAction SilentlyContinue }
    }
}
Say ("msvc env: LIB entries=" + ($env:LIB -split ';' | Where-Object { $_ }).Count)

# The static library for loadout.exe and the CLI, ed-loadout.exe.
Say "=== cargo build ==="
Push-Location (Join-Path $root 'rust')
cargo build --release 2>&1 | Select-Object -Last 30 | Tee-Object -FilePath $log -Append
$ok = $LASTEXITCODE
Pop-Location
Say ("cargo exit: " + $ok)
if ($ok -ne 0) { Say "=== BUILD FAILED (rust) ==="; exit 1 }

# LOADOUT_NO_DETECT=1: with no --facts the tests must see no facts, not this
# box's panel and GPU (facts::load). Debug profile: the fake game-running
# switch is read in debug builds only, and its test checks exactly that.
Say "=== cargo test ==="
Push-Location (Join-Path $root 'rust')
$env:LOADOUT_NO_DETECT = '1'
cargo test 2>&1 | Where-Object { "$_" -match '^test result|^test .* FAILED|panicked|^error|^failures' } | Tee-Object -FilePath $log -Append
$ok = $LASTEXITCODE
Remove-Item Env:\LOADOUT_NO_DETECT
Pop-Location
Say ("cargo test exit: " + $ok)
if ($ok -ne 0) { Say "=== BUILD FAILED (rust tests) ==="; exit 1 }

Say "=== cmake configure ==="
$qt = if ($env:HUMINT_QT) { $env:HUMINT_QT } else { 'C:\Qt\6.8.3\msvc2022_64' }
cmake -S $root -B (Join-Path $root 'build') -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH="$qt" 2>&1 | Select-Object -Last 20 | Tee-Object -FilePath $log -Append
$ok = $LASTEXITCODE
Say ("configure exit: " + $ok)
if ($ok -ne 0) { Say "=== BUILD FAILED (configure) ==="; exit 1 }

Say "=== cmake build ==="
cmake --build (Join-Path $root 'build') 2>&1 | Select-Object -Last 40 | Tee-Object -FilePath $log -Append
$ok = $LASTEXITCODE
Say ("build exit: " + $ok)
if ($ok -ne 0) { Say "=== BUILD FAILED (cmake build) ==="; exit 1 }

# A QML-only edit relinks nothing, so POST_BUILD copies nothing: restage.
Copy-Item (Join-Path $root 'qml\*') (Join-Path $root 'build\qml') -Recurse -Force
$exe = Join-Path $root 'build\loadout.exe'
$cli = Join-Path $root 'rust\target\release\ed-loadout.exe'
if (-not (Test-Path $exe)) { Say "NO EXE"; exit 1 }
if (-not (Test-Path $cli)) { Say "NO CLI ($cli)"; exit 1 }
Say ("EXE " + (Get-Item $exe).LastWriteTime.ToString('yyyy-MM-dd HH:mm:ss') + "  " + $exe)
Say ("CLI " + (Get-Item $cli).LastWriteTime.ToString('yyyy-MM-dd HH:mm:ss') + "  " + $cli)
Say "=== BUILD DONE ==="
exit 0
