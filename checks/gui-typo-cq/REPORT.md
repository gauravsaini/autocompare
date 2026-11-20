# Wave CQ-TYPO — REPORT (round-1)

Scope: `crates/gui/src/theme.rs` + `checks/gui-typo-cq/**` ONLY.
`views.rs` / `views_table.rs` / `tree.rs` / `gutter.rs` / `home.rs` / `app.rs` /
`screenshot.rs` UNTOUCHED. Colors FROZEN, geometry FROZEN. NO push.

## Kya mila (BC observe-only + code read)

- BC process live tha (PID 23316) — observe-ONLY rakha, koi click/edit nahi
  kiya. `screencapture -l` is macOS par supported nahi (illegal option), isliye
  full-screen `screencapture -x` liya: foreground me Codex app tha, BC window
  visible nahi tha. Isliye hierarchy match recorded BC-dense ladder se kiya jo
  theme docs me pehle se locked hai: titles (strong-large) > body > status,
  aur gutter text diff-body mono me (gutter kabhi chrome proportional nahi).
- Code read me gap: central ramp me gutter pole ka koi named alias nahi tha.
  `gutter.rs:168` me local `gutter_text_role()` helper hai (== content_grid),
  lekin `theme.rs` me gutter ka first-class naam missing tha — BN wave ne grid
  + caption diye the, gutter pole reh gaya tha. CQ wahi gap band karti hai.

## Kya kiya (theme.rs +47/-1, paint-neutral, koi naya size/color/geometry nahi)

- `TypoRole::gutter_text()` -> Mono (theme.rs): line numbers + gutter-adjacent
  rows ka single named target. Fixed 12 regular-monospace, tabular, mono
  line-height (1.30 -> 15.6px). Size `TYPO_MONO_SIZE` reuse, koi naya point
  size nahi. Pure `const fn`, koi `Ui`/`Context` nahi.
- Ramp ladder doc touch: gutter pole ab naam se —
  `content_grid()` grid ke liye, `gutter_text()` gutter text ke liye.
- `gutter.rs` ka local helper abhi bhi unchanged kaam karta hai (value-stable,
  dono `content_grid()` == Mono hain); uska central `gutter_text()` par
  delegate karna follow-up hai — `gutter.rs` is wave me out of scope tha.
- Naya test `gutter_text_alias_matches_grid_mono`: alias identity (Mono ==
  content_grid), size/family/weight/tabular/line-height/font_id locks,
  `spec()` delegation, ladder caption 11 < gutter/grid 12.

## Mapping table (single source: TypoRole — px / family / weight / line-height / spacing)

| Slot | Role | px | Family (stack) | Weight spelling (numeric) | Line-height | Tracking | Tabular |
|---|---|---|---|---|---|---|---|
| Header (app title, section) | Heading (chrome_section) | 18.0 | proportional (system-ui sans) | strong (500) | chrome 1.2 -> 21.6px | 0.02em intent | nahi |
| Card title (Home) | HomeTitle | 14.0 | proportional (system-ui sans) | strong (500) | chrome 1.2 -> 16.8px | 0.02em intent | nahi |
| Tab bar (medium step) | Body (chrome_tab) | 13.0 | proportional (system-ui sans) | default (400) | body 1.40 -> 18.2px | 0.0 | nahi |
| Card button / recent title | HomeButton | 13.0 | proportional (system-ui sans) | default paint (500 intent) | chrome 1.2 -> 15.6px | 0.0 | nahi |
| Grid (folder/hex/cells/paths) | Mono (content_grid) | 12.0 fixed | monospace (ui-monospace) | regular-mono (400) | mono 1.30 -> 15.6px | 0.0 | HAAN |
| Gutter text (line no, adjacent rows) | Mono (gutter_text) NEW | 12.0 fixed | monospace (ui-monospace) | regular-mono (400) | mono 1.30 -> 15.6px | 0.0 | HAAN |
| Hint/caption/counts | Small (caption) | 11.0 | proportional (system-ui sans) | weak (600 counts) | chrome 1.2 -> 13.2px | 0.0 | HAAN |
| Card desc / recent meta | HomeDesc | 11.0 | proportional (system-ui sans) | weak (400) | body 1.40 -> 15.4px | 0.0 | nahi |
| Status line | Small (chrome_status) | 11.0 | proportional (system-ui sans) | weak (600) | chrome 1.2 -> 13.2px | 0.0 | HAAN |

