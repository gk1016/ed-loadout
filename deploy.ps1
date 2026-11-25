# Stage the Qt runtime beside the exe (windeployqt). Its failure on Windows is
# a modal dialog at launch, so the platform plugin is checked here instead.
$ErrorActionPreference = 'Continue'
$root = $PSScriptRoot
$qt = if ($env:HUMINT_QT) { $env:HUMINT_QT } else { 'C:\Qt\6.8.3\msvc2022_64' }
& (Join-Path $qt 'bin\windeployqt.exe') --qmldir "$root\qml" --release --no-translations --no-system-d3d-compiler `
    --skip-plugin-types qmltooling "$root\build\loadout.exe" 2>&1 | Select-Object -Last 12
$ok = $LASTEXITCODE
if ($ok -ne 0) { Write-Output "=== DEPLOY FAILED (windeployqt exit $ok) ==="; exit $ok }
if (-not (Test-Path "$root\build\platforms\qwindows.dll")) { Write-Output "=== DEPLOY FAILED (no platforms\qwindows.dll) ==="; exit 1 }
Write-Output ("dlls: " + (Get-ChildItem "$root\build" -Filter *.dll).Count)
Write-Output "=== DEPLOY DONE ==="
exit 0
