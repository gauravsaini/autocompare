# GATES — exq-binary81 round-1 (release packaging)

OWNS: tools/release ONLY (never crates/**).

## G1: release scripts present + executable

- CHECK: `test -x tools/release/build-release.sh && test -f tools/release/INSTALL.md && echo GATES_OK`
- EXPECT: `GATES_OK`

## G2: release build succeeds (tmp-only, no network beyond cargo)

- CHECK: `bash tools/release/build-release.sh 2>&1 | tail -5`
- EXPECT: `RELEASE_BUILD_OK`

## G3: smoke --help works

- CHECK: `./target/release/autocompare-gui --help 2>/dev/null | head -3`
- EXPECT: `autocompare`

## G4: smoke --screenshot tmp PNG valid

- CHECK: `bash tools/release/smoke-test.sh 2>&1 | tail -5`
- EXPECT: `SMOKE_OK`

## G5: gui tests green (release-only scope; full suite attempted)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -3`
- EXPECT: `test result: ok`

## G6: fmt clean

- CHECK: `cargo fmt --check 2>&1 | head -5; echo FMT_EXIT:$?`
- EXPECT: `FMT_EXIT:0`

## G7: banned patterns 0 in scripts (shellcheck if available)

- CHECK: `bash tools/release/check-banned.sh 2>&1 | tail -3`
- EXPECT: `BANNED_ZERO`
