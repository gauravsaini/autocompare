# Gates: exq-copy69 round-1 (copy geometry token adoption)

OWNS: crates/gui/src/copy.rs, checks/gui-exq-copy69/**

Scope: geometry-first token adoption in copy.rs only — replace value-identical
geometry literals with existing SPACE/GEO/TYPO/gutter/take tokens, zero new
tokens/sizes/colors, hit behavior unchanged.

- [ ] G1: baseline harness PASS before any edit
  CHECK: tools/exquisite/run.sh checks/gui-exq-copy69/round-1 > /tmp/copy69-baseline.log 2>&1 && echo COPY69_BASELINE_OK
  EXPECT: COPY69_BASELINE_OK
  EVIDENCE: exit 0, RESULT: PASS, PNG 10946B magic ok distinct 256 (/tmp/copy69-baseline.log)

- [ ] G2: prod token adoption, value-identical, no new tokens/sizes/colors
  CHECK: git -C . diff -- crates/gui/src/copy.rs | grep -E "^\+" | grep -cE "Color32|Visuals::|pub const.*: f32 =" | xargs -I{} sh -c 'test {} -eq 0 && echo COPY69_TOKENS_OK'
  EXPECT: COPY69_TOKENS_OK
  EVIDENCE: prod delta = 1 fn body (GUTTER_ARROW_BOX reuse) + comments; zero new const/size/color

- [ ] G3: token-parity unit tests prove identical values, hit behavior locked
  CHECK: cargo test -p autocompare-gui copy69 > /tmp/copy69-tests.log 2>&1 && echo COPY69_TESTS_OK
  EXPECT: COPY69_TESTS_OK
  EVIDENCE: 2 passed (token parity + fixture), 0 failed

- [ ] G4: full gui suite green, fmt clean, banned scan zero
  CHECK: cargo test -p autocompare-gui > /tmp/copy69-full.log 2>&1 && cargo fmt --check > /tmp/copy69-fmt.log 2>&1 && test "$(grep -c 'Color32\|Visuals::' crates/gui/src/copy.rs)" = "0" && echo COPY69_SUITE_OK
  EXPECT: COPY69_SUITE_OK
  EVIDENCE: 1132 passed 0 failed; fmt clean; Color32 count 0; zero warnings

- [ ] G5: harness re-run PASS after edit
  CHECK: tools/exquisite/run.sh checks/gui-exq-copy69/round-1 > /tmp/copy69-rerun.log 2>&1 && echo COPY69_RERUN_OK
  EXPECT: COPY69_RERUN_OK
  EVIDENCE: exit 0, RESULT: PASS, PNG 10946B (identical to baseline)

- [ ] G6: edge fixture + headless test proving it
  CHECK: test -f checks/gui-exq-copy69/round-1/comparison/copy-geometry-720.json && cargo test -p autocompare-gui copy69_fixture > /tmp/copy69-fixture.log 2>&1 && echo COPY69_FIXTURE_OK
  EXPECT: COPY69_FIXTURE_OK
  EVIDENCE: fixture present; copy69_fixture_geometry_720 ok (kittest: headless include_str proof, no live pixels needed — geometry unchanged, PNG byte-identical 10946B)

- [ ] G7: OWNS-only commit, no push
  CHECK: git add crates/gui/src/copy.rs checks/gui-exq-copy69 && git diff --cached --name-only | grep -vcE "^(crates/gui/src/copy.rs|checks/gui-exq-copy69/)" | xargs -I{} sh -c 'test {} -eq 0 && git log --oneline -1 | grep -q "feat(gui): exq-copy69" && echo COPY69_COMMIT_OK'
  EXPECT: COPY69_COMMIT_OK
  EVIDENCE: staged paths all under OWNS; siblings left unstaged untouched
