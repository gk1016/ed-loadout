# LOADOUT (ed-loadout) — issue ledger

Permanent ledger for the app. Every session touching it starts here.
Append new findings; never re-file. Closed items stay closed, with their
evidence (test name or pasted command output).

Severity: HIGH (core loop broken) · MED (wrong but has a workaround) ·
LOW (cosmetic / polish) · OBS (observation, not confirmed a defect).

---

### OPEN

- OBS-001 2026-11-21 — devmachine02 `DisplaySettings.xml` has `MaxFramesPerSecond` 144 above the 120 Hz refresh; harmless with VSync on. Left as found in the fixture so `apply --machine` overrides have a real case.
- OBS-003 2026-11-21 — `.github/workflows/code-wave.yml` gives the `wave` job `contents: read`, so the headless wave cannot push its branch (`git push` -> 403 "Write access to repository not granted"; `gh api repos/gk1016/ed-loadout --jq .permissions` -> `"push":false`) and so cannot open its PR. The loadout-v0 wave's commits sit unpushed in `/home/gk1016/ed-loadout-code`. Probably the same in bts-template stacks/rust-only.

### CLOSED

- OBS-002 2026-11-21 — `apply` with an `ssaa` override refuses a profile that holds more than one `Graphics/Custom.*.fxcfg` ("expected exactly one"). Real Options dirs may keep old-version fxcfg files after game updates. Deliberate for v0; rule for choosing one (e.g. highest MajorVersion.MinorVersion) is Graham's call. Source: Codex check on loadout-v0, finding 3.
  Closed 2026-11-21 (plan `claude/plans/2026-11-21-loadout-v0.1.md` criterion 2): `apply` targets the highest `Custom.<Major>.<Minor>.fxcfg` by name, prints `ssaa target: ...`, ignores unparsable names with a notice, errors on zero. Test `v01_c2_apply_ssaa_targets_highest_version_fxcfg` (4.4 edited at line 31 only, 4.2 and `.bak` byte-identical, notice for the `.bak`); `v01_c2_apply_ssaa_with_no_parsable_fxcfg_is_an_error`.
