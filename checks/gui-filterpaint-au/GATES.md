# GATES — Wave AU-FILTERPAINT

## Scope (OWNS only)

- `crates/gui/src/views.rs`
- `checks/gui-filterpaint-au/**`

KEVL (mat chhoona; `tree.rs` / `app.rs` sirf read-only samajhne ke liye):
`app.rs`, `tree.rs`, `viewmodel.rs`, `views_table.rs`, `views_hex.rs`,
`home.rs`, `gutter.rs`, `theme.rs`, `screenshot.rs`, `copy.rs`, `take.rs`.

## Goal

Folder/compare view ka paint wired name/type/status filters adopt kare:
filtered-out rows chhupen, counts filtered rows par honest rahen, khali
view honest message + Clear de. Geometry first (row hiding, spacing,
alignment, empty-state layout), typography second. Colors FROZEN: koi
`Color32` / `Visuals` / hue change nahi.

## Bahar ka observation (Beyond Compare, apne shabdon me)

Bahar se dekhe folder compare me upar ek filter patti hoti hai: naam
me kuch likho to ped wahi ghata hai, file/dir ka chunta alag karta
hai, status chunta (same/diff/newer/only) wahi panktiyan rakhta hai.
Filter laga ho to patti khud batati hai kuch chhupa hai, ek safai
button sab wapas lata hai, ginati filtered panktiyon par hoti hai,
aur kuch na bache to khali sandesh jhoothi ginati nahi dikhata.

## API contract (paint, AT model ko adopt karta hai)

`views.rs` (paint + pure helpers, no I/O, no hues):

- `folder_kind_options()` / `folder_status_options()`: ComboBox
  options, model enums ka poora set, `All` pehle. Naya variant kabhi
  chipke na — test-locked.
- `folder_filter_bar_height_px()` / `folder_filter_bar_pad_px()` /
  `folder_filter_bar_gap_px()`: wired token aliases (24/8/8, MD
  scale) taaki paint hamesha model scale par rahe — test-locked.
- `folder_filter_nodes(rows, expanded, display)`: paint-side tree
  source = `tree::folder_tree_filtered`. Filtered-out rows kabhi
  paint nahi hoti.
- `folder_filtered_summary(rows, display)`: filtered rows par
  `count_folder_rows(...).summary()` — caller string ki jagah paint
  isi ko dikhata hai, ginati kabhi jhooth nahi bolti.
- `folder_paint_empty_hint(rows, visible, display)`: model rows vs
  visible nodes par `tree::folder_empty_state` → `folder_empty_hint`.
  `NoResults` (compare-first) vs `NoMatch` (clear-filters) alag.
- `folder_filter_rich(text, typo)`: `FolderTypo` → `RichText`
  (mono/size/strong mapping, koi rang nahi).
- `paint_folder_filter_bar(ui, display) -> bool`: naam field (mono
  body-regular, `FIND_NEEDLE_W` reuse) + Type/Status ComboBox +
  active chip (`N active`, mono caption-strong) + Clear button.
  Bar height + pad + gap wired tokens. `true` = kuch badla.
- `paint_folder(ui, rows, display, expanded, pager)`: signature
  badla — `summary: &str` gaya, `display: &mut FolderDisplay` aaya
  (koi caller nahi tha, `#[allow(dead_code)]` barkarar; App rewiring
  orchestrator ke paas pending). Andar: filter bar → filtered nodes
  → honest summary → pager sync (filter badle to `top = 0`) →
  page/header/grid (purana rhythm untouched) → khali par centered
  honest hint + (filters hon to) Clear button.

## Non-goals

- `paint_folder_sync` / `paint_folder_dual` ke signature nahi badle
  (dual ko App pehle se filtered rows + filtered summary deta hai;
  sync pending painter hai, orchestrator dekhega).
- `TREE_INDENT` / `tree_indent` nahi chhua (App inline paint shared
  const use karta hai — cross-wave paint badalne se bacha).
- Colors me koi badlav nahi; filesystem I/O nahi; engine rules nahi.
- `unwrap` / `expect` / `panic` / `todo` non-test code me mana hai.
  `forbid(unsafe_code)` barkarar.

## DoD

- [x] Naam/type/status filters paint me lagte + filtered rows chhupti.
- [x] Counts honest (filtered summary), khali states alag + Clear.
- [x] 9 naye headless unit tests, full `cargo test` green.
- [x] `cargo clippy --all-targets` zero warnings.
- [x] `rustfmt --edition 2024 --check` OWNS file clean.
- [x] Banned grep clean (non-test `unwrap()|expect(|panic!|todo!`).
- [x] `tools/gui-polish/run-geo-typo.sh` round-1 PASS (PNG magic ok).
- [x] `GATES.md` + `REPORT.md` + `round-1/` artifacts.
- [x] `git add` sirf OWNS paths + `git commit` (conventional English).
        No push. No stash. Repo ke bahar koi copy nahi.
