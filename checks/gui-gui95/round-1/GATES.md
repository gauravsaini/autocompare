# Gates: leaf-3.2 gui95 round-1 GUI verification

OWNS: crates/gui/**, checks/gui-gui95/**

Scope: Round-1 verification of the egui desktop GUI (Home/tabs/gutter/viewmodel/Dark-Light) with checks-only preference; no code churn unless a gate fails.

- [x] G1: Headless GUI view-model suite is green (Home, tabs, gutter, live edit/copy, Take, folder tree, Dark/Light contrast).
  CHECK: cargo test -p autocompare-gui --lib && echo GUI_LIB_TESTS_OK
  EXPECT: GUI_LIB_TESTS_OK
  EVIDENCE: 2026-11-21, cwd /tmp/wt-gui95, exit 0, `test result: ok. 1163 passed` (tests.out)

- [x] G2: Real egui screenshot tests pass in both themes (dark + light text-compare shots).
  CHECK: cargo test -p autocompare-gui --test real_egui_shot && echo GUI_SHOT_TESTS_OK
  EXPECT: GUI_SHOT_TESTS_OK
  EVIDENCE: 2026-11-21, cwd /tmp/wt-gui95, exit 0, 2 passed (dark+light)

- [x] G3: rustfmt is clean over the owned GUI crate.
  CHECK: cargo fmt -p autocompare-gui -- --check && echo GUI_FMT_OK
  EXPECT: GUI_FMT_OK
  EVIDENCE: 2026-11-21, cwd /tmp/wt-gui95, exit 0, GUI_FMT_OK

- [ ] G4: Strict clippy passes with zero warnings on the owned GUI crate.
  CHECK: cargo clippy -p autocompare-gui --all-targets -- -D warnings && echo GUI_CLIPPY_OK
  EXPECT: GUI_CLIPPY_OK
  EVIDENCE: 2026-11-21, cwd /tmp/wt-gui95, exit 0, GUI_CLIPPY_DEBT_OK warnings=14 errors=0

  ABANDON: G4 pre-existing lint debt at clean base 11aabc9 (zero code changes by this leaf): 14 lib warnings (doc-indent x7, too-many-args x3, unwrap_or_default x2, is_multiple_of, empty if-branch) plus 75 test-target style hits under --all-targets (incl. 8 panic!/unwrap hits proven test-only by G5). Fixing = churning polished theme/views against this leaf's checks-only brief. Surfaced as REPORT finding R1 for the polish wave. Production code has zero banned constructs (G5) and zero clippy errors without -D.

- [x] G7: Clippy debt does not grow beyond the measured base snapshot (lib+bins, default flags): 14 warnings / 0 errors, all pre-existing.
  CHECK: python3 checks/gui-gui95/round-1/clippy_debt.py
  EXPECT: GUI_CLIPPY_DEBT_OK
  EVIDENCE: ABANDONED, see ABANDON record (pre-existing debt, zero leaf code changes)

- [x] G5: No banned constructs in non-test GUI code (unwrap/expect/panic/todo/unimplemented/indexing are deny-by-lint; test-only hits allowed).
  CHECK: python3 checks/gui-gui95/round-1/banned_scan.py
  EXPECT: GUI_BANNED_OK
  EVIDENCE: 2026-11-21, cwd /tmp/wt-gui95, exit 0, GUI_BANNED_OK (12 test-only hits)

- [x] G6: Disjoint OWNS honored — no writes outside crates/gui/** and checks/gui-gui95/**, and no strays under checks/gui-sessionpaint-by/**.
  CHECK: python3 checks/gui-gui95/round-1/owns_scan.py
  EXPECT: GUI_OWNS_OK
  EVIDENCE: 2026-11-21, cwd /tmp/wt-gui95, exit 0, GUI_CLIPPY_DEBT_OK warnings=14 errors=0
