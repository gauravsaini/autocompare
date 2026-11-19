# GATES — Wave AV-HOMEGEO (checks/gui-homegeo-av)

Localrun feedback (`checks/localrun-au/REPORT.md` section 2, home.png):
card hit-targets chhote, title-vs-desc weight contrast nazar nahi aata,
recent list single column hai, Home par path picker vertical rhythm waste
karta hai. Geometry first, typography second, colors frozen.

## OWNS / KEVL

- OWNS: `crates/gui/src/home.rs` + `checks/gui-homegeo-av/**` ONLY.
- KEVL (haath mana — read-only samajh ke liye ok): `views.rs`,
  `views_table.rs`, `tree.rs`, `app.rs`, `viewmodel.rs`, `theme.rs`,
  `screenshot.rs`, `checks/localrun-au/**`, `checks/opencode-waves/STATUS.md`
  (orchestrator-owned, kabhi mat chhoona).
- Paint adoption (`views.rs` `paint_home`, `CARD_H`, path row) paint-owner
  wave ka hai; is wave me sirf headless contract + pure helpers + tests.
  Koi paint signature change nahi, koi color change nahi.

## Design (geometry first, typography second, colors frozen)

- Card hit-target: `HOME_CARD_BUTTON_MIN_H` 24 -> 28 (`HEADER_H`/`FIND_BAR_H`
  rhythm parity — paging bars/find bar wala standard hit row; 24px floor
  precedent se upar). `home_card_min_height` isi se 95.6 -> 99.6 banta hai
  (purane 75px `CARD_H` clip se mehfooz). Naya `HOME_CARD_BUTTON_MIN_W`
  (card aadha = 120, report ka dabaa 120-wide label budget) +
  `home_card_button_hit_area` (>= 3360 vs purana 120x22=2640).
  Grid gap `HOME_GRID_GAP` = 8 (`SECTION_GAP` parity, koi naya number nahi).
- Title-vs-desc contrast: koi naya size nahi — `HomeTitle` strong 14 medium
  vs `HomeDesc` small weak 11 regular reuse; naya `home_card_hierarchy_ok`
  (size gap + strong/weak split + weight gap >= 100 + proportional chrome).
- Recent columns: title flex + kind fixed + age fixed:
  `RECENT_COL_KIND_W` 96 (sabse lamba kind "FolderMerge" 11 chars body + pad),
  `RECENT_COL_AGE_W` 72 ("just now"/"23h ago" desc + bullet + pad),
  `RECENT_COL_GAP` = 4 (inner-gap reuse). `recent_row_title_width` /
  `recent_row_columns` pure width math (non-finite passthrough, narrow par 0).
- Path rhythm: `HOME_SHOWS_PATH_PICKER = false` (launcher par path row nahi;
  Compare tabs apne picker khud rakhte hain) + `HOME_PATH_ROW_H` 28 /
  `HOME_PATH_ROW_GAP` 4 parity se `home_saved_chrome_height` 32 +
  `home_content_top_offset` helper. Section rhythm `HOME_SECTION_GAP` 8
  (`SECTION_GAP` parity) + `home_recent_section_top` helper.
- Typography second: sab `TypoRole` reuse, koi naya point size nahi.
- Colors: zero `Color32`/`Visuals` touch (frozen).
- Honesty: narrow width par title 0 (clip ka jhooth nahi), unknown waqt par
  purane helpers already timestamp chhupate hain — naya code wahi rakhta hai.

## Gates (sab PASS chahiye)

1. `cargo test -p autocompare-gui` — full suite green (naye tests samait).
2. `cargo clippy -p autocompare-gui --all-targets -- -D warnings` — zero warnings.
3. `cargo fmt -- --check crates/gui/src/home.rs` — clean (sirf OWNS file;
   repo-wide check parallel waves ke karan fail ho sakta hai).
4. Banned grep OWNS non-test me: `unwrap()/expect()/panic!/todo!/
   unimplemented!/unreachable!/unsafe` + indexing `\[` — zero hits
   (`.get()` only precedent); test module me bhi banned APIs nahi.
5. `tools/gui-polish/run-geo-typo.sh checks/gui-homegeo-av/round-1` —
   `RESULT: PASS` (PNG magic ok, >= 8192B, non-blank).
6. `REPORT.md` + `round-1/` artifacts, phir `git add <sirf OWNS paths>` +
   `git commit` (conventional English). No push, no stash.

## Banned (MS rust guidelines)

- Non-test code me `unwrap/expect/panic/todo` mana; `forbid(unsafe_code)` kept.
- Repo ke bahar copy/sandbox nahi; dusre agent ki files mat chhoona.
- `git stash/push` kabhi nahi; `STATUS.md` ko haath nahi.
