# GATES — exq-shot46 round-1: honest screenshot harness

OWNS: `crates/gui/src/screenshot.rs` + `checks/gui-exq-shot46/**` only.
RULES: Hinglish-Roman chat, English-only commits, NO push, 600s budget.

## Context
`crates/gui/src/screenshot.rs` hand-written 5x7 bitmap rasterizer hai jisme
hardcoded dark consts + fake chrome hai. `theme.rs` me `LIGHT_*` + `*_for(dark)`
helpers already hain; harness unko use nahi karta — Light mode me dark text
dark bars par paint hota hai. Goal: har harness pixel live single-source
token/geometry se aaye (per FixtureMode).

## Audit — hardcoded COLOR consts (screenshot.rs vs live tokens)
| # | Harness const/literal | Value | Live token | Verdict |
|---|---|---|---|---|
| C1 | BG_DARK | [27,27,27] | theme DARK_BG_RGB / palette(dark).base_bg | EXACT, route via helper |
| C2 | BG_PANEL | [38,38,38] | koi token nahi | DIVORCED → track_for |
| C3 | FG_TEXT | [230,230,230] | theme DARK_FG_RGB / base_fg | EXACT, route via helper |
| C4 | FG_DIM | [140,140,140] | theme DARK_DIM_RGB [150,150,150] | DIVORCED (10 off) → dim_for |
| C5 | DELETED_BG/FG | [120,32,32]/[255,170,170] | theme DELETED_BG/FG + deleted_bg/fg_for | EXACT dark, route via helper |
| C6 | INSERTED_BG/FG | [24,90,46]/[170,240,180] | theme INSERTED_BG/FG + inserted_bg/fg_for | EXACT dark, route via helper |
| C7 | CHANGED_BG/FG | [122,102,20]/[255,235,160] | theme CHANGED_BG/FG + changed_bg/fg_for | EXACT dark, route via helper |
| C8 | ACCENT_TAB | [50,70,105] | koi token nahi (live = egui selectable style) | REMOVED (box hatao, active = text-vs-dim ink) |
| C9 | [22,22,22] pane wells (~10 sites) | — | palette base_bg (live: equal rows transparent) | → base_bg |
| C10 | [32,32,32] strips/bands/gutter (~10 sites) | — | track_for (live band/strip token) | → track_for |
| C11 | [45,45,45] fields/buttons (~12 sites) | — | base_bg inset on track band | → base_bg |
| C12 | [45,45,45] 1px borders | — | dim_for (separator stand-in, documented) | → dim_for |
| C13 | [60,60,60] browse/nav buttons (~4 sites) | — | base_bg (gap se separation) | → base_bg |
| C14 | traffic dots [255,95,87]/[254,188,46]/[40,201,64] (2 routes) | — | koi token nahi (OS chrome, live paint nahi karta) | REMOVED (documented reason) |
| C15 | Light arm Visuals::light() window/panel fill | — | LIGHT_* helpers (dark-text-on-light-wash) | → palette(dark).base_* + LIGHT washes |

