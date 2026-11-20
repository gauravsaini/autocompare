# REPORT — opencode-int integration re-verify (2026-11-20, Hinglish-Roman)

## 1. Kya chalaya (1 round, sab green — round-2 ki zaroorat nahi)
- `git pull --ff-only`: "Already up to date", branch `main` — PASS.
- `cargo run -p xtask -- layers`: `layers: ok`, exit 0 — PASS (`round-1/xtask-layers.out`).
- `target/debug/autocompare --help` + 10 subcommands sab exit 0, 10/10 PASS
  (`round-1/cli-help.out`, `round-1/cli-<cmd>.out`). Usage lines early-feedback-3 se same.
- `target/debug/autocompare-gui --screenshot round-1/smoke.png`: exit 0,
  9765B, 960x600, magic `89504e47` ok, sha12 `bde8ccf481c6` — PASS.
  Determinism: early-feedback-3 ke `fixture-text.png` se byte-same (9765B, same sha12) —
  render me ZERO drift.
- `shasum -a 256 -c dist/autocompare-0.5.0-macos-arm64.tar.gz.sha256` (repo root se):
  `OK`, exit 0 — PASS. Note: pehle `dist/` ke andar se chalaya tha to path miss hua tha
  (fail meri cwd galti thi, tarball me nahi); root se OK. Baaki 2 tarballs ke hash
  `round-1/sha256-all.out` me record (cli `165a2d2a…`, gui `ca1b2716…`).
- `CARGO_TARGET_DIR=/tmp/opencode-int-target`, `CARGO_BUILD_JOBS=2` — target bahar,
  source me ZERO edit. Bahar ke M files ko touch nahi kiya (dusre nodes ke hain).

## 2. Goal note: "9 subcommands" vs truth 10
Goal me CLI `--help` 9 subcommands likha hai, binary me 10 hain
(text folder hex merge table sync picture folder-merge audio session).
early-feedback-3 ne bhi 10 confirm kiye the. Binary sahi hai, goal ka number purana hai —
ye gap nahi, note hai. GATES.md G2 me record kiya.

## 3. TOP-10 gaps delta vs checks/early-feedback-3 (sirf observe, koi fix nahi)
Base: `checks/early-feedback-3/ISSUES.md` D1–D8. Is run me koi naya binary behavior nahi
dekha (CLI usage same, PNG byte-same), isliye:
1. D1 panes read-only (copy-to-side + Undo/Redo nahi) — AB BHI KHULA (sabse bhari).
2. D2 diff-jump buttons + count label nahi — KHULA.
3. D3 display filter (sirf-antar) nahi — KHULA.
4. D4 find/replace + bookmark nahi — KHULA.
5. D5 fixed side-by-side, minimap click-jump naksha nahi — KHULA.
6. D6 home cards text-only, akshar scale ek jaisa — KHULA.
7. D7 session save/load home se taar-juda nahi — KHULA.
8. Band hue gap: ZERO. Naya khula gap: ZERO. Regression: ZERO.
9. CLI flags superset note (early-feedback-3 D8): is run me bhi sab purane flags maujood —
   koi flag gayab nahi, drift ZERO.
10. dist: combined tarball sha256 OK; cli/gui tarballs ke alag `.sha256` file nahi hain
    (sirf combined wali hai) — ye pehle se aisa hai, naya gap nahi; record ke liye
    teeno hash `sha256-all.out` me hain.

## 4. Gates result
- G1 xtask layers: PASS. G2 CLI help 10/10: PASS (9-vs-10 note ke saath).
- G3 GUI screenshot: PASS (byte-same determinism). G4 dist sha256: PASS.
- G5 gaps delta: PASS (list upar, fix nahi). G6 hygiene: isi dir tak seemit, no push.

## 5. Agle node ke liye
- Sabse bhari D1 (copy-to-side + Undo/Redo) ab bhi khula — fix wala node uthaye.
- Mera commit sirf `checks/opencode-int/` pathspec se, push nahi.
- Round-1 me sab green tha, isliye round-2 nahi chalaya (max 6 ka koi kaam nahi tha).
