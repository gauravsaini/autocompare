# Gates: exq-lib86 round-1 module registry polish

OWNS: crates/gui/src/lib.rs

Scope: Polish the gui crate module registry (decl order + docs) with zero logic change; colors/typography/geometry frozen.

- [ ] G1: Gates ledger written before any code edit
  CHECK: test -f checks/gui-exq-lib86/round-1/GATES.md && echo GATES_PRESENT
  EXPECT: GATES_PRESENT
  EVIDENCE: pending

- [ ] G2: Baseline gui tests green before edit
  CHECK: cargo test -p autocompare-gui 2>&1 | tail -5
  EXPECT: test result: ok
  EVIDENCE: pending

- [ ] G3: Only crates/gui/src/lib.rs changed (disjoint OWNS, strays reverted)
  CHECK: git status --porcelain | grep -v '^?? checks/gui-exq-lib86/' | grep -v '^ M crates/gui/src/lib.rs' | grep -v '^?? /tmp' ; test $? -ne 0 && echo OWNS_CLEAN
  EXPECT: OWNS_CLEAN
  EVIDENCE: pending

- [ ] G4: Module decls alphabetical and documented, zero logic change
  CHECK: cargo test -p autocompare-gui 2>&1 | grep -E "test result" | grep -v "0 failed" ; test $? -ne 0 && echo REGISTRY_OK
  EXPECT: REGISTRY_OK
  EVIDENCE: pending

- [ ] G5: cargo fmt clean on owned file
  CHECK: cargo fmt -p autocompare-gui -- --check && echo FMT_CLEAN
  EXPECT: FMT_CLEAN
  EVIDENCE: pending

- [ ] G6: Banned patterns zero in non-test owned file (unwrap/expect/panic/todo/unsafe)
  CHECK: sh -c '! grep -nE "unwrap\(|\.expect\(|panic!|todo!|unimplemented!|[^_a-zA-Z]unsafe[^_a-zA-Z_]" crates/gui/src/lib.rs' && echo BANNED_ZERO
  EXPECT: BANNED_ZERO
  EVIDENCE: pending

- [ ] G7: Commit feat(gui): exq-lib86 module registry exists, no push
  CHECK: git log --oneline -1 | grep -F "feat(gui): exq-lib86 module registry" && echo COMMIT_OK
  EXPECT: COMMIT_OK
  EVIDENCE: pending

- [ ] G8: Done marker /tmp/wave-lib86.done with hash + tests
  CHECK: test -f /tmp/wave-lib86.done && grep -q . /tmp/wave-lib86.done && echo DONE_OK
  EXPECT: DONE_OK
  EVIDENCE: pending