Ladder: 11 < 12 < 13 < 14 < 18 (tests lock karte hain). Letter-spacing paint me
default 0.0 (`TYPO_LETTER_SPACING` — egui me tracking support nahi, isliye
hierarchy size + weight + line-height se aati hai; header 0.02em sirf headless
intent hai). Line-height paint me hamesha measured mono row height se aati hai
(`TYPO_LINE_HEIGHT_SOURCE` = `ui.text_style_height(Monospace)`).

## Gates evidence

- G1 baseline: 670 passed, 0 failed (pre-edit harness run; `round-1/tests.out`
  + `round-1/before.png`).
- G2 OWNS: mera diff sirf `theme.rs` (+47/-1) + `checks/gui-typo-cq/**`.
  OWNS-CLEAN, STATUS-UNTOUCHED, NO-COLOR-GEO-ADD (koi Color32/Visuals/
  add_space/width/Grid add nahi). NOTE: working tree me sibling ka untracked
  `checks/gui-localrun-cr/` dir maujood hai (parallel wave) — maine usko
  touch/stage nahi kiya, commit me nahi jayega.
- G3: `cargo test -p autocompare-gui` 671 passed (670 + 1 naya), 0 failed
  (`round-1/tests-tail.out`).
- G4: `rustfmt --edition 2024 --check theme.rs` clean (exit 0, sirf OWNS file;
  ek assert ko rustfmt ne single-line kiya, wahi +47/-1 ka hissa hai).
  ZERO-BANNED non-test code me. `forbid(unsafe_code)` kept. Clippy repo-wide
  exit 0, zero warnings (is baar working tree me koi sibling diff nahi tha).
- G5 harness: `run-geo-typo.sh checks/gui-typo-cq/round-1` -> RESULT:PASS.
  Dono PNG 960x600 RGBA, magic `89504e470d0a1a0a` ok (`file` output),
  9765 bytes, aur before/after sha256 IDENTICAL
  (`bde8ccf4...15dc` dono) — mera diff headless-only hai (additive const fn +
  docs + test; `rich()` / `size()` / `text_style()` unchanged), isliye paint
  behavior same hai. Text render me koi byte-diff nahi — expected, kyunki koi
  paint call-site change nahi hua.
- STATUS.md untouched (koi diff nahi).

## Lines changed

- `crates/gui/src/theme.rs`: +47/-1 (1 const alias + ramp doc touch + 1 test).
- `checks/gui-typo-cq/GATES.md`: new. `REPORT.md`: ye file.
- `checks/gui-typo-cq/round-1/`: before.png, after.png (byte-identical),
  geo-typo-report.md, tests.out, tests-tail.out, screenshot.out, clippy.out,
  fmt.out, banned-non-test.out, owns-status.out.
- `gutter.rs` / `home.rs` / `views*.rs` / `tree.rs`: 0 lines (read-only rakhe).

## Follow-up (EXACT wiring spec, agli wave ke liye — paint wali wave karegi)

- `gutter.rs:168` `gutter_text_role()` ka body
  `TypoRole::content_grid()` se `TypoRole::gutter_text()` karo (value same
  Mono, isliye paint same rahega — sirf naam single-source hoga).
- `gutter.rs:173` `gutter_text_size()` ka body
  `TypoRole::gutter_text().size()` karo (12.0 unchanged).
- `tree.rs` FOLDER_SIDE 12 vs central Small 11 reconcile abhi bhi open hai
  (BN wave se carry-over): folder side columns grid-mono 12 par hain jabki
  central caption Small 11 hai — kaun sa pole jeetega, ye paint/geometry
  waves decide karengi, typo wave sirf dono tokens lock rakhti hai.
- Caption-strong (folder headers) abhi bhi paint-side composition hai
  (koi naya size nahi) — future exquisite pass me decide hoga.
