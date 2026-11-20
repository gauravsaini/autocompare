# CD-STATUS round-1 - 3-zone paint adoption (BT model wiring)

## 1. Summary

BT wave ka headless model (`viewmodel.rs`: `StatusZones`, `cursor_pos_for`/`cursor_label`, `focused_line_preview_for`, `status_zones_for` + 7 tests) is wave me live paint me adopt hua. Sirf `crates/gui/src/views.rs` me 2 naye paint fns + 4-line call-site wiring + 2 wiring-contract tests; `viewmodel.rs` NO-TOUCH (uske `wiring-pending` allows ab stale hain, cleanup BT owner/next wave ka). Harness `round-1/` me `RESULT: PASS`. Koi color change nahi, koi `unwrap/expect/panic/todo` nahi (non-test).

## 2. Unlazy (GATES pehle, implement baad me)

`checks/gui-status-cd/GATES.md` CHECK/EXPECT table pehle likhi (C1-C10), phir paint implement kiya. Ponytail laziest: naye tokens zero, naya size/color/Grid/add_space pattern zero - sab toolbar/find-bar precedent ka reuse (`HEADER_H` row, `TIGHT_GAP` spacing, `BADGE_PAD` right dock, `chrome_status_role` + `TypoRole::Mono`).

## 3. Zone table (paint contract, BT section 5 se)

| Zone | Content | Source (reuse, koi math duplicate nahi) | Geometry | Typo |
|---|---|---|---|---|
| cursor strip (status ke UPAR) | line 1: `"3:1 \| L3 / R5 Changed"`, neeche left/right preview lines | `cursor_label(cursor_pos_for(selected))` + `focused_line_preview_for(rows, selected)`; preview `None` (no selection/OOB) par strip skip, koi fake row nahi | `ui.horizontal` + `TIGHT_GAP` (toolbar precedent) | line 1 `chrome_status_role`, preview lines `TypoRole::Mono` |
| left | `"Differences: 2 rows (1 shown)"` (honest counts) | `status_zones_for(&counts, model.filter, mid, &nav_badge)` | `ui.horizontal` row (`avail x HEADER_H`), `TIGHT_GAP` spacing | `chrome_status_role().rich(...)` |
| mid | focused verdict (`"Changed"`) ya `"no selection"` (kabhi jhootha verdict nahi) | `focused_kind_label(&rows, selected)` se `mid` | `separator` + label (toolbar precedent) | `chrome_status_role().rich(...)` |
| right | `"1/1 \| F3 next/prev"` | maujooda `diff_nav_count_label(diff_nav_position(..), nav_total)` reuse | `with_layout(right_to_left)` dock + `BADGE_PAD` (badge precedent) | `chrome_status_role().rich(...)` |

Call site: `paint_text_body` me purani single counts line (`Small.rich("{total} rows ({shown} shown): {diffs} diffs...")` + `selected row {i}` text) hata kar wahi 4 lines - `mid` -> `zones` -> `paint_text_cursor_strip` -> `paint_text_status_bar`. `counts`/`nav_badge` upar pehle se compute the, wahi reuse.

## 4. Harness + PNG evidence (round-1, SKILL loop)

- `sh tools/gui-polish/run-geo-typo.sh checks/gui-status-cd/round-1/` -> exit 0, `geo-typo-report.md` me `RESULT: PASS` (round 1/6).
- `after.png`: PNG magic `89504e470d0a1a0a` ok, 9765 bytes (>= 8192 ok), distinct 256 (> 64 ok, non-blank).
- `cargo test -p autocompare-gui`: 661 passed, 0 failed (mere 2 naye wiring tests samet).
- `cargo clippy -p autocompare-gui --all-targets`: 0 warnings.
- `rustfmt --edition 2024 --check crates/gui/src/views.rs`: clean.
- Banned grep (non-test, mere diff par): `unwrap(`/`expect(`/`panic!`/`todo!`/`unsafe` zero. Colors: `git diff` me `Color32`/`Visuals` zero new.

## 5. Tests (2 naye, headless wiring-contract)

- `text_status_wiring_zones_use_focused_verdict_and_nav_badge`: paint call-site ka data flow - counts + filter + focused verdict + nav badge -> `zones`; left me filter label + `"2 rows (1 shown)"`, mid `"Changed"`, right me badge + `"F3"`.
- `text_cursor_strip_skips_without_preview`: skip condition - `None`/OOB par preview `None` (strip skip, fake row nahi); cursor label `None` -> `"-"`, `Some(0)` -> `"1:1"`.

## 6. Follow-up (orchestrator/BT owner)

1. `viewmodel.rs` ke 8 `#[allow(dead_code)] // wiring-pending` comments ab stale hain (sab items paint se wired) - BT owner ya next touch wave me hataye; is wave me `viewmodel.rs` NO-TOUCH tha.
2. Cursor column aaj hamesha 1 hai (row cursor only) - col cursor chahiye to alag P-proposal, is paint me scope-creep nahi.
3. `after.png` default scene ka hai (text tab scene nahi) - visual zone proof ke liye text-scene screenshot chahiye to harness scene list me add karna hoga.

## 7. Notes

- Is wave me `app.rs`, `home.rs`, `main.rs`, `viewmodel.rs`, `views_table.rs`, `theme.rs`, `gutter.rs`, `tree.rs`, `screenshot.rs` ko touch nahi kiya. Start me tree me sibling dirt thi (`main.rs` M + `checks/gui-darkfix-ce/` untracked) - revert/haath nahi lagaya; gate time tak sibling waves commit kar chuki thin, final `git status` me sirf mere OWNS dirty hain.
- `checks/opencode-waves/STATUS.md` untouched. PUSH MANA - commit local rahega, orchestrator verify karke push karega.
