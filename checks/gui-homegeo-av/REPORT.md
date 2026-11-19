# REPORT — Wave AV-HOMEGEO (checks/gui-homegeo-av)

Wave: AV-HOMEGEO. Localrun feedback (`checks/localrun-au/REPORT.md`
section 2, home.png) ka headless geometry contract polish — sirf OWNS
(`crates/gui/src/home.rs`): card hit-targets bade, title-vs-desc weight
contrast pakka, recent list columnar alignment, path picker vertical rhythm.
Geometry first, typography second, colors frozen. Paint adoption
(`views.rs` `paint_home`, `CARD_H`) paint-owner wave ka hai; is wave me
koi paint signature/color change nahi.

## Kya bana (sirf OWNS: `crates/gui/src/home.rs`)

- Card hit-target: `HOME_CARD_BUTTON_MIN_H` 24 -> 28 (`HEADER_H`/`FIND_BAR_H`
  rhythm parity — report ka dabaa 120x22 button ab standard hit row par).
  `home_card_min_height` isi se 95.6 -> 99.6 (purane 75px `CARD_H` clip se
  mehfooz). Naya `HOME_GRID_GAP` 8 (`SECTION_GAP` parity),
  `HOME_CARD_BUTTON_MIN_W` = card aadha = 120 (report label budget, derive —
  magic nahi), `home_card_button_hit_area` 3360 vs purana 2640.
- Title-vs-desc contrast: koi naya size nahi — `HomeTitle` strong 14 medium
  vs `HomeDesc` small weak 11 regular reuse; naya `home_card_hierarchy_ok`
  (size gap + strong/weak split + weight gap >= 100 + proportional chrome).
- Recent columns: title flex + kind fixed + age fixed — `RECENT_COL_KIND_W`
  96 ("FolderMerge" 11 chars body + pad budget), `RECENT_COL_AGE_W` 72
  ("just now"/"23h ago" + bullet + pad budget), `RECENT_COL_GAP` = 4
  (inner-gap reuse). `recent_row_title_width` / `recent_row_columns` pure
  width math (non-finite passthrough, tang par title 0 — clip ka jhooth nahi).
- Path rhythm: `HOME_SHOWS_PATH_PICKER = false` (launcher par path row nahi),
  `HOME_PATH_ROW_H` 28 / `HOME_PATH_ROW_GAP` 4 parity se
  `home_saved_chrome_height` 32 + `home_content_top_offset` +
  `HOME_SECTION_GAP` 8 + `home_recent_section_top` helpers.
- Purana `recent_list_geometry` test ka ek assert theek kiya: single-line
  text row (27.4px, button nahi) ka "button floor se upar" daava naye 28px
  floor se takraya — ab desc-line clip check (text kabhi clip na ho);
  hit floor ka daava sirf card CTA par hai jahan button hai.

Tests (4 naye, `home.rs` tests module me, koi banned API nahi):

- hit area 3360 > 2640, min-h 28, grid gap 8, card height > 75
- hierarchy ok + weight gap >= 100
- columns tile (title+kind+age+pad+gap = total), tang par 0, NaN passthrough
- picker hidden, saved chrome 32, section top math, NaN passthrough

## Gates proof

- `cargo test -p autocompare-gui` — 522 passed, 0 failed (4 is wave ke;
  519 se 522 ka farak parallel KEVL wave ke 3 naye tests — chhooe nahi)
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings` — zero warnings
- `cargo fmt -- --check crates/gui/src/home.rs` — clean (sirf OWNS file)
- banned grep: `unwrap()/expect()/panic!/todo!/unimplemented!/unreachable!`
  — zero hits; `unsafe` sirf `#![forbid(unsafe_code)]` attr; non-test me
  indexing nahi (`.get()` only)
- `tools/gui-polish/run-geo-typo.sh checks/gui-homegeo-av/round-1` — PASS
  (PNG 9624 bytes, magic ok, distinct 256; `.size` values ['14.0'] — koi
  naya size nahi, colors untouched)
- Note: harness ke pehle run me `views.rs` (KEVL, parallel wave ki
  in-progress edit) transient syntax break par tha — chhooa nahi, unke
  theek karne par retry me full green; evidence `round-1/tests.out`

## Files

- `crates/gui/src/home.rs` — contract polish + 4 tests (+1 assert fix)
- `checks/gui-homegeo-av/GATES.md`, `REPORT.md`, `round-1/` — is wave ke
  artifacts (`round-1`: after.png, geo-typo-report.md, screenshot.out,
  tests.out)
