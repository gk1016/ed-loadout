# LOADOUT (ed-loadout) — issue ledger

Permanent ledger for the app. Every session touching it starts here.
Append new findings; never re-file. Closed items stay closed, with their
evidence (test name or pasted command output).

Severity: HIGH (core loop broken) · MED (wrong but has a workaround) ·
LOW (cosmetic / polish) · OBS (observation, not confirmed a defect).

---

### OPEN

- OBS-001 2026-11-21 — devmachine02 `DisplaySettings.xml` has `MaxFramesPerSecond` 144 above the 120 Hz refresh; harmless with VSync on. Left as found in the fixture so `apply --machine` overrides have a real case.
- OBS-004 2026-11-21 — `apply` without an `ssaa` override copies the profile's `Custom.<M>.<m>.fxcfg` into an Options dir that may already hold a newer version; Elite reads the newer one, so the graphics preset silently does not take. With `ssaa` this is now an error (Codex check 1 on loadout-v0.1, `v01_c2_apply_ssaa_refuses_when_target_has_a_newer_fxcfg`); without it there is no notice.
- OBS-005 2026-11-21 — `ed-loadout.exe` (x86_64-pc-windows-gnu, `PE32+ executable (console) x86-64`) has only been built and type-checked on the WSL runner, never run: no wine on the runner. The `tasklist` game-running check and the `%LOCALAPPDATA%` default are unexercised on real Windows.
- OBS-006 2026-11-21 — `apply` backs up only files that already existed in the Options dir. A file it adds (e.g. an old-version fxcfg the target lacked) is not in `_loadout-backup`, so copying the backup back does not remove it. Documented in README Two machines.

### CLOSED

- OBS-002 2026-11-21 — `apply` with an `ssaa` override refuses a profile that holds more than one `Graphics/Custom.*.fxcfg` ("expected exactly one"). Real Options dirs may keep old-version fxcfg files after game updates. Deliberate for v0; rule for choosing one (e.g. highest MajorVersion.MinorVersion) is Graham's call. Source: Codex check on loadout-v0, finding 3.
  Closed 2026-11-21 (plan `claude/plans/2026-11-21-loadout-v0.1.md` criterion 2): `apply` targets the highest `Custom.<Major>.<Minor>.fxcfg` by name, prints `ssaa target: ...`, ignores unparsable names with a notice, errors on zero. Test `v01_c2_apply_ssaa_targets_highest_version_fxcfg` (4.4 edited at line 31 only, 4.2 and `.bak` byte-identical, notice for the `.bak`); `v01_c2_apply_ssaa_with_no_parsable_fxcfg_is_an_error`.
- OBS-003 2026-11-21 — `.github/workflows/code-wave.yml` gives the `wave` job `contents: read`, so the headless wave cannot push its branch (`git push` -> 403 "Write access to repository not granted"; `gh api repos/gk1016/ed-loadout --jq .permissions` -> `"push":false`) and so cannot open its PR. The loadout-v0 wave's commits sit unpushed in `/home/gk1016/ed-loadout-code`. Probably the same in bts-template stacks/rust-only.
  Closed 2026-11-21: `code-wave.yml` `wave` job now has `contents: write` (commit `0f7cd07`); the loadout-v0.1 wave pushed `code/loadout-v0.1` itself (`* [new branch] code/loadout-v0.1 -> code/loadout-v0.1`).
