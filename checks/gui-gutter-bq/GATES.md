# GATES — gui-gutter-bq (text dual-pane + center gutter)

Wave: BQ-GUTTER. OWNS: `crates/gui/src/views.rs` + `checks/gui-gutter-bq/**` only.
Branch `main`, fresh pull at start. `checks/opencode-waves/STATUS.md` untouched (orchestrator-owned).
KEVL NO-TOUCH: theme.rs, gutter.rs, home.rs, tree.rs, viewmodel.rs, app.rs, screenshot.rs, views_table.rs, checks/opencode-waves/STATUS.md, checks/gui-table-bq/.

## Scope (livecomp REPORT G-T1 P0 + G-T3 P1)

- G-T1 (P0): ours single combined row `Lnum Ltext | Rnum Rtext` + copy buttons inline; BC do independent panes + beech me patli center gutter (per-diff arrow). Fix: `paint_text_body` me left pane column + fixed-width gutter column + right pane column; copy buttons gutter me.
- G-T3 (P1): number gutter text se chipka + sabse left `•` dots; BC right-aligned fixed-width gutter, koi dots nahi. Fix: number gutter fixed width + right-align (gutter_text shape), `•`/`·` dot column hatao. Header compress (pehle wave) bana rehna chahiye. Colors FROZEN. Typo: TypoRole aliases reuse, naye size nahi.

## Gate results (2026-11-20 UTC, round-1)

| # | Gate | Command | Result |
|---|---|---|---|
| 1 | Unit tests (full crate) | `cargo test -p autocompare-gui` | PASS — 611 passed, 0 failed (1 new: center-gutter token + copy gate) |
| 2 | Harness round-1 | `zsh tools/gui-polish/run-geo-typo.sh checks/gui-gutter-bq/round-1` | PASS — exit 0, `RESULT: PASS` (round 1/6) |
| 3 | PNG valid | `after.png` magic `89504e470d0a1a0a` + >= 8192B + distinct > 64 | PASS — 9695 B, magic ok, 255 distinct |
| 4 | Clippy, all targets | `cargo clippy -p autocompare-gui --all-targets` zero warnings | PASS — exit 0 |
| 5 | fmt, OWNS only | `rustfmt --edition 2024 --check crates/gui/src/views.rs` | PASS — clean |
| 6 | Banned, non-test code | grep `unwrap\|expect(\|panic!\|todo!` on diff + `#![forbid(unsafe_code)]` present | PASS — 0 hits |
| 7 | OWNS discipline | `git status --short` own delta | PASS — commit me sirf `crates/gui/src/views.rs` + `checks/gui-gutter-bq/**`; `views_table.rs` parallel wave ka (this wave untouched) |
| 8 | Colors frozen | `git diff` on views.rs | PASS — zero `Color32`/`Visuals`/palette touches (grep 0) |
| 9 | STATUS untouched | `git diff --name-only` me `checks/opencode-waves/STATUS.md` nahi | PASS |

## CHECK / EXPECT (unlazy)

- CHECK: `paint_text_body` me har row ek hi combined horizontal hai (`left_job` + inline `→/←` ya `" | "` + `right_job`) + sabse left `·/▶` select column.
- EXPECT: har row `left pane column | fixed-width center gutter (diff par copy buttons, equal par blank click-to-select) | right pane column`; koi `·/▶` leading column nahi; number gutter `gutter_text` right-aligned fixed width; toolbar + status line same; TypoRole reuse, koi `.size(` naya nahi; colors zero.
