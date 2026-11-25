# LOADOUT (ed-loadout) — repo rules

Qt 6 desktop app (`loadout.exe`, `cpp/` + `qml/`) over a Rust core (`rust/`, crate `loadout-core`, also built as the CLI `ed-loadout.exe`) that captures, diffs and applies an Elite Dangerous settings bundle (Graphics fxcfg/XML, GraphicsConfigurationOverride.xml, Bindings + StartPreset; ED-UP edvr*.ini not yet) across machines, with per-machine overrides, backups with restore and a bound-device check. Installed by `LOADOUT_Setup.exe`. TacOps / Boring Tech Stuff. No game-memory reads, no hooks, no network. Build host: EVO-X2 Windows runner `x2-ed-loadout-win` (MSVC, Qt 6.8.3, cargo, Inno Setup 6).

## Standing workflow

1. `ISSUES.md` at the repo root is the permanent ledger. Every session touching the
   app starts by reading it. Append new findings; never re-file. Closed items stay
   closed, with their evidence.
2. Passes are two-phase. Survey: `.\ship.ps1 -NoRelease` (build, `cargo test` from
   `rust/`, package, the scripted run), then `.\shots.ps1 -Survey <tag>`; exercise
   every CLI command against `fixtures/devmachine02`; record findings, fix nothing.
   Then fix: by severity, smallest change, rebuild, reproduce, one commit per fix.
3. Evidence is test output, `ship.log` lines and `diff` output pasted into the report,
   and for anything on screen the survey captures in `_shots\uipass\survey-<tag>\`.
4. After a fix pass: from `rust/`, `cargo test` (with `LOADOUT_NO_DETECT=1`; `build.ps1`
   sets it) + `cargo clippy -- -D warnings`; `.\ship.ps1 -NoRelease` must end
   `=== VERIFY OK (no release) ===`; push main. Never leave commits unpushed on this
   machine.
5. Releases only on explicit instruction. Path: `.github/workflows/release.yml` on a
   `v*` tag attaches `LOADOUT_Setup.exe` to a GitHub Release (being adjusted by
   Cowork for v0.3, ISSUES OBS-012). `ship.ps1` only verifies; it cuts no release.
6. EVO-X2 is the sole build host. devmachine02 is the game machine only, never builds;
   its settings are mirrored in `fixtures/devmachine02/` and never edited to pass a test.
7. A fix touching more than three files, or changing file-writing behaviour that a
   test cannot pin: stop and ask.
8. Files the tool writes on a user's machine are the user's. Every overwrite is
   preceded by a backup; element edits replace text in place and never reformat.

## Execution role

When told "Execute claude/plans/<file>":

1. Read the plan, CLAUDE.md, ISSUES.md, and the latest claude/reports/ entry.
2. Enter Plan mode: read every source file the plan names, then write your execution
   plan — what you'll change, in what order, how each acceptance criterion gets
   verified. If anything in the plan contradicts the code as it actually is, say so
   here, before touching anything.
3. Execute. Smallest change per criterion. Build, test, reproduce. One commit per
   criterion, message referencing the plan filename.
4. Halt on any stop-and-ask condition, or if a change would touch more than three
   files, or if a criterion can't be verified unattended. Ask; do not guess.
5. On completion: run the full suite, push the repo, then write
   claude/reports/<same filename as the plan>.md with: criteria met (with evidence),
   criteria not met and why, deviations from the plan and why, new issues found (also
   appended to ISSUES.md), what's left. Commit and push the report.

Never fix during a survey pass. Never leave commits unpushed. Never release without an
explicit instruction.

## Running as the GitHub Action

When you run as claude-code-action (`.github/workflows/claude.yml`, triggered by
`@claude` on an issue comment) or code-wave (`@code build`), there is nobody at the keyboard:

1. Take the plan path from the triggering comment and follow that plan file under the
   Execution role above, without the interactive Plan-mode approval: state any
   contradiction with the code in the PR body instead.
2. Work only on the Action's branch. Never push main.
3. After the last criterion commit, write the report to
   `claude/reports/<plan filename>` as usual and commit it on the branch. Then
   the Codex check, before the PR: run
   `codex exec --sandbox read-only --ephemeral -o .git/codex-check.md "Follow
   claude/review/codex-adversarial.md. The change is git diff main...HEAD."` as
   one command (no pipes, no `$(...)`), then Read `.git/codex-check.md`. Judge
   each finding against the code; fix the ones that hold up with the smallest
   change, one commit each. The PR body carries a `## Codex check` section:
   findings fixed (with commit), findings not fixed and why, and on its last
   line `<!-- codex-check <head-sha> -->`. If `codex` fails or times out, the
   section says so with the error line and the PR still opens.
4. Open a DRAFT PR (`gh pr create --draft --base main`) titled from the plan's
   first heading.
5. The runner is Windows (`x2-ed-loadout-win`, Graham's logon session, so there is a
   desktop): build with `.\ship.ps1 -NoRelease`, survey once per wave with
   `.\shots.ps1 -Survey <slug>`. Everything a run touches is a scratch folder under
   `%TEMP%`: never the box's real Elite Options dir, never the saved `LOADOUT.ini`,
   never an existing LOADOUT install. Windows-only code stays behind `cfg(windows)`
   and its pure logic is unit-tested. The stack's build workflow runs `.\build.ps1`
   (which runs `cargo test` from `rust/`) on the PR.
6. Blocked otherwise (a stop-and-ask condition, an ambiguous criterion): comment
   the question on the issue, add label `needs-graham` (`gh issue edit <n>
   --add-label needs-graham`), then stop. The comment goes first and the label
   second: notify-graham sends Discord the labeler's newest comment.
7. No release. Never print a secret.
8. If this run dies after criterion commits (turn cap, timeout), the branch
   stays; Cowork opens the PR from it and runs the Codex check. Nothing is lost.
