# TODO-APP-NAME — repo rules

TODO-APP-DESC. Build host: TODO-BUILD-HOST.

## Standing workflow

1. `ISSUES.md` at the repo root is the permanent ledger. Every session touching the
   app starts by reading it. Append new findings; never re-file. Closed items stay
   closed, with their evidence screenshot name.
2. End-to-end passes are two-phase. Survey: build with `TODO-BUILD-CMD`, run the
   verify pass with `TODO-VERIFY-CMD`, exercise every screen, record findings, fix
   nothing. Then fix: by severity, smallest change, rebuild, reproduce the
   original steps, screenshot the fixed state, one commit per fix. Never fix
   during survey.
3. Evidence screenshots: `_shots\uipass\`, named `fixed-<issue-id>.jpg`.
4. After a fix pass: full test suite (`TODO-TEST-CMD`), push main. Never leave
   commits unpushed on this machine.
5. Releases only on explicit instruction. Path: TODO-RELEASE-PATH.
6. EVO-X2 is the sole build host for Windows and Linux; studio.local builds iOS
   only. devmachine02 is the game machine only, never builds.
7. A fix touching more than three files, or changing behavior that cannot be
   verified on screen: stop and ask.
8. Production engine data is changed only through the engine's API endpoints.
   No direct SQL against the prod database without an explicit instruction
   naming the statement.

## Execution role

When told "Execute claude/plans/<file>":

1. Read the plan, CLAUDE.md, ISSUES.md, and the latest claude/reports/ entry.
2. Enter Plan mode: read every source file the plan names, then write your execution
   plan — what you'll change, in what order, how each acceptance criterion gets
   verified. If anything in the plan contradicts the code as it actually is, say so
   here, before touching anything.
3. Execute. Smallest change per criterion. Build, run, reproduce, verify on screen or
   by test. One commit per criterion, message referencing the plan filename.
4. Halt on any stop-and-ask condition, or if a change would touch more than three
   files, or if a criterion can't be verified unattended. Ask; do not guess.
5. On completion: run the full suite, push the repo, then write
   claude/reports/<same filename as the plan>.md with: criteria met (with evidence
   screenshot names), criteria not met and why, deviations from the plan and why, new
   issues found (also appended to ISSUES.md), what's left. Commit and push the report.

Never fix during a survey pass. Never leave commits unpushed. Never release without an
explicit instruction.

## Running as the GitHub Action

When you run as claude-code-action (`.github/workflows/claude.yml`, triggered by
`@claude` on an issue comment), there is nobody at the keyboard:

1. Take the plan path from the triggering comment (`Execute claude/plans/<file>`)
   and follow that plan file under the Execution role above, without the
   interactive Plan-mode approval: state any contradiction with the code in the PR
   body instead.
2. Work only on the Action's `claude/` branch. Never push main.
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
   first heading, with `Closes #<issue>` in the body.
5. The runner is WSL Linux: TODO-WSL-LIMITS. The stack's build workflow checks
   the PR and its verify workflow runs the screen pass and marks it ready; list
   only click-driven checks as needs-Graham. That alone is not a block.
6. Blocked otherwise (a stop-and-ask condition, an ambiguous criterion): comment
   the question on the issue, add label `needs-graham` (`gh issue edit <n>
   --add-label needs-graham`), then stop. The comment goes first and the label
   second: notify-graham sends Discord the labeler's newest comment.
7. No release, no prod, no engine data. Never print a secret.
8. If this run dies after criterion commits (turn cap, timeout), the branch
   stays; Cowork opens the PR from it and runs the Codex check. Nothing is lost.
