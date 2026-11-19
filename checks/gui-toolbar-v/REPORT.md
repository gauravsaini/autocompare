# REPORT — Wave V TOOLBAR breathing room

OWNS: `crates/gui/src/app.rs` + `checks/gui-toolbar-v/**`. views.rs / theme.rs untouched (read-only).

## Delta (RUN early-feedback #1: toolbar toggle row packs controls too tight)

Toolbar row at app.rs:2482 stays one `ui.horizontal` row (actions + view toggles).
Breathing room comes from GEO tokens only, no new literals:

- `toolbar_item_gap() -> views::TIGHT_GAP` (4px) drives `item_spacing.x` for every adjacent control.
- `toolbar_group_gap() -> views::SECTION_GAP` (8px) pads both sides of the `separator`, so actions vs toggles read as two groups.
- Outer rhythm unchanged (`TAB_BOTTOM_GAP` above/below already present).

Narrow-safe: row keeps `ui.horizontal` (no wrap call); egui clips rather than overlaps. Headless hierarchy test locks `group > item > 0` finite.

## Evidence

- `cargo test -p autocompare-gui` → 246 passed, 0 failed (245 baseline + 1 new `toolbar_gaps_reuse_geo_tokens_with_group_hierarchy`).
- `cargo test -p autocompare-gui toolbar_` → 1 passed.
- Owned clippy: `cargo clippy ... | grep app.rs` → zero hits. Full-worktree clippy red is sibling-owned `views.rs:2813 assertions_on_constants` (V-MINI OWNS, not touched here).
- `cargo fmt --check -p autocompare-gui` → clean.
- `tools/gui-polish/run-geo-typo.sh checks/gui-toolbar-v/round-V-1` → exit 0, RESULT: PASS (PNG 9914B, magic ok, distinct 256).
- `--screenshot checks/gui-toolbar-v/after.png` → exit 0, 9914 bytes, magic `89504e470d0a1a0a`, distinct 256.
- Banned scan on app.rs non-test code: `unwrap(`/`expect(`/`panic!`/`todo!` → 0 hits; `forbid(unsafe_code)` present.
- Colors: zero Color32/Visuals edits in app.rs diff (harness info count 22 is pre-existing tree-wide).
- `git status` at commit: only `crates/gui/src/app.rs` + `checks/gui-toolbar-v/**` staged (sibling `views.rs`/`theme.rs` + `gui-mini-v`/`gui-glyph-v` left uncommitted, disjoint waves).
