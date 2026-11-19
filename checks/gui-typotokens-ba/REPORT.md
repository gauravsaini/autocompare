# Wave BA-TYPOTOKENS — REPORT (round-1)

Scope: typography tokens only, geometry-free. `theme.rs` me TypoRole→FontId
resolution poora (pure helpers + single-source bundle + tests). `views.rs` /
`views_table.rs` / `screenshot.rs` UNTOUCHED (parallel wave) — paint ke liye
sirf EXACT wiring spec neeche, koi paint edit nahi. Colors FROZEN (koi hue nahi).

## Mapping table (single source: `TypoRole::spec()`, theme.rs)

| Role | px | FontId (exact) | Weight spelling | Numeric | Line-height | Tracking | Tabular |
|---|---|---|---|---|---|---|---|
| Heading | 18.0 | `proportional(18.0)` | strong | 500 | chrome 1.2 → 21.6px | 0.02em | no |
| Body | 13.0 | `proportional(13.0)` | default | 400 | body 1.40 → 18.2px | 0.0 | no |
| Small | 11.0 | `proportional(11.0)` | weak | 600 | chrome 1.2 → 13.2px | 0.0 | yes |
| Mono | 12.0 | `monospace(12.0)` | regular-mono | 400 | mono 1.30 → 15.6px | 0.0 | yes |
| HomeTitle | 14.0 | `proportional(14.0)` | strong | 500 | chrome 1.2 → 16.8px | 0.02em | no |
| HomeDesc | 11.0 | `proportional(11.0)` | weak | 400 | body 1.40 → 15.4px | 0.0 | no |
| HomeButton | 13.0 | `proportional(13.0)` | default | 500 | chrome 1.2 → 15.6px | 0.0 | no |

Ladder: Small 11 < Mono 12 < Body 13 < HomeTitle 14 < Heading 18 (tests lock karte hain).

## Proportional vs mono contract

- Non-diff UI (tabs, headers, cards, buttons, hints, counts, status): proportional
  (`FontFamily::Proportional`, UI sans intent `TYPO_UI_SANS_STACK`) — roles
  Heading/Body/Small/HomeTitle/HomeDesc/HomeButton.
- Diff panes only (code rows, summaries-as-code, gutter, table cells, folder rows):
  mono (`FontFamily::Monospace`, `TYPO_MONO_STACK`) — role Mono.
- `font_stack()` (string intent) unchanged; `font_family()` uska egui twin hai.
  Dono twins per-role tests me lock hain (`is_mono` ⇔ Monospace ⇔ `TYPO_MONO_STACK`).

## Naye helpers (theme.rs, sab pure/headless — koi `Ui`/`Context` nahi)

- `TypoRole::font_family()` (const) — Mono→Monospace, baaki Proportional.
- `TypoRole::font_id()` (const) — `FontId::new(size, family)`; har role exact px.
- `TypoRole::font_id_zoomed(level)` — `zoomed_size` px + role family; level 0 ==
  `font_id()`; clamp MIN..MAX; poore `ZOOM_LEVELS` band me family stable.
- `TypoRole::resolved_font_id(style)` — `text_style().resolve(style)`; runtime
  font overrides ka samman (aaj ka `views.rs:2290` precedent).
- `TypoSpec` + `TypoRole::spec()` (const) — role/size/font_id/weight/weight_value/
  line_height/line_height_px/letter_spacing_em/is_tabular ek bundle me; har field
  matching accessor ke barabar (tests per-role lock karte hain).

## Zoom behavior

`font_id_zoomed` existing `zoomed_size` table par baitha hai: ±1px per level,
family kabhi nahi badalti (mono code zoom hota hai, chrome se alag rehta hai),
extreme input edge par clamp, size `ZOOM_MIN_SIZE..=ZOOM_MAX_SIZE` me. Level 0
hamesha base `font_id()` hai.

## `views.rs` EXACT wiring spec (paint UNCHANGED is wave me)

Line numbers current `main` par verify (file 8665 lines). Har entry: aaj ka paint →
future helper (baad ki wave lagayegi, values bit-identical).
Note: parallel tablep234 wave ke uncommitted `views.rs` hunks working tree me hain
(~1979 header-grid hunk, post-~1985 lines +shift); upar numbers committed `main`
ke hain — wiring helper NAAM (contract) shift-proof hain, numbers re-verify kar lena.

**Chrome aliases (already TypoRole — koi change nahi, reference ke liye):**
- `276-277` `chrome_tab_role()` → Body; `283-284` `chrome_section_role()` → Heading;
  `290-291` `chrome_status_role()` → Small.
- `347` `chrome_section_role().rich("autocompare")` — already wired, exemplar.
- `240` `ui.text_style_height(&Monospace)` + `239` debug_assert — shared row height
  source; `TYPO_LINE_HEIGHT_SOURCE` unchanged, koi per-role override nahi.

