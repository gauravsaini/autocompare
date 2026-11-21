# GATES — exq-agyxswarm80 copy round-1 (leaf)

OWNS: `crates/gui/src/copy.rs` + `checks/gui-exq-agyxswarm80/copy/**` only.
GOAL: Copy path UX polish in `crates/gui/src/copy.rs` ONLY: copy path UX helpers, reuse theme tokens (`SPACE_*`, `TypoRole`), pure safe Rust (`#![forbid(unsafe_code)]`), 0 banned constructs (no unwrap/expect/panic/todo/unreachable/unsafe in non-test code), tests green.

- [x] G1: Baseline test PASS before implementing.
  CHECK: `cargo test -p autocompare-gui`
  EXPECT: `test result: ok`
  EVIDENCE: 1140 passed; 0 failed (baseline clean)

- [x] G2: Copy path UX polish reuses `SPACE_*` tokens (`SPACE_XS`, `SPACE_SM`, `SPACE_MD`, `SPACE_LG`, `SPACE_XL`) and `TypoRole` (`Small`, `Mono`, `chrome_status`) in `crates/gui/src/copy.rs`.
  CHECK: `cargo test -p autocompare-gui -- exq_agyxswarm80_copy_tests`
  EXPECT: `test result: ok`
  EVIDENCE: 7 passed; 0 failed (`agyxswarm80_tokens_geometry_reuse`, `agyxswarm80_tokens_typo_roles_reuse`, `agyxswarm80_copy_path_actions_and_labels`, `agyxswarm80_copy_path_formatting`, `agyxswarm80_copy_path_status_text`, `agyxswarm80_middle_truncate_and_ux_badges`, `agyxswarm80_copy_path_button_geometry_and_hit`)

- [x] G3: Zero banned constructs in non-test code (pure safe Rust, #![forbid(unsafe_code)]).
  CHECK: `git diff -U0 crates/gui/src/copy.rs | grep '^+' | grep -v '^+++' | sed -n '1,/mod exq_agyxswarm80_copy_tests/p' | grep -E 'unwrap\(|expect\(|panic!|todo!|unreachable!|unsafe ' || true`
  EXPECT: `^0$`
  EVIDENCE: 0 banned in non-test diff; pure safe Rust enforced

- [x] G4: All GUI tests green.
  CHECK: `cargo test -p autocompare-gui`
  EXPECT: `test result: ok`
  EVIDENCE: 1154 passed; 0 failed (full lib tests ok + real_egui_shot tests ok)

- [x] G5: `cargo fmt --check` clean.
  CHECK: `rustfmt --edition 2024 --check crates/gui/src/copy.rs`
  EXPECT: `0`
  EVIDENCE: exit code 0, COPY_FMT_OK

- [x] G6: Disjoint OWNS — staging touches ONLY owned files (`crates/gui/src/copy.rs` and `checks/gui-exq-agyxswarm80/copy/**`).
  CHECK: `git add crates/gui/src/copy.rs checks/gui-exq-agyxswarm80/copy/ && git diff --cached --name-only | grep -vcE '^(crates/gui/src/copy\.rs|checks/gui-exq-agyxswarm80/copy/)'`
  EXPECT: `0`
  EVIDENCE: staged paths all under OWNS; siblings untouched
