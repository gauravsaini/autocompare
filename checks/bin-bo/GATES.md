# BO-BIN GATES — macOS install binary early-feedback

Scope: sirf install path (scripts/ + tools/packaging/ + checks/bin-bo/).
GUI .rs edits MANA. Colors/geometry scope bahar.

## CHECK list

1. **CHECK: release build ok** — `cargo build --release -p autocompare-gui`
   exit 0, `target/release/autocompare-gui` exists.
2. **CHECK: smoke ok** — release binary `--screenshot <tmp>.png` exit 0,
   output valid PNG (magic bytes `89 50 4E 47`), size > 0.
3. **CHECK: artifact exists** — install script `scripts/install-macos.sh`
   exists + executable; version stamp file me version + git sha;
   packaged artifact (tar.gz ya .app copy) size > 0.
4. **CHECK: suite green** — koi crates/** touch nahi, isliye existing
   test suite green rehni chahiye (no new test failures introduced).

## EXPECT

- `target/release/autocompare-gui` size > 0 bytes.
- Smoke PNG valid PNG, size > 0 bytes.
- `scripts/install-macos.sh --help` (ya dry-run) exit 0 bina side-effect.
- `git status` me sirf owned paths dirty:
  scripts/, tools/packaging/, checks/bin-bo/ (+ README install section).
- NO-TOUCH clean: crates/**, checks/opencode-waves/STATUS.md,
  checks/gui-geo-bn/, checks/gui-typo-bn/, checks/localrun-bo/.
