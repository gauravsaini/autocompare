# GUI parity wave W1 — GATES

- [x] Scope gate: sirf `crates/gui/**` + `checks/gui-parity-w1/**` change.
      `git status` me baaki dirty paths (dusri waves) untouched.
- [x] Geometry-first gate: har change width/height/margin/padding/align
      hai; koi color value change nahi.
- [x] Typography-second gate: koi family/weight/size/line-height change
      nahi (sirf existing role reuse).
- [x] MS-rust gate: `forbid(unsafe_code)` intact; non-test me
      unwrap/expect/panic/todo/unreachable/index-slicing nahi (clippy
      `zero warnings` se lock).
- [x] Baseline gate: `cargo test -p autocompare-gui` 699 -> 703, sab green.
- [x] Clippy gate: `--all-targets` zero warnings.
- [x] Fmt gate: `cargo fmt --check` clean.
- [x] Evidence gate: round-1 + round-2 notes, REPORT.md, GATES.md maujood.
- [x] Screenshot gate: home + folder + text PNG 960x600, magic ok.
- [x] Commit gate: pathspec-only (`crates/gui`, `checks/gui-parity-w1`),
      conventional `feat(gui): ...`, no `-A`, no push.
