# MODEL9 REPORT — cursor plus selection (Hinglish-Roman)

Round: `checks/gui-exq-model9/round-1` — harness 1 round (max6 me se 1 hi laga).

## Kya gap tha (BC vs ours, apne shabdon me)

BC me ek shared row cursor hai (`1:1`, bina selection `-`) jo dono panes ke pane tops aur status verdicts chalata hai — khali compare par koi fake row nahi, bhare compare par pehli diff se khulta hai, tail diff par pane top selection ka peecha karta hai. Ours me tukde bikhre the: selection clamp `paint_text_body` me inline tha, diff-nav badge math `views::diff_nav_*` me tha, pane-top follow ka koi headless single source nahi tha. Status verdict (`focused_kind_label`) pehle se honest tha (`no selection`), par badge + tops ka model ek jagah nahi tha.

## Kya kiya (geometry FIRST, typo SECOND, colors FROZEN)

Geometry first (koi naya size nahi, sab reuse):
- `sanitize_selection(total, sel)` — OOB/empty par `None` (honest empty, koi fake cursor nahi); paint ka inline gate isi ko adopt karega.
- `initial_selection_for(rows)` — pehla difference, equal-only/empty par `None` (koi fake "row 1" nahi).
- `pane_top_for_selection(top, per_page, sel, total)` — selection window `[top, top+per_page)` ke bahar ho to top uska peecha karta hai (neeche ho to aakhri row, upar ho to pehli), andar ho to untouched; empty/zero-page par 0. `virtual_row_window` contract reuse.
- `diff_nav_total_for` / `diff_nav_position_for` / `diff_nav_badge_for` — kul diffs, 1-based pos, N/M badge (`0/0` jab koi hit nahi; `find_count_label` shape reuse). Status right zone + action-bar ka single source; paint agle wave me adopt karega.
- Paint wiring pending (`views.rs` text/status/action-bar) — is wave me sirf headless model + tests, koi paint edit nahi (OWNS bahar hota).

Typo second (koi naya size/weight nahi):
- `cursor_strip_head_role()` → Body (status verdict precedent reuse, 13), `cursor_strip_preview_role()` → Mono (pane body precedent reuse, 12) — hierarchy head > preview, preview tabular rehta hai.

Colors frozen: koi naya Color32/Visuals nahi — sirf usize/String/TypoRole pure fns; exquisite-report me colors 33 (pichhle wave jaisa, growth nahi).

## Edge fixture (empty-vs-loaded + tail-insertion)

`comparison/edge-model9.json`: empty model (0 rows) me sanitize None, initial None, badge `0/0`, top 0, cursor `-`, verdict `no selection`; loaded 5-row model (diffs 1,2,4) me initial 1, tail badge `3/3`, per_page 2 top 0 se pane top 3 (window (3,5)). Naye headless tests (`model9_empty_model_is_honest`, `model9_loaded_selection_drives_pane_top_and_badge`, `model9_cursor_roles_reuse_existing_sizes`) yehi gates lock karte hain; purane nav/status tests untouched green.

## Natija

- PNG 10401B, magic ok, distinct 256 (blank nahi) — `round-1/after.png`.
- 864 tests green (3 naye model9 tests samet), clippy 0, fmt clean (owned-file `rustfmt --edition 2024 --check`), banned 0 (owned scope non-test).
- Harness `RESULT: PASS` (1 round); diff 178 insertions, 0 deletions (pure additive).
