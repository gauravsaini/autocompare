# REPORT — Wave AT-FOLDERFILTER

## Summary

Folder filter UX ka headless App wiring poora: naam / type / status
filters real kaam karte hain, counts filtered rows par honest hain,
khali states (`no results` vs `no match`) alag hain, filter-bar
affordance state (active count, clear) headless tested hai. Paint
adoption agle wave ka hai — koi egui code nahi chhua.

## Files changed (sirf OWNS)

- `crates/gui/src/tree.rs`:
  `FolderEntryKind` (`All`/`FilesOnly`/`DirsOnly`),
  `FolderStatusFilter` (`All`/`Same`/`Different`/`NewerLeft`/
  `NewerRight`/`LeftOnly`/`RightOnly`/`Changed`) naye; `FolderDisplay`
  me `name_query`/`kind`/`status` jude (legacy 4 fields untouched,
  `Copy` hata — `String` field ki wajah se, ab `Clone`);
  `filter_pass` sab shartein lagata hai (naam case-insensitive
  contains, khali/whitespace = no filter; unknown glyph sirf `All`
  me dikhta hai); affordance (`is_active`/`active_count`/`clear`);
  khali states (`FolderEmptyState::NoResults`/`NoMatch`,
  `folder_empty_state`/`folder_empty_hint`); geometry
  (`FOLDER_FILTER_BAR_HEIGHT/PAD_X/GAP` = 24/8/8, scale par) +
  typography (`folder_filter_typo` body-regular mono,
  `folder_filter_active_typo` caption-strong mono); `FolderSessionState`
  helpers (`has_active_filters`, `active_filter_count`,
  `clear_filters`, `empty_state`, `empty_hint`). Paint-clone fix:
  ek jagah `display` ab `.clone()` se (Copy hata tha).
- `crates/gui/src/app.rs`: filter-bar setters
  (`set_folder_name_filter`, `set_folder_entry_kind`,
  `set_folder_status_filter`, `set_folder_hide_orphans`,
  `set_folder_left_newer_only`, `set_folder_right_newer_only`,
  `set_folder_flatten`) — har ek pager/dual sync + status line;
  `clear_folder_filters`; getters (`folder_name_filter`,
  `folder_entry_kind`, `folder_status_filter`,
  `folder_has_active_filters`, `folder_active_filter_count`,
  `folder_empty_state`, `folder_empty_hint`). View state
  compare/swap par persist karta hai, sirf explicit clear girata hai.
- `checks/gui-folderfilter-at/GATES.md` (pehle likha), `REPORT.md`,
  `round-1/` (harness artifacts).

## Tests (before → after)

- `cargo test -p autocompare-gui`: 498 passed, 0 failed. Is wave
  ke 11 naye tests (8 `tree::folderfilter_tests` + 3 `app::tests`
  folder-filter); baaki suite untouched green.
- Naye coverage: naam case-insensitive/trim/khali/no-match; kind
  files/dirs; status har glyph + `Changed` + unknown-glyph honesty;
  legacy+naye combined narrowing; active-count 7/clear; empty states;
  geo-typo tokens; session wiring; counts honesty (`0 entries`,
  `1 newer left`); pager/dual sync.
- `cargo clippy --all-targets`: zero warnings (antim verify par
  poore repo me koi warning nahi; `views.rs` KEVL ko chhua nahi).
- `rustfmt --edition 2024 --check` apne dono files par clean
  (`cargo fmt -- <files>` lagaya).
- Banned grep (`\.unwrap\(\)|\.expect\(|panic!|todo!|...`) apne files
  me clean (sirf `unwrap_or*` + doc shabd, jo allowed hain).
- `tools/gui-polish/run-geo-typo.sh round-1`: RESULT: PASS
  (PNG 9624 bytes, magic ok).

## Notes / risks

- `FolderDisplay` se `Copy` hata — bahar ke callers `&display`
  dete hain, isliye koi toot nahi; ek paint call-site par `.clone()`.
- `Changed` = gyaat changed glyphs (`! > < L R`); unknown glyph
  sirf `All` me — taaki filter kabhi anjaan pankti galat shreni me
  na gine.
- Parallel wave ki `views.rs` + `checks/gui-findhl-at/` changes ko
  chhua nahi; commit me sirf OWNS paths hain. Antim re-verify ke
  waqt parallel agent ka aadha refactor (`views.rs` signature)
  build tod raha tha — uske settle hone par dobara green confirm
  karke hi commit kiya jayega.
- Colors frozen, koi hue nahi; geometry 4/8/12/16/24 scale par.
