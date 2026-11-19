# Gates: Wave AO-SNAPSHOT snapshot parity

OWNS: crates/gui/src/screenshot.rs + checks/gui-snapshot-ao/**
KEVL (do not touch, read bhi avoid): views.rs/app.rs/viewmodel.rs/tree.rs/views_table.rs/views_hex.rs/home.rs/gutter.rs/theme.rs

Scope: screenshot dispatcher + fixture coverage badhao. Har tab
(text/folder/hex/table/merge/home) ka headless PNG route + geometry
assertions (widths/padding/row heights). Colors frozen (out of scope).
Clean-room: Beyond Compare bahar se observe only, apne shabdon me.
MS rust guidelines: forbid unsafe, non-test code me unwrap/expect/panic/todo mana.

- [x] G1: Dispatcher routed har tab ke liye
  CHECK: cargo test -p autocompare-gui screenshot -- --nocapture
  EXPECT: dispatcher_routes_all_tabs ok (text/folder/hex/table/merge/home alag PNG bytes)
  EVIDENCE: dispatcher_routes_all_tabs ok; foldermerge -> merge equal bytes; round-1 PNGs text 9624/folder 10068/hex 10030/table 9973/merge 8829/home 9011 sab magic ok

- [x] G2: Naye snapshot fixtures green (valid PNG, size, non-blank)
  CHECK: cargo test -p autocompare-gui screenshot -- --nocapture
  EXPECT: hex/table/merge/home fixture tests ok (magic ok, len >= 8192, distinct > 64)
  EVIDENCE: hex/table/merge/home_fixture_renders_valid_png ok (16 screenshot tests pass)

- [x] G3: Geometry assertions green (widths/padding/row heights)
  CHECK: cargo test -p autocompare-gui geometry -- --nocapture
  EXPECT: canvas 960x600, row_h 24, pane widths/padding within canvas, text_width fits panes
  EVIDENCE: geometry_shared_tokens_hold + merge_panes_share_equal_thirds + home_cards_fit_canvas ok

- [x] G4: Full gui tests green
  CHECK: cargo test -p autocompare-gui
  EXPECT: test result: ok, 0 failed
  EVIDENCE: 431 passed, 0 failed

- [x] G5: Clippy zero warnings
  CHECK: cargo clippy -p autocompare-gui --all-targets -- -D warnings
  EXPECT: exit 0, zero warnings
  EVIDENCE: exit 0; grep screenshot.rs hits 0

- [x] G6: Fmt clean (sirf OWNS files)
  CHECK: cargo fmt -- crates/gui/src/screenshot.rs --check (read-only check); fix ho to cargo fmt -- crates/gui/src/screenshot.rs
  EXPECT: FMT_OK (diff empty after own-file fmt)
  EVIDENCE: cargo fmt -- crates/gui/src/screenshot.rs --check exit 0

- [x] G7: Banned patterns absent in non-test code
  CHECK: rg -n "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" crates/gui/src/screenshot.rs; rg unsafe excluding forbid line
  EXPECT: no hits (test mod me unwrap_or_default/assert allowed, bare unwrap mana)
  EVIDENCE: banned1 hits 0, unsafe (non-forbid) hits 0; forbid(unsafe_code) only

- [x] G8: Harness PASS (headless proof)
  CHECK: tools/gui-polish/run-geo-typo.sh checks/gui-snapshot-ao/round-1
  EXPECT: RESULT: PASS
  EVIDENCE: round-1/geo-typo-report.md RESULT: PASS (PNG 9624 bytes, magic ok, distinct 256)
