# Report — exq-icons52 round-1: staged vector layer live-wiring audit

Verdict: **UNWIRED** (honest, by architecture — no fake wiring added).

## Wired-vs-not table

| Live site (`app.rs` call site) | Staged fn | Wired? | Reason |
|---|---|---|---|
| Toolbar row 1: `views::paint_session_action_bar` → `paint_toolbar_action_buttons` (Swap/Reload) | `paint_exq_icons50_mark` | NO | Button `Response.rect` never reaches `app.rs`; paint lives in FROZEN `views.rs`, which already paints exact-rect glyphs (`paint_toolbar_glyph`). Overlay from `app.rs` would use guessed coordinates = fake wiring + interaction risk (hit rects must stay untouched). |
| Toolbar row 2: Prev/Next jump cluster (`paint_action_nav_row`) | `paint_exq_icons50_mark` | NO | Same as above — rects owned by FROZEN `views.rs`. |
| Mode tabs: `views::paint_tabs` (10 `selectable_value`) | `paint_exq_icons50_tab_pill` | NO | No per-tab rect returned to `app.rs`; pill needs selected-tab rect + fill. Whole-row pill = wrong geometry. |
| Session strip: `views::paint_session_tab_strip` | `paint_exq_icons50_tab_pill` | NO | Same — tab rects owned by FROZEN `views.rs` (`paint_session_tab`). |
| 10-action TOPCHROME toolbar (`TOPCHROME_TOOLBAR_ACTIONS`) | `paint_exq_icons50_mark` (kinds 0–9) | N/A | Fixture-only toolbar: exists in `screenshot.rs` pixels + `home.rs` geometry, has **no live egui counterpart** (live bar = 4 buttons only). Nothing to wire to. |

## Pixel evidence

- Baseline `after.png` sha256: `ff5c187d773fc7b41831c4708dd5bd7daaf248c45780a9af5b3e609b698a2700`
- Post-change `after.png` sha256: identical (`cmp` clean, delta = 0 bytes).
- Reason: `--screenshot` (`main.rs:32-47`) calls
  `screenshot::render_fixture_png_with_mode` directly and never executes
  `App` egui paint. No `app.rs` change — wired or not — can move these
  bytes; that pixel path is FROZEN `screenshot.rs` (other leaf OWNS).

## What this round did (no dead live code, no fake paint)

- `crates/gui/src/app.rs` (tests module only): added
  `toolbar_exq_icons52_live_delegation_contract` — pins the staged
  chain (index → kind → origin → segments) finite for the 4 live
  buttons. Wiring-readiness contract, explicitly NOT wiring proof.
- Zero new `Color32`/tokens/sizes; hit rects untouched (no paint calls added).

## Handoff for a future wave

Live wiring needs either (a) `views.rs` owner returning button/tab
rects (or accepting a painter callback) at the 4 toolbar buttons +
session tabs, or (b) moving the staged mark/pipe fns into `views.rs`
OWNS. Until then any `app.rs`-side paint call is guessed-coordinate
decoration and must stay out.
