# Wave BS-TYPO — REPORT (round-1)

Scope: `crates/gui/src/gutter.rs` + `crates/gui/src/home.rs` + `checks/gui-typo-bs/**`
ONLY. `theme.rs` untouched (BN aliases reuse hue, koi naya token nahi).
`views.rs` / `views_table.rs` / `tree.rs` / `app.rs` / `screenshot.rs` UNTOUCHED.
Colors FROZEN, geometry VALUES unchanged (literals -> token reuse, koi number change
nahi — paint-neutral adoption). NO push.

## Kya mila (live observation ki root cause)

BN ne `TypoRole::content_grid()` (-> Mono 12) + `caption()` (-> Small 11) central me
define kiye, par paint-adjacent helpers ab bhi direct roles/literals par the:
`gutter_text_role()` -> `Mono` (alias nahi), `gutter_text_size()` -> `Mono.size()`,
ruler/font calibration `TYPO_MONO_SIZE` direct, ruler/segment consts literals
(16.0/8.0/4.0/2.0), `HOME_CARD_TITLE_SIZE` literal 14.0, `HOME_CARD_BUTTON_BODY_SIZE`
= `TYPO_BODY_SIZE` direct, card/pad/gap consts literals (8.0/4.0/28.0), recent
text/meta roles -> `HomeDesc` direct (caption alias nahi). `views.rs` paint ALREADY
in helpers se khata hai (`home_card_title_size()` views.rs:318,
`home_card_button_size()` views.rs:334, `recent_row_title_role().rich()` views.rs:499,
515, `recent_row_meta_role().rich()` views.rs:501,516,
`gutter_ruler_width_for_font()` views.rs:1026 + `gutter_bar_width_for_font()`
views.rs:377), isliye gutter/home me alias adoption bina views.rs chhede paint me
flow hota hai. G-T7 (row leading) + G-C1 (tab/card padding) ka PIXEL tightening
views.rs paint literals me hai (BR owns) — is wave me sirf token single-sourcing
(value-stable); pixel spec follow-up me.

## Kya kiya (gutter.rs + home.rs, koi naya size/color/geometry-value nahi)

gutter.rs:
- `gutter_text_role()` -> `TypoRole::content_grid()` (gutter.rs:169, == Mono).
- `gutter_text_size()` -> `content_grid().size()` (gutter.rs:174, 12.0, koi literal nahi).
- `gutter_ruler_width_for_font()` + `gutter_bar_width_for_font()` calibration
  `TYPO_MONO_SIZE` direct se `content_grid().size()` par (gutter.rs:155,188 — same 12.0).
- `GUTTER_RULER_WIDE` = `SPACE_LG`, `GUTTER_RULER_NARROW` = `SPACE_SM`,
  `GUTTER_SEGMENT_MIN_H` = `SPACE_XS`, `GUTTER_HIT_SLOP` = `GUTTER_MERGE_GAP`
  (gutter.rs:26,33,41,47 — values wahi 16/8/4/2).
- Tests: alias identity + token-equality locks (gutter.rs:408,412, ruler test).

home.rs:
- `HOME_CARD_TITLE_SIZE` = `HomeTitle.size()` (home.rs:114, 14.0, literal gaya).
- `HOME_CARD_BUTTON_BODY_SIZE` = `HomeButton.size()` (home.rs:121, 13.0, TYPO direct gaya).
- `HOME_CARD_PAD`/`HOME_GRID_GAP`/`HOME_SECTION_GAP` = `GEO_SECTION_GAP` (8),
  `HOME_CARD_INNER_GAP`/`HOME_PATH_ROW_GAP` = `GEO_TIGHT_GAP` (4),
  `HOME_CARD_BUTTON_MIN_H`/`HOME_PATH_ROW_H` = `CHROME_HEADER_BAR_H` (28, G-C1 hit-row
  rhythm) — home.rs:166,168,173,178,584,586,589. Values wahi.
- `recent_row_text_role()` + `recent_row_meta_role()` -> `caption()` (home.rs:622,662,
  == Small; HomeDesc se variant alag par paint-identical: `Small.rich == HomeDesc.rich`
  dono 11 weak proportional, theme test locked). Numeric helpers (line-height 15.4px,
  weight 400) HomeDesc par rakhe taaki row numbers value-stable rahen.
- Tests: caption alias locks (home.rs:1084,1085,1198) + G-C1 token-equality locks
  (home.rs:1244-1253).

## Mapping table (single source: TypoRole — before -> after, values same)

| Slot | Pehle | Ab | px | Family | Weight spelling (numeric) | Line-height |
|---|---|---|---|---|---|---|
| Gutter text | `Mono` direct | `content_grid()` | 12.0 | monospace | regular-mono (400) | mono 1.30 -> 15.6px |
| Gutter/ruler font calibration | `TYPO_MONO_SIZE` literal | `content_grid().size()` | 12.0 | monospace | regular-mono (400) | mono 1.30 -> 15.6px |
| Ruler wide/narrow/segment/slop | literals 16/8/4/2 | SPACE_LG/SM/XS + MERGE_GAP | same | — (geometry) | — | — |
| Home card title | literal 14.0 | `HomeTitle.size()` | 14.0 | proportional | strong (500) | chrome 1.2 -> 16.8px |
| Home card button | `TYPO_BODY_SIZE` direct | `HomeButton.size()` | 13.0 | proportional | default paint (500 intent) | chrome 1.2 -> 15.6px |
| Recent row text/meta | `HomeDesc` direct | `caption()` (== Small) | 11.0 | proportional | weak (paint; Small intent 600, numbers desc 400 par stable) | body 1.40 -> 15.4px (stable) |
| Card pad/grid/section gaps | literals 8.0 | `GEO_SECTION_GAP` | 8.0 | — (geometry) | — | — |
| Card inner/path gaps | literals 4.0 | `GEO_TIGHT_GAP` | 4.0 | — (geometry) | — | — |
| Card button/path hit-row | literals 28.0 | `CHROME_HEADER_BAR_H` (G-C1) | 28.0 | — (geometry) | — | — |

