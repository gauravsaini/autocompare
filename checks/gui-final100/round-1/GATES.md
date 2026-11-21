# final100 round-1 — root-4 Check loop evidence (current main ba9f76c)

Scope: checks-only. OWNS `checks/gui-final100/**` ONLY. Zero src touch.
Worktree: /tmp/wt-final100. Base: ba9f76c. No push. `CARGO_TARGET_DIR=/tmp/target-final100`.

- [x] G1: Workspace formatting is clean.
  - CHECK: `cargo fmt --all --check`
  - EXPECT: exit 0, no diff output
- [x] G2: Workspace clippy strict lints pass with zero warnings.
  - CHECK: `cargo clippy --workspace --all-targets -- -D warnings`
  - EXPECT: exit 0
- [x] G3: Full workspace test suite passes.
  - CHECK: `cargo test --workspace`
  - EXPECT: exit 0, all suites `test result: ok`
- [x] G4: Strict layering rules hold.
  - CHECK: `cargo run -p xtask -- layers`
  - EXPECT: `layers: ok`, exit 0
- [x] G5: Secret scan is clean.
  - CHECK: `cargo run -p xtask -- secrets`
  - EXPECT: `secrets: ok`, exit 0
- [x] G6: craft.json manifest validates (schema_version 1, mode clean-room).
  - CHECK: `python3 -c "import json; d=json.load(open('craft.json')); assert d['schema_version']==1 and d['mode']=='clean-room'; print('CRAFT_JSON_OK')"`
  - EXPECT: `CRAFT_JSON_OK`, exit 0
