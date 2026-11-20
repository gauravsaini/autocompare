# REPORT — CO-TBARCALL toolbar call-site adoption (App row -> views::paint_session_toolbar)

OWNS: `crates/gui/src/app.rs` + `checks/gui-toolcall-co/**`. NO-TOUCH: main.rs / views.rs /
theme.rs / screenshot.rs / STATUS.md / baaki crates / baaki checks (sab untouched, proof section 4).

## 1. Gap (toolwire REPORT section 5 loop: App wiring pending)

- Toolwire wave ne `views.rs` me icon-led contract + live paint fn banaya tha
  (`paint_session_toolbar`, `ToolbarAction`, `toolbar_swap_label` / `toolbar_reload_label`,
  `toolbar_button_role`, `session_toolbar_button_size`) par App row (app.rs toolbar block)
  abhi bhi plain-text paint karta tha — `"Swap Sides"` / `"Reload / Recompare"` default
  `egui::Button` par, koi glyph/role/floor reuse nahi.
- BC reference: toolbar-ch/toolwire audit se — baaki chrome me glyph+label pattern pehle se
  hai (gutter copy arrows, paging-nav `Next hunk`, mode toggle), sirf App toolbar row text-only thi.

## 2. Adoption (ponytail, smallest wiring diff — 14 insertions, 18 deletions, zero naya painter)

- App toolbar row ka Swap/Reload hissa `views::paint_session_toolbar(ui, toolbar_active)`
  call-site par gaya (`enabled = (tab != Home)`, pehle wala `toolbar_active` reuse).
- Hit handling caller side hai (paint sirf emit karta hai, `SessionAction` precedent):
  `Swap` -> `swap_sides()` safal par `run_active_tab()` (pehle jaisa Beyond Compare refresh),
  `Reload` -> `run_active_tab()`, `None` -> no-op.
- Glyph parity live-visible: labels ab `toolbar_swap_label` (`<-> Swap Sides`) /
  `toolbar_reload_label` (`▶ Reload / Recompare`), typo `toolbar_button_role` (Body 13
  proportional chrome), floor `session_toolbar_button_size` min_size (120x28) — sab views
  contract se, App me koi naya size/typo literal nahi.
- Jo chhuna nahi: `toolbar_item_gap` / `toolbar_group_gap` fns (outer rhythm same),
  Word Wrap / Line Numbers / Syntax Highlighting toggles (byte same), Home hint line
  (sirf descriptive text hai, row nahi), colors FROZEN (diff me `Color32`/`Visuals::` zero).
- Geometry/typo numbers zero change (sirf wiring): `git diff` me koi naya float literal nahi,
  sirf purane `add_enabled` blocks hate aur ek `match` call-site aaya.

## 3. Evidence

- `cargo test -p autocompare-gui` -> 673 passed, 0 failed.
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings` -> zero warnings.
- `rustfmt --edition 2024 --check crates/gui/src/app.rs` -> clean.
- Banned scan (diff-added lines): `unwrap` / `expect(` / `panic!` / `todo!` -> 0 hits;
  `forbid(unsafe_code)` kayam (app.rs line 10, untouched).
- Colors: `git diff crates/gui/src/app.rs | grep -c "Color32|Visuals::"` -> 0.
- Harness: `tools/gui-polish/run-geo-typo.sh checks/gui-toolcall-co/round-1/` -> exit 0,
  `RESULT: PASS` (round 1/6, PNG 9765B magic ok distinct 256).
- Fixture byte-stable: `before.png` (toolwire baseline) vs `after.png` (is wave) ->
  `cmp` BYTE-IDENTICAL, md5 `8ccf51e4824a03f84fb9e0d1b95fc521` dono. Wajah: `--screenshot`
  scene Home/launcher hai, toolbar row us scene me paint nahi hota — isliye fixture me
  koi pixel change nahi aana hi sahi behavior hai. Glyph parity ka live pixel verify
  round-7 scope hai; code path aisa rakha hai ki round-7 me Text-tab live capture par
  icon-led buttons (`<->` / `▶` prefix) verify ho saken.

## 4. Status proof (gate se pehle, shared-workdir savdhani)

```
M crates/gui/src/app.rs
?? checks/gui-toolcall-co/
```

## 5. Loop suggestion

- Agli wave (round-7): live Text-tab capture par glyph parity verify karo — Swap button
  `<->` prefix aur Reload button `▶` prefix ke saath dikhna chahiye, floor 120x28 same.
- `views.rs` ke `paint_session_toolbar` / `ToolbarAction` par laga `#[allow(dead_code)]`
  ab App live call-site ke baad extra hai — use hatana views.rs owner wave ka kaam hai
  (is wave me views.rs NO-TOUCH tha, isliye chhoda).
