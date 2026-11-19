# Wave BB-TABLEP34TYPO — GATES (P3 data-grid + P4 overlays + typo adoption, sequential)

OWNS: `crates/gui/src/views.rs` + `checks/gui-tablep34typo-bb/**` ONLY.
KEVL: `views_table.rs` / `views_hex.rs` / `theme.rs` / `tree.rs` / `screenshot.rs` / `app.rs` READ-ONLY (koi edit nahi — P3 signature change ka caller `paint_table_keyed` line 1-line `view` thread is `views.rs` ke andar hai).
`checks/opencode-waves/STATUS.md` KO HAATH MAT LAGANA (orchestrator-owned).
Sibling diffs KO HAATH NAHI LAGANA — stage sirf OWNS paths.
Colors FROZEN (koi naya hue/token nahi — koi `Color32::from_rgb`/`from_rgba`/`Visuals::`/`SetVisuals`/`.size(` add nahi).
NO push (orchestrator pushes). Repo ke bahar koi copy/sandbox nahi. `git stash/push` kabhi nahi.

Scope (isi order me, sequential):
1. P3 — AY REPORT §P3: `paint_table` data grid Grid ID + spacing swap, indexed data-cell loop + `paint_table_data_cell`, find-source switch `table_find_match_rows(&data_cells)` par, `view` threading.
2. P4 — AY REPORT §P4: key boxes `rect_stroke` + find underlines `line_segment`, paint-side measure, `views_table.rs` me koi change nahi.
3. Typo — TYPOTOKENS REPORT "views.rs EXACT wiring spec": `TypoRole::rich` / `resolved_font_id` swaps (line numbers old main se re-verify, file badal chuki thi). Judgment calls + boundary SKIP (spec khud defer karta hai).

## G1 — Baseline green (before P3 edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: green, `0 failed` (recorded 568 passed, 0 failed — HEAD `67c9915` par)
- EVIDENCE: `round-1/baseline-tests.out` (pre-edit tail; full run 569 = 568 + 1 naya)

## G2 — OWNS-only diff, KEVL + STATUS + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/views.rs|checks/gui-tablep34typo-bb/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "views_table.rs|views_hex.rs|screenshot|STATUS.md|app.rs|theme.rs|tree.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::|SetVisuals|\.size\()" || echo NO-COLOR-TYPO-ADD`
- EXPECT: mera diff sirf `views.rs` + `checks/gui-tablep34typo-bb/**`; KEVL-CLEAN; NO-COLOR-TYPO-ADD (`.size(` additions zero — purane `.size(` calls sirf hatae hain)
- EVIDENCE: `round-1/owns-status.out`

## G3 — P3 indexed loop + P4 overlays, unit-tested

- CHECK: `cargo test -p autocompare-gui tablep34`
- EXPECT: green, 1 naya test (`tablep34_data_index_map_skips_header_rows`: hdr-filtered fwd/rev maps); poora suite `569 passed, 0 failed` (baseline 568 + 1)
- WHAT: Grid ID `TABLE_CELL_GRID_ID`, spacing tokens, `paint_table_data_cell` indexed loop (grid coords `data_cells` par), find-source switch + `bring_into_view` reverse map, painter overlays (key boxes + underlines, clip-safe ScrollArea content se)
- EVIDENCE: `round-1/tablep34-tests.out`, `round-1/tests-tail.out`

## G4 — MS Rust guidelines + fmt clean (sirf OWNS file)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/views.rs; echo fmt:$?`
  (sirf OWNS file; bare `cargo fmt` mat chalao taaki sibling files untouched rahen)
  + `awk '/^#\[cfg\(test\)\]/{intest=1} !intest' crates/gui/src/views.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -30 crates/gui/src/views.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0`, ZERO-BANNED in non-test code, `forbid(unsafe_code)` kept
- EVIDENCE: `round-1/clippy.out`, `round-1/fmt.out`, `round-1/banned-non-test.out`

## G5 — Harness PASS + PNG magic-ok

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-tablep34typo-bb/round-1; echo harness:$?`
  + `file checks/gui-tablep34typo-bb/round-1/after.png`
  + `python3 -c "d=open('...after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` PNG 960x600, magic `89504e470d0a1a0a`, size >= 8192, distinct > 64
- NOTE (honest delta): PNG 9627B hai — baselines (9914B/9624B) se alag. Ye EXPECTED hai: typo adoption Mono 12px (runtime default 13px tha) + Small 11px (runtime default 9px tha) par laata hai — sizes ab TypoRole single-source hain, pixel-bit-stable nahi. Harness pixel-equality check nahi karta (sirf magic/size/distinct), isliye PASS sahi hai.
- EVIDENCE: `round-1/geo-typo-report.md`, `round-1/after.png`

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-tablep34typo-bb/REPORT.md checks/gui-tablep34typo-bb/round-1/after.png checks/gui-tablep34typo-bb/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views.rs checks/gui-tablep34typo-bb/GATES.md checks/gui-tablep34typo-bb/REPORT.md checks/gui-tablep34typo-bb/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed), sirf OWNS paths staged, NO push
