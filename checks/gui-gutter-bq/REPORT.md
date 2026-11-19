# REPORT — gui-gutter-bq (text dual-pane + center gutter, round-1)

Scope: livecomp REPORT G-T1 (P0) + G-T3 (P1). OWNS: `crates/gui/src/views.rs` + `checks/gui-gutter-bq/**`. Colors FROZEN, TypoRole reuse (koyi naya `.size` nahi), header compress untouched.

## Geometry table (before -> after)

| Area | Before | After | Line |
|---|---|---|---|
| Row shape | single combined row: `[dot/▶] left_job + inline →/← ya " \| " + right_job` | dual-pane: `left_job \| fixed center gutter \| right_job` (wrap + non-wrap dono) | views.rs `paint_text_body:2348`, wrap head `~2694`, continuation `~2750`, non-wrap `~2789` |
| Center gutter | koyi dedicated column nahi (buttons panes ke beech inline) | fixed-width column `TEXT_CENTER_GUTTER_W` = 48.0 (`HEADER_H + GUTTER_W + 2*TIGHT_GAP` reuse) | views.rs `101`, `105` |
| Gutter content | har row me inline pair | diff par `→` + `←` (gutter me, `TIGHT_GAP` spacing); equal par blank fixed rect + click-to-select | views.rs `111` (`text_gutter_has_copy`: Equal nahi, baaki haan) + row sites upar |
| Line-number gutter | number `gutter_text` se chipka + sabse left `·/▶` dot column | `gutter_text` shape same (right-aligned, fixed `gutter_chars` width); leading `·/▶` column hata di | views.rs `3483`, test `6145` |
| Selection affordance | `·` button / `▶` marker | bg strip (`CHANGED_BG` reuse) + equal-row blank gutter click + copy action par select; marker column nahi | views.rs row sites upar |
| Toolbar + status | full-width nav/filter/copy section row + `Small` status line | same, untouched (pehle wave ka compress bana hua) | views.rs `~2437` toolbar, `~2521` status |
| Continuation rows | leading `CONTENT_LEAD` + inline `" \| "` | same dual-pane columns, gutter blank fixed width (`Sense::hover`), copy sirf head par | views.rs `~2750` |

## Exact lines (landed)

- Token + helpers: views.rs `101` (`TEXT_CENTER_GUTTER_W`), `105` (`text_center_gutter_width`), `111` (`text_gutter_has_copy`).
- Paint: views.rs `paint_text_body:2348`; wrap head `~2694-2745`; wrap continuation `~2750-2765`; non-wrap row `~2789-2875`.
- Number gutter: views.rs `3483` (`gutter_text` — shape unchanged, right-align + fixed width).
- Test: views.rs `6152` (`text_center_gutter_reuses_tokens_and_gates_copy`: 48.0 lock + copy gate).

## Gates (round-1)

- `cargo test -p autocompare-gui`: 611 passed, 0 failed.
- `cargo clippy -p autocompare-gui --all-targets`: exit 0.
- `rustfmt --edition 2024 --check crates/gui/src/views.rs`: clean.
- Banned (`unwrap/expect(/panic!/todo!` diff par): 0; `#![forbid(unsafe_code)]` present.
- Colors: diff par zero `Color32`/`Visuals`/palette touch; typo me koyi naya `.size(` nahi (TypoRole reuse only).
- Harness `tools/gui-polish/run-geo-typo.sh checks/gui-gutter-bq/round-1`: `RESULT: PASS` (PNG 9695 B, magic ok, distinct 255).
- OWNS: commit me sirf `crates/gui/src/views.rs` + `checks/gui-gutter-bq/**`; `views_table.rs` parallel wave ka hai, untouched. `checks/opencode-waves/STATUS.md` untouched.

## Follow-up

- P0: koyi nahi — G-T1 + G-T3 isi wave me done.
- P1 (agle wave ke liye, is commit me nahi): G-T2 diff highlight full-row-width band; G-T4 status bottom bar + cursor/format info; G-T7 mono size/leading `theme.rs` me (paint adoption). G-F2 folder grid columns aur G-B1/B3 table grid parallel `sess-table-bq` wave ke paas hai.
