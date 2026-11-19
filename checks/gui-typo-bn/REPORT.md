# Wave BN-TYPO — REPORT (round-1)

Scope: `crates/gui/src/theme.rs` + `checks/gui-typo-bn/**` ONLY. gutter.rs/home.rs
read-only rakhe (dono pehle se Mono/Small-complete hain — ponytail smallest diff,
bina zaroorat file nahi chhedi). views.rs/views_table.rs/tree.rs/app.rs/
screenshot.rs UNTOUCHED. Colors FROZEN, geometry NO-TOUCH. NO push.

## Kya mila (live observation ki root cause)

Central ramp me grid + hint/caption ke liye koi named alias nahi tha: header
(chrome_section 18 strong), tab (chrome_tab 13 medium-step), grid (Mono 12 fixed),
hints (Small 11) — teeno chrome poles named the, content poles nahi. Isliye folder
grid/paths/hex-cells paint me direct `.monospace()` default size par hain
(views.rs:4408,4427,4429,4437,4744,4752,4780,4806,4809,4821; sync badges
4589-4590,4602-4603; filter rich 4239 tree.rs FolderTypo se). Aur tree.rs:933
`TypoRole::Caption` precedent mangta hai jo central me exist hi nahi karta tha
(tree.rs FOLDER_SIDE_SIZE 12 vs central Small 11 — reconcile follow-up me).

## Kya kiya (theme.rs +84/-4, paint-neutral, koi naya size/color/geometry nahi)

- `TypoRole::content_grid()` -> Mono (theme.rs:383): folder grid + hex/table cells
  + paths + gutter rows ka single named target. Fixed 12 regular-monospace,
  tabular, mono line-height. Sizes `TYPO_MONO_SIZE` reuse, koi naya point size nahi.
- `TypoRole::caption()` -> Small (theme.rs:392): hint/caption style. 11 weak
  proportional, tabular counts, chrome line-height. tree.rs ka manga `Caption`
  precedent ab central me defined hai (caption-regular = Small; caption-strong
  paint-side composition, follow-up me).
- Ramp ladder doc touch (theme.rs:116-124): 4 poles naam se — header strong-large /
  tab medium-step / grid mono-fixed / hints small. Sirf alias naam naye hain.
- Naya test `content_grid_and_caption_aliases_carry_ramp` (theme.rs:1264):
  alias identity, size/family/weight/tabular/line-height locks, spec()
  delegation, ramp order hint 11 < grid 12 < tab 13 < card-title 14 < section 18.

## Mapping table (single source: TypoRole, px / family / weight / line-height)

| Slot | Role | px | Family | Weight spelling (numeric) | Line-height |
|---|---|---|---|---|---|
| Header (app title, section) | Heading (chrome_section) | 18.0 | proportional | strong (500) | chrome 1.2 -> 21.6px |
| Card title (Home) | HomeTitle | 14.0 | proportional | strong (500) | chrome 1.2 -> 16.8px |
| Tab bar (medium step) | Body (chrome_tab) | 13.0 | proportional | default (400) | body 1.40 -> 18.2px |
| Card button / recent title | HomeButton | 13.0 | proportional | default paint (500 intent) | chrome 1.2 -> 15.6px |
| Grid (folder/hex/cells/paths) | Mono (content_grid) NEW | 12.0 fixed | monospace | regular-mono (400) | mono 1.30 -> 15.6px |
| Hint/caption/counts | Small (caption) NEW | 11.0 | proportional | weak (600 counts) | chrome 1.2 -> 13.2px |
| Card desc / recent meta | HomeDesc | 11.0 | proportional | weak (400) | body 1.40 -> 15.4px |

Ladder: 11 < 12 < 13 < 14 < 18 (tests lock karte hain). Letter-spacing default
0.0 (`TYPO_LETTER_SPACING`, egui me tracking nahi — hierarchy size/weight/
line-height se). Line-height paint me measured mono row height se
(`TYPO_LINE_HEIGHT_SOURCE`).

## Gates evidence

- G1 baseline: 606 passed, 0 failed (`round-1/baseline-tests.out`).
- G2 OWNS: mera diff sirf `theme.rs` (+84/-4) + `checks/gui-typo-bn/**`.
  NO-COLOR-GEO-ADD clean (koi Color32/Visuals/add_space/width/Grid add nahi).
  NOTE: working tree me sibling ka views.rs diff maujood hai (gui-geo-bn wave:
  header/tabs geometry rework) — maine usko touch/stage nahi kiya, commit me
  nahi jayega. `checks/gui-geo-bn/` + `checks/localrun-bo/` untracked sibling
  dirs hain, mere nahi.
- G3: `cargo test -p autocompare-gui` 607 passed (606 + 1 naya), 0 failed.
- G4: `rustfmt --edition 2024 --check theme.rs` clean (exit 0, sirf OWNS file).
  ZERO-BANNED non-test code me. `forbid(unsafe_code)` kept. Clippy: theme.rs
  ZERO hits — lekin repo-wide clippy exit 101 hai, single error sibling ke
  views.rs:372 `;;` (redundant_semicolons, unki added line) se. Ye KEVL hai,
  main usko haath nahi laga sakta; orchestrator sibling wave se fix karwaye.
  Mera gate evidence: `round-1/clippy.out` me `theme.rs` ke 0 hits.
- G5 harness: `run-geo-typo.sh checks/gui-typo-bn/round-1` -> RESULT:PASS.
  PNG 9695B, magic 89504e470d0a1a0a ok, distinct 255 ok. Mera diff headless-only
  hai (additive const fns + docs + test; rich()/size()/text_style() unchanged),
  isliye paint behavior same hai.
- STATUS.md untouched (koi diff nahi).

## Lines changed

- `crates/gui/src/theme.rs`: +84/-4 (2 const aliases + ramp doc + 1 test).
- `checks/gui-typo-bn/GATES.md`: new. `REPORT.md`: ye file.
- `checks/gui-typo-bn/round-1/`: after.png, geo-typo-report.md, tests.out,
  tests-tail.out, baseline-tests.out, screenshot.out, clippy.out, fmt.out,
  banned-non-test.out, owns-status.out.
- gutter.rs/home.rs: 0 lines (already complete — gutter Mono role/size/
  line-height-source, home card + recent title/meta roles).

## Follow-up (agli paint wave ke liye EXACT wiring spec, is wave me NO paint edit)

1. views.rs folder grid cells (4408,4427,4429,4437,4744,4752,4780,4806,4809,4821):
   `RichText::new(..).monospace()` -> `TypoRole::content_grid().rich(..)`
   (size default se fixed 12 par aayega). Paths (4429,4809 selectable_label)
   same alias.
2. Sync badges (4589-4590 `sync_badge_text_size`, 4602-4603
   `sync_summary_text_size`): Mono 12 par lao ya content_grid size se derive
   karo — aaj alag sizes hain.
3. Filter bar (4238 `folder_filter_rich` + tree.rs `FolderTypo`): Path
   body-regular-mono, sides caption-regular-mono, header caption-strong —
   `content_grid()`/`caption()` (+ paint-side `.strong()`) se map karo.
4. tree.rs:931-936 reconcile: FOLDER_BODY_SIZE 13 == Body 13 ok; FOLDER_SIDE_SIZE
   12 vs central caption 11 — 11 par lao ya 12 ka central token banao (decision
   orchestrator ki); FOLDER_HEADER_SIZE 12-strong caption-strong composition se.
5. Clippy: sibling views.rs:372 `;;` fix unki wave me (ye wave block nahi karti —
   mera OWNS scope clean hai).
