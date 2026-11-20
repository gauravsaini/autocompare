# REPORT — early-feedback-2 local run (2026-11-20, Hinglish-Roman)

## 1. Kya chalaya
- `git pull --ff-only`: "Already up to date", branch `main` — PASS.
- `target/debug/autocompare --help` + 10 subcommands (`text folder hex merge table sync
  picture folder-merge audio session`) — sab exit 0, 10/10 PASS. Poori table GATES.md Gate 1 me.
- `target/debug/autocompare-gui --screenshot` se 3 PNG isi dir me:
  `home.png` (15007B), `fixture-text.png` (9765B), `fixture-folder.png` (10149B).
  Teeno 960x600 RGBA, magic `89504e47` ok.
- Determinism note: `fixture-text.png` aur `fixture-folder.png` ke sha purane
  `gui-localrun-early` walo se byte-match (`bde8ccf481c6`, `f9c6a846ef24`) — render
  deterministic hai, ye gap nahi, achhi baat hai.
- `CARGO_BUILD_JOBS=2` set tha; dobara build nahi chalaya (pehle se bane binaries ka smoke hi goal tha).

## 2. Bahar se dekha (Beyond Compare, copy ZERO)
- `osascript` process list me `BCompare` live mila (saath me Finder/Terminal/Code/Chrome).
- Window par click nahi, AX dump nahi, koi string/asset copy nahi — sirf bahar se naam dekha.
- Session types `docs/study.md` headings + apne `--help` se confirm: 9 sessions
  (text folder hex merge table sync picture folder-merge audio) + 1 session loader.
  Study ke shabd nahi uthaye, apne words me likha.

## 3. TOP-10 antar (geometry pehle, typography baad, rang frozen)
Poori list file+line+expected ke saath ISSUES.md me hai. Saransh:
1. Panes read-only — dusri taraf bhejne wala action + Undo/Redo nahi.
2. Antar-se-antar jump buttons + count label nahi.
3. "Sirf antar dikhao" wala display filter nahi.
4. Find/replace + bookmark nahi.
5. Layout fixed side-by-side; wrap/number toggle taar-jude nahi.
6. Click-jump wala minimap naksha nahi (rangeen patti hai).
7. Home cards text-only, ghane lagte hain.
8. Akshar scale har jagah ek jaisa — heading/body/gutter me farak chahiye.
9. Folder ped me ghusav/nishan halke.
10. Recent sessions click-kholna + sides-swap ek jaisa nahi.

## 4. Gates result
- Gate 1 CLI smoke: PASS (10/10).
- Gate 2 screenshots: PASS (3/3, magic+size ok, deterministic match).
- Gate 3 BC observe: PASS (live process, zero copy).
- Gate 4 gaps: PASS (TOP-10 ISSUES.md me).
- Gate 5 commit hygiene: isi dir tak seemit, no push (neeche command me verify).

## 5. Agle node ke liye
- ISSUES.md G1–G6 sabse bhari hain (copy-to-side, jump, filter, find, layout, minimap).
- Koi source file nahi chhui — `git status` me sirf `checks/early-feedback-2/` untracked aana chahiye.
