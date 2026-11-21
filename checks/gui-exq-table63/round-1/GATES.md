# GATES exq-table63 round-1 — geometry-first token adoption in views_table.rs

OWNS: crates/gui/src/views_table.rs + checks/gui-exq-table63/** ONLY.
NO-TOUCH: tree.rs, app.rs, all other crates/gui files, all other checks dirs.
Colors FROZEN: zero new Color32. Zero new tokens/sizes.

## Gates

- [ ] G1 baseline harness PASS (pre-implement, fresh round dir)
  CHECK: tools/exquisite/run.sh checks/gui-exq-table63/round-1 2>&1 | tail -5
  EXPECT: RESULT: PASS

- [ ] G2 token adoption at 5 value-identical call sites (literals -> tokens)
  CHECK: grep -n "TABLE_KEY_BAR_H: f32 = \|TABLE_KEY_COL_GAP: f32 = \|TABLE_KEY_BADGE_PAD: f32 = \|TABLE_FIND_BAR_H: f32 = \|TABLE_FIND_BADGE_PAD: f32 = " crates/gui/src/views_table.rs
  EXPECT: CHROME_HEADER_BAR_H

- [ ] G3 full gui test suite green
  CHECK: cargo test -p autocompare-gui 2>&1 | tail -3
  EXPECT: ok

- [ ] G4 fmt clean + banned scan zero in non-test code
  CHECK: cargo fmt -p autocompare-gui -- --check 2>&1 | tail -2; echo FMT_DONE; grep -rn "unwrap()\|expect(\|panic!\|todo!\|unreachable!\|unsafe" crates/gui/src/views_table.rs | grep -v "^\s*//" | grep -v "#\[test\]" | awk -F: '{print $1":"$2}' | head -5; echo SCAN_DONE
  EXPECT: SCAN_DONE

- [ ] G5 harness re-run PASS after implement (after.png size-identical expected)
  CHECK: tools/exquisite/run.sh checks/gui-exq-table63/round-1 2>&1 | tail -5
  EXPECT: RESULT: PASS

- [ ] G6 commit OWNS only, English conventional, no push
  CHECK: git status --short | head -10
  EXPECT: views_table.rs
