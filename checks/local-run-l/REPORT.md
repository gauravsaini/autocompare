# Wave L LOCALBIN — early feedback report (round L-1)

Date: 2026-11-20. Scope: `checks/local-run-l/**` only. Zero source edits.
Reference render: `after-L.png` (headless `--screenshot` of text-compare fixture).
Comparison basis: outside observation of a mature dual-pane compare product
running on this machine (PID 23316). All notes below are own-words
descriptions of layout behavior; no strings, labels, or assets copied.

## 1. Release metrics

- `cargo build --release -p autocompare-gui`: wall 1m 09s
  (53.66s user, 2.89s system, 81% cpu), release profile, 15 dead-code
  warnings (unused table/hex helpers), build green.
- Binary `target/release/autocompare-gui`: 11022272 bytes (~10.5 MiB, du 11M).

## 2. PNG proof

- `checks/local-run-l/after-L.png`: 9914 bytes, magic `89504e470d0a1a0a` (PNG ok).
- Dimensions 960x600, 8-bit RGBA. Distinct decoded RGBA colors: 18
  (flat dark backdrop + few wash bands + light text, i.e. a simple but
  non-blank render). Harness byte-level distinct: 256 (>64 blank gate ok).
- `tools/gui-polish/run-geo-typo.sh checks/local-run-l/round-L-1`:
  RESULT PASS. PNG 9914 bytes (>= 8192 ok), magic ok.
  Geometry signals: `add_space` values none, `Grid::` x1, 9 width calls,
  `text_style_height` x4. Typography signals: `monospace` x33, `.strong` x5,
  `.small` x30, `.size` values only `14.0`.

## 3. Tests

- `cargo test -p autocompare-gui`: 189 passed, 0 failed, 0 ignored (~0.47s).

## 4. Gap table (own-words observation + severity + owner wave)

| # | Area | Observation (own words) | Sev | Owner |
|---|------|--------------------------|-----|-------|
| 1 | Panes | No visible center divider between the two panes; rows run edge to edge so left/right bleed together | P0 | GEO |
| 2 | Diff paint | Whole-row wash bands; changed segment inside the row gets no extra emphasis, reader must hunt the differing word | P0 | PARITY |
| 3 | Navigation | No jump-to-next/previous-change control; long files force manual scroll through unchanged stretches | P0 | PARITY |
| 4 | Gutter | Line-number strip is thin and flat; no change markers or overview ruler to see where edits sit in the file | P1 | PARITY |
| 5 | Toolbar | Path fields plus file-picker buttons plus option toggles all crowd one dense strip with uneven rhythm | P1 | GEO |
| 6 | Padding | Content touches pane edges; no side breathing room around code rows or section headers | P1 | GEO |
| 7 | Type scale | One small mono size almost everywhere; headings, code, and footer share the same visual weight, no hierarchy | P1 | TYPO |
| 8 | Weight | Changed words not bolder/heavier than context; entire row tinted instead of segment-level contrast | P1 | TYPO |
| 9 | Syntax | Code shown in single flat color; keywords/strings/comments indistinguishable | P1 | PARITY |
| 10 | Merge | No take-from-either-side action; product is view-only where comparable tools resolve edits inline | P1 | PARITY |
| 11 | Sessions | File list / multi-file session view absent; only one pair comparable at a time | P1 | PARITY |
| 12 | Tabs | Session-kind switcher is a long flat strip; active choice signaled weakly, easy to lose place | P2 | GEO |
| 13 | Status | Bottom summary is one tiny uniform line; counts and state share size/color with code text | P2 | TYPO |
| 14 | Line height | Rows sit tight; wrapped or tall content would collide, descenders feel cramped | P2 | TYPO |
| 15 | Find | No in-pane search; locating a symbol in a big diff is manual scan only | P2 | PARITY |

Why it feels far from comparable: panes lack separation (1, 6), the eye
gets no guidance to the actual change (2, 4, 8), and movement through a
large diff is on foot (3, 15) with no way to act on it (10). Typography
stays monotone (7, 13), so chrome and content compete at the same volume.

## 5. Top-5 fixes proposed for next waves

1. Segment-level change emphasis inside tinted rows (PARITY, fixes #2) — P0.
2. Real center divider + pane side padding and row insets (GEO, fixes #1/#6) — P0.
3. Next/previous-change jump with position readout in status (PARITY, fixes #3) — P0.
4. Type hierarchy pass: distinct sizes for chrome vs code, heavier weight on
   changed segments (TYPO, fixes #7/#8) — P1.
5. Gutter change markers + slim overview ruler (PARITY, fixes #4) — P1.

## 6. Artifacts

- `after-L.png` (9914 bytes, PNG ok, 960x600 RGBA, 18 distinct colors).
- `round-L-1/geo-typo-report.md` (PASS), `round-L-1/after.png`,
  `round-L-1/screenshot.out`, `round-L-1/tests.out`.
