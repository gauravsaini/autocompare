# round-1 REPORT — leaf-3.2 ux-keymap retry (typed keymap + AA + migration)

Scope (OWNS only): `crates/sessions/src/settings.rs`,
`crates/gui/src/theme.rs`, `checks/ux-keymap/**`. Nothing else touched
(working tree has sibling-wave dirty files; left alone). Clean-room:
outside observation only; no decompilation, no asset copying.

## What changed (resumed WIP + this retry)

- `crates/sessions/src/settings.rs`: `KeymapPreset` enum
  (default/vim/vscode/emacs) with `as_str` / `from_preset_name`
  (aliases: standard/mac/macos, vi, code) / `all`; `AppSettings`
  `keymap_preset()` / `set_keymap_preset` / `with_keymap_preset`,
  `is_dark_theme` / `is_light_theme`; v0 `migrate_settings` keymap alias
  keys (`keymap|keys|keybindings|key_map|shortcut_preset|shortcuts`) +
  theme/appearance aliases; unit tests incl. fixture migration test.
- `crates/gui/src/theme.rs`: typed `KeymapPreset` (gui mirror),
  `KeyAction` (Find/ToggleInfo/ResetZoom/ZoomIn/ZoomOut/NextDiff/
  PrevDiff/ToggleLineNumbers/ToggleWrap), `KeyBinding`
  (cmd/cmd_shift/ctrl/plain/with_shift/with_alt/matches),
  per-preset binding tables, `keymap_bindings` / `keymap_resolve` /
  `keymap_shortcut_hint` ("⌘F","⌘I","⌘0"), `keymap_has_core_mac_bindings`;
  AA tokens (`WCAG_AA_*`, `palette_all_text_tokens_aa`,
  `theme_all_palettes_satisfy_aa`); this retry: `cargo fmt` collapse of
  6 if/else token fns (zero semantics change).
- `checks/ux-keymap/round-1/fixtures/edge_keymap_settings.json`: v0 doc
  (`appearance: solarized_light`, `key_map: vi`, `tab_width: 2`,
  `read_only: true`, unicode + extra keys) → migrates to schema 1,
  light theme, vim preset, locked, tab_size 2,
  `extra.vim_esc_delay_ms == "25"`.

## Gates (all `CARGO_TARGET_DIR=/tmp/target-op-uxkey2 CARGO_BUILD_JOBS=2`)

- `cargo test -p autocompare-sessions -p autocompare-gui`: ok — gui lib
  743 passed; sessions 31 + corrupt 11 + migration 20 + round_trip 12
  passed; 0 failed.
- `cargo clippy -p autocompare-sessions -p autocompare-gui --all-targets
  -- -D warnings`: 0 errors.
- `cargo fmt --all --check`: clean (this retry fixed 6 theme.rs diffs).
- Banned-token scan of non-test src (`unwrap(`/`.expect(`/`panic!`/
  `todo!`/`unimplemented!`/`unreachable!`, `unsafe`): 0 hits in
  settings.rs (lines 1–701) and theme.rs (non-test part); test-only
  expect/unwrap live under pre-existing `allow(...)` headers.

## Contrast (WCAG AA ≥ 4.5, recomputed this round)

| Pair | Ratio |
|---|---|
| dark base | 13.80 |
| light base | 17.35 |
| dark dim | 5.82 |
| light dim | 6.49 |
| dark red | 5.95 |
| light red | 6.48 |
| dark green | 10.14 |
| light green | 5.98 |
| dark blue | 7.50 |
| light blue | 6.91 |

All 10 pairs AA; fixed diff pairs + `theme_all_palettes_satisfy_aa`
covered by unit tests.

## Limitations

1. Gui `KeymapPreset` mirrors sessions enum (no shared crate yet; kept
   disjoint per OWNS).
2. Paint wiring (`views.rs`/`app.rs`) is out of scope — headless
   contract only.

## RESULT: PASS
