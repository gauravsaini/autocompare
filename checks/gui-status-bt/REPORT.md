# BT-STATUS round-1 - 3-zone status + cursor model (P1-2/P1-3)

## 1. Summary

`checks/gui-localrun-bs/REPORT.md` section 5 ke P1-2 (status bar one-sided, BC 3 zones + cursor `1:1`) aur P1-3 (cursor-detail strip missing, focused-line preview nahi) ka model half is wave me land hua. Sirf `crates/gui/src/viewmodel.rs` me headless model + 7 unit tests; `views.rs` paint fn is wave me NO-TOUCH (BT-DIFF owns) - exact paint spec section 5 me. Harness `round-1/` me `RESULT: PASS`. Koi color change nahi, koi `unwrap/expect/panic/todo` nahi (non-test).

## 2. Unlazy (GATES pehle, implement baad me)

`checks/gui-status-bt/GATES.md` CHECK/EXPECT table pehle likha (C1-C10), phir model implement kiya. Ponytail laziest: naye types sirf 3 structs + 1 const + 7 pure fns, maujooda `FilterCounts`/`TextFilter`/`TextRow` reuse, koi naya token/color/size nahi.

## 3. Harness + PNG evidence (round-1, SKILL loop)

- `sh tools/gui-polish/run-geo-typo.sh checks/gui-status-bt/round-1/` -> exit 0, `geo-typo-report.md` me `RESULT: PASS` (round 1/6).
- `after.png`: PNG image data, 960x600, RGBA, 9765 bytes (>= 8192 ok), magic `89504e470d0a1a0a` ok, distinct 256 (> 64 ok).
- `cargo test -p autocompare-gui`: 629 passed, 0 failed (combined tree; mere 7 naye tests samet).
- `cargo clippy -p autocompare-gui --all-targets`: meri file se 0 warnings (naye items par `#[allow(dead_code)] // wiring-pending` precedent pattern par); baaki 5 warnings `views.rs` me sibling wave ke uncommitted filler helpers ke hain - mera scope nahi, haath nahi lagaya.
- `rustfmt --check` meri file par clean; baaki files me zero diff meri taraf se.
- Banned grep (non-test): `unwrap(`/`expect(`/`panic!`/`todo!` zero. Colors: `git diff` me `Color32`/`Visuals` zero new (`viewmodel.rs` me pehle bhi 0 the).

## 4. Model spec (`crates/gui/src/viewmodel.rs`, section "Status bar + cursor")

```rust
pub struct StatusZones { pub left: String, pub mid: String, pub right: String }
pub struct CursorPos { pub row: usize, pub col: usize }     // dono 1-based, col hamesha 1
pub struct FocusedLinePreview { pub line_no: String, pub kind: String, pub left: String, pub right: String }
pub const STATUS_PREVIEW_MAX_CHARS: usize = 120;

pub fn text_filter_label(filter: TextFilter) -> &'static str   // All/Differences/Same/Context
pub fn text_row_kind_label(kind: TextRowKind) -> &'static str  // Equal->Same, Changed/Deleted/Inserted
pub fn focused_kind_label(rows: &[TextRow], selected: Option<usize>) -> &'static str // None/OOB -> "no selection"
pub fn cursor_pos_for(selected: Option<usize>) -> Option<CursorPos> // 0-based -> 1-based, col 1
pub fn cursor_label(pos: Option<CursorPos>) -> String       // "3:1", None -> "-"
pub fn focused_line_preview_for(rows: &[TextRow], selected: Option<usize>) -> Option<FocusedLinePreview>
pub fn status_zones_for(counts: &FilterCounts, filter: TextFilter, mid: &str, nav_badge: &str) -> StatusZones
```

Semantics: left = `"{filter}: {total} rows ({shown} shown)"` (honest counts, `filter_counts` se); mid = focused verdict ya `"no selection"` (kabhi jhootha verdict nahi); right = `"{nav_badge} | F3 next/prev"` (nav badge paint ke maujooda `diff_nav_count_label` se aayega, math duplicate nahi); `line_no` shape: barabar numbers par `"3"`, alag par `"L3 / R5"`, ek taraf par `"L7"`/`"R7"`, bina numbers par `"-"`; preview text `STATUS_PREVIEW_MAX_CHARS` chars par `...` suffix se cut (exact cap par suffix nahi); selection None/OOB par preview `None` (khali par jhootha preview nahi).

Tests (7, sab pure): filter labels sab 4, kind labels sab 4, focused-kind honest (None/OOB/Equal/Changed/Deleted), cursor 1-based + `"-"`, zones compose (left/mid/right substrings), preview number shapes (5 shapes), preview cut (cap+3 length, exact-cap no suffix).

## 5. `views.rs` paint spec (BT-DIFF owns, is wave me ZERO edits)

Maujooda paint site: `paint_text_body` me single counts line (`views.rs` ~2521, `ui.label(TypoRole::Small.rich(...))`). BT-DIFF uski jagah ye do fns lagaye:

```rust
// views.rs (BT-DIFF owns)
pub fn paint_text_cursor_strip(ui: &mut Ui, rows: &[TextRow], selected: Option<usize>)
pub fn paint_text_status_bar(ui: &mut Ui, zones: &StatusZones)
```

- Data flow: `let counts = viewmodel::filter_counts(&model.rows, model.filter)` (maujooda, reuse); `mid = focused_kind_label(&model.rows, model.selected)`; `nav_badge` = maujooda `diff_nav_count_label(...)` (reuse, duplicate math nahi); `zones = status_zones_for(&counts, model.filter, mid, &nav_badge)`; cursor strip inputs = `cursor_label(cursor_pos_for(model.selected))` + `focused_line_preview_for(&model.rows, model.selected)`.
- Cursor strip order: status bar ke UPAR paint ho; line 1 me cursor label + `line_no`/`kind`; uske neeche left/right preview text lines; preview `None` par strip skip (khali par koi fake row nahi).
- Status bar layout: ek `ui.horizontal` row (`HEADER_H` height, `TIGHT_GAP` spacing - toolbar precedent jaisa); left me `zones.left`, beech me separator + `zones.mid`, right-end dock me `zones.right` (`with_layout(right_to_left)` + `BADGE_PAD`, toolbar badge precedent jaisa).
- Tokens: sirf maujooda - `chrome_status_role().rich(...)` labels ke liye, `TypoRole::Mono` preview text lines ke liye; naya size/color/palette MANA (colors frozen); `add_space` naya nahi, `Grid` naya nahi.

## 6. Follow-up (BT-DIFF / orchestrator)

1. Section 5 ke do paint fns + call-site wiring (selection sync samet) - OWNS `views.rs`, warna mera model dead rahega.
2. Clippy ke 5 `views.rs` dead-code warnings BT-DIFF wiring ke baad khatm hone chahiye (mere `#[allow(dead_code)]` items tab wire honge).
3. Cursor column aaj hamesha 1 hai (row cursor only) - col cursor chahiye to alag P-proposal, is model me scope-creep nahi.

## 7. Notes

- Is wave me `views.rs`, `views_table.rs`, `theme.rs`, `gutter.rs`, `home.rs`, `tree.rs`, `app.rs`, `screenshot.rs` ko touch nahi kiya. Tree me sibling waves ke uncommitted changes (`home.rs`, `screenshot.rs`, `views.rs`) dikhe - woh unke OWNS hain, revert/haath nahi lagaya; mera commit sirf apne OWNS paths par hoga.
- `checks/opencode-waves/STATUS.md` untouched. PUSH MANA - commit local rahega, orchestrator verify karke push karega.
