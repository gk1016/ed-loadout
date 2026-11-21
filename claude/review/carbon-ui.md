# Carbon UI critique brief (TODO-APP-NAME)

You are reviewing screenshots of TODO-APP-NAME, TODO-APP-DESC, against its brand. Read `index.md` in the survey folder, then view every image it lists. Do not read source to judge pixels; read source only to name the file:line a fix would touch.

## Brand rules
TODO-BRAND-RULES

## What to produce
One Markdown comment:
1. A table, severity-ranked, at most 25 rows: `Sev | Screen | Theme/posture | Finding | Rule broken | Source file:line | Smallest fix`. H = brand rule broken or unreadable; M = inconsistent across screens or themes; L = polish.
2. Five lines: the three worst screens and why, in order.
3. If run with a `vs` tag: an extra column per earlier finding — fixed / partly / not — and nothing new unless it is H.

Rules: write source references as plain repo-relative text, e.g. `TODO-SOURCE-EXAMPLE`, never as links or absolute paths; cite only what is visible in the images; if an image is missing or unreadable, say so for that screen and move on. No code. No praise. Under 900 words.
