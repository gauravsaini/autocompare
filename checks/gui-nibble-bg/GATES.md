# Wave BG-NIBBLE — GATES (one observable outcome per gate)

OWNS: `crates/gui/src/views.rs` (callsite only) + `crates/gui/src/app.rs`
(hex state threading) + `checks/gui-nibble-bg/**` ONLY.
`views_hex.rs` FROZEN (helpers ready hain), `screenshot.rs` READ-ONLY,
`tree.rs`/`theme.rs`/`views_table.rs` READ-ONLY,
`checks/opencode-waves/STATUS.md` ko CHHOONA MANA. Colors FROZEN.

Precedent: `checks/localrun-bf/REPORT.md` §4 P1-1 (OPEN hissa) +
`checks/gui-hexpure-bd/REPORT.md` §4 (callsite spec) +
`checks/gui-hexmini-ay/REPORT.md` §2 point 3 (OPEN wiring note).

1. Nibble-content threading — khaali slices ki jagah REAL byte slices.
   CHECK: `cargo test -p autocompare-gui hex_byte_snap_slices_feed_nibble_cells` EXPECT: pass
   (low-nibble diff par 1 cell, col 0, high false / low true; barabar par khali).
   CHECK: `cargo test -p autocompare-gui hex_byte_snap_none_keeps_old_empty_path` EXPECT: pass
   (`None` snap par khali slices = purana offset-only behavior).

2. App-owned snap lifecycle — set par thread, har wipe par clear.
   CHECK: `cargo test -p autocompare-gui hex_success_sets_and_resets_clear_byte_snap` EXPECT: pass
   (set par model/view/bytes/status; clear par teeno khali).
   CHECK: `cargo test -p autocompare-gui hex_swap_clears_byte_snap` EXPECT: pass
   (swap par snap clear; purani bytes kabhi stale hunks ke saath nahi).
   CHECK: `cargo test -p autocompare-gui hex_mode_change_clears_stale_results` EXPECT: pass
   (mode change par snap clear — purane test me naya assert).

3. Capped read — chhoti file round-trip, missing par khali, kabhi panic nahi.
   CHECK: `cargo test -p autocompare-gui read_hex_input_capped_round_trips_small_file` EXPECT: pass
   (temp file bytes wapas; missing path par khali vec).

4. Hygiene — MS guidelines + frozen colors + OWNS discipline.
   CHECK: `cargo test -p autocompare-gui` EXPECT: 589 passed, 0 failed.
   CHECK: `cargo test` (full workspace) EXPECT: sab suites ok, koi FAILED nahi.
   CHECK: `cargo clippy -p autocompare-gui --all-targets` EXPECT: zero warnings.
   CHECK: `rustfmt --edition 2024 --check` sirf OWNS 2 files par EXPECT: clean
   (bare `cargo fmt` MANA).
   CHECK: apne diff me `grep -nE 'unwrap\(|\.expect\(|panic!|todo!|unreachable!'` EXPECT: zero;
   `Color32::from_rgb|Visuals::` apne diff me EXPECT: zero;
   `git diff HEAD --stat` me KEVL files EXPECT: absent
   (`views_hex.rs`, `screenshot.rs`, `tree.rs`, `theme.rs`, `views_table.rs`, `STATUS.md`).

5. Harness proof — headless-first round passes with real PNGs.
   CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-nibble-bg/round-1` EXPECT: `RESULT: PASS`,
   PNG magic-ok (`89504e470d0a1a0a`), size >= 8192 B, distinct > 64.
   CHECK: `after-hex.png` (hex fixture route) magic-ok, 960x600 RGBA
   (fixture `screenshot.rs` read-only hai, isliye twin-stable — regression-absence proof).
