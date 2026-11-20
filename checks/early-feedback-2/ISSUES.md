# ISSUES — early-feedback-2 gap list (TOP-10 geometry/typography, apne words)

RULE: ye list sirf observe karti hai. Fix agle node ka kaam hai. Koi edit nahi kiya.

## G1 — panes read-only, "dusri taraf bhejo" action nahi
File: `crates/gui/src/app.rs:9518` (mirror test ke aas-paas copy-subset logic)
Expected: har diff row par ek chhota action ho jo us row ko dusri taraf bheje + Undo/Redo kaam kare.
Abhi: sirf dekho, badlo nahi.

## G2 — diff-se-diff jump buttons nahi
File: `crates/gui/src/views.rs:411` (session toolbar Swap/Reload area)
Expected: toolbar me agla/pichhla antar par jaane wale do button + count label ("3/12").
Abhi: Swap Sides + Reload/Recompare hi hai.

## G3 — display filter nahi (sirf antar dikhao / sandarbh sahit)
File: `crates/gui/src/app.rs:2620` (folder filtered-counts ke aas-paas)
Expected: "sab / antar / same / sandarbh" wala filter har session toolbar me ho.
Abhi: folder me name-filter hai, text me antar-filter nahi.

## G4 — dhoondo/badlo (find/replace) system nahi
File: `crates/gui/src/app.rs:640` (text split/filter area ke aas-paas)
Expected: Ctrl+F jaisa find bar + agla/pichhla + replace, bookmark ke saath.
Abhi: kuch nahi — lambi file me antar dhoondna mushkil.

## G5 — layout fixed side-by-side, wrap/number toggle taar-jude nahi
File: `crates/gui/src/views.rs:417` (Reload label area, toolbar toggles)
Expected: upar-neeche (over-under) layout option + Wrap/Numbers toggle sach me render badle.
Abhi: toolbar me chehre (glyphs) hain par layout ek hi hai.

## G6 — minimap/thumbnail patti nahi
File: `crates/gui/src/gutter.rs` (gutter paint module)
Expected: dayin kinare poori file ka chhota naksha, click par jump.
Abhi: dayin kinare rangeen antar-patti hai (screenshot me dikhi), par click-jump wala naksha nahi.

## G7 — home cards me tasveer/icon nahi, sab text cards
File: `crates/gui/src/views.rs:446` (session toolbar row; home cards isi view layer me)
Expected: har session card me ek pehchan-tasveer ho, 3x3 grid saans le.
Abhi: 9 text cards (screenshot `home.png` me dikhe) — kaam karte hain par bhare-bhare lagte hain.

## G8 — akshar/motan (typography) scale ek jaisa har jagah
File: `crates/gui/src/views.rs:414` (icon-led label area)
Expected: heading / body / gutter-ank me teen alag size-wazan ho, pankti-antar saaf dikhe.
Abhi: sab lagbhag ek hi scale — toolbar patti aur code pankti me farak kam lagta hai.

## G9 — folder pankti me ghusav (indent) aur nishan halke hain
File: `crates/gui/src/app.rs:2639` (folder filter-bar wiring ke neeche row paint)
Expected: ped (tree) ki shakh-rekha + gehre ghusav se parent-child turant samajh aaye.
Abhi: `> src/a.txt` / `= src/b.txt` nishan halke hain, ghusav kam (screenshot `fixture-folder.png`).

## G10 — session save/load + sides-swap home se juda nahi
File: `crates/gui/src/views.rs:431` (Swap vs Reload hit area)
Expected: home "recent sessions" par click se session khule, sides-swap har tab me ek jaisa ho.
Abhi: recent list dikhti hai (screenshot `home.png` me 5 entries) par click-kholna taar-juda nahi lagta.
