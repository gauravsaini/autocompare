# REPORT — early-feedback-3 local run (2026-11-20, Hinglish-Roman)

## 1. Kya chalaya
- `git pull --ff-only`: "Already up to date", branch `main` — PASS.
- `target/debug/autocompare --help` + 10 subcommands (`text folder hex merge table sync
  picture folder-merge audio session`) — sab exit 0, 10/10 PASS. Usage lines round-2
  se akshar-akshar same. Poori table GATES.md Gate 1 me.
- `target/debug/autocompare-gui --screenshot` se 3 PNG isi dir me:
  `home.png` (15007B), `fixture-text.png` (9765B), `fixture-folder.png` (10149B).
  Teeno 960x600, magic `89504e47` ok.
- Determinism note: teenon ke sha round-2 se byte-match
  (`ac9946b7acdc`, `bde8ccf481c6`, `f9c6a846ef24`) — binaries ke timestamp round-2
  commit ke baad ke hain phir bhi render me ZERO drift. Ye gap nahi, achhi baat hai.
- `CARGO_BUILD_JOBS=2` set tha; dobara build nahi chalaya (bane binaries ka smoke hi goal tha).

## 2. Bahar se dekha (Beyond Compare, copy ZERO)
- `osascript` process list me `BCompare` live mila.
- Window par click nahi, AX dump nahi, koi string/asset copy nahi — sirf bahar se naam dekha.
- Session types `docs/study.md` headings + apne `--help` se confirm: 9 sessions
  (Folder Compare, Folder Merge, Folder Sync, Text Compare, Text Merge, Hex Compare,
  Media Compare, Picture Compare, Table Compare) + 1 session loader.
  Study ke shabd nahi uthaye, apne words me likha.

## 3. Round-2 se TOP antar (delta list, apne words)
Poori list file+line+expected ke saath ISSUES.md me hai. Saransh:
1. CLI surface me koi badlav nahi — 10/10 usage same, flags superset (sab purane maujood).
2. Render byte-same — round-2 ke baad visual drift ZERO, determinism phir prove hua.
3. Panes ab bhi read-only — dusri taraf bhejne wala action + Undo/Redo nahi (sabse bhari).
4. Antar-se-antar jump buttons + count label ab bhi nahi.
5. "Sirf antar dikhao" wala display filter ab bhi nahi.
6. Find/replace + bookmark ab bhi nahi.
7. Layout ab bhi fixed side-by-side; minimap click-jump naksha ab bhi nahi.
8. Home cards ab bhi text-only; akshar scale ab bhi ek jaisa; folder ped nishan halke.
9. Naya band gap ZERO — round-2 ki koi item is run me band hoti nahi dikhi.
10. Naya khula gap ZERO — koi naya regression nahi dikha.

## 4. Gates result
- Gate 1 CLI smoke: PASS (10/10, usage same).
- Gate 2 screenshots: PASS (3/3, magic+size ok, round-2 se byte-match).
- Gate 3 BC observe: PASS (live process, zero copy).
- Gate 4 gaps: PASS (delta list ISSUES.md me).
- Gate 5 commit hygiene: isi dir tak seemit, no push (neeche command me verify).

## 5. Agle node ke liye
- ISSUES.md D1 sabse bhari hai (copy-to-side + Undo/Redo), phir D2–D5 (jump, filter, find, layout/minimap).
- Koi source file nahi chhui — mera commit sirf `checks/early-feedback-3/` pathspec se.
- Note: working tree me `crates/**` ke uncommitted badlav dusre node ke hain; unhe maine
  na chhua, na commit kiya, na evaluate kiya.
