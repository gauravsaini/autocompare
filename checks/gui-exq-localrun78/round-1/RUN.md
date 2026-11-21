# Local run — autocompare-gui (exq-localrun78 round-1)

## Run command (early feedback, ek command)

```sh
cargo run -p autocompare-gui
```

Boot par console me ye hint aata hai (stderr):

```text
autocompare-gui: local run (cargo run -p autocompare-gui) — state memory/tmp only, no network; --help for flags
```

## Headless smoke (bina window khole)

```sh
./target/debug/autocompare-gui --help                 # exit 0, usage + hint
OUT="${TMPDIR:-/tmp}/smoke-localrun78.png"
./target/debug/autocompare-gui --screenshot "$OUT"    # exit 0, valid PNG sirf tmp me
```

## Guarantees (sirf `crates/gui/src/main.rs` me)

- `persist_window: false` + `persistence_path = tmp/autocompare-gui` — window/state
  kabhi HOME me persist nahi hota (chahe `persistence` feature aage on ho).
- `--screenshot <path>` user ka diya path rehta hai (harness `checks/` me likhta
  hai) — us par rok nahi; app ka apna state sirf memory/tmp me.
- Koi network path nahi (`std::net`/socket/http crates zero); errors panels/exit
  codes me, panic nahi.

## Captured evidence (is round dir me)

- `smoke-help.log` — `--help` output (exit 0, hint line 1 par).
- `smoke-shot.log` — `--screenshot` output (exit 0, 10946B valid PNG tmp me).
- `cargo-test.log` — `cargo test -p autocompare-gui`: 1140 + 2 passed, 0 failed.
- `cargo-test-baseline.log` — pre-change baseline: 1140 + 2 passed, 0 failed.
- `GATES.md` — G1..G7 ledger.
