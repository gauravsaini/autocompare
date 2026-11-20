# GATES — gui-localrun-early (2026-11-20)

Scope: read-only local run. Source me ZERO edit (`git status` me sirf `??` naye dirs).

## Gate 1 — build

`cargo build -p autocompare-gui` exit 0 — PASS.

## Gate 2 — screenshots valid (12 PNGs, sab 960x600, magic ok, >= 8192B)

| File | Bytes | Sha12 | Gate |
|---|---|---|---|
| after.png (dark) | 9765 | bde8ccf481c6 | PASS |
| after-light.png (light) | 9728 | 7c7f8c8eea3f | PASS (dark se alag — switch kaam karta hai) |
| fixture-text.png | 9765 | bde8ccf481c6 | PASS (after == text, default route) |
| fixture-folder.png | 10149 | f9c6a846ef24 | PASS |
| fixture-hex.png | 10004 | 81be8452fae2 | PASS |
| fixture-merge.png | 10005 | 2d794be62a8a | PASS |
| fixture-table.png | 9996 | e1ddba386f22 | PASS |
| fixture-sync.png | 10831 | e4ae6ca1956d | PASS |
| fixture-picture.png | 8358 | ab26860ef9ae | PASS |
| fixture-foldermerge.png | 10005 | 2d794be62a8a | PASS (merge jaisa same — REPORT gap #2) |
| fixture-home.png | 15007 | ac9946b7acdc | PASS |
| fixture-audio.png | 9765 | bde8ccf481c6 | PASS (text jaisa same — REPORT gap #1) |

12/12 PASS (do same-sha wale gap ke taur par darj, gate-fail nahi — render deterministic hai).

## Gate 3 — determinism + content-row proof

Re-render `cmp` clean (TEXT-DETERMINISTIC) — PASS.
Pixel probe: text/folder/hex/merge/table/picture me diff bands + gutter hits maujood — PASS.
Sync/home me bands nahi (by design — preview/cards) — noted, PASS.

## Gate 4 — harness + BC observe

`tools/ac-loop/run.sh checks/ac-loop/round-lr1` — `RESULT: PASS` — PASS.
BC read-only: `BCompare` process live, window `t_left.txt <--> t_right.txt - Text Compare` — PASS.
Own 9 sessions `--help` me maujood (text folder hex merge table sync picture folder-merge audio) — PASS.

## Gate 5 — gaps listed

REPORT.md Sec-3 me 9 geometry/typo gaps, bina sugarcoat — PASS.
