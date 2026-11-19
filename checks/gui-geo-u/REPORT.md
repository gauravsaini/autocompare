# Wave U GEO — exquisite geometry REPORT (round-U-geo-1)

Scope: `crates/gui/src/views.rs` + `checks/gui-geo-u/**` ONLY.
`theme.rs`/`app.rs` read-only (referenced consts, never edited).
No hue/saturation/color-token, font-family, weight, or letter-spacing changes.

## 1. Baseline (pre-change, HEAD a3ea288 at plan time)
- `cargo test -p autocompare-gui`: 231 passed, 0 failed.
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: clean.
- `cargo fmt --check`: clean. Release `target/release/autocompare-gui`: 11074624B.
- Harness PNG (synthetic fixture): 9914B, magic `89504e470d0a1a0a` ok, distinct 256.

## 2. Outside observation (own words, clean-room)
Comparable dual-pane product, observed from outside only: outer frame keeps
even breathing room; both panes share one left content origin; the splitter is
a wide easy grab with a hairline visual; the find bar sits full-width above the
text pane with its N/M badge pinned right, never under the buttons; every
header/nav bar reads the same height; rows march on one beat. No strings,
assets, or behavior copied; geometry rebuilt from the repo's own spacing scale.

## 3. Fix (smallest root-cause diff, `views.rs` only)
- Tokens (reuse first): `GUTTER_W` = `theme::GUTTER_BAR_WIDE`, `ROW_PAD` =
  `theme::GUTTER_MIN_SEGMENT_H` (2.0), `SPLIT_HIT_W` = `SPLIT_HANDLE_W` (8.0),
  `BADGE_PAD` = `TIGHT_GAP`; new: `HEADER_H` 28.0, `FIND_BAR_H` = `HEADER_H`,
  `SPLIT_VISUAL_W` 1.0, `FIND_NEEDLE_W` 120.0 (was literal), `HEX_MINIMAP_H`
  220.0 (dedupes 2 literals), `CONTENT_LEAD` `"  "` (dedupes 3 literals).
- Helpers (pure, headless): `split_visual_inset` (centers 1px visual in 8px
  grab), `row_height_for_line` (line + pad, NaN-safe), `indented_content`
  (shared left-baseline for table header/rule + folder placeholder).
- Paint: splitter allocates `SPLIT_HIT_W` with centered 1px visual; all six
  row sites use `row_height_for_line`; hex/table/folder nav rows + find bar
  claim `HEADER_H`; find bar docks full-width (`set_min_size`) with badge in
  `right_to_left` + `BADGE_PAD` air; table header/rule route through
  `indented_content`; minimap uses `GUTTER_W`.
- 6 new tests (tokens reuse, split centering, row rhythm, content lead, find
  bar dock, gutter uniformity).

## 4. Proof (G1–G8)
- G1 scope: `git diff --name-only` shows only `crates/gui/src/views.rs`
  modified by this wave (`app.rs` + `checks/gui-{layout,run}-u/` belong to a
  parallel wave, untouched here, committed separately).
- G2 tokens: 6/6 new `views::tests` green (targeted runs).
- G3 suite: `cargo test -p autocompare-gui` → 245 passed, 0 failed
  (231 baseline + 6 U-geo + 8 parallel-wave app.rs tests).
- G4 lints: `cargo clippy -p autocompare-gui --all-targets -- -D warnings`
  exit 0; `rustfmt --check crates/gui/src/views.rs` clean (workspace
  `cargo fmt` untouched — parallel wave owns `app.rs`).
- G5 bans: non-test `grep -nE '\.unwrap\(|\.expect\(|panic!|todo!|unsafe'`
  → zero hits (`unwrap_or`/`ok()?` only, pre-existing pattern).
- G6 freeze: `monospace` 37→37, `.strong()` 5→5, `.weak()` 44→44,
  `Color32` 23→23, `FontId` 6→6, `.size(` 2→2.
- G7 harness: `run-geo-typo.sh checks/gui-geo-u/round-U-geo-1` exit 0,
  RESULT: PASS, after.png 9914B magic-ok distinct-256 (identical: synthetic
  fixture never reads egui tokens).
- G8 commit: local only, no push (orchestrator verifies + pushes).
- Release: 11074624B → 11077424B (+2800B, +0.025%; measured with parallel
  wave's uncommitted `app.rs` present, so not purely U-geo attributable).

## 5. Rounds used: 1/6. Follow-up: none — re-verify after sibling waves land.
