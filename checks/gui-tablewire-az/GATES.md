# Wave AZ-TABLEWIRE — GATES (P1 only: paint_table_keyed key-bar wiring)

OWNS: `crates/gui/src/views.rs` + `checks/gui-tablewire-az/**` + P1 caller borrow-only line in `crates/gui/src/app.rs:5163` (`&` → `&mut`, no logic).
KEVL: `views_table.rs` / `tree.rs` / `screenshot.rs` KO HAATH MAT LAGANA (read-only: `paint_table_key_bar:291`, `table_header_cells` caller in `views.rs:1352`, hex `&mut` caller precedent `app.rs:4999`). `checks/opencode-waves/STATUS.md` KO HAATH MAT LAGANA (orchestrator-owned).
Colors FROZEN (koi naya hue/token nahi — callee `paint_table_key_bar` ke andar, yahan koi `Color32::from_rgb`/`from_rgba`/`Visuals::`/`SetVisuals` add nahi, koi `.size(` nahi).
Geometry first, typography second (yahan koi naya spacing/size token nahi — sirf existing status/positional/note rhythm reuse).
NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo. Repo ke bahar koi copy/sandbox nahi. `git stash/push` kabhi nahi (baseline check ke liye local `stash push/pop` use hua, wapas restore kiya).

Scope (sirf P1 — P2/P3/P4 mat chhuna):
- `views.rs` `paint_table_keyed` (~1398): signature `view: &TableKeyView` → `view: &mut TableKeyView`; status line ke baad `if let Some(cells) = table_header_cells(model).as_ref()` gate me `views_table::paint_table_key_bar(ui, view, cells)` (unwrap NAHI).
- Existing `positional_note` block rakha (honest note single wording `POSITIONAL_NOTE`, dono `.small().weak()` — duplicate-safai agle wave ka call).
- `paint_table(ui, model, pager)` signature untouched (P3 OPEN hai — `find_ctx`/`style` threading nahi).
- Header/data Grid untouched (P2/P3 OPEN), painter overlays nahi (P4 OPEN).
- `app.rs:5163` caller `(&self.table_view, …)` → `(&mut self.table_view, …)` (disjoint-field borrow, hex precedent `app.rs:4999` jaisa; bina iske `cargo check` fail — P1 compile ke liye mandatory 1-line threading).

## G1 — Baseline green (before P1 edit)

- CHECK: `git stash push -- crates/gui/src/views.rs crates/gui/src/app.rs && cargo test -p autocompare-gui 2>&1 | tail -n 3; git stash pop`
- EXPECT: `561 passed; 0 failed` (recorded 2026-11-20, `main@3e4e5e7` sibling-emptyhint-merged HEAD, pre-P1 run)
- EVIDENCE: `round-1/baseline-tests.out` (pre-edit tail)

## G2 — OWNS-only diff, KEVL + STATUS + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/views.rs|crates/gui/src/app.rs|checks/gui-tablewire-az/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "views_table.rs|tree.rs|views_hex.rs|screenshot|STATUS.md" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views.rs crates/gui/src/app.rs | grep -E "^\+\.*(Color32::from_rgb|from_rgba|Visuals::|SetVisuals|\.size\()" || echo NO-COLOR-TYPO-ADD`
  + `git diff -- crates/gui/src/app.rs` must be exactly the 1-line borrow change (no logic)
- EXPECT: diff sirf `views.rs` (P1 wiring ~8 lines) + `app.rs` (1-line `&mut`) + `checks/gui-tablewire-az/**`; `views_table.rs`/`tree.rs`/`views_hex.rs`/`screenshot` diff zero (read-only); `STATUS.md` untouched; stage sirf OWNS paths (+ app.rs caller line same commit me taaki HEAD green rahe); KEVL-CLEAN; NO-COLOR-TYPO-ADD
- EVIDENCE: `round-1/owns-diff.out`

## G3 — P1 wiring, full suite green (koi naya test nahi — paint-only)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `561 passed; 0 failed` (baseline 561 → after 561, delta 0 — P1 sirf paint caller threading hai; headless geometry `tablecell_geo_*` AY me locked hai; `paint_table_keyed` ka koi headless assert nahi)
- WHAT: `paint_table_keyed` me `&mut view` + `table_header_cells(model).as_ref()` if-let gate + `paint_table_key_bar(ui, view, cells)`; `table_header_cells` returns `Option<Vec<String>>` isliye `.as_ref()` gate (`.unwrap_or_default()` NAHI); positional-note block untouched rakha; `paint_table` call untouched
- EVIDENCE: `round-1/tablewire-tests.out`

## G4 — MS Rust guidelines + fmt clean (sirf touched files par fmt)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/views.rs; echo fmt:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/app.rs; echo fmt:$?`
  (sirf touched files; bare `cargo fmt` mat chalao taaki sibling files untouched rahen)
  + `git diff -- crates/gui/src/views.rs crates/gui/src/app.rs | grep -nE "^\+\.*(unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!)" || echo ZERO-BANNED-ADDED`
  + `head -25 crates/gui/src/views.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0` dono files, ZERO-BANNED-ADDED (added lines me `unwrap()`/`expect()` nahi — pre-existing `unwrap_or_default`/`unwrap_or` allowed hain), `forbid(unsafe_code)` kept
- EVIDENCE: `round-1/clippy.out`, `round-1/fmt.out`

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-tablewire-az/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-tablewire-az/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`, size >= 8192, distinct > 64
- EVIDENCE: `round-1/geo-typo-report.md`, `round-1/after.png`

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-tablewire-az/REPORT.md checks/gui-tablewire-az/round-1/after.png checks/gui-tablewire-az/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views.rs crates/gui/src/app.rs checks/gui-tablewire-az/GATES.md checks/gui-tablewire-az/REPORT.md checks/gui-tablewire-az/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed), sirf OWNS paths + caller line staged, working tree clean after commit, NO push
