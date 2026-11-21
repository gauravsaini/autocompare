# GATES — exq-typo89 round-1: typography + light Visuals polish (theme.rs ONLY)

OWNS: `crates/gui/src/theme.rs` + `checks/gui-exq-typo89/**`. No other files.
Geometry frozen, zero new color tokens, forbid unsafe, Hinglish-Roman chat, English commits, never push.

- [ ] G1 GATES.md likha BEFORE implement (ye file khud proof)
  - CHECK: `test -f checks/gui-exq-typo89/round-1/GATES.md && echo GATES-PRESENT`
  - EXPECT: `GATES-PRESENT`

- [ ] G2 baseline theme tests green (implement se pehle)
  - CHECK: `cargo test -p autocompare-gui --lib theme 2>&1 | tail -3`
  - EXPECT: `test result: ok`

- [ ] G3 OWNS-only diff (theme.rs + checks/gui-exq-typo89 sirf)
  - CHECK: `git status --short | awk '{print $2}' | grep -v '^$' | grep -v '^checks/gui-exq-typo89/' | grep -v '^crates/gui/src/theme.rs$' | grep -v '^/tmp/wave-typo89.done$' | wc -l`
  - EXPECT: `0`

- [ ] G4 fmt clean
  - CHECK: `cargo fmt -p autocompare-gui -- --check && echo FMT-CLEAN`
  - EXPECT: `FMT-CLEAN`

- [ ] G5 banned tokens zero non-test (unwrap/expect/panic/todo/unreachable/unsafe)
  - CHECK: `grep -n 'unwrap(\|expect(\|panic!\|todo!\|unreachable!\|unsafe' crates/gui/src/theme.rs | grep -v 'unwrap_or' | grep -v 'forbid(unsafe_code)' | grep -vc '^\s*[0-9]*:\s*//' ; echo BANNED-SCAN-DONE`
  - EXPECT: `BANNED-SCAN-DONE`

- [ ] G6 zero new color tokens (koi naya pub const Color32 nahi; sirf LIGHT_* reuse)
  - CHECK: `git diff 71bd65d -- crates/gui/src/theme.rs | grep -c '^+.*pub const.*Color32' ; echo COLOR-TOKEN-SCAN-DONE`
  - EXPECT: `COLOR-TOKEN-SCAN-DONE`

- [ ] G7 typo89 headless contract green (sizes + line-height + tracking + light wash reuse)
  - CHECK: `cargo test -p autocompare-gui --lib typo89 2>&1 | tail -3`
  - EXPECT: `test result: ok`

- [ ] G8 harness PASS byte-stable ya documented (headless-only to PNG same rehna chahiye)
  - CHECK: `tools/exquisite/run.sh checks/gui-exq-typo89/round-1 && grep -c 'RESULT: PASS' checks/gui-exq-typo89/round-1/exquisite-report.md`
  - EXPECT: `RESULT: PASS`
