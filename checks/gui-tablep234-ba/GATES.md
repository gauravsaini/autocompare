# Wave BA-TABLEP234 — GATES (P2 header-grid: Grid ID + spacing + glyph single-source)

OWNS: `crates/gui/src/views.rs` + `checks/gui-tablep234-ba/**` ONLY.
KEVL: `views_table.rs` read-only (tokens wahi se — koi edit nahi), `views_hex.rs` / `app.rs` / `viewmodel.rs` / `screenshot.rs` KO HAATH MAT LAGANA.
`checks/opencode-waves/STATUS.md` KO HAATH MAT LAGANA (orchestrator-owned).
Sibling diffs (jaise `theme.rs` typotokens wave) KO HAATH MAT LAGANA — stage sirf OWNS paths.
Colors FROZEN (koi naya hue/token nahi — koi `Color32::from_rgb`/`from_rgba`/`Visuals::`/`SetVisuals` add nahi, koi `.size(` nahi).
P2-only: header Grid 3-token swap; cells loop + rule line untouched; P1/P3/P4 OPEN (orchestrator ke paas).
NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo. Repo ke bahar koi copy/sandbox nahi. `git stash/push` kabhi nahi.

Scope: AY spec §P2 (`checks/gui-tablecell-ay/REPORT.md` §"Agle views.rs wave ke liye EXACT wiring patch spec", P2 bullet) — `paint_table` header Grid (`views.rs:1977-1994`):
`egui::Grid::new("table_header_grid")` → `views_table::TABLE_CELL_HEADER_GRID_ID`;
`.spacing([SECTION_GAP, TIGHT_GAP])` → `.spacing([views_table::TABLE_GRID_COL_GAP, views_table::TABLE_GRID_ROW_GAP])` (same values 8.0/4.0);
glyph `CONTENT_LEAD` → `views_table::TABLE_CELL_GLYPH_PAD` (same `"  "`).
`.num_columns(table_grid_columns(&widths))` rakha (shape `table_cell_grid_columns` jaisa); cells loop (strong mono header) + rule line untouched.

## G1 — Baseline green (before P2 edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: green, `0 failed` (recorded 565 passed, 0 failed — sibling waves ke 6 tests samet)
- EVIDENCE: `round-1/baseline-tests.out` (pre-edit tail)

## G2 — OWNS-only diff, KEVL + STATUS + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/views.rs|checks/gui-tablep234-ba/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "views_table.rs|views_hex.rs|screenshot|STATUS.md" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::|SetVisuals|\.size\()" || echo NO-COLOR-TYPO-ADD`
- EXPECT: mera diff sirf `views.rs` header-grid 3-token swap + 1 test aur `checks/gui-tablep234-ba/**` hai; `views_table.rs` diff zero (read-only); `STATUS.md` untouched; stage sirf OWNS paths (sibling `theme.rs` diff stage mat karo); KEVL-CLEAN; NO-COLOR-TYPO-ADD
- EVIDENCE: `round-1/owns-diff.out`

## G3 — P2 single-source, unit-tested

- CHECK: `cargo test -p autocompare-gui tablep234`
- EXPECT: green, 1 naya test (`tablep234_header_grid_tokens_single_source`: header Grid ID literal + COL_GAP==SECTION_GAP + ROW_GAP==TIGHT_GAP + GLYPH_PAD==CONTENT_LEAD=="  ")
- WHAT: `views.rs` header Grid me 3-token swap (values same, single-source); `.num_columns` / cells loop / rule line untouched; poora suite `566 passed, 0 failed` (baseline 565 + 1)
- EVIDENCE: `round-1/tablep234-tests.out`, `round-1/tests-tail.out`

## G4 — MS Rust guidelines + fmt clean (sirf OWNS file)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/views.rs; echo fmt:$?`
  (sirf OWNS file; bare `cargo fmt` mat chalao taaki sibling files untouched rahen)
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -25 crates/gui/src/views.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0`, ZERO-BANNED in non-test code, `forbid(unsafe_code)` kept
- EVIDENCE: `round-1/clippy.out`, `round-1/fmt.out`

## G5 — Harness PASS + PNG magic-ok (paint-same proof: values same, IDs single-source)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-tablep234-ba/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-tablep234-ba/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`, size >= 8192, distinct > 64
- EVIDENCE: `round-1/geo-typo-report.md`, `round-1/after.png`

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-tablep234-ba/REPORT.md checks/gui-tablep234-ba/round-1/after.png checks/gui-tablep234-ba/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views.rs checks/gui-tablep234-ba/GATES.md checks/gui-tablep234-ba/REPORT.md checks/gui-tablep234-ba/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed), sirf OWNS paths staged, NO push
