# Wave AE-HEXFIND — GATES (Gap #12 hex half: hex find model + count label)

OWNS: `crates/gui/src/views_hex.rs` + `checks/gui-hexfind-ae/**` ONLY.
KEVL `views.rs` / `app.rs` / `tree.rs` / `viewmodel.rs` / `views_table.rs` / `gutter.rs` / `theme.rs` / `screenshot.rs` mat chhoona
(read-only, grep-proof only).
Colors FROZEN (koi hue/token change nahi — ye wave pure model hai, paint agla wave me).
Geometry first, typography second (model me koi geo/typo token nahi — sirf state + pure helpers).
NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo.

Scope: Gap #12 ka hex half. Byte-slice `hex_find_next` / `hex_find_prev`
(offset, wrap) Wave L me landed hai — verify-only, usme edit sirf tab jab test red ho.
Is wave ka delta naya position/count layer hai: grid `(row, col)` hit
(`HEX_BYTES_PER_ROW` reuse, `row = offset / 16`, `col = offset % 16`),
overlapping `hex_find_count`, aur N/M `hex_find_count_label`
(model-side judvaan `views::text_find_badge_label` / `viewmodel::find_count_label` ka —
paint-side KEVL hai, chhoona nahi). `views.rs` paint wiring is scope se BAHAR hai (agla wave).

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `317 passed; 0 failed` (recorded 2026-11-20, `main@26c4a01`)
- EVIDENCE: `round-1/baseline-tests.out` (pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/views_hex.rs|checks/gui-hexfind-ae/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "views.rs|app.rs|viewmodel.rs|views_table.rs|gutter.rs|theme.rs|screenshot.rs|tree.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views_hex.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::|\.size\()" || echo NO-COLOR-TYPO-ADD`
- EXPECT: only `views_hex.rs` + `checks/gui-hexfind-ae/**`;
  KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-TYPO-ADD (model wave — koi paint token nahi)

## G3 — Hex find position + count-label unit-tested

- CHECK: `cargo test -p autocompare-gui hex_find`
- EXPECT: green, kam se kam ye tests:
  + `hex_find_wraps_forward_and_backward` (purana, same values pass)
  + `hex_find_positions_report_row_col_with_wrap` (naya: `(row, col)` hit,
    `offset = row * 16 + col`, wrap aage/peeche, invalid col par `None`)
  + `hex_find_count_and_label_formats_n_of_m` (naya: overlapping count,
    khali needle par `0`, label `"2/5"` / `None` par `"0/5"` / empty par `"0/0"`)
- WHAT: `views_hex.rs` me pure helpers (paint agla wave use karega,
  `#[allow(dead_code)]` file precedent ke saath):
  + `hex_offset_to_row_col(offset) -> (u64, u64)`
  + `hex_row_col_to_offset(row, col) -> Option<u64>` (`col >= 16` par `None`)
  + `hex_find_next_position(haystack, needle, from_row, from_col) -> Option<(u64, u64)>`
    (from inclusive, wrap; andar `hex_find_next` reuse)
  + `hex_find_prev_position(haystack, needle, from_row, from_col) -> Option<(u64, u64)>`
    (from exclusive, wrap; andar `hex_find_prev` reuse)
  + `hex_find_count(haystack, needle) -> usize` (overlapping occurrences, khali par `0`)
  + `hex_find_count_label(position: Option<usize>, total: usize) -> String`

## G4 — MS Rust guidelines + fmt clean (sirf OWNS files par fmt)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `cargo fmt -- crates/gui/src/views_hex.rs && git diff --stat`
  + `rustfmt --edition 2024 --check crates/gui/src/views_hex.rs; echo fmt:$?`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views_hex.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -20 crates/gui/src/views_hex.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0` (OWNS file par), ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naye helpers pure fns (koi indexing/slicing nahi, `.get()` only;
  saturating arithmetic; `format!` only allocation)
- NOTE: bare `cargo fmt` mat chalao — sibling files untouched rahen

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-hexfind-ae/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-hexfind-ae/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64 (model-only wave — PNG bit-same rehna expected hai)

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-hexfind-ae/REPORT.md checks/gui-hexfind-ae/round-1/after.png checks/gui-hexfind-ae/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views_hex.rs checks/gui-hexfind-ae/GATES.md checks/gui-hexfind-ae/REPORT.md checks/gui-hexfind-ae/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
