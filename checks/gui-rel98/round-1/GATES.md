# rel98 round-1 Gates (release verification, checks-only)

OWNS: checks/gui-rel98/** ONLY (zero src touch; strays checks/gui-sessionpaint-by/** revert if touched)

Scope: base 8f220e1 par release verification round-1 — root-4 G2/G3 release smoke checks (release build, release diff sweep if feasible, owns verification).
Target directory: isolated CARGO_TARGET_DIR=/tmp/target-rel98

## G1 — release build green (root-4:G3)

- CHECK `CARGO_TARGET_DIR=/tmp/target-rel98 cargo build --release --workspace > checks/gui-rel98/round-1/build.out 2>&1; echo "exit=$?" >> checks/gui-rel98/round-1/build.out; tail -15 checks/gui-rel98/round-1/build.out`
- EXPECT `exit=0` aur `build.out` me Finished release profile cleanly.

## G2 — release diff sweep if feasible (root-4:G2)

- CHECK `CARGO_TARGET_DIR=/tmp/target-rel98 cargo test -p autocompare-diff --release -- --ignored > checks/gui-rel98/round-1/release-tests.out 2>&1; echo "exit=$?" >> checks/gui-rel98/round-1/release-tests.out; tail -15 checks/gui-rel98/round-1/release-tests.out`
- EXPECT `exit=0` with tests passed, ya fir timeout/heavy hone par honest skip recorded with reasons in REPORT.md.

## G3 — OWNS-only dirt, strays untouched, zero src touch, no push

- CHECK `git status --porcelain -- checks/gui-sessionpaint-by/ ; echo "---"; git status --porcelain | head -20`
- EXPECT pehla block khaali (strays untouched); doosre block me sirf `checks/gui-rel98/**`; zero src changes; `git log -1 --format=%H` commit hash recorded in REPORT + /tmp/wave-rel98.done; no push.
