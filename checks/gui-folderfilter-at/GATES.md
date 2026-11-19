# GATES — Wave AT-FOLDERFILTER

## Scope (OWNS only)

- `crates/gui/src/tree.rs`
- `crates/gui/src/app.rs`
- `checks/gui-folderfilter-at/**`

KEVL (mat chhoona, read bhi avoid): `views.rs`, `viewmodel.rs`,
`views_table.rs`, `views_hex.rs`, `home.rs`, `gutter.rs`, `theme.rs`,
`screenshot.rs`.

## Goal

Folder filter UX ka headless App wiring: name / type / status filters
real kaam karte hain, counts honest rehte hain, khali states jhooth
nahi bolti. Paint adoption agle wave ka hai — is wave me koi egui
code nahi.

## Bahar ka observation (Beyond Compare, apne shabdon me)

Bahar se dekhe folder compare me upar ek filter patti hoti hai: naam
me kuch likho to ped wahi ghata hai, file/dir ka chunta alag karta
hai, status chunta (same/diff/newer/only) wahi panktiyan rakhta hai.
Filter laga ho to patti khud batati hai kuch chhupa hai, ek safai
button sab wapas lata hai, ginati filtered panktiyon par hoti hai,
aur kuch na bache to khali sandesh jhoothi ginati nahi dikhata.

## API contract (headless, paint agle wave me inhi se banega)

`tree.rs` (pure, no egui, no I/O):

- `FolderEntryKind`: `All` / `FilesOnly` / `DirsOnly` + `label()`.
- `FolderStatusFilter`: `All` / `Same` / `Different` / `NewerLeft` /
  `NewerRight` / `LeftOnly` / `RightOnly` / `Changed` +
  `matches_glyph(glyph)` + `label()`.
- `FolderDisplay` naye fields: `name_query: String`, `kind`,
  `status`. Purane fields (`hide_orphans`, `left_newer_only`,
  `right_newer_only`, `flatten`) untouched rehte hain taaki purane
  tests/callers na tooten. `Copy` hatkar `Clone` hota hai (String
  field ki wajah se).
- `filter_pass(row, display)`: purane bools + kind + status + naam
  (case-insensitive contains, khali/whitespace = no filter) sab
  lagte hain. Filtering kabhi glyph rewrite nahi karti, sirf rows
  chhupti hain.
- Affordance state: `FolderDisplay::is_active()`,
  `FolderDisplay::active_count()`, `FolderDisplay::clear()`.
- Khali states: `folder_empty_state(total, visible, has_filters)` —
  `total == 0` par `NoResults`, `visible == 0 && has_filters` par
  `NoMatch`, warna `None`. `folder_empty_hint(state)` static Hindi-free
  English line deta hai taaki paint/snapshot stable rahe.
- Counts: `count_folder_rows` / `filtered_counts` untouched rahenge;
  filtered summary hi honest line hai.
- Geometry first (4/8/12/16/24 scale, px):
  `FOLDER_FILTER_BAR_HEIGHT = 24.0`, `FOLDER_FILTER_BAR_PAD_X = 8.0`,
  `FOLDER_FILTER_BAR_GAP = 8.0`. Indent (`12.0`) aur row height
  (`24.0`) frozen tokens reuse hote hain, badalte nahi.
- Typography second (mono + size + weight, `FolderTypo` reuse, koi
  theme import nahi, koi hue nahi): `folder_filter_typo()` body-regular
  mono, `folder_filter_active_typo()` caption-strong mono (active
  chip ek nazar me), colors frozen — yahan koi hue nahi.

`app.rs` (App wiring, pure state + status lines):

- Setters: `set_folder_name_filter`, `set_folder_entry_kind`,
  `set_folder_status_filter`, `set_folder_hide_orphans`,
  `set_folder_left_newer_only`, `set_folder_right_newer_only`,
  `set_folder_flatten`. Har setter pager/dual sync karta hai taaki
  scroll stale na rahe.
- `clear_folder_filters()` — poora display default par, pager/dual
  sync + status line.
- Affordance getters: `folder_has_active_filters()`,
  `folder_active_filter_count()`, `folder_empty_state()`,
  `folder_empty_hint()`. `folder_counts_summary()` pehle se filtered
  counts par hai — yahi honest line rehti hai.
- Filter view state compare/swap par persist karta hai (view state
  hai, model nahi). Sirf explicit clear girata hai.

## Non-goals

- Koi paint/egui code nahi (`views.rs` dusre agent ka hai).
- Colors me koi badlav nahi.
- Filesystem I/O nahi, engine rules nahi badalte.
- `unwrap` / `expect` / `panic` / `todo` non-test code me mana hai.
  `forbid(unsafe_code)` barkarar.

## DoD

- [ ] Name/type/status filters real kaam karte + counts sahi.
- [ ] Honest empty states (no-results vs no-match alag).
- [ ] Filter bar affordance state (active count, clear) headless tested.
- [ ] Unit tests (tree + app) + full `cargo test -p autocompare-gui` green.
- [ ] `cargo clippy --all-targets` zero warnings.
- [ ] `cargo fmt --check` (apne OWNS files par `cargo fmt -- <files>`).
- [ ] Banned grep clean (non-test `unwrap|expect|panic|todo`).
- [ ] `tools/gui-polish/run-geo-typo.sh` headless proof PASS.
- [ ] `REPORT.md` + `round-1/` ant me.
- [ ] `git add` sirf OWNS paths + `git commit` (conventional English).
        No push. No stash. Repo ke bahar koi copy nahi.
