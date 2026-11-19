# REPORT — Wave BD-FOLDERHOME (folder AllSame paint wiring + home CARD_H verify)

## Summary

Do kaam sequential order me hue. P0-1: BB-FOLDER3 REPORT ke patch spec
1/2 + 2/2 EXACT adopt — `views.rs` me `folder_paint_empty_hint_full_rows`
naya fn, `paint_folder` khali-view branch usi ko padhta hai (1-line
callsite), `app.rs` me `folder_empty_state_full_rows` +
`folder_empty_hint_full_rows` spec-verbatim; legacy glyph-blind fn +
uske tests byte-identical frozen (sirf `allow(dead_code)` test-lock,
repo precedent par). P1-3: verify par nikla home CARD_H adoption pehle
se live hai (commit `1a7a572` — paint 99.6 + CTA 120x28 + 3-col grid) —
is wave me home block me zero diff, sirf test-lock verify.

## Files changed (sirf OWNS)

- `crates/gui/src/views.rs`:
  - Naya `folder_paint_empty_hint_full_rows(rows, visible, display)` —
    spec 1/2 verbatim (`has_model` hamesha `true`, rows model se aate hain).
  - Callsite: `paint_folder` khali branch ab legacy ke bajaye
    `_full_rows` padhta hai (1 line). Legacy fn frozen + `allow(dead_code)`.
  - Naya test `folder_paint_empty_hint_full_rows_routes_all_same`:
    khaali model → `NoResults`; single `'='` row + `Different` filter →
    `Some(AllSame)` + `"all same"`; visible = 1 → `""`.
- `crates/gui/src/app.rs`:
  - Naye `folder_empty_state_full_rows` + `folder_empty_hint_full_rows`
    — spec 2/2 verbatim (`visible_tree(rows).len()`, `None` → `NoResults`).
  - Naya test `folder_empty_full_rows_routes_identical_vs_pruned`:
    no-model → `NoResults`; single `'='` row + `Different` filter →
    `AllSame` + `"all same"`.
- `checks/gui-folderhome-bd/GATES.md`, `REPORT.md` (yeh file),
  `round-1/after.png` (harness shot).

## Tests

- `cargo test -p autocompare-gui`: **578 passed, 0 failed**. Naye 2 ok;
  padosi legacy (`folder_paint_empty_hint_stays_honest`,
  `folder_empty_hints_separate_no_results_from_no_match`,
  `folder_counts_line_uses_honest_summary` family) bhi green — regression nahi.
- `cargo clippy -p autocompare-gui --all-targets`: zero warnings.
- Banned grep apne diff par zero new
  (`unwrap` / `expect(` / `panic!` / `todo!` / `unsafe` — koi match nahi).
- Harness `tools/gui-polish/run-geo-typo.sh`: RESULT: PASS
  (PNG 9627 B, magic ok, distinct 256 — text fixture route).
- `rustfmt --edition 2024 --check` sirf OWNS files par: clean.

## P1-3 verify (zero diff, kyun)

- `paint_home` pehle se `ui.set_height(home_paint_card_height())` chalata
  hai = `home_card_min_height().max(CARD_H)` = 99.6 (test `99.6 ± 0.01`
  lock karta hai, `CARD_H` 75 se upar assert hai).
- CTA pehle se `home_paint_cta_size()` = `(120.0, 28.0)`; grid pehle se 3
  columns (`HOME_GRID_COLUMNS`); width `CARD_W` 240 = `HOME_CARD_MIN_W`
  240 (contract barabar). Purana 75 paint me kahin nahi bacha — sirf
  legacy `CARD_H` const floor ke roop me hai (test `6223` usko lock karta
  hai, chhua nahi).
- Isliye P1-3 par koi paint edit nahi ki — single-token adoption pehle
  se poori hai; dobara haath lagana redesign hota.

## Notes / risks

- Paint adopt karne se filtered-to-empty mixed case ki wording bhi
  `_full_rows` classifier se aati hai (`FilterPruned`:
  `"filters hide every row — clear filters"`, legacy `"no rows match
  filters"` ke bajaye). Yeh classifier ka intended honest route hai
  (az-wave se frozen pada tha, paint ne kabhi padha nahi) — P0-1 ka hissa.
- `Clear filters` button gate (`display.is_active()`) byte-identical hai:
  AllSame + active filter par Clear ab bhi dikhta hai. Filter hatane par
  identical rows wapas dikhti hain, isliye misleading nahi — spec me gate
  change nahi tha, chhua nahi.
- Sibling wave (`views_hex.rs`, `checks/gui-hexpure-bd/`) same tree me
  parallel chal rahi hai — commit me sirf OWNS paths stage kiye.
- Commit conventional English, NO PUSH (orchestrator verify + push karega).
