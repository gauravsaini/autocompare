# AGYX-DIFF-I GATES — Exquisite Diff Core (Beyond Compare Parity)

Track: AGYX-DIFF-I diff parity harden
Scope: `crates/diff/src/text_diff.rs`, `crates/diff/src/merge.rs`, `crates/diff/src/merge3.rs`, `crates/diff/src/grammar.rs`, `crates/diff/src/word_diff.rs`, `crates/diff/tests/**`, `checks/agyx-diff-i/**` ONLY.
Goal: Diff parity harden — Myers/Patience minimality, 3-way Take resolution, grammar unimportant-text suppression, word/char diff spans (pure function, zero I/O).

## Acceptance Gates

- [ ] **G1: Unit & integration test suite green**
  - CHECK: `CARGO_BUILD_JOBS=2 cargo test -p autocompare-diff`
  - EXPECT: exit 0, all tests passed, 0 failed.

- [ ] **G2: Clippy zero warnings**
  - CHECK: `cargo clippy -p autocompare-diff --all-targets -- -D warnings`
  - EXPECT: exit 0, zero warnings.

- [ ] **G3: Rustfmt check clean**
  - CHECK: `cargo fmt --check -- crates/diff/src/text_diff.rs crates/diff/src/merge.rs crates/diff/src/merge3.rs crates/diff/src/grammar.rs crates/diff/src/word_diff.rs crates/diff/src/lib.rs`
  - EXPECT: exit 0, no formatting diffs.

- [ ] **G4: Banned pattern scan zero in non-test code**
  - CHECK: `python3 -c "import re, glob, sys; files=['crates/diff/src/text_diff.rs','crates/diff/src/merge.rs','crates/diff/src/merge3.rs','crates/diff/src/grammar.rs','crates/diff/src/word_diff.rs']; banned=re.compile(r'\b(unwrap|expect|panic|todo|unimplemented|unreachable)\b|unsafe'); failed=False
for f in files:
    content=open(f).read().split('#[cfg(test)]')[0]
    for i, line in enumerate(content.splitlines(), 1):
        line=line.split('//')[0]
        if banned.search(line):
            print(f'{f}:{i}: {line}'); failed=True
sys.exit(1 if failed else 0)" && echo ZERO-BANNED`
  - EXPECT: ZERO-BANNED

- [ ] **G5: ac-loop verification PASS**
  - CHECK: `tools/ac-loop/run.sh checks/agyx-diff-i/round-1`
  - EXPECT: RESULT: PASS

- [ ] **G6: Disjoint OWNS pathspec verification**
  - CHECK: Staging pathspec check (`git add crates/diff/src/text_diff.rs crates/diff/src/merge.rs crates/diff/src/merge3.rs crates/diff/src/grammar.rs crates/diff/src/word_diff.rs crates/diff/src/lib.rs crates/diff/tests checks/agyx-diff-i`)
  - EXPECT: Only files in OWNS touched and committed locally; no forbidden files touched.
