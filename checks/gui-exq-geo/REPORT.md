# Wave L GEO — exquisite geometry REPORT (round L)

Scope: `crates/gui/src/views.rs` + `crates/gui/src/app.rs` geometry only.
No color/Visuals/Color32 edits. No typography edits (no font/weight/size/line-height).
Clean-room: comparable dual-pane product observed from outside only,
behavior described in own words, nothing copied.

## 1. Baseline (round-L-1)
- `cargo test -p autocompare-gui`: 182 passed, 0 failed.
- Harness `tools/gui-polish/run-geo-typo.sh checks/gui-exq-geo/round-L-1`: PASS.
- PNG: 9914 bytes, magic `89504e470d0a1a0a` ok, distinct 256 (>64 ok).
- Harness metrics: add_space literals `[]` (constants used, not literals),
  Grid 1, text_style_height x4 (crate-wide), width calls 9.
- Tokens: OUTER_MARGIN 12 (i8), SECTION_GAP 8.0, TIGHT_GAP 4.0,
  FOOTER_GAP 6.0, TAB_BOTTOM_GAP 2.0, TREE_INDENT 14.0,
  CARD_W/H 260.0/75.0 (`views.rs:43-50`).

Note: harness PNG is a synthetic raster fixture (`screenshot.rs`
`render_text_fixture_png`, 960x600), not live egui. Egui token edits
do not change its bytes by design. Proof below is static px math +
fixture bytes.

## 2. Outside observation (own words)
Comparable product layout: top tab strip, then one path row per side
(field + picker button + compare action), then central split with two
equal panes and a narrow middle action lane, thumbnail overview strip
at far right, summary bar at bottom. Outer frame keeps even breathing
room; vertical rhythm is even between tab strip, path row, panes, footer.
Narrow windows scroll inside panes, panes never crush.

Gaps found vs that behavior:
- Tab strip sat 2px above identity/toolbar row, rhythm break vs 6/8px elsewhere.
- Home 3-col cards 3*260+2*12=804px overflow typical ~776px avail (960-24 margin),
  third card wraps/scrolls.
- Picture side-by-side columns subtracted one 4px gap, leaving center
  squeeze when item spacing applies.

## 3. Fix (round-L-2, smallest root-cause diff, `views.rs` only)
1. `views.rs:47` TAB_BOTTOM_GAP 2.0 -> 6.0 (+4px, matches FOOTER_GAP rhythm).
2. `views.rs:49` CARD_W 260.0 -> 240.0 (-20px/card; grid 804 -> 744px, fits 776px avail).
3. `views.rs:1088-1090` `picture_column_width`: `(avail-TIGHT_GAP)/2` ->
   `(avail-SECTION_GAP)/2` (-2px/col at 800px: 398.0 -> 396.0, breathing room).
4. Tests updated to new targets only: `views.rs:1809,1811,1887`
   (geometry_tokens_match_targets, picture_columns_split_avail_with_floor).
No `app.rs` edit needed; `app.rs:1832,1837,2385` already consume
OUTER_MARGIN/TAB_BOTTOM_GAP/FOOTER_GAP tokens.

No unsafe added, no unwrap/expect/panic/todo in non-test code
(only pre-existing `ok()?`/`if let` paths touched by other waves).
Headless-first kept: `--screenshot` path untouched.

## 4. Proof
- Before PNG (round-L-1/after.png): 9914 bytes, magic ok, distinct 256.
- After PNG (round-L-2/after.png): 9914 bytes, magic ok, distinct 256.
  Identical expected: fixture raster does not read egui tokens.
- Px deltas (static, headless-verified by unit tests):
  - TAB_BOTTOM_GAP: +4px (`views.rs:47`, test `views.rs:1809`).
  - CARD_W: -20px/card, grid total -60px (`views.rs:49`, test `views.rs:1811`).
  - picture_column_width(800): 398.0 -> 396.0, -2px/col
    (`views.rs:1088-1090`, test `views.rs:1887`).
- Crate-wide recount after: add_space literals `[]`, Grid 1,
  text_style_height x3 (drop 4->3 from parallel TYPO wave gutter edit,
  not GEO), width calls 9, monospace x33, strong x6 (+1 TYPO), small x30.
- Gates (owned scope): `views::` tests 28 passed; `cargo clippy
  -p autocompare-gui --all-targets -- -D warnings` clean (exit 0);
  `cargo fmt --check` clean for `views.rs` (only remaining diff is
  parallel TYPO wave `theme.rs:502`, untouched per ownership).
  Full `cargo test` currently 190/191 pass with 1 failure in
  `theme::tests::typo_rich_carries_size_and_weight_per_role` caused by
  concurrent TYPO wave edit to `theme.rs`, not GEO files.

## 5. Rounds used: 2/6. Next: re-run full green once TYPO/PARITY land.

---

# Wave GEO1 — cr section-8 ke 7 geometry gaps (round-geo1)

Scope: `crates/gui/src/views.rs` + `app.rs` + `home.rs` geometry only
(width/height/margin/padding/align/position). Zero Color32/Visuals,
zero typo (family/weight/size/line-height untouched). Clean-room:
BC outside-observe only, apne words me, kuch copy nahi.

