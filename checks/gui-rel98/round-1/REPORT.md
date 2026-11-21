# rel98 round-1 REPORT (release verification, checks-only)

Base: 8f220e1 | Date: 2026-11-21 | Mode: checks-only, zero src touch
Target Dir: isolated CARGO_TARGET_DIR=/tmp/target-rel98

## Gate natija

- G1 release build (root-4:G3): PASS (`exit=0`, `Finished release profile [optimized] target(s) in 2m 27s`, binaries `autocompare`, `autocompare-gui` aur workspace crates cleanly built).
- G2 release diff sweep (root-4:G2): PASS (`exit=0`, 5 exhaustive/random diff property sweep tests passed in `tests/api.rs`: `exhaustive_alphabet4_len4`, `exhaustive_alphabet2_len7`, `exhaustive_alphabet3_len5`, `random_pairs_medium`, `random_pairs_deep`; 0 failed).
- G3 owns/strays: PASS (strays `checks/gui-sessionpaint-by/` completely clean/untouched; dirt strictly isolated to `checks/gui-rel98/**`; zero src touch; no push).

## Evidence files

- `checks/gui-rel98/round-1/GATES.md` (written BEFORE checks execution)
- `checks/gui-rel98/round-1/build.out` (captured full workspace release build log, ending with `exit=0`)
- `checks/gui-rel98/round-1/release-tests.out` (captured release ignored property sweep log, ending with `exit=0`)
- `checks/gui-rel98/round-1/REPORT.md` (this report)
