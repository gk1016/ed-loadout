# LOADOUT (ed-loadout) — issue ledger

Permanent ledger for the app. Every session touching it starts here.
Append new findings; never re-file. Closed items stay closed, with their
evidence (test name or pasted command output).

Severity: HIGH (core loop broken) · MED (wrong but has a workaround) ·
LOW (cosmetic / polish) · OBS (observation, not confirmed a defect).

---

### OPEN

- OBS-001 2026-11-21 — devmachine02 `DisplaySettings.xml` has `MaxFramesPerSecond` 144 above the 120 Hz refresh; harmless with VSync on. Left as found in the fixture so `apply --machine` overrides have a real case.

### CLOSED

(none)
