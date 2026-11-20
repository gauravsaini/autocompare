# BZ-SELWIRE REPORT — P0-2 selective filler-hatch live paint wiring

Scope: BU-LOCALRUN2 P0-2 + BX-SELFIX follow-up. Selective palette live paint me pehle se wired tha (`selective_side_palette` wrap-on/wrap-off dono paths me; `selective_span_format`/`selective_find_format` pane jobs me). Sirf filler-hatch helpers contract-only dead the (`filler_hatch_step`, `filler_hatch_count` par `#[allow(dead_code)]`). Is wave me unhe default wrap-off live paint me wire kiya (minimal call-site), dead_code hataya, harness se prove kiya.
OWNS: `crates/gui/src/views.rs` + `checks/gui-selwire-bz/`.

## Natija (1 line)

Filler hatch ab live paint me hai (wrap-off left/right filler sides par step-spaced ticks, existing `gutter_dim` stroke, koi naya rang/token nahi) — 655 tests green, clippy/fmt/banned/colors clean, harness `RESULT: PASS`, fixture PNG me highlight rect visible (accent block x597..641 measured, expected se pixel-aligned).

## Wiring table

| Helper (pehle) | Ab (call-site) | Geometry / typo reuse |
|---|---|---|
| `filler_hatch_step()` dead (`allow(dead_code)`) | `paint_filler_hatch` me step reuse, wrap-off row paint se indirect | `TIGHT_GAP` reuse, koi naya token nahi |
| `filler_hatch_count()` dead (`allow(dead_code)`) | `paint_filler_hatch` me tick-count reuse, dono `allow(dead_code)` hate | width = label `Response.rect` (highlight rect alignment) |
| (naya) `paint_filler_hatch(painter, rect, color)` private paint fn | 2 call-sites: wrap-off `left_resp`/`right_resp` par `text_side_is_filler` gate | stroke `INTRA_SPAN_UNDERLINE_W` (1px) reuse; color caller ka existing `gutter_dim`; TypoRole/mono rhythm untouched (tick height = row height ka aadha, vertically centered) |
| (naya test) `filler_side_paint_contract_transparent_base_with_hatch` | filler side = transparent base + nonzero hatch lock | `selective_side_palette` reuse, `Color32::WHITE/TRANSPARENT` existing |

## Before/after geometry

| Cheez | Pehle (before.png, pre-edit binary) | Ab (after.png, wired binary) |
|---|---|---|
| Changed row base (default text, y 172..196) | full-row CHANGED_BG bands dono panes (L 12..471, R 476..596 + 642..935) | SAME — `cmp` identical (fixture bytes `screenshot.rs`+`viewmodel` se, `views::` paint call nahi hota; BX precedent) |
| Accent block right (`accent_b` (18,27)) | CH_FG run x 597..641, y 190 (45px `std::fs::` span) | SAME — expected ax = 508 + text_width(18-char prefix) = 508 + 89 = 597, measured min 597, max 641 |
| Filler sides live paint (Deleted-right / Inserted-left) | plain transparent, koi hatch nahi (helpers dead) | hatch ticks: step 4.0px (`TIGHT_GAP`), count = floor(rect_w/4), 1px `gutter_dim` stroke, rect me vertically centered |
| `allow(dead_code)` (hatch) | 2 (lines ~3758/3765) | 0 — `grep hatch allow(dead_code)` empty (HATCH-LIVE) |
| Tests | 654 passed, 0 failed | 655 passed (1 naya wiring-contract test), 0 failed |

## Visibility proof (pixel-scan, `round-1/pixel-scan.out`)

- `after.png`: 9765B, magic `89504e470d0a1a0a`, 960x600, distinct 256.
- Changed-row band: y 173/190/195 par L 12..471 + R 476..596/642..935 CHANGED_BG runs — full-row band dono panes.
- Accent block: y 190 par CH_FG run x 597..641 (45px) — selective highlight rect fixture me VISIBLE, expected=measured (shift hypothesis refuted, BX se same).
- Cross-scene diff (`after.png` vs `after-accent.png`): rows 124..319 me 104612 px differ; highlight window (y150..220, x500..700) me 4799 px nonzero — accent scene me dono panes ke blocks trigger hote hain (`after-accent.png` 8734B).
- Same-scene `before.png` vs `after.png` identical + `before-accent.png` vs `after-accent.png` identical — views.rs wiring se fixture bytes invariant (decouple by design, screenshot.rs KEVL frozen).
- Harness `geo-typo-report.md`: `RESULT: PASS` (tests green + magic/size/distinct ok). Geometry signals: Grid x14, mono x50, `.size(14.0)` single value.

## Tests / lints (gates evidence)

- `cargo test -p autocompare-gui`: 655 passed, 0 failed (baseline pre-edit 654/0; test run ke waqt `git status --short` me sirf OWNS dirty the).
- `cargo test -p autocompare-gui filler`: 3 passed (geometry + naya wiring contract + palette filler).
- `cargo clippy --all-targets -- -D warnings`: 0 warnings. `cargo fmt --check`: exit 0 (note: seedha `rustfmt --check` edition flag ke bina pre-existing let-chain par fail karta hai — `cargo fmt` sahi gate hai).
- Non-test banned (`unwrap/expect/panic/todo`): zero. `forbid(unsafe_code)` kept. NO-COLOR-ADD (diff me `from_rgb/from_rgba/Visuals::` nahi). STATUS.md untouched.
- Harness: `run-geo-typo.sh checks/gui-selwire-bz/round-1/` exit 0, round-1 me (1 round used, max 6 me se).

## Follow-up (screenshot/paint owner waves ke liye)

1. `screenshot.rs` `render_text_fixture_png` match arms (~:301-306) ab bhi full-row CHANGED_BG band paint karte hain — selective plain-base + accent-only band adoption wahi hona baaki hai (KEVL is wave me MANA tha; BX REPORT section 68 me exact spec hai). Tab fixture default scene bhi selective dikhega.
2. Fixture Changed row left `accent_a` ab bhi khaali hai — scene data me left span bharo taaki left highlight trigger ho (accent scene me dono taraf visible hai).
3. Wrap-on path (continuations + blank sub-jobs) me hatch wiring nahi hai — ponytail minimal sirf default wrap-off path cover karta hai; wrap-owner chahe to `side_jobs_wrapped_*` sites par same `paint_filler_hatch` reuse kare (koi naya helper nahi chahiye).
