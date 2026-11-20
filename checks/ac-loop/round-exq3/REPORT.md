# ac-loop round report (round-exq3)
- CLI parity: PASS (log cli.log, report cli/parity-report.md)
- GUI geo-typo: PASS (log gui.log, report gui/geo-typo-report.md)
- BC live observe:  BCompare
- own sessions: text folder hex merge table sync picture folder-merge audio (own-help.txt)
- comparison dir: add new edge fixtures here each round, rerun same dir on fix, advance N only on PASS series
RESULT: PASS
- exq3 wave (geometry first, typo second, colors frozen): lib+bin split (view-model pub API, egui thin); zero dead-code suppressions; viewport 960x600 + 720 fallback (window min-size, narrow fixture 720x600 proves zero raw clip); dual_pane_widths/clip_text_end/clip_to_width headless + paint-chrome/status adoption; TypoRole::rank locks hierarchy; edge-02 tabs-vs-spaces+unicode added
- exq3a wave (geometry first, typo second, colors frozen): completed lib+bin split (new crates/gui/src/lib.rs pub API x12, main.rs thin launcher) so `pub` headless helpers no longer need suppression; dead-code suppressions 253 -> 0 with zero suppressions added. Geometry: toolbar gap single-sourced (`toolbar_icon_gap()` adopted in `paint_session_toolbar`), split grab width single-sourced (`SPLIT_HANDLE_W = viewmodel::SPLIT_HANDLE_HIT_W`), 720 fallback fixture (`render_narrow_text_fixture_png`) on the shared headless budget (`VIEWPORT_MIN_W` + `dual_pane_widths` + `clip_to_width`), proof `comparison/narrow-720-proof.png` 720x600 magic-ok. Typography: status mid verdict wired to `status_verdict_role()` (Body 13) vs side counts `status_side_role()` (Small 11), filter labels at tab chrome Body 13; colors untouched (Color32 touches 29 = baseline). Fixtures added: edge-01 long-line wrap/clip, edge-02 narrow-table; edge-02 narrow-expected.
- exq3a gates: cargo test -p autocompare-gui 712 pass; cargo clippy --all-targets -D warnings clean; cargo fmt --check clean; dead-code-suppression grep 0 in OWNS; after.png 9765B magic-ok distinct-256; narrow-720-proof.png 8162B 720x600 magic-ok.
- RESULT: PASS
