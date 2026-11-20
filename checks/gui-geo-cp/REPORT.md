# CP-GEO REPORT — exquisite geometry pass (views.rs only)

Wave: CP-GEO (sess-geo-cp). SCOPE: `crates/gui/src/views.rs` + `checks/gui-geo-cp/`
sirf. `app.rs`, `theme.rs`, `main.rs`, `screenshot.rs`, `STATUS.md` untouched.
Colors FROZEN, typography dusri wave ka — koi touch nahi.

## 1. BC observe-only notes (koi click/edit/focus-change nahi)

- BC process live mila (`BCompare`), window: `t_left.txt <--> t_right.txt - Text Compare`.
- Window frame (System Events se read-only): position (720, 31), size 720x869.
- Pixel capture is machine par possible nahi tha bina focus change ke:
  `screencapture -l` is macOS par supported nahi (illegal option), aur `-R`
  region capture sirf foreground pixels deta (BC Codex window ke peeche tha).
  Isliye frame geometry + standard BC Text Compare layout (top toolbar, dual
  panes with center gutter, bottom status bar) note kiya; BC me koi interaction
  nahi kiya.
- Fullscreen observe capture: `/tmp/cp-geo-bc-observe.png` (2880x1800) — isme
  BC foreground me nahi tha, isliye ye sirf environment proof hai, BC proof nahi.

## 2. Geometry audit — kya inconsistent tha

- Button-bar rhythm: 12 button/tool bars me se 11 me
  `item_spacing.x = TIGHT_GAP` (4.0) set tha, par 5 paging bars (hex, table,
  folder, folder-sync, folder-dual) + merge Take/apply-all rows + picture
  Layout/zoom/steps/pan rows + dual pending row me default egui spacing chal
  raha tha (8.0). Toolbar ek scale par nahi the.
- Token misuse: picture SideBySide panes me vertical gap ke liye
  `tree_indent(1)` (14.0, horizontal indent token) use ho raha tha jabki
  vertical rhythm token `TIGHT_GAP` (4.0) hai.
- Dead anomaly: dual-tree row-cell grids me `spacing([SECTION_GAP, 0.0])` —
  single-row grids me y-gap ka koi visual effect nahi, par scale par 0.0
  anomaly tha jabki har sibling grid `TIGHT_GAP` use karta hai.

## 3. Changes (sirf layout numbers, existing tokens ka reuse)

1. 5 paging bars (hex hunk, table, folder, folder-sync, folder-dual):
   `item_spacing.x = TIGHT_GAP` add — baaki bars jaisa.
2. Merge Take row + Take-all row: `item_spacing.x = TIGHT_GAP` add.
3. Dual pending row (label + Clear): `item_spacing.x = TIGHT_GAP` add.
4. Picture Layout radio row, zoom row, steps row, pan row:
   `item_spacing.x = TIGHT_GAP` add.
5. Picture SideBySide left/right panes: `add_space(tree_indent(1))` (14.0)
   → `add_space(TIGHT_GAP)` (4.0), dono panes symmetric — pane alignment barkarar.
6. Dual row-cell grids (left/right): `spacing([SECTION_GAP, 0.0])` →
   `spacing([SECTION_GAP, TIGHT_GAP])` — single-row grids, visual no-op, scale unified.
7. `paint_picture_compare` doc comment se stale `TREE_INDENT` mention hataya
   (comment-only, code same).

Pane alignment: left/right pane widths (`three_pane_widths`, `split_*`,
`dual_tree_pane_widths`) untouched — equal-share math pehle se BC-jaisa hai.
Toolbar/prehdr rows sab `HEADER_H` (28.0) par hain, status lines
`chrome_status_role` par — positioning BC-jaisi rakhi, move nahi kiya.

## 4. Frozen guarantees

- Colors: `Color32|Visuals|bg_fill|set_visuals` count 94 → 94 (zero new touches).
- Typography: koi `FontId`/`RichText` size/weight/line-height change nahi.
- `#![forbid(unsafe_code)]` kayam. Non-test code me bare
  `unwrap(`/`expect(`/`panic!`/`todo!` zero (`unwrap_or*` family allowed hai).
- Fixture PNGs byte-stable: `before.png` == `after.png`
  (md5 `8ccf51e4824a03f84fb9e0d1b95fc521`). Wajah: edits sirf egui paint
  call-sites par hain; headless software rasterizer (`screenshot.rs` + pure
  helpers) untouched, isliye render same raha. Live UI me button-bar gaps
  8.0 → 4.0 tight honge aur picture pane vertical gap 14.0 → 4.0 hoga —
  ye live-only pixel change hai, fixture me nahi dikhta.

## 5. Files

- `checks/gui-geo-cp/before.png` — change se pehle fixture (960x600).
- `checks/gui-geo-cp/after.png` — change ke baad fixture (byte-identical).
- `checks/gui-geo-cp/GATES.md` — gate proofs.
- `checks/gui-geo-cp/REPORT.md` — ye file.
