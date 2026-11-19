# GATES — Wave BT-CARDCTA (checks/gui-cardcta-bt)

Live feedback (`checks/gui-localrun-bs/REPORT.md` P1-4): live-home me
"Open Text / Compare" jaise card CTA labels 2 lines me wrap hote hain —
button width budget tight (120px = card aadha). Geometry first,
typography second, colors frozen.

## OWNS / KEVL

- OWNS: `crates/gui/src/home.rs` + `checks/gui-cardcta-bt/**` ONLY.
- KEVL (haath mana — read-only samajh ke liye ok): `views.rs`,
  `views_table.rs`, `theme.rs`, `gutter.rs`, `tree.rs`, `viewmodel.rs`,
  `app.rs`, `screenshot.rs`, `checks/opencode-waves/STATUS.md`
  (orchestrator-owned, kabhi mat chhoona), baaki checks dirs.
- Paint adoption (`views.rs` `paint_home` `add_sized` CTA size) paint-owner
  wave ka hai; is wave me sirf headless contract + pure helpers + tests.
  Koi paint signature change nahi, koi font size change nahi (HomeButton
  body 13 reuse), koi color change nahi.

## Design (geometry first, typography second, colors frozen)

- CTA width: `HOME_CARD_CTA_W` = `HOME_CARD_MIN_W - HOME_CARD_PAD * 2`
  (240 - 16 = 224, full inner width) — purana `HOME_CARD_BUTTON_MIN_W`
  (120, card aadha) untouched rehta hai taaki `views.rs` ke lock tests
  (`home_paint_cta_size() == (120, 28)`, area 3360) green rahen.
- Single-line budget: sabse lamba label "Open Picture Compare" (20 chars,
  body 13 ~7px advance + 24px button frame pad = 164, kind-column
  precedent) 224 me fit — `home_card_cta_single_line_ok()` har 9 card par
  check karta hai. Font size change nahi, sirf width/align/pad geometry.
- Align: card `vertical_centered` parity (centered), isliye naya align
  helper nahi — width full-inner hone se centering moot hai.
- Hit area: `home_card_cta_hit_area()` = 224x28 = 6272 (purane 3360 se
  dugna floor, paint adoption ke baad milega).
- Typography second: `home_card_cta_text` sirf "Open {title}" format karta
  hai (paint parity, koi naya size/role nahi).
- Colors: zero `Color32`/`Visuals` touch (frozen).

## CHECK / EXPECT

| # | CHECK | EXPECT |
|---|---|---|
| C1 | `cargo test -p autocompare-gui` green | full suite pass, naye CTA tests samait |
| C2 | `cargo clippy -p autocompare-gui --all-targets -- -D warnings` | zero warnings |
| C3 | `rustfmt --edition 2024 --check crates/gui/src/home.rs` | clean (sirf OWNS file) |
| C4 | Banned grep OWNS: `unwrap/expect/panic/todo/unimplemented/unreachable/unsafe` non-test me | zero hits; test module me bhi banned APIs nahi |
| C5 | Colors: `Color32::from_rgb` / `Visuals::` naye touches | zero new (diff me koi color line nahi) |
| C6 | `tools/gui-polish/run-geo-typo.sh checks/gui-cardcta-bt/round-1` | `RESULT: PASS` (max 6 rounds) |
| C7 | PNG gate: `after.png` magic + size + distinct | magic `89504e470d0a1a0a`, size >= 8192, distinct > 64 |
| C8 | `checks/opencode-waves/STATUS.md` untouched | `git status` me nahi dikhna chahiye |
| C9 | `REPORT.md` Hinglish-Roman, Devanagari MANA | geometry table before/after + exact lines + follow-up |
| C10 | Commit `feat(gui): ...` local, PUSH MANA | `git log --oneline -1` me conventional commit, push nahi |

## Result (implement + harness ke baad bharo)

| Gate | Actual | Status |
|---|---|---|
| C1 test | 629 passed, 0 failed (naya CTA test samait) | PASS |
| C2 clippy | OWNS `home.rs` zero hits; repo-wide sibling `views.rs`/`viewmodel.rs` dead_code par fail (KEVL, haath nahi lagaya; apne 2 doc-lazy fix kiye) | PASS (OWNS) / repo-wide BLOCKED-sibling |
| C3 rustfmt | `rustfmt --edition 2024 --check crates/gui/src/home.rs` clean | PASS |
| C4 banned | non-test + tests zero (sirf `forbid(unsafe_code)` attribute text) | PASS |
| C5 colors | diff me zero `Color32`/`Visuals` line | PASS |
| C6 harness | round-1 `RESULT: PASS` (1/6 rounds) | PASS |
| C7 PNG | `after.png` 9765B, magic `PNG image data 960x600 RGBA`, distinct 256 | PASS |
| C8 STATUS | `git status` me nahi hai | PASS |
| C9 report | Hinglish-Roman, geometry before/after + exact lines + follow-up | PASS |
| C10 commit no-push | `feat(gui)` local commit, push nahi | PASS |
