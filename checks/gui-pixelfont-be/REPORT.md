# BE-PIXELFONT — 5x7 headless harness font ceiling (P2-1 research)

Tareekh: 2026-11-20 UTC
Workspace: /Users/ektasaini/autocompare (branch main)
Scope: `checks/localrun-bb/REPORT.md` section 4 P2-1 ka research answer.
Sirf `checks/gui-pixelfont-be/**` likha gaya. `crates/**` me ZERO edit
(read-only grep + `cargo test` run allowed). `checks/opencode-waves/STATUS.md`
untouched. Colors FROZEN (koi hue/token edit nahi).

## 1. Sawal (P2-1, BB REPORT se)

Headless 5x7 font me strong/small/weight antar nahi dikhta, isliye
`after*.png` ka same rehna sahi hai — par ye line ek claim hai, proof nahi.
Is wave ka kaam: harness font ka exact budget document karna, har TypoRole ke
liye classify karna ki kya harness PNG me kabhi verify ho sakta hai aur kya
sirf live egui paint me dikhega, aur future live-egui wave ke liye exact
protocol likhna.

## 2. Harness proof (is wave ka run)

Command:

```
zsh tools/gui-polish/run-geo-typo.sh checks/gui-pixelfont-be/round-1
```

Output (`round-1/geo-typo-report.md`): `RESULT: PASS` (exit 0). Signals:

- geometry: `Grid::10`, `text_style_height x4`, 25 width calls, add_space none
- typography: `monospace x53`, `strong x23`, `small x50`, `.size values ['14.0']`
- colors untouched check: info only (colors out of scope, koi paint edit nahi)

`cargo test -p autocompare-gui`: 578 passed, 0 failed (`round-1/tests.out`).

PNG verify (magic + size + dimensions):

| file | bytes | md5 | note |
|---|---|---|---|
| round-1/after.png | 9627 | 0ae0097f | harness fresh render, BB text twin se byte-identical |
| round-1/after-table.png | 9973 | 9a7b904f | localrun-bb se byte-copy reference twin, 1px underline signal wala scene |

Sab `PNG image data, 960 x 600, 8-bit/color RGBA, non-interlaced`.
Magic `89504e470d0a1a0a` ok, sab >= 8192 B, distinct 256 (blank nahi).

## 3. Harness font ka exact budget (screenshot.rs, read-only evidence)

Renderer ek hi bitmap font use karta hai — 5 columns x 7 rows per glyph
(`screenshot.rs:2288 draw_glyph`, `screenshot.rs:2313 glyph_pattern`).
Har byte ke liye 5-column pattern fixed hai, unknown byte `?` outline me
girta hai, solid block kabhi nahi (`screenshot.rs:2412-2413`).

- Size: ek hi size. `draw_string` (`screenshot.rs:2265`) sirf `(x, y, s, color)`
  leta hai — koi size/weight/family parameter hai hi nahi. 12px/11px/13px/14px/18px
  ka antar is renderer me represent karne ka rasta nahi hai.
- Advance: default 6px (5 cols + 1 gap), single-stem/punctuation 4px, space 4px
  (`screenshot.rs:2152 glyph_advance`). Ye live egui ke proportional advance ko
  copy nahi karta, sirf fixture rows ko 460px pane me fit karne ka budget hai.
- Kerning: `kern_adjust` (`screenshot.rs:2164`) sirf -1px tuck deta hai fixed
  pairs par, kabhi push nahi. Live tracking (`TYPO_TRACKING_HEADING_EM` 0.02em)
  ka koi counterpart nahi.
- Weight: koi bold/dim path nahi. Pixel ya to poori `color` se set hota hai ya
  bilkul nahi (`screenshot.rs:2288-2309`). `.strong()`/`.weak()`/`regular-mono`
  ka antar is renderer me collapse ho jata hai.
- Family: monospace vs proportional ka antar nahi — sab glyphs ek hi 5x7 bitmap
  se aate hain.
- Line-height: har row fixed `ROW_H` 24px par paint hoti hai
  (`screenshot.rs:45 SNAP_W/H`, `ROW_H`). `TypoRole::line_height` multipliers
  (chrome 1.2 / body 1.40 / mono 1.30, `theme.rs:99-106`) ka koi effect nahi.
- Letter-spacing: paint me hamesha 0.0 (`TYPO_LETTER_SPACING`, `theme.rs:77`),
  isliye iska harness-blind hona live se match karta hai — dono jagah no-op.
- Jo DIKHTA hai: `fill_rect` geometry (spacing, bar heights, card rects,
  1px rules), `draw_string` ki presence/position, aur 1px underline strips
  (`screenshot.rs:1988-1993` me `(ux, hit_y+14, uw, 1)` fill — live
  `TABLE_FIND_UNDERLINE_W` 1.0 ka headless judvaan), 1px frame border
  (`screenshot.rs:1585`), column guides (`screenshot.rs:830`).

