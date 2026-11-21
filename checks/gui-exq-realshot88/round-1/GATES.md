# GATES exq-realshot88 round-1 — live egui screenshot pipeline hardening

OWNS: `crates/gui/tests/real_egui_shot.rs`, `checks/gui-exq-realshot88/**` ONLY.
Never touch: `crates/gui/src/theme.rs`, `crates/gui/src/views.rs`,
`crates/gui/src/screenshot.rs`, `checks/gui-sessionpaint-by/**`, `crates/sessions/**`.

- [ ] G1 baseline real_egui_shot green (dark + light)
  CHECK: cargo test -p autocompare-gui --test real_egui_shot
  EXPECT: REALSHOT88_BASELINE_PASS
- [ ] G2 live capture hardened: dark + light kittest shots, PNG magic + size + non-blank asserts, geometry signals intact
  CHECK: cargo test -p autocompare-gui --test real_egui_shot -- --nocapture
  EXPECT: REALSHOT88_CAPTURE_PASS
- [ ] G3 exquisite harness RESULT PASS on own round dir
  CHECK: tools/exquisite/run.sh checks/gui-exq-realshot88/round-1
  EXPECT: REALSHOT88_HARNESS_PASS
- [ ] G4 fmt clean + banned tokens 0 in non-test code + OWNS disjoint
  CHECK: cargo fmt --check && banned-scan
  EXPECT: REALSHOT88_HYGIENE_PASS
- [ ] G5 commit feat(gui) exists locally, no push, done file written
  CHECK: git log --oneline -1 && test -f /tmp/wave-realshot88.done
  EXPECT: REALSHOT88_COMMIT_PASS