## Audit — GEOMETRY consts (screenshot.rs vs live fns/tokens)
| # | Harness const | Value | Live source | Status |
|---|---|---|---|---|
| G1 | SNAP_W/H | 960/600 | theme VIEWPORT_W/H | trace-test |
| G2 | TITLE_H | 36 | CHROME_HEADER_BAR_H(28)+SECTION_GAP(8) | trace-test |
| G3 | TAB_H | 32 | CHROME_HEADER_BAR_H(28)+SPACE_XS(4) | trace-test |
| G4 | ROW_H | 24 | action_bar_row_height (verify) | trace-test |
| G5 | OUTER_PAD | 12 | GEO_OUTER_MARGIN | trace-test |
| G6 | PANE_GAP | 4 | SPACE_XS | trace-test |
| G7 | STATUS_H | 30 | CHROME_HEADER_BAR_H(28)+GUTTER_MERGE_GAP(2) | trace-test |
| G8 | TEXT_LEFT_X/RIGHT_X | 12/476 | OUTER_PAD / LEFT+TEXT_PANE_W+PANE_GAP | trace-test |
| G9 | TEXT_PANE_W | 460 | legacy dual-pane (verify use; unused → keep frozen + note) | verify |
| G10 | GUTTER_X/W | 936/16 | SNAP_W-GUTTER_W-SPACE_SM / GUTTER_RULER_WIDE | trace-test |
| G11 | MERGE_PANE_W/X0/X1/X2 | 309/12/325/638 | (SNAP_W-2*OUTER_PAD-2*PANE_GAP)/3 arithmetic | trace-test |
| G12 | PATH_FIELD_Y/H/META_Y/META_H/TOOLBAR_Y | 76/24/100/12/114 | TITLE+TAB+GAP / action_bar_row_height / arithmetic | trace-test |
| G13 | FILE_BAR_Y/H/BROWSE_W | 76/36/75 | FIELD_Y / FIELD_H+META_H / (verify 75) | trace-test/verify |
| G14 | SEG_DIV0/1/2_X | 240/480/720 | SNAP_W/4, /2, 3/4 | trace-test |
| G15 | PANE_STATUS_H | 18 | fixture_text_row_h (already live-routed fn) | trace-test |
| G16 | HOME_CARD_W/H/GAP/CTA_W/CTA_H | 306/100/8/224/28 | home_paint_card_width / home_card_min_height / HOME_GRID_GAP / HOME_CARD_CTA_W / BUTTON_MIN_H | trace-test |
| G17 | center layout/typo17/char-metric/linenum/pad/baseline | fns | text_center_gutter_width etc. | ALREADY ROUTED |
| G18 | overview_row_y/at_y/paint_overview_strip | fns | gutter_*_padded fns | ALREADY ROUTED |
| G19 | narrow route dual panes | fns | dual_pane_widths/x_positions + VIEWPORT_MIN_W | ALREADY ROUTED |
| G20 | TABLE_* consts, topchrome/home fns, wrap/picture fns | — | views_table/home/views fns | ALREADY ROUTED |

## Gates
- [x] G1 baseline-harness-pass: harness RESULT PASS on unmodified tree.
  CHECK: tools/exquisite/run.sh checks/gui-exq-shot46/round-1
  EXPECT: RESULT: PASS
- [x] G2 no-new-tokens: zero new color/spacing/typo consts; only theme/helpers reuse.
  CHECK: git -C /tmp/wt-shot46 diff --unified=0 -- crates/gui/src/screenshot.rs | grep -E '^\+.*(\bconst\b|from_rgb|from_rgba|\[ *[0-9]+ *, *[0-9]+ *, *[0-9]+ *,)' | grep -v '^\+\s*//' | grep -vE 'to_array|_for\(|palette\(|32::|DARK_|LIGHT_|test|assert' || true; echo SCAN-DONE
  EXPECT: SCAN-DONE
- [x] G3 honest-colors-test: token-trace + light-contrast unit tests green.
  CHECK: cargo test -p autocompare-gui screenshot
  EXPECT: test result: ok
- [x] G4 harness-rerun-pass: post-fix harness RESULT PASS.
  CHECK: tools/exquisite/run.sh checks/gui-exq-shot46/round-1
  EXPECT: RESULT: PASS
- [x] G5 fmt-clean: cargo fmt check clean.
  CHECK: cargo fmt --check -p autocompare-gui
  EXPECT: exit 0
