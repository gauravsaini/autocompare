# ISSUES — early-feedback-4 gap list (TOP-10 geometry/typography, apne words)

RULE: ye list sirf observe karti hai. Fix agla node karega. Maine koi source edit nahi kiya.
Line numbers is run me grep se taze ki hain. Colors FROZEN hain — koi color delta nahi.
BC side `docs/study.md` UI Observation & Parity se paraphrase, copy ZERO.

## G1 — panes read-only, "dusri taraf bhejo" action nahi (sabse bhari, P0)
File: `crates/gui/src/app.rs:262-281` (mirror-copy outcome line + folder dual-pane area)
Expected: har diff row par ek chhota action ho jo us row ko dusri taraf bheje + Undo/Redo kaam kare.
Abhi: sirf dekho, badlo nahi. `fixture-text.png` round-3 se byte-same hai — kuch badla nahi.

## G2 — antar-se-antar jump buttons + count label nahi (P0)
File: `crates/gui/src/views.rs:440-474` (Swap Sides + Reload/Recompare toolbar row)
Expected: toolbar me agla/pichhla antar par jaane wale do button + count label ("3/12").
Abhi: Swap Sides + Reload/Recompare hi hai.

## G3 — display filter nahi (P1)
File: `crates/gui/src/app.rs:2625` (folder filtered-counts summary) + `app.rs:2645-2651` (folder_name_filter)
Expected: "sab / antar / same / sandarbh" wala filter har session toolbar me ho.
Abhi: folder me naam-filter hai, text me antar-filter nahi.

## G4 — dhoondo/badlo (find/replace) + bookmark strip nahi (P1)
File: `crates/gui/src/app.rs:2057-2069` (text find cursor/badge precedent area)
Expected: Ctrl+F jaisa find bar + agla/pichhla + replace + bookmark gutter nishan.
Abhi: kuch nahi — lambi file me antar dhoondna mushkil.

## G5 — layout fixed side-by-side, over-under option nahi (P2)
File: `crates/gui/src/theme.rs:71-73` (`TYPO_WRAP_PANE_CHARS 80`, `TYPO_FULL_PANE_CHARS 200`)
Expected: side-by-side / upar-neeche switch sach me panes ko dobara bahaye.
Abhi: ek hi layout.

## G6 — dayin kinare click-jump naksha nahi, patli antar-patti hai (P2)
File: `crates/gui/src/gutter.rs:40` (`GUTTER_SEGMENT_MIN_H = SPACE_XS`) + `gutter.rs:46` (`GUTTER_HIT_SLOP = GUTTER_MERGE_GAP`)
Expected: poori file ka chhota naksha, click par jump.
Abhi: rangeen antar-patti hai par click-jump wala naksha nahi.

## G7 — akshar scale flat, tracking zero (typography)
File: `crates/gui/src/theme.rs:58-76` (heading 18 / body 13 / small 11 / mono 12 / home-title 14; `TYPO_LETTER_SPACING 0.0`; weights 400/500/600)
Expected: heading/body/gutter me teen alag size-wazan step + ruler/gutter me fixed-chaudai ank.
Abhi: padanukram sirf size se hai; chrome/tab/status body/small baant-te hain.

## G8 — home cards text-only, grid me saans kam
File: `crates/gui/src/home.rs:163` (`HOME_GRID_COLUMNS 3`) + `home.rs:179` (`HOME_GRID_GAP`) + `home.rs:205` (`HOME_CARD_CTA_W`)
Expected: har session card me pehchan-tasveer + saans leta grid-gap.
Abhi: 9 text cards bhare-bhare (screenshot `home.png` dekho).

## G9 — folder ped indent + nishan halke, recent-row lay kamzor
File: `crates/gui/src/home.rs:782` (`RECENT_ROW_GAP`) + `crates/gui/src/gutter.rs:72-103` (segment grow/merge)
Expected: gehre ghusav + saaf shakh-rekha; `>`/`=` nishan saaf dikhen; recent rows do-line lay me.
Abhi: ped scan karna mushkil; nishan halke.

## G10 — wrap/numbers/whitespace toggles paint se jude nahi lagte
File: `crates/gui/src/theme.rs:71-73` (wrap/full pane chars) + `theme.rs:99-105` (line-height chrome 1.2 / body 1.40 / mono 1.30)
Expected: toggle dabane par wrap chaudai, ruler chaudai aur row-break live badle.
Abhi: toggle hone par bhi render me farq nahi dikhta.

## N1 — naya observation (gap nahi, drift note)
`home.png` (14991B, sha `36ebb7a8eccb`) aur `fixture-folder.png` (10153B, sha `c5b54d0989f8`)
round-3 se byte-drift dikhate hain; `fixture-text.png` byte-same hai. Kaaran tay nahi:
working tree me dusre node ke uncommitted `crates/**` badlav hain (19 files) aur binaries
18:35 bane hain — isliye ye drift committed-main ka nahi lagta. Maine na chhua, na commit kiya.
Band hue gap: is run se ZERO prove (drift ka matlab gap-band hona nahi hai).
