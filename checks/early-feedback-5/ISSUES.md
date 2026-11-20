# ISSUES — early-feedback-5 gap list (TOP-10 geometry/typography, apne words)

RULE: ye list sirf observe karti hai. Fix agla node karega. Maine koi source edit nahi kiya.
Line numbers is run me grep se taze ki hain (dirty tree me thoda shift sambhav hai).
Colors FROZEN hain — koi color delta nahi.
BC side `docs/study.md` UI Observation & Parity se paraphrase, copy ZERO.
Label: HIGH = seedha dekha/code me pakka; MEDIUM = code anchor + bahar-observation
se anuman; UNVERIFIED = headless me confirm nahi ho paya.

## G1 — panes read-only, "dusri taraf bhejo" action nahi (sabse bhari, P0) [HIGH]

File: `crates/gui/src/views.rs:491-520` (session toolbar: Swap Sides + Reload/Recompare hi hai)
Expected: har diff row par ek chhota action ho jo us row ko dusri taraf bheje + Undo/Redo kaam kare.
Abhi: sirf dekho, badlo nahi. `gui.png` round-3/4 se byte-same hai — kuch badla nahi.

## G2 — antar-se-antar jump buttons + count label nahi (P0) [HIGH]

File: `crates/gui/src/views.rs:457-520` (toolbar me Swap Sides U+2194 + Reload/Recompare glyph hi hai)
Expected: toolbar me agla/pichhla antar par jaane wale do button + count label ("3/12").
Abhi: Swap Sides + Reload/Recompare hi hai.

## G3 — display filter nahi (P1) [MEDIUM]

File: `crates/gui/src/views.rs:411-433` area toolbar + `crates/gui/src/app.rs:280-292` (find-cursor state hai, filter control nahi)
Expected: "sab / antar / same / sandarbh" wala filter har session toolbar me ho.
Abhi: folder me naam-filter hai, text me antar-filter nahi.

## G4 — dhoondo/badlo (find/replace) + bookmark strip nahi (P1) [MEDIUM]

File: `crates/gui/src/app.rs:280-292` (`text_find_cursor`, `text_find: FindState` state maujood)
+ `crates/gui/src/views.rs:108` (`FIND_BAR_H = HEADER_H`, constant bhar hai)
Expected: Ctrl+F jaisa find bar + agla/pichhla + replace + bookmark gutter nishan.
Abhi: state/constant hai par poori strip UI nahi dikhti — lambi file me antar dhoondna mushkil.

## G5 — layout fixed side-by-side, over-under option nahi (P2) [MEDIUM]

File: `crates/gui/src/theme.rs:71-73` (`TYPO_WRAP_PANE_CHARS 80`, `TYPO_FULL_PANE_CHARS 200`)
+ `crates/gui/src/views.rs:95-106` (`SPLIT_HANDLE_W`, `SPLIT_HIT_W`)
Expected: side-by-side / upar-neeche switch sach me panes ko dobara bahaye.
Abhi: ek hi layout; splitter hit rect ek hi disha me.

## G6 — dayin kinare click-jump naksha nahi, patli antar-patti hai (P2) [HIGH]

File: `crates/gui/src/views.rs:111-115` (`HEX_MINIMAP_W = 2*GUTTER_W`, `HEX_MINIMAP_H 220.0`)
+ `crates/gui/src/gutter.rs:40` (`GUTTER_SEGMENT_MIN_H = SPACE_XS`)
+ `crates/gui/src/gutter.rs:46` (`GUTTER_HIT_SLOP = GUTTER_MERGE_GAP`)
Expected: poori file ka chhota naksha, click par jump.
Abhi: rangeen antar-patti hai par click-jump wala naksha nahi.

## G7 — akshar scale flat, tracking zero (typography) [HIGH]

File: `crates/gui/src/theme.rs:15-20` (heading 18 strong / body 13 / small 11 / mono 12)
+ `crates/gui/src/theme.rs:76` (`TYPO_LETTER_SPACING 0.0`)
Expected: heading/body/gutter me teen alag size-wazan step + ruler/gutter me fixed-chaudai ank.
Abhi: padanukram sirf size se hai; tracking documented 0.0 no-op hai.

## G8 — home cards text-only, grid me saans kam [MEDIUM]

File: `crates/gui/src/home.rs:163` (`HOME_GRID_COLUMNS 3`) + `home.rs:179` (`HOME_GRID_GAP`)
+ `home.rs:205` (`HOME_CARD_CTA_W`, assert `224.0`) + `home.rs:782` (`RECENT_ROW_GAP`)
Expected: har session card me pehchan-tasveer + saans leta grid-gap.
Abhi: 9 text cards bhare-bhare (note: is run me home shot nahi liya — code anchor + pichhle shots se anuman).

## G9 — folder ped indent + nishan halke, recent-row lay kamzor [MEDIUM]

File: `crates/gui/src/gutter.rs:72-103` (segment grow/merge) + `crates/gui/src/home.rs:782-860`
(`RECENT_ROW_GAP`, `recent_row_two_line_height`) + `crates/gui/src/views.rs:148` (`row_height = line_h + ROW_PAD`)
Expected: gehre ghusav + saaf shakh-rekha; `>`/`=` nishan saaf dikhen; recent rows do-line lay me.
Abhi: ped scan karna mushkil; nishan halke.

## G10 — wrap/numbers/whitespace toggles paint se jude nahi lagte [UNVERIFIED]

File: `crates/gui/src/theme.rs:71-73` (wrap/full pane chars) + `crates/gui/src/views.rs:148` (row_height)
Expected: toggle dabane par wrap chaudai, ruler chaudai aur row-break live badle.
Abhi: headless me toggle dabana sambhav nahi — code anchor se anuman, interactive confirm baki.

## N1 — naya observation (gap nahi, determinism note)

`gui.png` (9765B, sha `bde8ccf481c6…`) round-3/round-4 ke text fixture se BYTE-SAME hai —
isolated-target (`target/agent-local5`) fresh rebuild ke baad bhi ZERO drift.
Band hue gap: is run se ZERO prove (byte-same ka matlab gap-band hona nahi hai).
Naya khula gap: ZERO.
