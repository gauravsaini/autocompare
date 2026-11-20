# HOME5 Exquisite Report — home cards plus session tree

## Kya kiya
- Home cards + session tree ka headless contract `crates/gui/src/home.rs` me joda: naya HOME5 block (geometry first, typography second, colors frozen).
- Geometry: new-session button 224x28 (card CTA parity, hit area 6272), session tree row 24 (compact hit floor parity), tree width 240 (card min width parity), content margins 12 (outer margin parity), card gaps 24 (card rhythm parity).
- Typography: HomeTitle/HomeDesc/HomeButton + caption reuse, koi naya size nahi; `home5_typo_reuses_home_roles` teenon hierarchy (card, recent, tree) ek saath pakadta hai.
- Edge fixture: `comparison/home5-empty-8tabs.json` — empty workspace (9 collapsed groups, tree height 224) + 8-tabs overflow (960 par fit, 300 par scroll, clip nahi) + 8-card grid narrow wrap (600 par 2 cols x 4 rows).

## Harness
- `tools/exquisite/run.sh checks/gui-exq-home5/round-1` — max6 me round-1 me hi PASS, dobara run ki zaroorat nahi padi.
- PNG: 10401 bytes (min 8192 ok), magic ok, distinct 256 (blank nahi).
- Geometry signals: margin/gap consts x274, Grid x14, row-h x112, width calls x67, gutter x805.
- Typo signals: TypoRole x720, monospace x26, strong x41, .size ['14.0'], weights MEDIUM/REGULAR/SEMIBOLD, line-heights BODY/CHROME/MONO/SOURCE.
- Colors frozen: Color32/Visuals touches 33 — home4 jitna hi, ek bhi naya rang nahi.

## Test
- Naya test `home5_session_tree_cards_margins_empty_8tabs_edge`: button/row/margin/align + empty tree build + 8-tabs scroll + narrow wrap + typo reuse, sab ek jagah.
- Lib tests: 817 passed, 0 failed (baseline 816 + 1 naya).

## Note (blocker, meri file se bahar)
- Main working copy me sibling TREE5 wave ka `tree.rs` WIP dirty hai (`folder_tree_scoped_global` missing) — isliye main copy par `cargo test`/`clippy` red hai. Mere pathspec ke gates isolated worktree (HEAD + sirf meri `home.rs`) me prove kiye: test green, fmt clean, banned 0, clippy me meri file se zero warning (ek pre-existing error `views_table.rs:3110` HEAD par hai, forbidden file — maine chhua nahi).
