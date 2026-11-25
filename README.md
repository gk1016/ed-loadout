# LOADOUT

Carry your Elite Dangerous settings from one machine to another. LOADOUT reads what
the machine in front of you is (panel, GPU, connected controllers), shows what it will
write for it, writes it on APPLY, and keeps a backup of everything it replaced. No
game-memory reads, no hooks, no network. TacOps / Boring Tech Stuff.

The settings bundle is these files under the Elite Options dir
(`%LOCALAPPDATA%\Frontier Developments\Elite Dangerous\Options`):

- `Graphics/Custom.*.fxcfg`, `Graphics/Settings.xml`, `Graphics/DisplaySettings.xml`,
  `Graphics/StartPreset.start`, `Graphics/GraphicsConfigurationOverride.xml`
- `Bindings/*.binds`, `Bindings/StartPreset.*.start`

ED-UP `edvr*.ini` is not in the bundle.

## Install

Download `LOADOUT_Setup.exe` from the latest
[GitHub Release](https://github.com/gk1016/ed-loadout/releases/latest) and run it. It
installs for the current user into `%LOCALAPPDATA%\LOADOUT` (no administrator prompt)
and adds LOADOUT to the Start menu. Open LOADOUT from there: it is a windowed app, not
a console program.

Uninstall from Windows Settings > Apps. Your profiles (`%LOCALAPPDATA%\LOADOUT\profiles`
by default) are yours and are left in place.

## What the screen shows

One window:

- **Command bar** (top): the machine's hostname, GPU, panel (`width x height @ Hz`),
  its tier (T1, T2 or T3) and the app version.
- **Tool rail** (left): HOME, PROFILES, LOG, and SETTINGS at the bottom.
- **HOME**: the PROFILES pane on the left, and on the right:
  - **DETECTED**: what LOADOUT read from this machine just now: hostname, panel,
    refresh rate, GPU, VRAM, tier, and each connected controller as the ID Elite's
    bindings use (`045E0B22` is an Xbox pad, `231D....` is VKB).
  - **WILL APPLY**: for the selected profile, one row per setting LOADOUT will set:
    the key, the value, where the value comes from (`derived` from this machine, or
    a `[machine.<hostname>]` section in the profile) and why. Under it, every file
    that will be written, and in red every controller the profile's bindings use that
    is not connected right now.
  - **DETECT AGAIN**, **APPLY <profile>**, **RESTORE LAST BACKUP**.
- **PROFILES**: the same list with the selected profile's files and their SHA-256.
- **LOG**: the lines LOADOUT printed for what it did, the same lines the command line
  prints.
- **Status bar** (bottom): the Options dir in use, the stamp of the last backup,
  whether the game is running (red when it is), and the build.

While `EliteDangerous64.exe` is running, APPLY and RESTORE are off: the game rewrites
these files on exit.

## Two machines

On the desktop, with the game closed: open LOADOUT and press **CAPTURE THIS MACHINE**.
That creates a profile named after the desktop in the profiles folder
(`%LOCALAPPDATA%\LOADOUT\profiles\<hostname>`).

Copy that profile folder to the laptop (USB stick, network share, anything). On the
laptop, with the game closed and the controllers plugged in: open LOADOUT, press
**IMPORT FOLDER** and pick the copied folder, select the profile, read WILL APPLY, and
press **APPLY**. A confirm lists every file that will be written and the backup folder;
**APPLY NOW** writes them.

Nothing to type: the laptop's resolution, refresh rate, frame cap, Supersampling,
terrain quality and override XML are derived from the laptop itself (see
[How the values are derived](#how-the-values-are-derived)). To pin a value for one
machine, add a section named after its hostname to the end of the profile's
`profile.toml`; its keys win over the derived ones, one by one, and WILL APPLY shows
them with that section as their source:

```
[machine.FIELDLAPTOP]
ssaa = 1.25
```

## Backups and restore

Every APPLY first creates `Options\_loadout-backup\<timestamp>\`, copies into it every
file it is about to overwrite (laid out like the Options dir), and writes
`manifest.toml` there: every file the apply writes, and whether it existed before.

**RESTORE LAST BACKUP** undoes the last APPLY from that folder: files that existed
before are put back byte-for-byte, and files the apply added are removed. What the
restore itself replaces is copied to its own timestamp folder first, so nothing is
ever lost. Backup folders are never deleted by LOADOUT.

A backup folder made before v0.3 has no `manifest.toml`; copy its `Graphics` and
`Bindings` folders back by hand.

## Settings

- **Profiles folder**: where profiles are listed, captured and imported. Default
  `%LOCALAPPDATA%\LOADOUT\profiles`.
- **Options dir override**: set it only if Elite's Options dir is not at the default
  path. The dir in use is shown under it and in the status bar.
- **Theme**: Deepfield (dark) or Daylight.

Settings are kept in `%APPDATA%\Boring Tech Stuff\LOADOUT.ini`.

## How the values are derived

LOADOUT reads three facts from the machine:

- the primary display's current mode (`EnumDisplaySettingsExW`): width, height,
  refresh rate;
- the first hardware GPU (DXGI): name and dedicated VRAM;
- the connected HID devices (Raw Input), as Elite's binds `Device` GUID `VVVVPPPP`
  (USB vendor + product, upper-case hex).

| Tier | GPU name contains | `ssaa` | `terrain_quality` | `override_tier` |
|---|---|---|---|---|
| T1 | `RTX 5090`, `RTX 5080`, `RTX 4090` (not `Laptop`) | budget 16.6 Mpx | 4 | `t1` |
| T2 | `RTX 5090 Laptop`, `RTX 5080 Laptop`; `RTX 5070 Ti`, `RTX 4080` (`SUPER`), `RTX 4070 Ti SUPER` (not `Laptop`) | budget 10.0 Mpx | 3 | `t2` |
| T3 | anything else, including every other `Laptop` GPU | 1.0 | 3 | `none` |

`screen_width`, `screen_height` and `refresh_hz` come from the panel. `max_fps` is the
refresh rate up to 170 Hz; above that it is the refresh rate divided by the smallest
whole number that brings it to 170 or under (240 -> 120, 360 -> 120, 175 -> 87;
165, 144 and 120 stay as they are), and the reason names the divisor. VRR is not
detected and `<VSync>` is never edited: it stays as captured.

For T1 and T2, `ssaa` = sqrt(budget / (width x height)), rounded to 0.05 and clamped
to 1.0-1.5. The T1 budget is measured: devmachine02, a desktop RTX 5090, holds a flat
120 Hz at 5120x1440 with Supersampling 1.5 (~16.6 Mpx rendered) at 81-85% GPU use.
Every derived Supersampling is a starting point, not a tuned value: raise it while GPU
use stays under ~85%. Render scale is per GPU: do not pin devmachine02's 1.5 on a
smaller card.

`t1` is devmachine02's beyond-Ultra `GraphicsConfigurationOverride.xml`
(`rust/tiers/t1.xml`); `t2` is the same without the `DirectionalShadows_Ultra`,
`SpotShadows_Ultra` and `Volumetrics` blocks (`rust/tiers/t2.xml`); `none` leaves the
target's override file alone.

What each key edits. Element text is replaced in place and no other byte changes; an
element that is absent is an error and nothing is written (elements are never
inserted):

| Key | Element(s) |
|---|---|
| `ssaa` (written `%.6f`) | `<SSAAMultiplier>` in `Graphics/Custom.*.fxcfg` |
| `screen_width`, `screen_height` | `<ScreenWidth>`, `<ScreenHeight>` in `Graphics/DisplaySettings.xml` |
| `refresh_hz` | `<DX11_RefreshRateNumerator>`, and `<DX11_RefreshRateDenominator>` set to `1` |
| `max_fps` | `<MaxFramesPerSecond>`, and `<LimitFrameRate>` set to `true` |
| `terrain_quality` (`3` or `4`) | `<TerrainQuality>` in `Graphics/Custom.*.fxcfg` |
| `override_tier` (`"t1"`, `"t2"`, `"captured"`, `"none"`) | `Graphics/GraphicsConfigurationOverride.xml`: `t1.xml` or `t2.xml`, the profile's own file (`captured`), or nothing (`none`) |

The fxcfg that `ssaa` and `terrain_quality` edit is the
`Graphics/Custom.<Major>.<Minor>.fxcfg` with the highest version, the one Elite reads.
If the Options dir already holds a newer one than the profile's (the target machine is
on a later game version), the apply is an error and nothing is written: capture from a
machine on the same game version.

## The command line: ed-loadout.exe

The engine under the app is also installed as `ed-loadout.exe`, beside `loadout.exe`,
for scripting. It is a console program: run it from PowerShell or `cmd`. Its flags are
the same as in v0.2.

```
ed-loadout capture [--options <Options dir>] --out <profile dir> --machine <name>
ed-loadout diff <profile-a> <profile-b>
ed-loadout detect [--facts <file>] [--dump <file>]
ed-loadout apply <profile> [--options <Options dir>] [--machine <name>] [--facts <file>] [--strict-binds]
ed-loadout check-binds [<Bindings dir>] [--devices <GUID,...> | --from-log | --facts <file>] [--binds <file>]
ed-loadout restore-preset [<Bindings dir>] [--preset Custom]
```

Exit codes: `0` ok / identical, `1` differences or problems found, `2` refused (game
running) or bad arguments, `3` error. Without `--options` (or the `<Bindings dir>`
argument) the default Options dir is used and printed; if `LOCALAPPDATA` is unset or
the dir does not exist it exits 3 and names the path it tried.

- **capture** copies the bundle byte-for-byte (line endings untouched) into
  `<profile>/files/` and writes `<profile>/profile.toml` with `source_machine`,
  `captured_at` (RFC 3339) and a sha256 per file. It refuses to write over an existing
  profile.
- **diff** prints one line per changed element for the flat graphics XML
  (`Graphics/Custom.4.4.fxcfg: SSAAMultiplier 1.500000 -> 1.250000`), a unified diff
  for `.start`, `.binds` and the override XML, and `only in a:` / `only in b:`.
- **detect** prints the facts, the tier and every derived value with its reason.
  `--facts <file>` supplies the facts as TOML instead of detecting (to try a machine
  you are not sitting at); `--dump <file>` writes them to a new file.
- **apply** is what the APPLY button runs: same derivation, same backup and
  `manifest.toml`, same lines. `--strict-binds` refuses (exit 1, nothing written) when
  a bound device is not connected. Restore is in the app; from the command line, copy
  the backup folder's files back.
- **check-binds** reports bound devices that are missing and a clobbered
  `StartPreset.4.start`. Elite rewrites that file when a bound device is missing at
  launch, so the binds file is taken from `--binds`, else line 1 of
  `BindingLoadingErrors.log`, else the newest `Custom.4.*.binds`.
- **restore-preset** copies `StartPreset.4.start` to
  `StartPreset.4.start.bak-<timestamp>`, then rewrites it as four lines of the preset
  name, CRLF. Plug the device back in first.

```
$ ed-loadout detect --facts rust/tests/facts/laptop-5090m.toml
hostname: fieldlaptop
panel: 2560x1600 @ 240 Hz
gpu: NVIDIA GeForce RTX 5090 Laptop GPU (24576 MB)
devices: 231D0126, 231D0127, 045E0B22
tier: T2
derived screen_width = 2560  (panel 2560x1600)
derived screen_height = 1600  (panel 2560x1600)
derived refresh_hz = 240  (panel refresh rate)
derived max_fps = 120  (= refresh_hz / 2 (k = 2, the smallest divisor that brings 240 Hz to 170 or under); VRR is not detected, VSync is left as captured)
derived ssaa = 1.5  (T2 budget 10.0 Mpx over 2560x1600, 1.55 clamped to 1.5; starting point: raise while GPU use stays under ~85%)
derived terrain_quality = 3  (T2)
derived override_tier = t2  (T2: t1 without DirectionalShadows_Ultra, SpotShadows_Ultra, Volumetrics)
```

`LOADOUT_NO_DETECT=1` makes the engine detect nothing (no `--facts` then means no
facts); the test suite sets it so a Windows build host's own hardware never leaks into
a test.

## A note on ED-UP and DLSS

At Supersampling >= 1.0, ED-UP labels DLSS as DLAA, and that is correct.

## Build

On Windows (EVO-X2), with MSVC, Qt 6.8.3 (`C:\Qt\6.8.3\msvc2022_64` or `HUMINT_QT`),
cargo and Inno Setup 6:

```
.\build.ps1                 # cargo build + cargo test (rust\), cmake + ninja -> build\loadout.exe
.\ship.ps1 -NoRelease       # build, package, run the packaged exe, scripted capture/apply/restore, installer check
.\shots.ps1 -Survey <tag>   # the UI survey -> _shots\uipass\survey-<tag>\
```

The Rust core alone: `cargo test` and `cargo clippy --all-targets -- -D warnings` from
`rust\` (set `LOADOUT_NO_DETECT=1` on Windows). The repo layout: `rust\` the core
(`loadout-core`: the library, the cxx bridge, the `ed-loadout` CLI), `cpp\` and `qml\`
the Qt shell, `fixtures\devmachine02\` the settings corpus the tests run against.
