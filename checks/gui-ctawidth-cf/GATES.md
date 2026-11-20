# GATES — Wave CF-CTAWDITH (checks/gui-ctawidth-cf)

Live feedback (`checks/gui-localrun-ca/REPORT.md` section 5 P1-4, bu2
section 5 P1-4 same): headless contract `HOME_CARD_CTA_W`=224 landed
(BT-CARDCTA `c97b677`), live paint me abhi bhi 120 floor — live-home me
"Open Text / Compare" 2-line wrap. Geometry first, typography second,
colors frozen.

## OWNS / KEVL

- OWNS: `crates/gui/src/views.rs` + `checks/gui-ctawidth-cf/**` ONLY.
- KEVL (haath mana — read-only samajh ke liye ok): `home.rs`
  (BT contract `HOME_CARD_CTA_W`=224 + `home_card_cta_width()` +
  `home_card_cta_single_line_ok()` 9/9), `theme.rs` (TypoRole chrome
  reuse), `app.rs`, `main.rs`, baaki `crates/gui/src/*`, baaki checks
  dirs, `checks/opencode-waves/STATUS.md` (orchestrator-owned, kabhi
  mat chhoona).
- BT contract reuse: koi naya width/height literal nahi, koi font size
  change nahi (HomeButton body 13 reuse), koi color change nahi.

## Design (geometry first, typography second, colors frozen)

- CTA width: `home_paint_cta_size()` ab `(home_card_cta_width(), 28)`
  = (224, 28) — full inner width (240 − 2×8 pad), sabse lambe label
  "Open Picture Compare" (20 chars ≈ 164 need) single line me fit.
- Toolbar/path-row floor: `(HOME_CARD_BUTTON_MIN_W, 120×28)` par pin
  rehta hai (chhote labels — Browse/Swap/Reload — ko 224 nahi chahiye);
  `home_paint_cta_size()` se decouple, taaki CTA widen toolbar ko na khaye.
- Align: card `vertical_centered` parity untouched (full-inner width se
  centering moot hai) — spacing `TIGHT_GAP`/`HOME_GRID_GAP` untouched.
- Typography second: `TypoRole::HomeTitle` / `HomeDesc` / `HomeButton`
  reuse, koi naya size/weight nahi.
- Colors: zero `Color32`/`Visuals` touch (frozen).

## CHECK / EXPECT

| # | CHECK | EXPECT |
|---|---|---|
| C1 | `cargo test -p autocompare-gui` green | full suite pass, 2 update tests samait |
| C2 | `cargo clippy -p autocompare-gui --all-targets -- -D warnings` | zero warnings |
| C3 | `rustfmt --edition 2024 --check crates/gui/src/views.rs` | clean (sirf OWNS file) |
| C4 | Banned grep OWNS diff: `unwrap/expect/panic/todo/unimplemented/unreachable/unsafe` | zero hits |
| C5 | Colors: `Color32` / `Visuals::` naye touches | zero new (diff me koi color line nahi) |
| C6 | `tools/gui-polish/run-geo-typo.sh checks/gui-ctawidth-cf/round-1` | `RESULT: PASS` (max 6 rounds) |
| C7 | PNG gate: `after.png` magic + size + distinct | magic `89504e470d0a1a0a`, size >= 8192, distinct > 64 |
| C8 | `checks/opencode-waves/STATUS.md` untouched | `git status` me nahi dikhna chahiye |
| C9 | `REPORT.md` Hinglish-Roman, Devanagari MANA | geometry table before/after + exact lines + follow-up |
| C10 | Commit `feat(gui): ...` local, PUSH MANA | `git log --oneline -1` me conventional commit, push nahi |

## Result (implement + harness ke baad bharo)

| Gate | Actual | Status |
|---|---|---|
| C1 test | 661 passed, 0 failed | PASS |
| C2 clippy | zero warnings | PASS |
| C3 rustfmt | `views.rs` clean | PASS |
| C4 banned | zero hits | PASS |
| C5 colors | zero new | PASS |
| C6 harness | round-1 `RESULT: PASS` (1/6 rounds) | PASS |
| C7 PNG | harness report me (magic + size + distinct) | PASS |
| C8 STATUS | `git status` me nahi hai | PASS |
| C9 report | Hinglish-Roman, Devanagari zero | PASS |
| C10 commit no-push | `feat(gui)` local commit, push nahi | PASS |
