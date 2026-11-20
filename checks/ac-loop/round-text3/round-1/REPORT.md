# round-text3 round-1 REPORT — Text/Merge/Grammar polish (PLAN C4 follow-up)

Scope (OWNS only): `crates/diff/src/text*`, `crates/diff/src/merge*`, `crates/diff/src/grammar*`, `checks/ac-loop/round-text3/**`.
Clean-room: outside observation only, no decompilation, no asset/wording copying.

## Changes (minimal, stdlib-only, no new deps)

1. `crates/diff/src/text_diff.rs`: added `test_myers_patience_replay_and_minimality` — Myers/Patience scripts must tile both inputs, report `exact`, replay via `apply_diff_slices` to `b`, Myers single-replace and moved-block cases assert minimal 1 delete + 1 insert.
2. `crates/diff/src/grammar.rs`: added `test_ignore_string_contents_suppression` — `ignore_string_contents` with `double_quote` marks string-only change unimportant, code change outside strings stays important, strict mode stays important.
3. `crates/diff/src/merge.rs`: added `test_merge_take_edge_empty_base_and_chunks` — empty base identical inserts merge clean, divergent inserts conflict with `conflict_chunks` count 5 and start 0, Take Both orders/custom/clear verified, both-delete same line merges clean.
4. Fixture (1 edge set): `checks/ac-loop/round-text3/round-1/comparison/edge-empty-base-{base,left,right,expected}.txt` — empty base, left `a`, right `b`, expected default-marker conflict render (mirrors new merge test).

## Verify (only `/tmp/target-op-text3`)

- `CARGO_TARGET_DIR=/tmp/target-op-text3 CARGO_BUILD_JOBS=2 cargo test -p autocompare-diff`: PASS — lib 92 passed (baseline 89 + 3 new), suites 92 + 33 + 44 + 20 + 4 + 38, 0 failed.
- `CARGO_TARGET_DIR=/tmp/target-op-text3 CARGO_BUILD_JOBS=2 cargo clippy -p autocompare-diff --all-targets -- -D warnings`: PASS — zero warnings/errors.
- `rustfmt --edition 2024 --check` on touched owned files: PASS — clean.
- Banned-scan non-test (`unwrap(`, `expect(`, `panic!`, `todo!`, `unimplemented!`, `unreachable!` before `#[cfg(test)]` in owned files): PASS — 0 hits (remaining hits are inside `#[cfg(test)]` allows).
- `forbid(unsafe_code)` workspace lint holds; no `unsafe` in owned non-comment code.

## Gates

- leaf-1.2:G1 (Myers/Patience replay, minimality, grammar suppression, 3-way Take): PASS
- clippy zero + banned-scan zero non-test: PASS
- rustfmt clean owned files: PASS

RESULT: PASS
