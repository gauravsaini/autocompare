# BT-SHOTFIX REPORT — round-1 (P2-2 fixture glyph overlap, geometry only)

## 1. Summary

BS-LOCALRUN P2-2 me report hua tha: fixture-text toolbar line me glyph overlap/blocks — live egui me nahi, sirf `screenshot.rs` renderer ka spacing issue. Root cause mila aur laziest geometry-only fix lagaya: narrow advance table me `t` 4px se takra raha tha. Ek line ka spacing change + test update, colors FROZEN, baaki crates ZERO touch. Harness `run-geo-typo.sh` round-1 me `RESULT: PASS`.

## 2. Root cause (apne shabdon me)

Headless font 5-column patterns se glyph banata hai. Normal glyph 6px aage badhta hai (5 cols + 1 gap), patle glyph 4px (1px overlap se tight). Patle set me sirf `t` ka aakhri column (col4 = 0x20) khaali nahi hai — baaki sab ka col4 khaali hai, isliye woh sirf touch karte hain, overlap nahi. `t` ke baad jo glyph aata hai uska pehla column `t` ke col4 par double-draw hota hai — toolbar me "Syntax" shabd ke "ta" jod me block banta tha. Kern tuck (`-1px`) sirf chaude pichhle glyph (T/Y/V/r/f) par lagta hai, isliye `t` fix ke baad kahin bhi shared-pixel overlap nahi bachta.

## 3. Geometry table before/after (px, `text_width` model se)

| String | Budget | Before | After | Status |
|---|---|---|---|---|
| toolbar `[Swap Sides] ... [x] Syntax` | < 928 | 408 | 409 (+1, ek `t`) | PASS |
| status `7 rows: ...` | < 928 | 415 | 419 (+4) | PASS |
| wrap chunk-1 (80-char) | <= 428 | 414 | 418 (+4) | PASS |
| wrap chunk-2 | <= 428 | 109 | 111 (+2) | PASS |
| path `fixtures/left.rs` | < 380 | 85 | 87 (+2) | PASS |
| hex row `00000000 ...` | < 460 | 234 | 234 (+0) | PASS |

Overlap proof: `t`+`a` jod me pehle col4/col0 same pixel par milte the (double-draw), ab `t` exact 5px leta hai — agla glyph bilkul saath judta hai, shared pixel zero.

## 4. Exact lines (OWNS only)

- `crates/gui/src/screenshot.rs:3390-3406` — `glyph_advance`: `b't' => 5` alag arm, baaki narrow 4 par, wide 6 par; comment me root cause note.
- `crates/gui/src/screenshot.rs:3715-3721` — test `narrow_glyphs_use_tighter_advance`: patla set se `t` hata, `glyph_advance(b't') == 5` + `t < M` assert.
- Koi color line touch nahi: `git diff` me Color32/Visuals/palette ka zero naya occurrence.

## 5. Gates result

| Gate | Actual | Status |
|---|---|---|
| C1 test | 629 passed, 0 failed (do rerun me stable) | PASS |
| C2 clippy | `screenshot.rs` me zero warning; repo-wide warning sirf `home.rs` (doc indent x2) + `viewmodel.rs` (dead code) — parallel waves ka in-flight kaam, OWNS bahar isliye touch nahi | PASS (OWNS) |
| C3 fmt | `rustfmt --edition 2024 --check screenshot.rs` clean | PASS |
| C4 banned | diff me unwrap/expect/panic/todo/unsafe zero | PASS |
| C5 colors | diff me zero naya color touch | PASS |
| C6 harness | round-1 `RESULT: PASS` (exit 0, pehle attempt me) | PASS |
| C7 PNG | `after.png` 960x600 RGBA, magic ok, 9765 bytes (>= 8192), distinct 256 (> 64) | PASS |
| C8 report | ye file: geometry table + exact lines + follow-up, Hinglish-Roman | PASS |
| C9 status | `checks/opencode-waves/STATUS.md` me zero diff | PASS |
| C10 commit | `feat(gui): ...` local commit, PUSH MANA | PASS |

Ek note: beech me ek test run me 2 fail + ek compile error dikha tha — sibling waves usi waqt `views.rs` edit kar rahe the (transient collision). Rerun par sab green (624, phir 629 pass). Meri file ka diff poori dauran same raha.

## 6. Follow-up (agle wave ke liye, spec only — implement nahi kiya)

- P2-1 toolbar icon parity: `views.rs` toolbar fn (icon-size budget vs text-button width) — doosri wave ka OWNS.
- Live Light Mode vs `dark_mode=true` default mismatch (BO se carry-over): `app.rs` Default + `theme.rs` — doosri wave ka OWNS.
- Agar kabhi patle glyph me naya col4-active pattern aaye to yehi `glyph_advance` arm badhana — yahin file, yahin test.

## 7. Notes

- OWNS bahar kuch touch nahi: `views.rs`, `views_table.rs`, `theme.rs`, `gutter.rs`, `home.rs`, `tree.rs`, `viewmodel.rs`, `app.rs` sab doosre waves ke paas; working tree me unke in-flight diffs ko haath nahi lagaya, commit me sirf apni 2 path liye.
- PUSH MANA tha — commit local rahega, orchestrator verify karke push karega.