## 4. Kya kabhi NAHI dikhega vs kya DIKHTA hai

Kabhi nahi dikhega (harness ceiling — chahe paint kuch bhi kare):

- point-size antar: 18 / 14 / 13 / 12 / 11 sab ek hi bitmap me collapse.
- weight antar: strong (Heading/HomeTitle) vs default (Body/HomeButton) vs weak
  (Small/HomeDesc) vs regular-mono (Mono) — sab same pixels.
- numeric weight: 400 / 500 / 600 (`TYPO_WEIGHT_*`, `theme.rs:94-98`) headless
  intent hai, PNG me uska koi pixel counterpart nahi.
- family antar: monospace cells vs proportional chrome — ek hi bitmap.
- tracking: headers 0.02em vs rest 0.0 (`theme.rs:463-465`) — sub-pixel, 5x7
  grid par invisible; live me bhi `extra_letter_spacing` 0.0 hai, isliye ye
  antar live me bhi paint nahi hota (design note, gate nahi).
- line-height multipliers: 1.2 / 1.40 / 1.30 sab fixed 24px row me dab jate hain.
- tabular numerals intent (`is_tabular`, Mono + Small) — bitmap digits fixed
  width hain, intent ka proof unit test hai, pixel nahi.

Dikhta hai (harness-verifiable, paint-independent):

- spacing scale: 4 / 8 / 12 / 16 / 24 (`theme.rs:239-248`), OUTER_PAD 12,
  PANE_GAP 4, TAB_H 32, TITLE_H 36, STATUS_H 30.
- row rhythm: ROW_H 24 har fixture me (text/table/folder/home).
- 1px hierarchy: table find underline 1px (`TABLE_FIND_UNDERLINE_W` 1.0,
  `views_table.rs:669`; intra-line `INTRA_SPAN_UNDERLINE_W` 1.0, `views.rs:2836`),
  frame border 1px, column guides, key-box `rect_stroke` layout (live me
  `views.rs:2135-2146`, headless me key-bar badge + grid rows).
- card/box layout: HOME_CARD 306x100, gap 8, CTA 150x28 (`screenshot.rs:53-57`);
  key bar 28px + needle 120px find bar 28px (`views_table.rs:238/575/577`).
- text presence + wrap: two-line desc wrap + `...` collapse
  (`screenshot.rs:2211 wrap_desc_two_lines`), status/hint strings, badge labels.
- tone slot (frozen hues ke andar): FG_TEXT vs FG_DIM alag brightness se dikhte
  hain — kaun sa slot kis role ka hai ye verify hota hai, hue nahi (FROZEN).

## 5. Classification table — har TypoRole

Role contract (`theme.rs:261-278`, size `theme.rs:312-322`, weight
`theme.rs:379-386`, rich `theme.rs:531-541`). Paint call-sites `views.rs` me
78 TypoRole uses (BB count): HomeTitle/HomeDesc/HomeButton card sites
(413/416/425), Mono panes (650 aas-paas), Small captions (1200 aas-paas).

| TypoRole | size / weight | harness-verifiable | live-only |
|---|---|---|---|
| Heading (18, strong, proportional) | 18px + strong | section slot ki presence + uske upar/neeche ka spacing (SECTION_GAP, header bar 28px) | 18 vs 13/14 ka glyph-height antar; strong stem ka extra pixel weight |
| Body (13, default, proportional) | 13px + default | label/button slot ki presence + position | 13 ka exact glyph height; default vs weak tone ka live dim antar |
| Small (11, weak, proportional) | 11px + weak | status/hint/count string ki presence + row slot | 11 vs 13 ka height antar; weak dimming ka live alpha antar |
| Mono (12, regular-mono, monospace) | 12px + mono | diff row content + changed/deleted/inserted row band + gutter ruler alignment | monospace constant advance vs proportional; 12 vs 13 height antar |
| HomeTitle (14, strong, proportional) | 14px + strong | card title slot (card grid 306x100, gap 8) | 14 vs 13/18 height antar; strong weight |
| HomeDesc (11, weak, proportional) | 11px + weak | two-line wrap + `...` collapse, desc slot | 11 height + weak dimming (Small ke barabar — control pair) |
| HomeButton (13, default, proportional) | 13px + default | CTA rect 150x28 + label presence | 13 height (Body ke barabar — control pair, identical hona chahiye) |

Control pairs (live gate me equality check): Body == HomeButton (dono 13
default proportional — live me identical bbox hona chahiye), Small == HomeDesc
(dono 11 weak proportional — identical hona chahiye). Ye pairs weight/size
regression ko pakadte hain bina absolute ruler ke.

## 6. Future live-egui verification wave ke liye EXACT protocol

Ye protocol estimates nahi deta — har step me scene, screenshot aur numeric
gate fixed hai. Wave owner isi ko copy karke chalaye.

### 6a. Scenes (10, file-name routing se — `screenshot.rs:75-80` jaisi live views)

