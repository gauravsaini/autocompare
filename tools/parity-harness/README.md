# parity-harness

One-round console parity check for the clean-room Beyond Compare replica.

- `make-fixtures.py` regenerates `fixtures/` (original byte patterns only, stdlib python3).
- `run-round.sh <round-dir>` builds the CLI, runs all 9 session commands on
  fixtures, asserts diff-convention exits (1 = different, 0 = same), and writes
  `<round-dir>/parity-report.md` (`RESULT: PASS/FAIL` + per-command exits).
- GUI `--screenshot` visual checks plug in here once Wave 3 lands the flag.

The `parity-harness` skill drives the fix-and-repeat loop around this script.
