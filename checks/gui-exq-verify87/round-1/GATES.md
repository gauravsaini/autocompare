# exq-verify87 round-1 — GATES (verify-only, crates/** ZERO edits)

Scope OWNS: sirf `checks/gui-exq-verify87/**`. NO-TOUCH: `crates/**`, `tools/**`, baaki checks dirs.
Strays revert: `checks/gui-sessionpaint-by/**`, `crates/sessions/**` me agar local diff dikhe to revert karo.
Bhasha: chat Hinglish-Roman, commit English conventional, push MANA.

## Gates (run se PEHLE likha)

| # | Gate | CHECK | EXPECT |
|---|---|---|---|
| V1 | GATES first | `test -f checks/gui-exq-verify87/round-1/GATES.md` | file maujood, code run se pehle |
| V2 | Workspace tests green | `cargo test --workspace` (agar bahut heavy to `-p autocompare-gui --lib` + `real_egui_shot` fallback, REPORT me wajah likho) | sab tests PASS, failures 0 |
| V3 | Harness report | `tools/exquisite/run.sh checks/gui-exq-verify87/round-1/` (ya gui-polish harness jo repo me ho) | `RESULT: PASS`, PNG magic-ok + size>=8192 + non-blank |
| V4 | fmt clean (report-only) | `cargo fmt --check` | exit 0; agar fail to REPORT me note, fix MANA (crates touch nahi) |
| V5 | Banned scan (report-only) | `grep -rn 'unwrap(\|\.expect(\|panic!\|todo!' crates/ --include='*.rs'` non-test me | count REPORT me; fix MANA |
| V6 | Outputs filed | `ls checks/gui-exq-verify87/round-1/` | REPORT.md + tests.out + screenshot.out maujood |
| V7 | Commit no-push | `git log --oneline -1` + `git status --short` | `docs(checks): exq-verify87 verification report` local commit, sirf OWNS pathspec, push nahi, `crates/**` zero diff |
| V8 | Done file | `cat /tmp/wave-verify87.done` | hashes + counts maujood |

## Result (run ke baad bharo)

| Gate | Actual | Status |
|---|---|---|
| V1 | GATES.md tests se pehle likha | PASS |
| V2 | workspace EXIT=0, 50 suites ok, 1786 passed 0 failed | PASS |
| V3 | harness-exit=0, RESULT: PASS, PNG 10946B magic-ok distinct-256 | PASS |
| V4 | fmt exit=1, 4 diffs sab crates/sessions pre-existing, fix MANA | FAIL-noted |
| V5 | non-test banned hits 0 | PASS |
| V6 | REPORT.md + tests.out + screenshot.out filed | PASS |
| V7 | 2538058 docs(checks): exq-verify87 verification report, OWNS-only 4 files, no push | PASS |
| V8 | /tmp/wave-verify87.done me hashes + counts | PASS |
