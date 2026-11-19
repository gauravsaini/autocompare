# REPORT — Wave AU-FILTERPAINT

## Summary

Folder/compare view ka paint ab wired name/type/status filters adopt
karta hai: tree source `viewmodel::folder_tree` (unfiltered) se
`folder_tree_filtered` par aaya, summary caller string se filtered
honest counts par aaya, khali view me honest hint + Clear button
aaya. Geometry first (bar 24 / pad 8 / gap 8 wired tokens, purana
page/header/grid rhythm untouched, centered empty-state layout),
typography second (naam field mono body-regular, active chip mono
caption-strong, hint status-role). Colors frozen — koi hue nahi.

## Files changed (sirf OWNS)

- `crates/gui/src/views.rs` (+~380 lines):
  - Naye pure helpers: `folder_kind_options` (3), `folder_status_options`
    (8, `All` pehle), `folder_filter_bar_height/​pad/​gap_px` (wired
    aliases 24/8/8), `folder_filter_nodes` (filtered tree source),
    `folder_filtered_summary` (filtered honest counts),
    `folder_paint_empty_hint` (NoResults vs NoMatch),
    `folder_filter_rich` (FolderTypo → RichText, no color).
  - Naya painter: `paint_folder_filter_bar` (naam field + Type/Status
    ComboBox + `N active` chip + Clear; change bool lautata hai).
  - `paint_folder` rework: signature `summary: &str` → `display:
    &mut FolderDisplay` (koi caller nahi tha); filter bar → filtered
    nodes → honest summary → pager sync (filter badle to `top = 0`) →
    khali par centered hint + Clear. Page/header/grid geometry same.
- `checks/gui-filterpaint-au/GATES.md` (pehle likha), `REPORT.md`,
  `round-1/` (harness artifacts: `after.png`, `geo-typo-report.md`,
  `screenshot.out`, `tests.out`).

## Tests (before → after)

- `cargo test -p autocompare-gui`: 515 passed, 0 failed. Is wave ke
  9 naye `views::tests` (kind/status options lock, bar tokens,
  typos, name/kind/status narrowing, legacy+new combine, summary
  honesty, empty-hint honesty). Baaki suite untouched green.
- Naye coverage: naam case-insensitive contains; kind files-only /
  same-glyph narrowing; `hide_orphans` + `Changed` combine → sirf
  `diff.txt`; default summary == full counts (`6 entries`); status
  filter par `1 entries … 1 newer left` + nodes len match; empty →
  NoResults hint, all-filtered → NoMatch (`clear filters`), visible
  → `""`.
- `cargo clippy --all-targets`: zero warnings, zero errors (antim
  verify par poore repo me koi warning nahi).
- `rustfmt --edition 2024 --check` apni file par clean (in-place
  `rustfmt` sirf `views.rs` par lagaya; parallel wave ki files ko
  chhua nahi).
- Banned grep apni file me clean (sirf `#![forbid(unsafe_code)]`
  line; non-test me `unwrap()` / `expect(` / `panic!` / `todo!`
  zero — sirf `unwrap_or_*` jo allowed hai).
- `tools/gui-polish/run-geo-typo.sh checks/gui-filterpaint-au/round-1`:
  RESULT: PASS (PNG 9624 bytes, magic ok, distinct 256; geometry
  Grid x7 / widths 24; typo monospace x67 / strong x19 / small x79).

## Notes / risks

- `paint_folder` ka signature badla hai — purana caller koi nahi tha
  (dead-code painter), isliye koi toot nahi. App-side rewiring
  (Folder tab inline paint → `views::paint_folder` + session display)
  orchestrator ke paas pending hai; dual ko App pehle se filtered
  rows/summary deta hai isliye `paint_folder_dual`/`paint_folder_sync`
  untouched hain.
- `TREE_INDENT` (14.0) nahi chhua: App inline paint wahi shared const
  use karta hai, badalne se KO-owned paint hilta — alag wave dekhegi.
- Harness PNG `screenshot.rs` pixel-path se aata hai (views egui
  paint nahi), isliye PASS views-logic ke unit tests + build par
  tika hai; geometry/typo signals expected direction me badhe
  (monospace +1, strong +1, widths +1).
- Parallel waves ki `views_table.rs` working-tree changes ko chhua
  nahi; commit me sirf OWNS paths hain.
