# Gates: exq-typoinst61 round-1 (consume typo54 chains in app.rs)

OWNS: crates/gui/src/app.rs, checks/gui-exq-typoinst61/**

Scope: typo54 candidate chains ko app.rs me consume karna — har TypoRole ka chain single-source resolve + ek startup log line (pure builder + existing eprintln pattern par call site).

- [x] G1: GATES.md implements se pehle likha gaya
  EVIDENCE: file 14:26 bani, app.rs edit 14:3x — git log order se verifiable

- [x] G2: baseline exquisite harness PASS
  CHECK: tools/exquisite/run.sh checks/gui-exq-typoinst61/round-1
  EXPECT: RESULT: PASS
  EVIDENCE: baseline run PASS (PNG 10946B, magic ok); report me recorded

- [x] G3: har TypoRole apne typo54 chain par resolve hota hai (Mono->MONO, chrome->SANS), koi duplicated stack string nahi
  CHECK: cargo test -p autocompare-gui typoinst61
  EXPECT: test result: ok
  EVIDENCE: typo_exq_typoinst61_chains ok (1 passed); chain fns sirf TypoRole::candidate_stack par delegate

- [x] G4: full gui test suite green + fmt clean + non-test code me banned tokens 0
  CHECK: cargo test -p autocompare-gui && cargo fmt -p autocompare-gui -- --check
  EXPECT: test result: ok
  EVIDENCE: lib 1123 passed 0 failed; fmt exit 0; banned grep count 0

- [x] G5: harness re-run PASS + paint-neutral (kittest render DOCUMENTED, chains headless hain)
  CHECK: tools/exquisite/run.sh checks/gui-exq-typoinst61/round-1
  EXPECT: RESULT: PASS
  EVIDENCE: re-run PASS, after.png 10946B baseline-identical (paint-neutral); eprintln call site live-proof: --nocapture me "live font chains: proportional <- SF Pro | ..." stderr par; screenshot.out me nahi (headless path ui frame nahi chalata — expected, chains headless)

- [x] G6: commit me sirf OWNS files (app.rs + checks/gui-exq-typoinst61/**), strays revert, NO PUSH
  EVIDENCE: pending (manual: git show --stat)
