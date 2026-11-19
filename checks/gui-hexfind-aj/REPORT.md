# Wave AJ-HEXFIND — REPORT (Gap #12 hex hissa: find box + Next/Prev + N/M count, paint)

## Summary

Hex window ke andar find row ka paint layer `views_hex.rs` me landed hai (table-find precedent `checks/gui-tablefind-ai/REPORT.md` reuse).
Naya paint `paint_hex_find_bar` + do pure helpers (`parse_hex_needle_text`, `hex_find_match_offsets`) — sab local state par (`&mut String` needle + badge `&str`, returns `(prev, next)`), App/viewer struct me koi field edit nahi.
Caller wiring (hex tab se call) KEVL callers me hai isliye is scope se BAHAR — nayi items file precedent `#[allow(dead_code)]` ke saath wiring-pending hain, caller signature same hai.

## Metrics

- Tests: 364 passed (baseline) → 373 passed, 0 failed (full `cargo test -p autocompare-gui`)
  - Note: +9 me se 3 is wave ke hain, baaki 6 parallel sibling wave (home-aj) ke — working tree me uska diff alag hai, is commit me excluded
- `hex_find` focused: 7 passed (3 purane AE/app + 1 app parse + 3 naye AJ)
- Clippy `--all-targets -- -D warnings`: 0 warnings
- `rustfmt --check` (sirf OWNS file): clean (`cargo fmt` sirf `crates/gui/src/views_hex.rs` par chalaya)
- Banned (`unwrap(`/`expect(`/`panic!`/`todo!`...) non-test code me: zero
- Harness: `RESULT: PASS`; `after.png` 9624 bytes, magic `89504e470d0a1a0a`, distinct 256
  (paint fn caller-wired nahi hai is wave me — PNG bit-same rehna expected tha)

## Highlights

- ✅ `views_hex.rs` (+182): `parse_hex_needle_text` (App needle semantics ka views_hex-local mirror — separators hatakar hex-jode decode, warna raw utf-8; views_hex app par depend nahi kar sakta), `hex_find_match_offsets` (sab overlapping hits order me, `len() == hex_find_count`), `paint_hex_find_bar` (Find: small weak + needle 120px + hint + Prev/Next small_button + badge right-end small weak + 4px pad, bar 28px), 3 tests
- ✅ Geometry/typo table precedent reuse (`28.0/120.0/4.0`), koi naya token/color nahi
- ✅ AE model verify-only (koi edit nahi — purane tests same values par pass)

## Decisions

- Paint fn caller ke locals par chalti hai taaki KEVL `app.rs`/`views.rs` me koi edit na ho; wiring orchestrator/follow-up karega
- Needle parse views_hex me dobara rakha (app fn reuse nahi ho sakta — dependency direction galat hota); semantics same, vectors app test se align (`"14 1e"` → `[0x14, 0x1e]`, `"banana"` raw)
- Count overlapping hai (ek-ek byte khisakkar) taaki badge koi hit na chhupaye — `hex_find_count` se barabar

## Files changed (OWNS only, is commit me)

- `crates/gui/src/views_hex.rs` (+182: egui import, 3 consts, 2 pure fns, 1 paint fn, 3 tests)
- `checks/gui-hexfind-aj/GATES.md`, `REPORT.md`, `round-1/`

## Note (sibling waves)

- Working tree me `crates/gui/src/app.rs` + `crates/gui/src/home.rs` ka diff sibling waves (home-aj etc.) ka hai — is commit me staged nahi, untouched chhoda. `checks/gui-home-aj/` untracked bhi unka hai.
- NO push (orchestrator pushes).
