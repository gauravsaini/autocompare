# GATES exq-table85 round-1 — table geometry token adoption in views_table.rs

OWNS: crates/gui/src/views_table.rs + checks/gui-exq-table85/round-1/** ONLY.
NO-TOUCH: theme.rs app.rs views.rs main.rs copy.rs take.rs gutter.rs tree.rs
home.rs + checks/gui-sessionpaint-by/** crates/sessions/**.
Colors FROZEN: zero new Color32. Typo FROZEN: zero new sizes/roles.
Geometry FIRST: 2 value-identical literal -> SPACE_* token adoptions.

## Gates

- [ ] G1 baseline harness PASS (pre-implement, fresh round dir)
  CHECK: tools/exquisite/run.sh checks/gui-exq-table85/round-1 2>&1 | tail -5
  EXPECT: RESULT: PASS

- [ ] G2 needle width token-adopted (120.0 value-identical, SPACE_XL * 5.0)
  CHECK: grep -n "TABLE_FIND_NEEDLE_W: f32 = " crates/gui/src/views_table.rs
  EXPECT: SPACE_XL

- [ ] G3 wide canvas token-adopted (1200.0 value-identical, SPACE_XL * 50.0)
  CHECK: grep -n "TABLE30_CANVAS_W: f32 = " crates/gui/src/views_table.rs
  EXPECT: SPACE_XL

- [ ] G4 full gui test suite green
  CHECK: cargo test -p autocompare-gui 2>&1 | tail -3
  EXPECT: ok

- [ ] G5 fmt clean + banned scan zero in non-test code
  CHECK: cargo fmt -p autocompare-gui -- --check 2>&1 | tail -2; echo FMT_DONE; grep -rn "unwrap()\|expect(\|panic!\|todo!\|unreachable!\|unsafe" crates/gui/src/views_table.rs | grep -v "^\s*//" | grep -v "#\[test\]" | awk -F: '{print $1":"$2}' | head -5; echo SCAN_DONE
  EXPECT: SCAN_DONE

- [ ] G6 harness re-run PASS after implement
  CHECK: tools/exquisite/run.sh checks/gui-exq-table85/round-1 2>&1 | tail -5
  EXPECT: RESULT: PASS

- [ ] G7 commit OWNS only, English conventional, no push
  CHECK: git status --short | head -10
  EXPECT: views_table.rs