**Home card (sizes already token-routed, future: rich/font_id):**
- `296-298` → `405-407` `.strong().size(home_title_paint_size())` ⇒ `TypoRole::HomeTitle.rich(title)` (14 strong proportional — exact twin).
- `301-303` → `409-412` conditional `.small().weak()` ⇒ `TypoRole::HomeDesc.rich(desc)` (11 weak).
- `306-308` → `420-421` `.size(home_button_paint_size())` ⇒ `TypoRole::HomeButton.rich(..)` / `.font_id()` (13 default).

**Mono diff panes (future: `TypoRole::Mono.font_id()` / `.rich()` / `.resolved_font_id()`):**
- `2290` `TextStyle::Monospace.resolve(ui.style())` ⇒ `TypoRole::Mono.resolved_font_id(ui.style())` (exact drop-in, same value).
- `2792` `span_text_format`, `2931` `find_hit_format`, `3059` `find_segment_format`
  + job builders `3076/3114/3578/3633/3691` (caller `font_id` params) ⇒ caller
  `TypoRole::Mono.font_id()` (static path) ya `resolved_font_id(style)` (style path).
- Row/cell paint: `646` diff lines, `1229` selectable lines, `1326-1328` hex addr/hex/ascii,
  `1638/1649/1660` merge pane lines, `1741` changed lines, `1983` CONTENT_LEAD,
  `2009/2019/2038/2047` table glyph/cell/body rows, `2501` CONTENT_LEAD,
  `2486/2507/2595` gutter `" | "`, `2458/2538` `▶` ⇒ `TypoRole::Mono.rich(..)` (+`.strong()` jahan aaj strong hai).
- Emphasis twins (helper + tone suffix, size/family helper se, emphasis suffix rehta hai):
  `1985` key cells `.monospace().strong()`, `2067-2069` error `.monospace().strong()`
  ⇒ `TypoRole::Mono.rich(..).strong()`; `1990-1992` rule + `4118/4120-4124` folder header
  `.monospace().weak()` ⇒ `TypoRole::Mono.rich(..).weak()`. Colors untouched.

**Small/weak chrome captions (future: `TypoRole::Small.rich()`):**
- `1194-1204` pager, `1302-1304` byte-grid note, `1593-1599` merge summary,
  `1617-1622` merge decision, `1635/1646/1657` `(empty)`, `1666` Take:,
  `1838` section title (small strong), `1612-1614` hunk title (small strong),
  `1633/1644/1655` pane headers (small strong), `1947-1957` table summary,
  `2078` error text, `2207` Show:, `2233` nav badge, `2282-2287` status,
  `4100-4110` + `4436-4446` + `4661-4672` folder/dual summaries,
  `4812` pending, `4992` Layout:, `5010-5020` dim badges
  ⇒ `TypoRole::Small.rich(..)` (+`.strong()` jahan aaj strong hai).
- Judgment calls (baad ki wave tay kare, is wave me koi change nahi):
  `348/392/634/1207/1724/...` default-size `.weak()` labels ka koi exact role twin
  nahi (Body 13 default, Small 11 weak) ⇒ `TypoRole::Body.rich(..).weak()`
  (size/family helper se, weak tone suffix); `4692-4714` dual meta
  `.monospace().small().weak()` (mono family + small size — koi role twin nahi)
  ⇒ wave faisla kare (Mono family + Small size ad-hoc rehta hai tab tak).

**Boundary (dusre domain, haath nahi lagana):**
- `3974-3981` `folder_filter_rich` + `4004` `FontId::monospace(FOLDER_BODY_SIZE=13.0`,
  tree.rs:890) — folder domain `FolderTypo` se chalta hai; Mono 12 se alag size
  jaan-boojhkar (folder chrome hai, diff code nahi). Future: `spec()` se cross-check only.
- `4318-4326` `sync_badge_text_size` / `sync_summary_text_size` — tree model sizes;
  `4327-4344` rich builders model-driven rehte hain.
- `views_table.rs` (`296/303-311/318-330/626/640/990-1011/1043-1105`, `2008` test
  `FontId::monospace(14.0)`) + `screenshot.rs` (koi typo call nahi) — sibling/parallel
  waves ke paas; yahan sirf note.

## Gates evidence

- G1 baseline: `cargo test -p autocompare-gui` → `561 passed; 0 failed`
  (`round-1/baseline-tests.out`).
- G2 owns-only: `git diff --stat` → sirf `theme.rs` (+273); KEVL-CLEAN
  (views/views_table/screenshot untouched); NO-COLOR-GEO-ADD.
- G3: `cargo test -p autocompare-gui typo` → 21 passed; full suite → `565 passed;
  0 failed` (+4 naye: font_ids / spec / zoomed / resolved).
- G4: clippy `--all-targets -- -D warnings` clean (0); `rustfmt --check theme.rs`
  clean; non-test banned grep ZERO (`unwrap(`/`expect(`/`panic!`/`todo!` nahi);
  `forbid(unsafe_code)` kept.
- G5: `tools/gui-polish/run-geo-typo.sh checks/gui-typotokens-ba/round-1` →
  `RESULT: PASS` (PNG 9627B, magic ok, distinct 256; `.size` values `['14.0']`
  unchanged — paint bit-stable).
- G6: ye REPORT + `round-1/` artifacts; conventional commit; NO push.