Letter-spacing default 0.0 (`TYPO_LETTER_SPACING`, egui me tracking nahi — hierarchy
size/weight/line-height se). Line-height paint me measured mono row height se
(`TYPO_LINE_HEIGHT_SOURCE`). Ladder unchanged: 11 < 12 < 13 < 14 < 18.

## Gates evidence

- G1 baseline: 611 passed, 0 failed (pre-edit tail, transcript me).
- G2 OWNS: mera diff sirf `gutter.rs` + `home.rs` + `checks/gui-typo-bs/**`
  (`git diff --stat`: 46 + 81 lines; theme.rs zero diff). KEVL files (views.rs,
  views_table.rs, app.rs) working tree me sibling waves ke hain — maine touch/stage
  nahi kiye. NO-COLOR-ADD clean. STATUS.md untouched (zero diff).
- G3: `cargo test -p autocompare-gui` 616 passed (611 baseline + sibling ke 5 naye;
  mere 0 new test-fn, 4 existing tests extend hue), 0 failed. NOTE: caption() Small
  variant hai HomeDesc nahi — 2 purane asserts nayi sach par update hue
  (paint-identical, theme test `Small.rich == HomeDesc.rich` locked).
- G4: `cargo clippy -p autocompare-gui --all-targets -- -D warnings` exit 0
  (beech me sibling mid-edit states par red tha — views_table.rs brace breakage,
  KEVL, unke fix ke baad green; meri files hamesha clean thi). `rustfmt --check`
  gutter+home clean. ZERO-BANNED non-test me. `forbid(unsafe_code)` dono files me.
- G5 harness: `run-geo-typo.sh checks/gui-typo-bs/round-1` -> RESULT:PASS. PNG 9695B
  (>= 8192 ok), magic 89504e470d0a1a0a ok, distinct 255 ok. Harness signals: Grid 14,
  text_style_height x4, `.size` values ['14.0'] (sirf comments me — paint me koi
  `.size(N)` literal nahi bacha; sibling ne paint_home roles par kar diya).
- STATUS.md untouched (koi diff nahi).

## Lines changed

- `crates/gui/src/gutter.rs`: role alias (2 fns) + font calibration (2 fns) + 4 const
  token reuse + 2 tests extended.
- `crates/gui/src/home.rs`: 2 size consts role-single-source + 7 geometry consts token
  reuse + 2 role fns caption alias + 4 tests extended (2 alias locks + G-C1 locks).
- `checks/gui-typo-bs/GATES.md`: new. `REPORT.md`: ye file.
- `checks/gui-typo-bs/round-1/`: after.png, geo-typo-report.md, tests.out,
  screenshot.out, tests-tail.out, clippy.out, fmt.out, png.out, owns-status.out.
- `theme.rs`: 0 lines (BN aliases reuse hue, kuch jodna nahi tha — ponytail).

## Follow-up (views.rs BR wave ke liye EXACT wiring spec, is wave me NO paint edit)

1. views.rs:377 `gutter_bar_width_for_font(TYPO_MONO_SIZE, ...)` ->
   `gutter_bar_width_for_font(TypoRole::content_grid().size(), ...)` (same 12.0,
   single-source; import `TypoRole` already views.rs:39 me hai).
2. views.rs:1026 `gutter_ruler_width_for_font(TYPO_MONO_SIZE, ...)` -> same alias
   (same 12.0). Dono call-sites ke baad `TYPO_MONO_SIZE` import views.rs:39 se hata
   sakta hai agar aur koi use na ho.
3. Folder grid cells (views.rs:4468,4487,4489,4497,4804,4812,4842,4868,4871,4883):
   `RichText::new(..).monospace()` -> `TypoRole::content_grid().rich(..)` (size
   default se fixed 12 par lock; `.weak()` jahan hai wahan composition rakho —
   content_grid rich weak nahi deta).
4. Table/hex/merge panes (views.rs:5145,5193,5691,5694,5704,5752,5755,5778,5915,5916,
   6000 + hex/sync sites 4649,4662): same `content_grid().rich(..)` mapping.
5. Sync badge/summary (views.rs:4649-4664, `sync_badge_text_size` /
   `sync_summary_text_size`): Mono 12 par lao ya content_grid size se derive karo.
6. Filter bar (views.rs:4297 `folder_filter_rich` + tree.rs `FolderTypo`): Path
   body-regular-mono, sides caption-regular-mono, header caption-strong —
   `content_grid()`/`caption()` (+ paint-side `.strong()`) se map karo (tree.rs
   FOLDER_SIDE_SIZE 12 vs central caption 11 reconcile decision orchestrator ki).
7. G-T7 pixel tightening (BC rows zyada hawaadar): row leading views.rs paint
   `add_space`/item_spacing literals me hai — `TypoRole::Mono.line_height_px()`
   (15.6px) ya Body (18.2px) beat par lao; is wave me sirf headless single-sourcing
   hui, pixel change BR wave me.
8. G-C1 pixel tightening (tab strip active-pill + spacing): app.rs tab strip + views.rs
   paint spacing — `GEO_SECTION_GAP`/`GEO_TIGHT_GAP`/`CHROME_HEADER_BAR_H` beat par;
   is wave me home-side consts single-source hue, paint BR/app wave me.
