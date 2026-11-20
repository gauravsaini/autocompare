# Gates: Exquisite GUI10 geometry-first typo-second vs BC (clean-room)

OWNS:
- chrome: crates/gui/src/app.rs, crates/gui/src/home.rs, checks/gui-exq-chrome10/**
- text: crates/gui/src/views.rs, crates/gui/src/gutter.rs, checks/gui-exq-text10/**
- theme: crates/gui/src/theme.rs, crates/gui/src/viewmodel.rs, crates/gui/src/tree.rs, checks/gui-exq-theme10/**
- hextable: crates/gui/src/views_hex.rs, crates/gui/src/views_table.rs, crates/gui/src/copy.rs, crates/gui/src/take.rs, checks/gui-exq-hextable10/**

Scope: geometry first (width/height/margin/padding/align/position), typography second (family/weight/line-height/spacing). Colors FROZEN. Clean-room outside-observe only. MS rust guidelines: forbid unsafe, no unwrap/expect/panic/todo/unimplemented/unreachable in non-test, no indexing_slicing, headless-first, cargo fmt clean.
Loop discipline: RESULT: PASS = tests green + PNG valid (magic, >=8192B, non-blank). OWNS disjoint per agent. Max 3-4 parallel agents per wave; poll with exponential backoff (base 30s, x2, max 10min). Mix opencode default model for edits + agyx swarm for fan-out.

- [ ] G1: exquisite harness PASS chrome (geometry signals present, PNG valid)
  CHECK: tools/exquisite/run.sh checks/gui-exq-chrome10/round-1 crates/gui/src
  EXPECT: RESULT: PASS
- [ ] G2: exquisite harness PASS text (gutter + panes aligned, no clip)
  CHECK: tools/exquisite/run.sh checks/gui-exq-text10/round-1 crates/gui/src
  EXPECT: RESULT: PASS
- [ ] G3: exquisite harness PASS theme (TypoRole reuse, no new sizes, colors frozen)
  CHECK: tools/exquisite/run.sh checks/gui-exq-theme10/round-1 crates/gui/src
  EXPECT: RESULT: PASS
- [ ] G4: gui tests green + clippy + fmt on OWNS files
  CHECK: cargo test -p autocompare-gui --quiet && cargo clippy -p autocompare-gui --all-targets -- -D warnings && cargo fmt --check -p autocompare-gui
  EXPECT: test result: ok
- [ ] G5: zero banned tokens in OWNS non-test code
  CHECK: python3 -c 'import subprocess,sys; out=subprocess.run(["rg","-n","unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!|unsafe", "crates/gui/src/app.rs","crates/gui/src/views.rs","crates/gui/src/gutter.rs","crates/gui/src/theme.rs","crates/gui/src/home.rs","crates/gui/src/tree.rs","crates/gui/src/viewmodel.rs"],capture_output=True,text=True); print(out.stdout[:2000] if out.stdout.strip() else "ZERO-BANNED"); sys.exit(0 if len(out.stdout.strip())==0 else 1)'
  EXPECT: ZERO-BANNED
- [ ] G6: local run screenshot all 9 tabs valid (early feedback)
  CHECK: test -f checks/localrun/after-text.png && test -f checks/localrun/after-folder.png && file checks/localrun/after-*.png | grep -q PNG && echo LOCALRUN-OK
  EXPECT: LOCALRUN-OK
- [ ] G7: install tarball builds (early feedback binary)
  CHECK: test -f dist/autocompare-gui-*-macos-*.tar.gz && echo INSTALL-OK
  EXPECT: INSTALL-OK
