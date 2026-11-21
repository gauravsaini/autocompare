# GATES — exq-viewmodel71 round-1: geometry token consolidation in viewmodel.rs

OWNS: `crates/gui/src/viewmodel.rs` ONLY. No new tokens, no color/typo moves, behavior unchanged.

## Gates

- [ ] G1: Baseline harness PASS before any edit
  - CHECK: `tools/exquisite/run.sh checks/gui-exq-viewmodel71/round-1`
  - EXPECT: `RESULT: PASS`
- [ ] G2: Magic geometry replaced with value-identical existing tokens only
  - CHECK: `cargo test -p autocompare-gui viewmodel_token_parity 2>&1 | tail -3`
  - EXPECT: `test result: ok`
- [ ] G3: Full gui test suite green after edit
  - CHECK: `cargo test -p autocompare-gui 2>&1 | tail -3`
  - EXPECT: `test result: ok`
- [ ] G4: cargo fmt clean (no diff)
  - CHECK: `cargo fmt -p autocompare-gui -- --check 2>&1 | tail -2`
  - EXPECT: `(empty output, exit 0)`
- [ ] G5: Banned-call scan 0 in non-test viewmodel code
  - CHECK: `python3 -c "import re,sys;src=open('crates/gui/src/viewmodel.rs').read();src=src[:src.index('#[cfg(test)]')];hits=[l for l in src.splitlines() if re.search(r'\bunwrap\(|\bexpect\(|panic!|todo!|unimplemented!',l)];print('CLEAN' if not hits else hits);sys.exit(0 if not hits else 1)"`
  - EXPECT: `CLEAN`
- [ ] G6: Harness re-run PASS after edit + kittest documented
  - CHECK: `tools/exquisite/run.sh checks/gui-exq-viewmodel71/round-1`
  - EXPECT: `RESULT: PASS`

## Considered and rejected (no value-identical token exists)

- `TEXT_PANES_MIN_H = 100.0` — no 100px token; new token banned.
- `panes_h_600 = 476.0` / `panes_h_450 = 326.0` — derived viewport-minus-chrome, no token.
- Zoom pcts (25/800/100), split fractions (0.20/0.80/0.50), `+ 1.0` floor, `* 100.0` pct, `>4` format width — not px geometry on the SPACE scale.
- `SPLIT_HANDLE_HIT_W / 2.0` divisor — arithmetic, not a width literal.

## Planned replacements (all value-identical)

- `SPLIT_HANDLE_HIT_W: f32 = 8.0` → `crate::theme::SPACE_SM` (8.0).
- `const OUTER_PAD: u32 = 12` (x2, in `dual_pane_x_positions` + `textcenter_x_positions`) → `crate::theme::GEO_OUTER_MARGIN as u32` (12.0 → 12).
- `const PANE_GAP: u32 = 4` (x2, same fns) → `crate::theme::GEO_TIGHT_GAP as u32` (4.0 → 4).
- Unit tests: token-identity asserts (`SPLIT_HANDLE_HIT_W == SPACE_SM`, x-position fns unchanged at 960/720 incl. stacked origin).
