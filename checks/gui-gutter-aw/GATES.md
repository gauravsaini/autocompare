# GATES — gui-gutter-aw (overview ruler parity)

Wave: AW-GUTTER. OWNS: `crates/gui/src/gutter.rs` + `checks/gui-gutter-aw/**` only.
Branch `main`, fresh pull at start. `checks/opencode-waves/STATUS.md` untouched (orchestrator-owned).

## Gate results (2026-11-20 UTC)

| # | Gate | Command | Result |
|---|---|---|---|
| 1 | Unit tests (full crate) | `cargo test -p autocompare-gui` | PASS — 537 passed, 0 failed (gutter:: 18/18, 7 new) |
| 2 | Harness round-1 | `zsh tools/gui-polish/run-geo-typo.sh checks/gui-gutter-aw` | PASS — exit 0, `RESULT: PASS` |
| 3 | PNG valid | `after.png` 9624 B (min 8192 ok), magic `89504e470d0a1a0a` ok, 256 distinct bytes (non-blank) | PASS |
| 4 | Clippy, OWNS file | `cargo clippy -p autocompare-gui` (lib+bins) | PASS — exit 0, zero `gutter.rs` mentions |
| 5 | fmt, OWNS file | `rustfmt --edition 2024 --check crates/gui/src/gutter.rs` | PASS — clean |
| 6 | Banned patterns, non-test code | grep `unwrap\|expect(\|panic!\|todo!` on `gutter.rs` + `#![forbid(unsafe_code)]` present | PASS — 0 hits |
| 7 | OWNS discipline | `git status --short` own delta | PASS — only `crates/gui/src/gutter.rs` modified + `checks/gui-gutter-aw/**` added; `views.rs`/`views_table.rs`/`views_hex.rs`/`tree.rs`/`app.rs`/`home.rs` untouched by this wave |
| 8 | Colors frozen | `git diff` on `gutter.rs` | PASS — zero `Color32`/`Visuals`/palette touches; segment kind→color mapping still 100% in paint |

## Known out-of-OWNS noise (not failures of this wave)

- `cargo clippy -p autocompare-gui --all-targets` (test profile) reports 4
  `clippy::indexing-slicing` deny-errors at `crates/gui/src/views.rs:8308-8309`
  (`rows[0]`/`rows[1]` in wrap test code) plus 5 `dead_code` warnings for
  `wrap_*` helpers in `views.rs`. That file is dirty in the working tree
  from the parallel wave and is explicitly out of this wave's OWNS
  (`views.rs` KO HAATH MAT LAGANA), so it was not touched. Plain
  `cargo test` (rustc, 537 green) and `cargo clippy` (non-test, exit 0)
  are unaffected.
- `after.png` (9624 B) is byte-identical in size to the localrun text
  fixture: the 7-row fixture bands (~57px) never hit the old 2px floor,
  so the 2→4px min-height only changes tall-file rendering (covered by
  the new unit tests, not by this fixture).
