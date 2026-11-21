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

Exit codes: `0` ok / identical, `1` differences or problems found, `2` refused
(game running) or bad arguments, `3` error.

The examples below run against the devmachine02 corpus in `fixtures/devmachine02`
(RTX 5090, 5120x1440 @ 120 Hz).

## capture

```
ed-loadout capture --options <Options dir> --out <profile dir> --machine <name>
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
ed-loadout check-binds <Bindings dir> (--devices <GUID,...> | --from-log) [--binds <file>]
```

Elite rewrites `StartPreset.4.start` when a bound device is missing at launch, so that
file is never used to find the binds file. The binds file is, in order: `--binds` if
given; the file named on line 1 of `BindingLoadingErrors.log` if it exists; the newest
`Custom.4.*.binds`. The resolved path is printed.

Every `Device="..."` in the binds file other than `Keyboard`, `Mouse` and `{NoDevice}` is
checked against `--devices` (the GUIDs connected now), or the missing GUIDs are taken
from the `Failed to find GUID for device:` lines of the log with `--from-log`. It also
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
ed-loadout restore-preset <Bindings dir> [--preset Custom]
```

Copies `StartPreset.4.start` to `StartPreset.4.start.bak-<timestamp>`, then rewrites it
as four lines of the preset name, CRLF. Plug the device back in first.

```
$ ed-loadout restore-preset Options/Bindings
backup: Options/Bindings/StartPreset.4.start.bak-20261121-174541-298
wrote Options/Bindings/StartPreset.4.start (Custom x4, CRLF)
```

## apply

```
ed-loadout apply <profile> --options <Options dir> --machine <name>
```

Refuses with exit 2 while `EliteDangerous64.exe` is running (checked with `tasklist`
on Windows; other platforms print a notice and skip the check). Otherwise: copies
every file it will overwrite to `<Options>/_loadout-backup/<timestamp>/`, writes the
profile files, and applies the `[machine.<name>]` overrides from `profile.toml` by
replacing element text in place. No other byte changes. An override whose element is
absent is an error and nothing is written; elements are never inserted.

| Override | Element(s) |
|---|---|
| `ssaa` (written `%.6f`) | `<SSAAMultiplier>` in `Graphics/Custom.*.fxcfg` |
| `screen_width`, `screen_height` | `<ScreenWidth>`, `<ScreenHeight>` in `Graphics/DisplaySettings.xml` |
| `refresh_hz` | `<DX11_RefreshRateNumerator>`, and `<DX11_RefreshRateDenominator>` set to `1` |
| `max_fps` | `<MaxFramesPerSecond>`, and `<LimitFrameRate>` set to `true` |

Render scale is per GPU: do not copy devmachine02's SSAA 1.5 to a smaller card, give
that machine its own `ssaa`.

```
$ cat >> devmachine02-profile/profile.toml
[machine.laptop]
ssaa = 1.25
max_fps = 120

$ ed-loadout apply devmachine02-profile --options Options --machine laptop
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
cargo clippy -- -D warnings
```
