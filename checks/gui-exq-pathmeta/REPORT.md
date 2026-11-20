# EXQ-PATHMETA round-1 — path metadata strip plus segmented status (headless half)

## 1. Summary

local6 TOP-10 #4,7 ka headless half is wave me land hua: BC per-pane path bar me
dropdown + folder icon + second metadata line (timestamp/bytes/format/rules) + bottom
4-segment status (cursor/format/result/edit) hai, ours me single field + Browse +
single summary line tha. `viewmodel.rs` me pathmeta strip + segmented status ka pure
model aaya (join/history/roles, honest-empty, existing `clip_to_width` reuse),
`screenshot.rs` fixture me per-pane strip paint hua (`[ ]` icon + clipped path +
`v` caret + Browse, neeche second meta line) + bottom 4-zone segmented status
(dividers `SEG_DIV*`). Live paint (`views.rs`/`app.rs`) agle wave ka OWNS hai —
haath nahi lagaya. Harness `round-1/` me `RESULT: PASS`. Colors zero new, banned zero.

## 2. Unlazy (GATES pehle, implement baad me)

`checks/gui-exq-pathmeta/GATES.md` me CHECK/EXPECT table pehle likha (G1-G8), phir code
implement kiya. Ponytail laziest: koi naya size/color/token nahi — geometry sirf
existing `OUTER_PAD`/`PANE_GAP`/`STATUS_H` + 5 chhote y/x consts (`PATH_*`/`SEG_DIV*`),
typo sirf existing `TypoRole::Body`/`caption()` reuse (path Body 13, meta/side Small 11 —
`status_*_role` precedent), clip sirf existing `clip_to_width` + `text_width` reuse,
colors sirf existing `fixture_palette()` hues ka reuse.

## 3. Harness + gates evidence (round-1, SKILL loop)

- `sh tools/exquisite/run.sh checks/gui-exq-pathmeta/round-1` -> exit 0, `exquisite-report.md` me `RESULT: PASS` (round 1/6).
- `after.png`: PNG image, 10363 bytes (>= 8192 ok), magic ok, distinct 256 (> 64 ok).
- `cargo test -p autocompare-gui`: 773 passed, 0 failed (mere 6 naye tests samet: viewmodel me 5, screenshot me 1).
- `cargo test -p autocompare-gui pathmeta -- --nocapture`: pathmeta subset PASS.
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: exit 0, zero warnings (ek `assertions_on_constants` aaya tha, existing precedent jaisa `#[allow]` lagaya).
- `cargo fmt -- crates/gui/src/viewmodel.rs crates/gui/src/screenshot.rs`: clean (sibling `gutter.rs`/`copy.rs`/`tree.rs`/`views.rs` dirt ko touch nahi kiya).
- Banned grep (non-test): koi match nahi (unwrap/expect/panic/todo/unsafe zero; sirf `#![forbid(unsafe_code)]` header).
- Colors: `git diff` me `Color32`/`Visuals` zero new.
- Sibling dirt stage nahi kiya (sessions/*, copy/gutter/tree/views, doosre checks dirs untouched).

## 4. Geometry table — before/after

| Row | Before | After |
|---|---|---|
| Path strip | single field `(12,76,380,24)` + Browse, no meta line | per-pane strip: `[ ]` icon + `clip_to_width(path,300)` + `v` caret + Browse; neeche second meta line `(16,100)`/`(494,100)` h12 dim, honest-empty par skip |
| Toolbar | y104 | y114 (`PATH_TOOLBAR_Y`), panes y124 untouched (budget 100..112 meta, 114 toolbar, 124 panes) |
| Bottom status | single dim line `7 rows: ... \| text compare done` | 4 zones `cursor \| format \| result \| edit` at x16/250/490/730 + dividers `SEG_DIV*` (240/480/720), strip `(0,570,960,30)` same |
| Narrow (<960) | `paint_chrome` 960-layout overflow karta | compact single field + strip, overflow nahi (width guard) |
| Typo roles | fixture strings ad-hoc | path Body, meta/side caption(Small 11 weak) — `path_strip_*`/`path_segment_*` roles, `status_*` precedent reuse |

## 5. Exact lines (OWNS only)

- `crates/gui/src/viewmodel.rs:2480+` pathmeta section — `PathPaneMeta`, `format_path_pane_meta`, `path_pane_meta_line`, `path_history_label`, `PathSegmentedStatus`, `path_segmented_status_for`, `path_segmented_status_line`, `path_strip_path_role`, `path_strip_meta_role`, `path_segment_cursor_role`, `path_segment_side_role`.
- `crates/gui/src/viewmodel.rs` tests — 5 naye: second-line join, history honest, segmented line, roles reuse, edge empty-vs-long.
- `crates/gui/src/screenshot.rs:155+` `PATH_*`/`SEG_DIV*` consts; `paint_pathmeta_strip` + `paint_segmented_status` helpers; text fixture + `paint_chrome` strip wiring; `pathmeta_strip_geometry_holds` test.
- `checks/gui-exq-pathmeta/GATES.md`, `round-1/exquisite-report.md`, `round-1/after.png`, `round-1/comparison/edge-pathmeta.md`.

## 6. Follow-up (orchestrator ke liye)

1. Live paint (`views.rs`/`app.rs`) abhi purana hai — fixture se match karne ke liye agle wave me `paint_path_meta_row` + `paint_text_pane_status` ko is headless model se wire karna hai (doosre agent ka OWNS).
2. Fixture meta abhi fixed demo strings dikhata hai (`2026-01-02… | 128 B | Text: exact`); live me `path_meta_for` se aayega — headless me honest-empty path khula hai.
3. `paint_chrome` narrow guard compact strip deta hai; 720 fallback route apna summary box upar se paint karta hai — dono me double-draw nahi hai (guard + route order verified).
