# Round 1 — Folder drill-down geometry (2026-11-20)

## Study (read-only, existing notes)
- `capture/opencode-study/round-3-geometry-search.md`: BC window 720x869,
  toolbar patti ~30px, status ~22px, do pane har ek ~340px, darmiyan
  patla splitter ~6px, line-number gutter ~40px (UNVERIFIED andaze).
- `docs/study.md` + `capture/opencode-study/round-2-view-tools.md`: folder
  compare = recursive tree, per-node status, size/date columns.
- Apna canvas: `--screenshot` PNG 960x600 (TITLE 36 + TAB 32 + STATUS 30
  chrome, row 24, gutter track 16). BC 720x869 se canvas chhota nahi —
  hamara fixture landscape 960x600 hai, BC portrait-ish 720x869; aspect
  gap note me darj, canvas change is wave me nahi.

## Build (geometry FIRST, colors untouched)
1. `views::TREE_INDENT` 14.0 -> `theme::GEO_TREE_INDENT_STEP` (12.0):
   paint ab headless `tree::tree_indent_for_depth` contract par hai.
   Drill-down depth-2 row: 28px -> 24px (BC dense tree ki taraf).
2. `tree::FOLDER_EXPANDER_W` = 16.0 + `folder_row_x_offset(depth)` =
   indent + 16 (pure, headless-tested): har row ka path-column origin
   fixed — file rows ka path dir rows se aage-peeche nahi hota.
3. `app.rs` Folder single-list paint: bina-button rows (files + childless
   dirs) ab fixed 16px expander cell reserve karti hain (`add_space`);
   pehle file rows ke paas kuch nahi tha, childless dirs ke paas `  `
   label tha — teeno row-kinds ab ek baseline par.
4. Folder fixture indent: depth-step 4u -> 2u bitmap units = 14px/depth
   (live 12px ka nearest, pehle 28px/depth tha — do-guna gehra tha).

## Check
- `cargo test -p autocompare-gui`: 699 -> 701 (2 naye: expander offset,
  paint-headless indent parity). Baseline green barkarar.
- `cargo clippy -p autocompare-gui --all-targets`: zero warnings.
- Screenshot: `/tmp/w1-folder.png` 960x600, 10153 bytes, PNG magic ok.
  Depth-1 `src/...` rows ka glyph x: 34 + 14 = 48px (pehle 62px tha).

## Bacha hua gap (agle round / wave ke liye)
- Dual-pane (`paint_folder_dual`) me expander beech-column me hai, left
  pane me indent-only — single-list jaisa fixed cell wahan nahi.
- Small-button `▶/▼` hit width (~20-24px) vs 16px cell: path origin
  expandable rows par button ke baad shuru hota hai, exact 16 nahi.
  Residual ~4-8px, note me darj.
- BC gutter ~40px (line numbers samait) vs hamara 16px ruler track:
  fixture `GUTTER_W` = 16 live ruler contract par hai; 40px BC andaza
  UNVERIFIED hai, isliye 16 barkarar.