1. TEXT (default route): do-pane diff + gutter ruler + status line.
2. TABLE (`table`): grid header + 6 rows + changed-row bands.
3. TABLE-FIND (`table` + `find`): key bar (col 0 selected) + find bar needle
   `bob` + 1px underline hit + `table: key=0 | find:bob 1 hit` status.
4. FOLDER (`folder`): tree rows + indent + empty-hint slot.
5. FOLDER-EMPTY (`folder` + `empty`): filtered-empty note (P0-1 wiring ke baad
   AllSame wording sahit).
6. HOME (`home`): 3-card grid + CTA + recent rows.
7. HEX (`hex`): addr + hex + ascii columns + gutter.
8. MERGE (`merge`): 3-pane thirds + pick badges.
9. PICTURE (`picture`): frame + 1px border + zoom label.
10. WRAP (`wrap`): wrap-on + find-active text scene.

### 6b. Screenshots (live egui, har scene x 4 shots = 40 shots)

- Mode: dark + light (dono palettes — tone gates dono me pass hone chahiye).
- Scale: 1x (960x600 logical, pixel gates isi par) + 2x (glyph-height measure
  ke liye, gate value x2).
- Capture: live `views.rs` paint path se (screenshot fixture renderer se nahi —
  wahi antar is protocol ka point hai). Har shot ka scene + mode + scale file
  name me: `live-<scene>-<dark|light>-<1x|2x>.png`.
- Precondition gates (har shot par): PNG magic ok, >= 8192 B, distinct > 64,
  canvas 960x600 (1x) — harness wali hi sanity.

### 6c. Acceptance thresholds (1x scale; 2x par value x2)

Size gates (glyph bbox height, caps `H`/`E` par, har scene me 10 samples ka mean):

- Heading (18) vs Body (13): delta >= 3px.
- HomeTitle (14) vs Body (13): delta >= 1px (tight — 10-sample mean par gate).
- Small (11) vs Body (13): delta <= -1px (Small chhota).
- Mono (12) vs Body (13): delta me -2px..-1px band.
- Order gate (har live shot par): Small <= Mono < Body <= HomeButton < HomeTitle < Heading.

Weight gates (same glyph, same size — weight ka isolated proof):

- strong vs default: vertical stem ki pixel-column count >= +1
  (glyph `H`/`E` par), YA same-glyph mean darkness delta >= 15%.
- weak vs default: same-glyph mean brightness delta >= 10% (weak dimmer,
  dono modes me).
- Control equality: Body vs HomeButton bbox identical (±0px); Small vs HomeDesc
  bbox identical (±0px). Violation = regression, wave FAIL.

Family gate:

- Mono advance histogram: har glyph ka advance constant (±0px variance).
- Body advance histogram: variance > 0 (proportional — `i`/`l` vs `M`/`W`).
- Dono ek hi live TEXT shot se (pane vs tab-bar label).

Underline / 1px gates (live paint rects):

- TABLE-FIND hit underline: exactly 1px row at 1x (tolerance ±0, 2x par exactly
  2px). Color overlay_fg (harness me FG_TEXT twin).
- INTRA_SPAN underline (`views.rs:2900`): same 1px gate, WRAP scene me.
- Key-box `rect_stroke` (`views.rs:2138`): 1px stroke, corners square.
- Frame border (PICTURE scene): 1px, charon taraf continuous.

Spacing gates (rect coords, ±1px tolerance):

- add_space values 4 / 8 / 12 / 16 / 24 exact.
- ROW_H 24, TAB_H 32, TITLE_H 36, STATUS_H 30.
- HOME cards 306x100, gap 8; CTA 150x28.
- Gutter ruler 16 wide; key bar 28px; find bar 28px + needle 120px.

Non-gates (record-only, FAIL nahi karenge):

- 0.02em header tracking: 1x/2x par sub-pixel — sirf note karo.
- `weight_value` 400/500/600 numbers: headless intent hai, live pixels par
  strong/weak spelling hi gate hai.
- Hues: FROZEN — koi color gate nahi, sirf weak-vs-default relative brightness.

Pass rule: 10 scenes x (size order + control equality + spacing) sab pass +
TABLE-FIND/WRAP underline 1px pass + ek dark + ek light shot par weight gates
pass = wave PASS. Ek bhi numeric gate fail = wave FAIL, paint fix bina naye
threshold ke.

## 7. Rule follow proof

- `git status --porcelain`: sirf `?? checks/gui-pixelfont-be/` (round-1 me
  `.out`/geo-report stage honge — harness PASS proof ka hissa hain; koi
  `after*.png` ke bahar binary nahi).
- `crates/**` me zero edit (sirf `cargo build` via harness + `cargo test`
  read-only run).
- `checks/opencode-waves/STATUS.md` untouched (orchestrator-owned).
- Colors FROZEN (koi palette/Visuals edit nahi).
- Commit conventional English message, NO PUSH (orchestrator verify + push karega).
