# CHROME3 REPORT — tabs plus toolbar (round-1)

## Kya badla (`crates/gui/src/app.rs`, `crates/gui/src/home.rs`, views FROZEN)

1. **Header height from TypoRole chrome line-height (geometry/typo):**
   `CHROME3_HEADER_H` (21.6) single-source `TypoRole::Heading.line_height_px()`
   se derive kiya gaya (18.0 * 1.2 = 21.6), jo purane 28.0 hit-row se tight hai
   aur BC single ribbon jaisi dense vertical rhythm deta hai.
2. **Toolbar spacing via SPACE tokens (geometry):**
   Toolbar spacing `SPACE_*` tokens se compose ki gayi:
   - Item gap: `SPACE_XS` (4.0)
   - Group gap: `SPACE_SM` (8.0)
   - Side margin: `SPACE_MD` (12.0)
   - Button height: `SPACE_XL` (24.0)
   - Button uniform width: 96.0px (10 buttons total = 996.0px).
3. **Session tab strip with overflow scroll (geometry):**
   Session tab strip cells 150px (120 max + 6 close gap + 24 close edge),
   8px strip gap (`SPACE_SM`). Tang 720px window par 4 tabs fit hote hain,
   aur 8 tabs (1256px) overflow karte hain aur horizontal scroll karte hain
   (2 tabs 308px fit without scroll).
4. **Caller contracts in `app.rs`:**
   `chrome3_header_h`, `chrome3_status_h`, `chrome3_toolbar_button_w`,
   `chrome3_toolbar_w`, `chrome3_toolbar_item_gap`, `chrome3_toolbar_group_gap`,
   `chrome3_session_tab_cell_w`, `chrome3_session_strip_w`,
   `chrome3_session_strip_needs_scroll`, `chrome3_content_avail_height`.

## Geometry / typo / colors

- Geometry: SPACE tokens (`SPACE_XS`, `SPACE_SM`, `SPACE_MD`, `SPACE_XL`) reuse, naya magic number zero.
- Typo: `TypoRole::Heading` (18.0 * 1.2 = 21.6), `TypoRole::Small` (11.0 * 1.2 = 13.2 + 8 = 21.2), zero new point sizes.
- Colors: FROZEN — `Color32/Visuals` touches 33 (exact baseline, zero growth).

## Headless proof

- `home::tests::chrome3_header_tabs_toolbar_space_tokens_narrow720_edge`: pass
- `app::tests::chrome3_header_tabs_toolbar_space_tokens_narrow720`: pass
- Edge fixture: `checks/gui-exq-chrome3/round-1/comparison/chrome-narrow-720.json`

## Harness

- `tools/exquisite/run.sh checks/gui-exq-chrome3/round-1 crates/gui/src` → **RESULT: PASS**
- PNG: 10401 bytes (≥ 8192), magic ok, distinct 256.