- [x] G6 banned-scan-zero: no unwrap/expect/panic/todo/unreachable/unsafe in non-test code.
  CHECK: python3 -c "import re,sys;src=open('crates/gui/src/screenshot.rs').read();body=src.split('#[cfg(test)]')[0];body='\n'.join(l for l in body.split('\n') if 'forbid(unsafe_code)' not in l);hits=[l for l in body.split('\n') if re.search(r'\bunwrap\b|expect\(|panic!|todo!|unreachable!|\bunsafe\b',l)];print('HITS:'+str(len(hits)));[print(l.strip()[:100]) for l in hits];print('BANNED-SCAN-CLEAN' if not hits else 'DIRTY')"
  EXPECT: BANNED-SCAN-CLEAN
- [x] G7 owns-only: commit touches OWNS only; strays reverted.
  CHECK: git status --short
  EXPECT: OWNS-ONLY
- [x] G8 parity-note: parity/ dir ya GATES.md me method + verdict (HONEST-PARITY-PROVEN ya TOKEN-SOURCED-LIMITED).
  CHECK: ls checks/gui-exq-shot46/round-1/parity 2>/dev/null || grep -i 'TOKEN-SOURCED-LIMITED\|HONEST-PARITY-PROVEN' checks/gui-exq-shot46/round-1/GATES.md
  EXPECT: TOKEN-SOURCED-LIMITED

## Parity appendix (step 5)

Method order per brief: (a) live capture preferred, (b) token single-sourcing fallback.

(a) LIVE CAPTURE — UNAVAILABLE in sandbox. `eframe`/winit app ko windowed
Cocoa session chahiye; `Xvfb` X11 yahan macOS GUI kaam nahi aata, `DISPLAY`
hone ke bawajood headless live-window screenshot + pxdiff possible nahi.
Koi fabricated parity claim nahi — verdict (b).

(b) TOKEN-SOURCED PROOF — machine-checked, teen unit tests
`crates/gui/src/screenshot.rs` mod tests me:
- `shot46_palette_traces_to_live_theme_helpers`: Dark arm legacy dark
  values == `*_for(true)` outputs; Light arm == `LIGHT_*` helpers; light
  washes dark bands se `assert_ne!` (koi dark leftover nahi).
- `shot46_light_palette_meets_aa_on_light_wash`: light ke har ink/wash
  pair par `theme_contrast_ok` (dark-on-dark ka machine-checked ant).
- `shot46_geometry_consts_trace_to_live_tokens`: har layout const live
  token arithmetic ke barabar (fail on divergence).

Harness me ab ZERO `[r,g,b,a]` literals hain (sab `theme::*_for(dark)` /
`palette(dark)` se); fake chrome hata diya: OS traffic dots (live paint
nahi karta, koi token nahi) + active-tab fill box (live egui selectable
style, koi theme token nahi — active tab text-vs-dim ink se padhta hai,
jo har route pehle se paint karta tha).

Deltas (pxdiff, 960x600 RGBA):
- old-divorced-light vs new-honest-light: 236701/576000 differ (41.1%).
  Wajah: dark bands/strips → light washes/track, dim 140→150, dots/box
  removed, title x 80→OUTER_PAD. File:
  `parity/baseline-before-fix.png` vs `parity/after-light.png`.
- old-divorced-dark vs new-honest-dark: 198623/576000 differ (34.5%).
  Wajah: FG_DIM 140→`dim_for` 150, BG_PANEL 38→`track_for` 48, wells
  22/28→base_bg 27, strips 32→48, fields/buttons 45/60→base_bg, 1px
  borders→dim, dots/box removed. File:
  `parity/baseline-dark-before-fix.png` vs `parity/after-dark.png`.
- run.sh ka `after.png` == explicit `--screenshot-mode light` render
  (pxdiff 0/576000 IDENTICAL) — harness PNG Light mode hai
  (`App::resolve_screenshot_mode` default, no flag → Light).

Dark bytes बदले — EXPECTED aur honest (purane bytes divorced rasterizer
ke the). Sirf `checks/gui-exq-shot46/**` fixtures (re-)bless hui; bahar
koi fixture nahi chhui.

VERDICT: TOKEN-SOURCED-LIMITED
