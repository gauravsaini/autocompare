# REPORT — early-feedback-4 local run (2026-11-20, Hinglish-Roman)

## 1. Kya chalaya
- `git pull --ff-only`: "Already up to date", branch `main` — PASS.
- `target/debug/autocompare --help` + 10 subcommands
  (`text folder hex merge table sync picture folder-merge audio session`) —
  sab exit 0, 10/10 PASS. Usage lines round-3 se akshar-akshar same. Poori table GATES.md Gate 1 me.
- `target/debug/autocompare-gui --screenshot` se 3 PNG isi dir me:
  `home.png` (14991B), `fixture-text.png` (9765B), `fixture-folder.png` (10153B).
  Teenon 960x600, magic `89504e47` ok.
- Determinism note: `fixture-text.png` round-3 se byte-same (`bde8ccf481c6`) —
  text render me ZERO drift. Par `home.png` aur `fixture-folder.png` round-3 se
  byte-drift dikhate hain (home 15007B→14991B, folder 10149B→10153B).
  Kaaran tay nahi: working tree me dusre node ke uncommitted `crates/**` badlav
  hain (19 files) aur binaries 18:35 bane hain — ye drift committed-main ka nahi
  lagta. Maine unhe na chhua, na commit kiya, na evaluate kiya.
- `CARGO_BUILD_JOBS=2` set tha; dobara build nahi chalaya (bane binaries ka smoke hi goal tha).

## 2. Bahar se dekha (Beyond Compare, copy ZERO)
- `osascript` process list me `BCompare` live mila.
- Window par click nahi, AX dump nahi, koi string/asset copy nahi — sirf bahar se naam dekha.
- Session types `docs/study.md` headings + apne `--help` se confirm: 9 sessions
  (Folder Compare, Folder Merge, Folder Sync, Text Compare, Text Merge, Hex Compare,
  Media Compare, Picture Compare, Table Compare) + 1 session loader.
  Study ke shabd nahi uthaye, apne words me likha.

## 3. TOP antar (geometry/typography, apne words)
Poori list file+line+expected ke saath ISSUES.md me hai (G1–G10 + N1 drift note). Saransh:
1. Panes ab bhi read-only — dusri taraf bhejne wala action + Undo/Redo nahi (sabse bhari).
2. Antar-se-antar jump buttons + count label ab bhi nahi.
3. "Sirf antar dikhao" wala display filter ab bhi nahi.
4. Find/replace + bookmark ab bhi nahi.
5. Layout ab bhi fixed side-by-side; over-under option nahi.
6. Minimap click-jump naksha ab bhi nahi (patli antar-patti hai).
7. Akshar scale ab bhi flat; tracking zero.
8. Home cards ab bhi text-only; grid me saans kam.
9. Folder ped nishan halke; recent-row lay kamzor.
10. Wrap/numbers toggles paint se jude nahi lagte.
11. Naya: home/folder PNG me byte-drift (kaaran anishchit — dirty tree, mera edit nahi).

## 4. Gates result
- Gate 1 CLI smoke: PASS (10/10, usage same).
- Gate 2 screenshots: PASS-valid (3/3 magic+size ok) + DRIFT NOTE (home/folder round-3 se alag, text same).
- Gate 3 BC observe: PASS (live process, zero copy).
- Gate 4 gaps: PASS (TOP-10 ISSUES.md me, file+line+expected).
- Gate 5 commit hygiene: isi dir tak seemit, no push (neeche command me verify).

## 5. Agle node ke liye
- ISSUES.md G1 sabse bhari hai (copy-to-side + Undo/Redo), phir G2–G5 (jump, filter, find, layout/minimap).
- N1 drift ko agla node note kare: saaf tree par rebuild karke home/folder dobara compare kare.
- Koi source file nahi chhui — mera commit sirf `checks/early-feedback-4/` pathspec se.
