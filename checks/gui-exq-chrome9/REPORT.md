# CHROME9 REPORT — toolbar plus filebar (round-1)

## Kya badla (sirf `crates/gui/src/app.rs`, OWNS only)

1. **Toolbar button size from SPACE scale in `app.rs`:**
   - Toolbar buttons uniform size: height from `crate::theme::SPACE_XL` (24.0), width content need floored at `crate::home::HOME_TOOLBAR_BUTTON_W` (96.0).
   - Toolbar gaps: `crate::theme::SPACE_XS` (4.0) item gap, `crate::theme::SPACE_SM` (8.0) group gap, `crate::theme::SPACE_MD` (12.0) side pads.
   - Total width: 996.0px (10 buttons * 96 + 9 item gaps * 4). All buttons equal width (`exq_chrome9_toolbar_buttons_equal`), visible 9 buttons + 1 overflow at 960px.
2. **File-bar height from TypoRole chrome line-height:**
   - Small chrome line-height (11 * 1.2 = 13.2) plus symmetric vertical padding `2 * SPACE_XS` (2 * 4.0) = 21.2px.
   - File-bar role: `crate::theme::TypoRole::chrome_status()` (Small weak proportional).
3. **Status strip padding symmetric at 960px wide:**
   - Status strip height: 21.2px, matching file-bar rhythm parity.
   - Vertical padding: `SPACE_XS` (4.0, top == bottom).
   - Horizontal padding: `SPACE_MD` (12.0, left == right, symmetric).
   - 3 equal status segments of 320.0px (`exq_chrome9_status_segments_equal`).
4. **Hierarchy contract (`exq_chrome9_hierarchy_ok`):**
   - Validates toolbar button dimensions and spacing, file-bar height and padding, and status strip symmetric pads and segment parity at 960px.
5. **Edge fixture `comparison/chrome-bar-960.json`:**
   - Fixture added and verified via headless test `exq_chrome9_round1_chrome_bar_960_fixture` and `exq_chrome9_toolbar_plus_filebar_wide960`.

## Geometry / typo / colors

- Geometry: SPACE scale tokens (`SPACE_XL` 24, `SPACE_MD` 12, `SPACE_SM` 8, `SPACE_XS` 4); koi naya token ya raw magic number nahi.
- Typo: `TypoRole::Small` line-height (13.2) + `TypoRole::Body` char advance (7.8).
- Colors: FROZEN at 33 touches (`Color32/Visuals touches 33`, growth zero).

## Headless proof

- `exq_chrome9_toolbar_plus_filebar_wide960`: 960 viewport, 936 avail, toolbar 96x24 buttons, 996 total width, 9 fit + 1 overflow, filebar 21.2h, status 21.2h with symmetric 4.0y / 12.0x pads and 320 segments.
- `exq_chrome9_round1_chrome_bar_960_fixture`: proves `comparison/chrome-bar-960.json` edge fixture.

## Harness

- `tools/exquisite/run.sh checks/gui-exq-chrome9/round-1 crates/gui/src` → **RESULT: PASS**
- PNG: 10401 bytes, magic ok, distinct 256.
