# ISSUES — early-feedback-3 gap delta list (vs round-2, apne words)

RULE: ye list sirf observe karti hai. Fix agle node ka kaam hai. Koi edit nahi kiya.
Round-2 ke G1–G10 sab khule hain (screenshots byte-same hone se confirm) — neeche
unhe delta-nazar se dobara likha hai. Line numbers is run me grep se taze ki hain
jahaan anchor mila; jahaan nahi, round-2 position likhi hai.

## D1 — panes read-only, "dusri taraf bhejo" action nahi (round-2 G1, ab bhi sabse bhari)
File: `crates/gui/src/app.rs:3352` (mirror copy-subset logic; test `9518` ke aas-paas)
Expected: har diff row par ek chhota action ho jo us row ko dusri taraf bheje + Undo/Redo kaam kare.
Abhi: sirf dekho, badlo nahi. Koi drift nahi — jaisa round-2 me tha waisa hi hai.

## D2 — diff-se-diff jump buttons nahi (round-2 G2)
File: `crates/gui/src/views.rs:411` (session toolbar Swap/Reload area, taza grep se confirm)
Expected: toolbar me agla/pichhla antar par jaane wale do button + count label ("3/12").
Abhi: Swap Sides + Reload/Recompare hi hai.

## D3 — display filter nahi (round-2 G3)
File: `crates/gui/src/app.rs:2626` (folder filtered-counts summary; round-2 me 2620 tha, lines khiski hain)
Expected: "sab / antar / same / sandarbh" wala filter har session toolbar me ho.
Abhi: folder me name-filter hai (`folder_name_filter` 2646 ke aas-paas), text me antar-filter nahi.

## D4 — dhoondo/badlo (find/replace) system nahi (round-2 G4)
File: `crates/gui/src/app.rs:640` (round-2 position; text split/filter area ke aas-paas)
Expected: Ctrl+F jaisa find bar + agla/pichhla + replace, bookmark ke saath.
Abhi: kuch nahi — lambi file me antar dhoondna mushkil.

## D5 — layout fixed side-by-side + minimap naksha nahi (round-2 G5+G6)
File: `crates/gui/src/views.rs:417` (Reload label area) + `crates/gui/src/gutter.rs` (gutter paint module)
Expected: upar-neeche (over-under) layout option + Wrap/Numbers toggle sach me render badle;
dayin kinare poori file ka chhota naksha, click par jump.
Abhi: ek hi layout; dayin kinare rangeen antar-patti hai par click-jump wala naksha nahi.

## D6 — home / akshar / ped wale halke antar (round-2 G7+G8+G9, ek me)
File: `crates/gui/src/views.rs:446` (home cards view layer) + `views.rs:414` (icon-led label area)
Expected: har session card me pehchan-tasveer + saans leta grid; heading/body/gutter me
teen alag akshar size-wazan; ped me gehre ghusav + saaf shakh-rekha.
Abhi: 9 text cards bhare-bhare; scale ek jaisa; `> src/a.txt` / `= src/b.txt` nishan halke
(screenshot `fixture-folder.png` byte-same hone se confirm — kuch badla nahi).

## D7 — session save/load + sides-swap home se juda nahi (round-2 G10)
File: `crates/gui/src/views.rs:431` (Swap vs Reload hit area)
Expected: home "recent sessions" par click se session khule, sides-swap har tab me ek jaisa ho.
Abhi: recent list dikhti hai (screenshot `home.png` me) par click-kholna taar-juda nahi lagta.

## D8 — delta-nazar samksh (net change vs round-2)
- Band hue gap: ZERO. Khule naye gap: ZERO. Regression: ZERO (byte-same renders).
- CLI usage lines akshar-same; top flags me sab purane + kuch extra dikhe
  (`--exit-code-mode`, `--report-title`, `--save-session`, `--session`, `--title2..4`) —
  ye naye hain ya round-2 me bas list nahi hue the, is run se tay nahi hota; gap nahi mana.
