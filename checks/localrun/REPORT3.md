# Localrun GUI21 Refresh — REPORT3

Fresh early-feedback run after the hex7 / table7 / tree8 landings
(`exq-hex7 offset plus gap`, `exq-table7 sort plus align`, `exq-tree8 cols
plus select`). No code edits — build, capture, and dist refresh only.

Build: `CARGO_TARGET_DIR=/tmp/target-localrun3 cargo build -p autocompare-gui
--quiet` → exit 0. Headless capture via
`/tmp/target-localrun3/debug/autocompare-gui --screenshot
checks/localrun/after-<tab>.png` (one per tab, stderr logs empty).

## Results

| tab | file | bytes | magic | dims | distinct fills | status |
| --- | --- | --- | --- | --- | --- | --- |
| home | after-home.png | 14991 | ok | 960x600 | 13 | PASS |
| text | after-text.png | 10401 | ok | 960x600 | 18 | PASS |
| folder | after-folder.png | 10153 | ok | 960x600 | 17 | PASS |
| foldermerge | after-foldermerge.png | 10421 | ok | 960x600 | 19 | PASS |
| hex | after-hex.png | 10576 | ok | 960x600 | 15 | PASS |
| merge | after-merge.png | 10421 | ok | 960x600 | 19 | PASS |
| table | after-table.png | 10582 | ok | 960x600 | 15 | PASS |
| sync | after-sync.png | 11416 | ok | 960x600 | 13 | PASS |
| picture | after-picture.png | 8936 | ok | 960x600 | 17 | PASS |
| audio | after-audio.png | 10401 | ok | 960x600 | 18 | PASS |

Threshold: PNG magic ok, size >= 8192, non-blank. 10/10 PASS.
Byte-identical to REPORT2 values: the three landings added interactive-view
pieces (offset column, sort header, tree columns) while the headless fixture
routes are untouched — no fixture regression.

Note: `foldermerge` resolves through the shared merge route; `audio` falls
back to the default fixture (no dedicated audio route). Both stable.

## Geometry deltas vs reference capture (own words)

- Reference frame is 2880x1800; ours is fixed 960x600 (`SNAP_W x SNAP_H`).
  Same 3:2 aspect; reference carries exactly 3x linear pixels (retina
  density), so layout proportions compare 1:1 after scaling.
- Our chrome contract is unchanged: title band 36, tab band 32, row step 24,
  status band 30. Content band is therefore 502px ≈ 20 full rows per pane.
  The reference shows a denser grid with more visible rows and narrower
  inter-row rhythm — our row step reads airier, and long lists paginate
  sooner.
- Column geometry exists (offset gutter, two data panes, sort/header band in
  table) but the reference packs more columns per width with tighter gutters;
  our panes use wider fixed splits, so horizontal space runs out faster.
- Reference chrome layers (toolbar clusters, file-path bars, side panels)
  have no counterpart in fixtures: ours go title → tabs → content → status
  with nothing between, so vertical budget skews toward empty pane area.

## Typography deltas vs reference capture (own words)

- Fixture renderer paints flat rectangular fills only (13–19 distinct fills
  per shot) versus ~9350 distinct shades in the reference, which comes from
  real glyph rasterization, icons, and tonal imagery. No letterforms exist
  in our shots yet, so weight / line-height / letter-spacing cannot be
  compared at glyph level.
- Structural slots are present where type will sit: header band, per-row
  cells, status band. Row step 24 implies room for standard UI text plus
  padding, but without shaped text we cannot verify cap-height rhythm,
  tabular-figure alignment in numeric columns, or header-vs-cell hierarchy.
- Next step for typo parity is rendering real glyphs into the existing slots
  (header, cells, status) and then measuring size/weight/spacing against the
  reference's denser text bands.

## Dist

Refreshed via `scripts/release-early-pack.sh` (release build + smoke PASS):
`dist/autocompare-cli-0.5.0-macos-arm64.tar.gz` and
`dist/autocompare-gui-0.5.0-macos-arm64.tar.gz` rebuilt, `dist/VERSION`
re-pinned to current short SHA. Combined `autocompare-*.tar.gz` left as the
script left it (script does not rewrite split `.sha256` sidecars).

RESULT: PASS
