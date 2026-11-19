# BE-PIXELFONT GATES (research wave, zero code edits)

Scope: `checks/localrun-bb/REPORT.md` section 4 P2-1. Sirf `checks/gui-pixelfont-be/**`
likha gaya. `crates/**` me ZERO edit (read-only grep + test run allowed).
`checks/opencode-waves/STATUS.md` untouched. Colors FROZEN.

- [x] G1 — harness PASS proof: `zsh tools/gui-polish/run-geo-typo.sh
      checks/gui-pixelfont-be/round-1` exit 0, `RESULT: PASS`
      (`round-1/geo-typo-report.md` me recorded).
- [x] G2 — test suite green: `cargo test -p autocompare-gui` 578 passed,
      0 failed (`round-1/tests.out`).
- [x] G3 — REPORT me classification table: har TypoRole ke liye
      harness-verifiable vs live-only column (REPORT section 5).
- [x] G4 — REPORT me future live-egui protocol: exact scenes + screenshots +
      acceptance thresholds (REPORT section 6).
- [x] G5 — PNG proof: `round-1/after.png` (harness fresh, 960x600 RGBA,
      magic `89504e470d0a1a0a` ok, 9627 bytes, BB text twin se byte-identical)
      + `round-1/after-table.png` (localrun-bb se byte-copy, 1px underline
      signal wala reference twin, md5 match).
- [x] G6 — report bhasha: poori REPORT Hinglish-Roman, Devanagari zero.
- [x] G7 — commit conventional message se, NO PUSH.
