# INSTALL — autocompare-gui release binary

Early-feedback install docs. Dev workflow unaffected.

## Build

```sh
bash tools/release/build-release.sh
```

Runs `cargo build --release -p autocompare-gui` only.
Tmp-only: smoke artifacts go to `$TMPDIR`, never repo.
No network beyond what cargo itself needs.

## Smoke test

```sh
bash tools/release/smoke-test.sh
```

Checks:

1. `target/release/autocompare-gui --help` mentions `autocompare`.
2. `autocompare-gui --screenshot $TMPDIR/<tmp>.png` writes a valid
   PNG (magic + >= 8192 bytes), then deletes it.

## Install (local)

```sh
cp target/release/autocompare-gui ~/.local/bin/
~/.local/bin/autocompare-gui --help
```

## Notes

- GUI state stays in memory/tmp only, no network calls.
- Headless screenshot mode needs no display.
- Full app launch needs a display server; CI/headless me
  `--help` + `--screenshot` hi smoke signal hai.
