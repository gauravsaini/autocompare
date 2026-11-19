# Gates: GUI-TEXT P0 Text Compare Navigation, Copy Mutations & Display Filters

OWNS: crates/gui/src/views.rs crates/gui/src/copy.rs crates/gui/src/take.rs crates/gui/src/gutter.rs crates/gui/src/viewmodel.rs checks/gui-text/**

Scope: Make text compare panes editable and navigable with Copy Section/Line, Next/Prev difference navigation, and Display-filter segmented control.

- [x] G1: Viewmodel unit tests pass for Copy Section/Line, Next/Prev Diff navigation, selection tracking, and Display Filters with honest counts.
  CHECK: cargo test -p autocompare-gui
  EXPECT: test result: ok.
  EVIDENCE: test result: ok. 92 passed; 0 failed (2026-11-20, debug). Covers copy_line/copy_section (copy.rs), next/prev/wrap nav, section_bounds, filtered_indices per filter, filter_counts honesty (viewmodel.rs).

- [x] G2: Strict MS Rust guidelines pass with clippy -D warnings on gui crate.
  CHECK: cargo clippy -p autocompare-gui --all-targets -- -D warnings
  EXPECT: Finished
  EVIDENCE: Finished dev profile, zero warnings (2026-11-20). Fixed 4 collapsible_if lints in owned views.rs.

- [x] G3: Rustfmt clean on all owned source files.
  CHECK: rustfmt --check crates/gui/src/views.rs crates/gui/src/copy.rs crates/gui/src/take.rs crates/gui/src/gutter.rs crates/gui/src/viewmodel.rs
  EXPECT:
  EVIDENCE: FMT-CLEAN, no diff (2026-11-20, rustfmt --edition 2024).

- [x] G4: Screenshots before.png and after.png exist and verify as valid PNG image files.
  CHECK: file checks/gui-text/before.png checks/gui-text/after.png
  EXPECT: PNG image data
  EVIDENCE: both PNG image data, 960x600, 8-bit RGBA (2026-11-20, debug --screenshot). Hash differs from before (sibling fixture update in flight); renderer is fixture-only, functional proof is G1-G3.
