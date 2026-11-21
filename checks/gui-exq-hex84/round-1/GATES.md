# GATES — exq-hex84 round-1: hex geometry (views_hex.rs ONLY)

Scope: `crates/gui/src/views_hex.rs` only. No edits to theme/app/views/main/copy/take/gutter/tree/home.
Gap: `hex_stacked_panes_fit` uses inline `61.0 * char_w` char budget; stacked
line widths must single-source from `HEX_RULER_CHARS` / `HEX_HEX_PANE_CHARS` /
`HEX_ASCII_PANE_CHARS` so ruler-vs-grid columns never drift. Colors frozen,
typography frozen (Mono/Body reuse only), tmp-only (no new deps).

## G1 baseline gui tests green BEFORE edits

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -5`
- EXPECT: `test result: ok`

## G2 stacked line char budgets single-sourced, no inline 61

- CHECK: `cargo test -p autocompare-gui views_hex 2>&1 | tail -3`
- EXPECT: `test result: ok`

## G3 fmt clean

- CHECK: `cargo fmt -p autocompare-gui -- --check 2>&1 | tail -3`
- EXPECT: ``

## G4 banned constructs zero in non-test code

- CHECK: `grep -n "unwrap()|\.expect(|panic!|todo!|unimplemented!|Color32|Visuals" crates/gui/src/views_hex.rs | grep -v ":[[:space:]]*//" | awk -F: '$1 < 3684' | wc -l`
- EXPECT: `0`

## G5 scope disjoint: only views_hex.rs + this ledger changed

- CHECK: `git status --short`
- EXPECT: `crates/gui/src/views_hex.rs`
