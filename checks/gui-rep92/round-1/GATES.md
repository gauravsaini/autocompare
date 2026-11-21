# Gates: rep92 round-1 reports builders

OWNS: crates/reports/**, checks/gui-rep92/**

Scope: rep92 round-1 me HTML color/mono + linked child, Plain Text, XML, Unified/Context Patch builders ko verify/harden karna, bina GUI touch kiye.

- [ ] rep92:G1: reports test suite green hai.
  CHECK: cargo test -p autocompare-reports 2>&1 | tail -5; echo TESTS_DONE_REP92
  EXPECT: TESTS_DONE_REP92
  CWD: /tmp/wt-rep92
  EVIDENCE: 31 passed 0 failed, exit 0 (tests.out)

- [ ] rep92:G2: reports crate strict clippy clean hai.
  CHECK: cargo clippy -p autocompare-reports --all-targets -- -D warnings 2>&1 | tail -3; echo CLIPPY_OK_REP92
  EXPECT: CLIPPY_OK_REP92
  CWD: /tmp/wt-rep92
  EVIDENCE: exit 0, warnings 0

- [ ] rep92:G3: reports crate fmt clean hai.
  CHECK: cargo fmt -p autocompare-reports -- --check && echo FMT_OK_REP92
  EXPECT: FMT_OK_REP92
  CWD: /tmp/wt-rep92
  EVIDENCE: exit 0, FMT_OK

- [ ] rep92:G4: prod src me banned tokens zero hain (unsafe block, unwrap/expect, panic!/todo!/unimplemented!/unreachable!).
  CHECK: rg -n "unwrap\(|expect\(|\btodo!|\bunimplemented!|\bunreachable!|\bpanic!|\bunsafe\s*\{" crates/reports/src && echo BANNED_FOUND_REP92 || echo BANNED_ZERO_REP92
  EXPECT: BANNED_ZERO_REP92
  CWD: /tmp/wt-rep92
  EVIDENCE: rg clean, BANNED_ZERO (forbid unsafe_code only hit excluded)

- [ ] rep92:G5: OWNS ke bahar koi change nahi (no GUI touch, sessionpaint-by stray untouched).
  CHECK: git status --porcelain -- crates/gui checks/gui-sessionpaint-by | wc -l | grep -q "^ *0$" && echo DISJOINT_OK_REP92
  EXPECT: DISJOINT_OK_REP92
  CWD: /tmp/wt-rep92
  EVIDENCE: 0 changes outside OWNS
