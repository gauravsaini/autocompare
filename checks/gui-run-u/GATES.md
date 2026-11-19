# Wave U GATES (report-only, owns checks/gui-run-u/**)
- [ ] G1: Release binary rebuilt fresh via cargo build --release -p autocompare-gui
  CHECK: time cargo build --release -p autocompare-gui && stat target/release/autocompare-gui
  EXPECT: exit 0, size bytes recorded vs baseline 11074624B
- [ ] G2: Headless screenshot PNG written
  CHECK: target/release/autocompare-gui --screenshot checks/gui-run-u/after.png; python3 magic+size+distinct check
  EXPECT: PNG magic ok, size vs baseline 9914B, distinct-colors>64
- [ ] G3: Geo-typo round passes
  CHECK: tools/gui-polish/run-geo-typo.sh checks/gui-run-u/round-U-run-1
  EXPECT: exit 0, geo-typo-report.md contains RESULT: PASS
- [ ] G4: Workspace tests match baselines
  CHECK: cargo test --workspace 2>&1 | summary; cargo test -p autocompare-gui summary
  EXPECT: workspace passed vs 579, gui passed vs 231, 0 failed
- [ ] G5: Local-run smoke observed + top-3 BC deltas in own words
  CHECK: launch release binary headless/smoke, record render path
  EXPECT: smoke note + 3 deltas recorded, no BC strings/assets copied
- [ ] G6: clippy zero + fmt clean
  CHECK: cargo clippy --workspace --all-targets; cargo fmt --check
  EXPECT: 0 warnings/errors; fmt exit 0
- [ ] G7: Zero source edits + zero banned patterns in owned files
  CHECK: git status --short (only checks/gui-run-u/**); grep -rn unwrap/expect/panic/todo owned non-test code
  EXPECT: no source diffs, 0 hits
