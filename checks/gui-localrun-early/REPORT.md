# LOCALRUN early-feedback — gui-localrun-early (2026-11-20)

HEAD `5593c02` ke aas-paas tree (dirty: sibling M files ko haath nahi lagaya).
Scope: sirf rebuild + headless recapture + read-only BC observe. Source me ZERO edit.

## 1. Kya chalaya

- `cargo build -p autocompare-gui` — green (7.4s, lock-wait ke baad), exit 0.
- `tools/ac-loop/run.sh checks/ac-loop/round-lr1` — `RESULT: PASS`
  (CLI parity PASS 10/10, GUI geo-typo PASS: Grid x14, mono x49, strong x32, small x41).
- `--screenshot` dark + light: `after.png` (9765B) + `after-light.png` (9728B), dono 960x600 PNG ok.
- 9 session tabs: headless fixture scenes — text/folder/hex/merge/table/sync/picture/foldermerge/home
  (audio ka koi route nahi, woh text par girta hai — Sec-3 gap #1).
- Fixtures byte-check: re-render `cmp` clean (deterministic); `after.png == fixture-text.png`
  (default route text hai).
- Content-row photo-proof (pixel probe, rows y124-560): text me CHANGED x13 / DELETED x6 /
  INSERTED x8 bands + gutter hits x27; folder 12/0/12 + gutter 6; hex 15 changed + gutter 4;
  merge 7/4/3; table 49 changed + gutter 17; picture me teeno bands ~20.
  Sync/home me koi diff band nahi (sync = preview panel, home = cards — by design, proof me note).
- BC observe (read-only, koi click/focus nahi): process list me `BCompare` live hai;
  uski window title: `t_left.txt <--> t_right.txt - Text Compare`.
  Apna title pattern waisa hi hai (`left <--> right - Text`) — pattern class-level milta hai,
  wording apni hai (clean-room hold).

## 2. BC vs apna — apne shabdon me tulna (study.md session types se)

BC ke Nau session prakar (study.md, AX-observed): Folder Compare, Folder Merge, Folder Sync,
Text Compare, Text Merge, Hex Compare, Media Compare, Picture Compare, Table Compare.
Apne Nau: text, folder, hex, merge, table, sync, picture, folder-merge, audio (+ session runner).

- Sankhya barabar hai (9 vs 9), naam alag rakhe hain — match jaanboojh kar nahi kiya.
- BC ka "Media Compare" audio-video dono dhakta hai; apna `audio` sirf WAV/MP3 sample-check
  tak seemit hai, video ka koi daava nahi — dayra chhota hai, imaandaari se chhota rakha hai.
- BC ka Text Merge poora sampadan-ksham (editable) hai; apna merge CLI marker + GUI take-row
  tak hai — dekhna alag, badalna alag. Ye sabse badi khaai hai.
- BC me Folder Sync / Folder Merge natija seedha lagoo hota hai; apna Sync/FolderMerge
  dry-run yojana dikhata hai, lagoo karna CLI karta hai — suraksha ke liye thheek, par
  comparable product wali poornata nahi.

## 3. Geometry + typo gaps — user sahi keh raha hai, UI abhi horrible hai (no sugarcoat)

1. **Audio ka koi fixture/scene nahi** — `fixture-audio.png` sha text jaisa exact same
   (`bde8ccf481c6`): audio naam se text scene girta hai. 9 tabs ka daava, 8 ka proof.
2. **FolderMerge Merge jaisa exact same** (`2d794be62a8a`): alag route hone ke bajaye
   merge scene reuse — do tab, ek tasveer.
3. **FolderMerge/Audio kabhi active nahi dikhte** — highlight box sirf Home..Picture (0-7)
   tak defined hai; 8-9 par fallback Text box highlight hota hai. Tab hai, ujala kabhi nahi.
4. **Light mode aadha-flip hai** — title 38->248 halka hota hai, par content pane
   [22,22,22] kala hi rehta hai, status/tab patti [32,32,32] kaali. Aadhi roshni, poora dhokha.
5. **Pixel-font proof, asli font nahi** — headless tasveeren 5px block aksharon se banti hain;
   live egui font se inka rishta sirf andaza hai. Typo parity structural hai, aankhon wali nahi.
6. **Tab patti me 10 cheezen 960px me thusi hui** — Home + 9 tabs, FolderMerge (x474) aur
   Audio (x574) bina highlight, bina saans. Bheed hai, hierarchy nahi.
7. **Toolbar text-only** (`[Swap Sides] [Reload/Recompare]...`) — pichhle round se wahi haal,
   koi glyph nahi. Comparable product me icon+label ka toolbar hota hai; yahan line-bhar shabd.
8. **Panes par koi dhoondho/badlo, koi agla-pichhla antar button nahi** — study.md me ye
   P0/P1 gaps pehle se darj hain; is run me phir confirm: dekh sakte ho, kood nahi sakte.
9. **Sync scene me koi rangeen band nahi** (probe 0/0/0) — yojana-suchi ka scene diff-scene
   jaisa mehsoos nahi hota; khaali-pan aur jaankari me farq saaf nahi.

## 4. Files (sab `checks/gui-localrun-early/` me)

after.png, after-light.png, fixture-{text,folder,hex,merge,table,sync,picture,foldermerge,home,audio}.png
(sab 960x600, PNG magic ok, >= 8192B), GATES.md, REPORT.md (yeh).

## 5. Commit

Sirf `checks/gui-localrun-early/**` + `checks/ac-loop/round-lr1/**` ka docs commit,
NO PUSH. Sibling dirty files untouched.
