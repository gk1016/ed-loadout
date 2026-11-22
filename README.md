# ed-loadout (LOADOUT)

Capture, diff and apply an Elite Dangerous settings bundle across machines, with
per-machine overrides, backups and a bound-device check. No game-memory reads, no
hooks, no network. TacOps / Boring Tech Stuff.

The bundle is these files under the Elite Options dir
(`%LOCALAPPDATA%\Frontier Developments\Elite Dangerous\Options`):

- `Graphics/Custom.*.fxcfg`, `Graphics/Settings.xml`, `Graphics/DisplaySettings.xml`,
  `Graphics/StartPreset.start`, `Graphics/GraphicsConfigurationOverride.xml`
- `Bindings/*.binds`, `Bindings/StartPreset.*.start`

ED-UP `edvr*.ini` is not in v0.

## Install

Download `ed-loadout.exe` from the latest
[GitHub Release](https://github.com/gk1016/ed-loadout/releases/latest). Put it in a
folder on your `PATH`, or run it from the Options dir. It is a console program: run it
from PowerShell or `cmd`, not by double-clicking.

Without `--options` (or the `<Bindings dir>` argument of `check-binds` and
`restore-preset`), `capture`, `apply`, `check-binds` and `restore-preset` use
`%LOCALAPPDATA%\Frontier Developments\Elite Dangerous\Options` (and its `Bindings`
dir) and prints the path it resolved. If `LOCALAPPDATA` is unset or that dir does not
exist it exits 3 and names the path it tried.

## Two machines

On the desktop, with the game closed:

```
ed-loadout capture --out desk --machine desk
```

Copy the `desk` folder to the laptop (USB stick, network share, anything). On the
laptop, with the game closed and the controllers plugged in:

```
ed-loadout apply desk
```

`apply` reads the laptop's panel, GPU and connected controllers, derives the
resolution, refresh, frame cap, Supersampling, terrain quality and override XML from
the tier table (see [Hands-off](#hands-off)), prints every value with its reason,
warns for each device the desktop's bindings use that is not connected, and applies.
Nothing to type. To pin a value, add a section named after the laptop's hostname to
the end of `desk\profile.toml`; its keys win over the derived ones, one by one:

```
[machine.FIELDLAPTOP]
ssaa = 1.25
```

`ed-loadout detect` shows what `apply` would derive without writing anything. To
refuse the apply when a bound device is missing, add `--strict-binds`.

Before writing, `apply` copies every file it is about to overwrite into
`Options\_loadout-backup\<timestamp>\`, laid out like the Options dir. To undo, copy
the `Graphics` and `Bindings` folders from that timestamp folder back over the ones in
the Options dir (a file the laptop did not have before is not in the backup; delete it
by hand if you want it gone).

## Hands-off

On Windows, `detect`, `apply` and `check-binds` (without `--devices` / `--from-log`)
read three facts from the machine itself:

- the primary display's current mode (`EnumDisplaySettingsExW`): width, height,
  refresh rate;
- the first hardware GPU (DXGI): name and dedicated VRAM;
- the connected HID devices (Raw Input), as Elite's binds `Device` GUID `VVVVPPPP`
  (USB vendor + product, upper-case hex: `045E0B22` an Xbox pad, `231D....` VKB).

The hostname is `%COMPUTERNAME%`. On other platforms, and to try a machine you are not
sitting at, pass `--facts <file>`: the same facts as TOML, as written by
`ed-loadout detect --dump <file>`:

```
hostname = "devmachine02"
screen_w = 5120
screen_h = 1440
refresh_hz = 120
gpu_name = "NVIDIA GeForce RTX 5090"
gpu_vram_mb = 32607
devices = ["231D0126", "231D0127", "231D2204", "045E0B22"]
```

From the facts:

| Tier | GPU name contains | `ssaa` | `terrain_quality` | `override_tier` |
|---|---|---|---|---|
| T1 | `RTX 5090`, `RTX 5080`, `RTX 4090` (not `Laptop`) | budget 16.6 Mpx | 4 | `t1` |
| T2 | `RTX 5090 Laptop`, `RTX 5080 Laptop`, `RTX 5070 Ti`, `RTX 4080` (`SUPER`), `RTX 4070 Ti SUPER` | budget 10.0 Mpx | 3 | `t2` |
| T3 | anything else | 1.0 | 3 | `none` |

`screen_width`, `screen_height` and `refresh_hz` come from the panel and `max_fps` =
`refresh_hz` (VSync stays on; VRR is not detected). For T1 and T2, `ssaa` =
sqrt(budget / (width x height)), rounded to 0.05 and clamped to 1.0-1.5. The T1 budget
is measured: devmachine02, a desktop RTX 5090, holds a flat 120 Hz at 5120x1440 with
Supersampling 1.5 (~16.6 Mpx rendered) at 81-85% GPU use. Every derived Supersampling
is a starting point, not a tuned value: raise it while GPU use stays under ~85%.

`t1` is devmachine02's beyond-Ultra `GraphicsConfigurationOverride.xml`
(`tiers/t1.xml`); `t2` is the same without the `DirectionalShadows_Ultra`,
`SpotShadows_Ultra` and `Volumetrics` blocks (`tiers/t2.xml`); `none` leaves the
target's override file alone.

Any key in `[machine.<name>]` (name = `--machine`, else the hostname) overrides the
derived value for that key only. `apply` prints each effective value with its source,
`derived` or `[machine.<name>]`. With no facts and no `[machine.<name>]`, `apply`
writes the profile exactly as captured.

## Exit codes

Exit codes: `0` ok / identical, `1` differences or problems found, `2` refused
(game running) or bad arguments, `3` error.

The examples below run against the devmachine02 corpus in `fixtures/devmachine02`
(RTX 5090, 5120x1440 @ 120 Hz).

## capture

```
ed-loadout capture [--options <Options dir>] --out <profile dir> --machine <name>
```

Copies the bundle byte-for-byte (line endings untouched) into `<profile>/files/` and
writes `<profile>/profile.toml` with `source_machine`, `captured_at` (RFC 3339) and a
sha256 per file. Refuses to write over an existing profile.

```
$ ed-loadout capture --options Options --out devmachine02-profile --machine devmachine02
captured Bindings/Custom.4.2.binds
...
wrote devmachine02-profile/profile.toml (7 files)

$ cat devmachine02-profile/profile.toml
source_machine = "devmachine02"
captured_at = "2026-11-21T17:45:41Z"

[files]
"Graphics/Custom.4.4.fxcfg" = "36d2898de631ae53362c58edd23ee937ef7f5e7c049c853d8a591fdbb2a69e32"
...
```

## diff

```
ed-loadout diff <profile-a> <profile-b>
```

For `Graphics/Custom.*.fxcfg`, `Settings.xml` and `DisplaySettings.xml`, one line per
changed element (`<path>: <Element> <old> -> <new>`). For `.start`, `.binds` and
`GraphicsConfigurationOverride.xml`, a unified diff. Files on one side only are listed
as `only in a:` / `only in b:`. Exit 0 identical, 1 different.

```
$ ed-loadout diff devmachine02-profile laptop-profile
Graphics/Custom.4.4.fxcfg: SSAAMultiplier 1.500000 -> 1.250000
```

## check-binds

```
ed-loadout check-binds [<Bindings dir>] [--devices <GUID,...> | --from-log | --facts <file>] [--binds <file>]
```

Elite rewrites `StartPreset.4.start` when a bound device is missing at launch, so that
file is never used to find the binds file. The binds file is, in order: `--binds` if
given; the file named on line 1 of `BindingLoadingErrors.log` if it exists; the newest
`Custom.4.*.binds`. The resolved path is printed.

Every `Device="..."` in the binds file other than `Keyboard`, `Mouse` and `{NoDevice}` is
checked against `--devices` (the GUIDs connected now), or the missing GUIDs are taken
from the `Failed to find GUID for device:` lines of the log with `--from-log`. With
neither, the connected devices are detected (Windows) or read from `--facts`; on other
platforms without `--facts` it exits 3. It also
warns for each of the four lines of `StartPreset.4.start` that is not `Custom`. Exit 1
when a device is missing or a preset line is not `Custom`.

```
$ ed-loadout check-binds Options/Bindings --devices 231D0126,231D0127,231D2204
binds: Options/Bindings/Custom.4.2.binds (from BindingLoadingErrors.log)
missing device: 045E0B22 (3 bindings)

$ ed-loadout check-binds Options/Bindings --from-log     # after the 2026-11-20 launch without the pad
binds: Options/Bindings/Custom.4.2.binds (from BindingLoadingErrors.log)
missing device: 045E0B22 (3 bindings)
warning: StartPreset.4.start line 1: "KeyboardMouseOnly", not Custom
warning: StartPreset.4.start line 2: "GenericJoystick", not Custom
warning: StartPreset.4.start line 3: "KeyboardMouseOnly", not Custom
warning: StartPreset.4.start line 4: "KeyboardMouseOnly", not Custom
```

## restore-preset

```
ed-loadout restore-preset [<Bindings dir>] [--preset Custom]
```

Copies `StartPreset.4.start` to `StartPreset.4.start.bak-<timestamp>`, then rewrites it
as four lines of the preset name, CRLF. Plug the device back in first.

```
$ ed-loadout restore-preset Options/Bindings
backup: Options/Bindings/StartPreset.4.start.bak-20261121-174541-298
wrote Options/Bindings/StartPreset.4.start (Custom x4, CRLF)
```

## detect

```
ed-loadout detect [--facts <file>] [--dump <file>]
```

Prints the facts (detected on Windows, else from `--facts`), the tier and every
derived value with its reason. `--dump` writes the facts as TOML to a new file (never
over an existing one). Off Windows without `--facts` it exits 3.

```
$ ed-loadout detect --facts tests/facts/laptop-5090m.toml
hostname: fieldlaptop
panel: 2560x1600 @ 240 Hz
gpu: NVIDIA GeForce RTX 5090 Laptop GPU (24576 MB)
devices: 231D0126, 231D0127, 045E0B22
tier: T2
derived screen_width = 2560  (panel 2560x1600)
derived screen_height = 1600  (panel 2560x1600)
derived refresh_hz = 240  (panel refresh rate)
derived max_fps = 240  (= refresh_hz; VSync stays on, no VRR detection)
derived ssaa = 1.5  (T2 budget 10.0 Mpx over 2560x1600, 1.55 clamped to 1.5; starting point: raise while GPU use stays under ~85%)
derived terrain_quality = 3  (T2)
derived override_tier = t2  (T2: t1 without DirectionalShadows_Ultra, SpotShadows_Ultra, Volumetrics)
```

## apply

```
ed-loadout apply <profile> [--options <Options dir>] [--machine <name>] [--facts <file>] [--strict-binds]
```

Refuses with exit 2 while `EliteDangerous64.exe` is running (checked with `tasklist`
on Windows; other platforms print a notice and skip the check). Otherwise: derives
values from the facts (see [Hands-off](#hands-off)), lays the `[machine.<name>]` keys
from `profile.toml` over them, copies every file it will overwrite to
`<Options>/_loadout-backup/<timestamp>/`, writes the profile files, and applies the
effective values by replacing element text in place. No other byte changes. An
override whose element is absent is an error and nothing is written; elements are
never inserted.

With facts, it first checks the profile's highest `Bindings/Custom.4.<n>.binds` (the
file it is about to write) against the connected devices and prints
`warning: bound device <GUID> (<n> bindings) is not connected` for each one missing,
then applies anyway. With `--strict-binds` a missing device is exit 1 and nothing is
written; `--strict-binds` without facts is exit 3. Without facts it prints
`binds check skipped (no device facts)`.

| Override | Element(s) |
|---|---|
| `ssaa` (written `%.6f`) | `<SSAAMultiplier>` in `Graphics/Custom.*.fxcfg` |
| `screen_width`, `screen_height` | `<ScreenWidth>`, `<ScreenHeight>` in `Graphics/DisplaySettings.xml` |
| `refresh_hz` | `<DX11_RefreshRateNumerator>`, and `<DX11_RefreshRateDenominator>` set to `1` |
| `max_fps` | `<MaxFramesPerSecond>`, and `<LimitFrameRate>` set to `true` |
| `terrain_quality` (`3` or `4`) | `<TerrainQuality>` in `Graphics/Custom.*.fxcfg` |
| `override_tier` (`"t1"`, `"t2"`, `"captured"`, `"none"`) | `Graphics/GraphicsConfigurationOverride.xml`: write `tiers/t1.xml` or `tiers/t2.xml`, the profile's own file (`captured`, the default without facts), or nothing (`none`, the target's file is left alone) |

The fxcfg target for `ssaa` and `terrain_quality` is the
`Graphics/Custom.<Major>.<Minor>.fxcfg` with the highest version, the one Elite reads
(printed as `fxcfg target:`); older versions left by game updates are written as
captured. Other `Custom.*fxcfg*` names (a `.bak`, say) are ignored with a notice. If
the Options dir already holds a newer `Custom.*.fxcfg` than the profile's (the target
machine is on a later game version), either key is an error and nothing is written.

Render scale is per GPU: do not copy devmachine02's SSAA 1.5 to a smaller card. With
facts, `apply` derives a starting value; without, give that machine its own `ssaa`.

```
$ cat >> devmachine02-profile/profile.toml
[machine.laptop]
ssaa = 1.25
max_fps = 120

$ ed-loadout apply devmachine02-profile --options Options --machine laptop
notice: no device facts on this platform; pass --facts <file>
effective max_fps = 120  ([machine.laptop])
effective ssaa = 1.25  ([machine.laptop])
binds check skipped (no device facts)
fxcfg target: Graphics/Custom.4.4.fxcfg
ssaa target: Graphics/Custom.4.4.fxcfg
override Graphics/Custom.4.4.fxcfg: SSAAMultiplier = 1.250000
override Graphics/DisplaySettings.xml: MaxFramesPerSecond = 120
override Graphics/DisplaySettings.xml: LimitFrameRate = true
backup: Options/_loadout-backup/20261121-174541-303 (7 files)
wrote Bindings/Custom.4.2.binds
...
```

## A note on ED-UP and DLSS

At Supersampling >= 1.0, ED-UP labels DLSS as DLAA, and that is correct.

## Build

```
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release --target x86_64-pc-windows-gnu   # ed-loadout.exe, needs mingw-w64
```