## 1. Baseline (round-geo1 start)

- `git pull --ff-only`: Already up to date. Start-time dirty tree me
  OWNS (`views.rs`/`home.rs`/`tree.rs`/`app.rs`, `checks/gui-exq-geo`)
  clean the — baaki M files (compare/diff/cli/vfs/viewmodel/views_hex/
  views_table) doosri waves ke, ignore kiye (DIRTY RULE v3).
- `cargo test -p autocompare-gui`: 678 passed, 0 failed.
- Harness `tools/gui-polish/run-geo-typo.sh checks/gui-exq-geo/round-geo1`:
  PASS (PNG 9765 B, magic ok, distinct 256).
- Harness signals tree-wide hain (sibling waves land hue: Grid 14, mono
  x49, strong x32, small x40, widths 28) — GEO1 diff inme zero add karta
  hai (diff me koi Grid/mono/strong/small nahi).
- `tools/ac-loop/run.sh checks/ac-loop/round-geo1`: CLI PASS + GUI PASS
  => `RESULT: PASS`. BC observe read-only (`BCompare` process mila).

## 2. Gap table — section-8 ke 7 gaps vs fix

| # | Gap | Fix (file:line) | Px delta |
|---|---|---|---|
| 1 | 4-row chrome stack (pills+tabs+identity+toolbar) | identity label row + uska TAB_BOTTOM_GAP hataya (`app.rs` central panel); window title + pills wahi identity rakhte hain | -1 row (~14px) - 6px gap |
| 2 | Toolbar oversized (120x28 pills + 3 checkboxes) | `session_toolbar_button_size` 120x28 -> 96x24 (`views.rs`, `home.rs` `HOME_TOOLBAR_BUTTON_W/H`); height 24 find-hit floor precedent | row 28 -> 24 (-4px); Swap ~120 -> ~96 (-24px, content-driven floor) |
| 3 | Right-edge overflow (path row avail se 238px wide) | `split_field_widths`/`third_field_width` ab real chrome ghatate hain (`path_row_chrome_width` 388, `triple_path_row_chrome_width` 504; `split_picture_widths` precedent) | 800 avail par fields 650 -> 412, fields + chrome = avail exact (overflow 238 -> 0); triple 279 -> 0 |
| 4 | Double status (inline hint + bottom bar ek hi baat) | 8 inline "no X results yet..." labels hataye (Text/Hex/Merge/Table/Sync/Picture/FolderMerge/Audio); bottom `status_line` + Home guide + in-pane note rehte hain; Folder ka state-aware hint retained | -8 rows |
| 5 | Table uneven density (Keys tight, Tolerance bheed) | Keys + Tolerance rows par `HEADER_H` (28) min-height floor (`app.rs`, path-row precedent) | dono rows >= 28px uniform floor |
| 6 | Khali-pan void (~55% khali) | Mitigated: in-pane notes in-zone rahe (Text "(nothing to display...)"), Folder guided hint retained; placeholder structure (gutter/ruler) paint wave ka kaam, deferred | 0 px is wave me (structure deferred) |
| 7 | Strip alignment (bade gaps, chhota x, tairte pills) | `SESSION_TAB_MAX_W` 160 -> 120, `session_tab_close_edge` 28 -> 24 (hit floor barkarar); pill 192 -> 148px (-23%); 3 pills 576 -> 444, 8-pill scroll threshold test hold | -44px/pill |

## 3. Proof

- `cargo test -p autocompare-gui`: 678 passed, 0 failed (pehle jaisa).
- Clippy: `crates/gui` me zero diagnostics (fail sirf `autocompare-diff`
  lib me 11 pre-existing errors — doosri wave ki dirty `hex.rs`, OWNS
  bahar, untouched).
- `cargo fmt --check`: `views.rs`/`app.rs`/`home.rs` clean (4 diffs sirf
  `crates/diff` me — pre-existing, untouched).
- Banned: diff me zero `Color32`/`Visuals::`, zero unwrap/expect/panic/
  todo non-test me.
- Harness round-geo1 `RESULT: PASS`; ac-loop round-geo1 `RESULT: PASS`
  (CLI PASS + GUI PASS, BC read-only observed).
- Edge fixture: `round-geo1/edge-narrow800.md` (+ copy ac-loop
  `comparison/` me) — 800px live window par exact-fit math lock.
- Tests updated to new targets only (wave-L precedent): toolbar floor,
  half/third/split widths (+ row-fit asserts), strip geometry, home
  toolbar consts.

## 4. Files changed (OWNS only)

- `crates/gui/src/views.rs` (chrome-width fns + width math + toolbar/strip tokens + tests)
- `crates/gui/src/app.rs` (identity row + 8 inline hints hataye, table row floors)
- `crates/gui/src/home.rs` (`HOME_TOOLBAR_BUTTON_W/H` + locks)
- `checks/gui-exq-geo/round-geo1/` (harness out + edge fixture)
- `checks/gui-exq-geo/REPORT.md` (ye section)

## 5. Rounds used: 1/6. Next: live-capture wave me pixel verify (G6 structure samet).
