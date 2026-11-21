# devmachine02 fixture corpus (captured by hand 2026-11-20)

Real files from Graham's game box after the flat-fidelity pass: RTX 5090, 5120x1440 @ 120 Hz, no VRR, ED-UP 0.22.3 (DLAA preset K). Line endings in this repo are LF; the live `Custom.4.2.binds` is CRLF, the live `.fxcfg` is LF. Tests that need CRLF generate it.

| File | Status |
|---|---|
| `Graphics/Custom.4.4.fxcfg` | real, verbatim (SSAA 1.5, Terrain 4, Terrain Work 1.0, AA Off, Upscaling Off) |
| `Graphics/Settings.xml` | real, verbatim |
| `Graphics/DisplaySettings.xml` | real, verbatim (VSync on, cap 144 > 120 Hz refresh — deliberately left as found) |
| `Graphics/StartPreset.start` | real (`High`) |
| `Graphics/GraphicsConfigurationOverride.xml` | real, verbatim (beyond-Ultra override of 2026-09-18/19) |
| `Bindings/StartPreset.4.start` | real, the repaired state (`Custom` x4) |
| `Bindings/StartPreset.4.start.broken` | real, as Elite rewrote it at 13:06:56 CT when the Xbox pad (045E0B22) was absent at launch |
| `Bindings/BindingLoadingErrors.log` | real, first six lines of the 122-line log |
| `Bindings/Custom.4.2.binds` | REDUCED: first ~160 lines of the 790-binding file plus three 045E0B22 bindings and the closing tag. Device GUIDs present: 231D0126 (VKB L), 231D0127 (VKB R), 045E0B22 (Xbox pad, 3 bindings), Keyboard, {NoDevice}. 231D2204 is bound in the live file but not in this cut. |

Do not edit these to make a test pass.
