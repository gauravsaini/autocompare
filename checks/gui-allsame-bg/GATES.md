# GATES — Wave BG-ALLSAME (micro)

## Scope (OWNS only)

- `crates/gui/src/screenshot.rs` (fixture + dispatcher route + tests only)
- `checks/gui-allsame-bg/**`

MAT CHHOONA (parallel waves chal rahi hain — sirf read kiya): `views.rs`,
`app.rs`, `tree.rs` (paint/classifier live hain), `checks/opencode-waves/STATUS.md`.
Colors FROZEN.

## Goal

BF REPORT S4 P2-1 close karna: accent-scene precedent par naya headless
fixture route jo identical-folders AllSame case render kare (do same folders,
`visible == 0` + all-same). Wiring (`folder_paint_empty_hint_full_rows` +
app state) pehle se live hai — sirf headless PNG proof missing tha.

## API contract (headless, pure — no egui, no I/O beyond PNG write)

- Dispatcher: file naam me `allsame` -> `render_folder_allsame_fixture_png`
  (accent check ke baad, folder checks se pehle; `after-allsame.png` me
  `folder`/`empty` substring nahi, phir bhi order robust hai).
- `render_folder_allsame_fixture_png(out_path)`:
  - Rows: 3 identical (`src` dir + 2 files, sab glyph `=`).
  - Display: `FolderDisplay { status: Different, ..default }` (diff-seeking;
    `=` rows prune -> `folder_tree_filtered` len 0).
  - Hint: live paint wiring `views::folder_paint_empty_hint_full_rows`
    -> `"folder: folders match — all same, nothing to sync"`.
  - Filter laga hai to live `paint_folder` jaisa Clear affordance; sync panel
    folder-empty precedent par (identical rows par steps khaali).
- 3 naye tests: PNG valid, AllSame contract (state + hint + width budgets),
  dispatcher distinctness (text/folder/folder-empty se alag).

## Non-goals

- `views.rs` / `app.rs` / `tree.rs` me koi edit nahi (paint + classifier
  frozen-live hain).
- Koi palette/Visuals edit nahi; naye glyph/hue nahi (Clear/Apply boxes
  frozen `[60,60,60]` + `FG_TEXT` reuse karte hain).
- Non-test me `unwrap`/`expect`/`panic`/`todo` mana; `forbid(unsafe_code)`
  barkarar.

## DoD

- [x] `cargo test -p autocompare-gui` full green (584 passed, 0 failed).
- [x] `cargo clippy -p autocompare-gui --all-targets` zero warnings.
- [x] `cargo fmt -p autocompare-gui` clean (sirf OWNS file touch hui).
- [x] Banned grep: non-test diff me zero new (`unwrap` hits sirf tests mod me).
- [x] Harness `tools/gui-polish/run-geo-typo.sh` RESULT: PASS.
- [x] PNGs: magic ok, dono >= 8192 B, distinct > 64, apas me + twins se alag.
- [x] `REPORT.md` + `round-1/after-allsame.png` + `round-1/after.png`.
- [x] Conventional commit, NO PUSH.
