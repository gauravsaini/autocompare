---
name: parity-harness
description: Runs the clean-room parity check loop (console fixtures, exit codes, reports) for the Beyond Compare replica. Use when verifying comparison features, running a check round, or driving fix-and-repeat refinement.
---

# Parity Harness

Deterministic one-round console parity check plus the fix-and-repeat loop.

## Quick start

```sh
python3 tools/parity-harness/make-fixtures.py   # regenerate fixtures (rarely needed)
tools/parity-harness/run-round.sh checks/round-N # writes checks/round-N/parity-report.md
```

`RESULT: PASS` means all 9 session commands return diff-convention exits
(1 = different, 0 = same). `RESULT: FAIL` names the red command(s) with
per-command `.out` logs beside the report.

## Loop workflow (fix-and-repeat)

1. Run a round into a fresh `checks/round-N/` dir.
2. Green: report PASS, move on. Red: read the failing `<cmd>.out`.
3. Fix the smallest root-cause diff in the owning crate, re-run the same round
   (do not advance N on a re-run of the same fix attempt).
4. Repeat to max 6 rounds. Escalate if 2 consecutive rounds change no files
   or 10 consecutive checks fail: stop and write `checks/inbox/<issue>.md`
   with repro instead of guessing further.

## Rules

- Fixtures are original byte patterns only; never copy files, strings, or
  assets from the original app.
- GUI `--screenshot` visual checks plug into this loop once the flag exists;
  until then this skill covers console parity only.
- Keep every round dir; never rewrite history.
