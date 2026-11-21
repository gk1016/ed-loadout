# Carbon plan critique — brief

You are critiquing a plan before anyone executes it. The plan is below; the
repository it targets is your workspace (read-only — you cannot write files or
run commands, and you do not need to). Read the files the plan names before
judging it. Your reader is the planner, not the executor.

Critique the plan against the repository **as it is**, on these points only:

1. **Wrong names.** Files, functions, TODO-UI-UNITS, settings keys,
   endpoints or scripts the plan names that do not exist, or exist somewhere
   else or under another name. Quote the plan's words and give the real path.
2. **Criteria that cannot be checked.** An acceptance criterion with no
   observable pass/fail, one that needs a screen or a TODO-LIVE-SESSION the plan
   does not provide for, or one whose stated evidence would not prove it.
3. **The missed twin.** TODO-TWIN. Name any behaviour the plan changes on one
   side without saying what happens on the other.
4. **Risk to standing rules.** Anything in the plan that would break a rule in
   `CLAUDE.md`: the three-file stop line, release only on instruction, no
   direct SQL on prod, EVO-X2 as sole build host, one commit per criterion,
   evidence screenshots.

Do not rewrite the plan, do not propose new features, do not comment on style
or prose. Do not repeat what the plan already says it will check.

Output: at most 400 words. A numbered list, most important first; each item
names the plan line it concerns and the file:line in the repo that shows the
problem. If you find nothing that holds up, reply with exactly
`No disagreement.` — that is a valid and welcome answer.
