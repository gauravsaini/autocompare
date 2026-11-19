# Wave AW-WRAPNAV — GATES (text wrap-continuation navigation parity)

OWNS: `crates/gui/src/views.rs` + `checks/gui-wrapnav-aw/**` ONLY.
KEVL: `gutter.rs` / `views_table.rs` / `views_hex.rs` / `tree.rs` / `app.rs` / `home.rs` / `viewmodel.rs` KO HAATH MAT LAGANA (parallel wave chal rahi hai).
`checks/opencode-waves/STATUS.md` KO HAATH MAT LAGANA (orchestrator-owned).
Colors FROZEN: koi Color32/Visuals/hue change nahi.
Geometry first (wrap row mapping, continuation offsets, viewport math), typography second (same mono FontId, koi naya size nahi).
MS rust guidelines: `forbid(unsafe_code)`, non-test code me unwrap/expect/panic/todo mana.
NO PUSH (orchestrator verifies then pushes). Conventional commit, English only.

Scope: text compare me wrap-continuation navigation parity — wrap mode me find
matches continuations par track rahen, next/prev-match jump wrap rows ko sahi
map kare, current-row scroll continuations ke saath consistent rahe.

## G1 — Green suite (post-change, full)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `537 passed; 0 failed` (8 naye `wrapnav_` tests samahit)
- EVIDENCE: `round-1/tests.out` (harness run, full suite green)
- NOTE: pre-edit baseline alag se record nahi ki — sibling gutter wave ne
  beech me commit kiya (`ae79a1a`); post-change full green + harness PASS
  evidence hai.

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/views.rs|checks/gui-wrapnav-aw/)" || echo OWNS-CLEAN`
  + `git diff -- crates/gui/src/views.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::)" || echo NO-COLOR-ADD`
- EXPECT: tracked diff sirf `views.rs`; untracked sirf `checks/gui-wrapnav-aw/`;
  OWNS-CLEAN; NO-COLOR-ADD (sirf existing `fg/bg` + `span_text_format` /
  `find_segment_format` reuse, koi naya hue/token nahi)
- EVIDENCE (recorded): `M crates/gui/src/views.rs`, `?? checks/gui-wrapnav-aw/`,
  OWNS-CLEAN, NO-COLOR-ADD.

## G3 — Wrap-continuation navigation parity, unit-tested

- CHECK: `cargo test -p autocompare-gui --bin autocompare-gui views::tests::wrapnav`
- EXPECT: green, ye 8 tests:
  + `wrapnav_find_sub_rows_tracks_head_and_continuation` (head sub 0 + continuation subs track, khali/missing needle empty, case flag)
  + `wrapnav_row_find_sub_picks_earliest_pane_hit` (left/right me earliest sub, dono hit par min, no-hit/empty None)
  + `wrapnav_scroll_sub_clamps_to_pair_height` (matched sub, pair height me clamped, no-hit par 0)
  + `wrapnav_visual_offset_and_vis_pos_map_jumps` (flat offset + vis-pos mapping, OOB None)
  + `wrapnav_jump_visual_lands_on_matched_continuation` (jump matched continuation par, miss par row head, filtered-out/OOB None)
  + `wrapnav_viewport_window_clamps_without_panic` (viewport clamp)
  + `wrapnav_sub_indent_matches_chunk_indent` (continuation indent contract)
  + `wrapnav_continuation_find_format_stays_mono_size` (continuation hits current-format me, same mono 12.0 size)
- WHAT: `views.rs` me pure fns (koi egui ctx nahi, koi indexing nahi `.get()` only, saturating arithmetic):
  + `wrap_find_sub_rows(line, needle, ignore_case, width) -> Vec<usize>` (per-sub hit list, `wrap_chunk_spans` intersect se)
  + `wrap_row_find_sub(row, needle, ignore_case, width) -> Option<usize>` (dono panes ka earliest hit sub)
  + `wrap_find_scroll_sub(row, needle, ignore_case, width, pair_count) -> usize` (clamped scroll target, no-hit par 0)
  + `wrap_visual_offset(counts, vis_pos, sub) -> Option<usize>` (flat offset; paint `want_scroll` guard isi se)
  + `wrap_vis_pos_for_model(visible, model_row) -> Option<usize>` (jump mapping; wiring-pending allow)
  + `wrap_find_jump_visual(counts, visible, rows, model_row, needle, ignore_case, width) -> Option<usize>` (jump target; wiring-pending allow)
  + `wrap_viewport_window(total, top, viewport) -> (usize, usize)` (clamped window; contract-only allow)
  + `wrap_sub_indent_chars(line, width, sub) -> usize` (indent contract; contract-only allow)
  + paint wiring: wrap-on loop me `scroll_sub` + `want_scroll` — matched row par hit wale sub par scroll, bina find par 0 (purana head behavior).

## G4 — MS Rust guidelines + fmt clean (sirf OWNS file) + clippy zero

- CHECK: `cargo clippy -p autocompare-gui --all-targets 2>&1 | tail -n 3` (expect zero warnings/errors)
  + `cargo fmt -p autocompare-gui -- --check; echo fmt-clean:$?` (expect 0; sirf `views.rs` me diffs the, `cargo fmt -p` ke baad clean — package me aur koi diff nahi tha)
  + non-test banned: tests module ke pehle wale hisse me `\.unwrap\(\)|\.expect\(|panic!|todo!|unimplemented!` zero (`unwrap_or`/`unwrap_or_default` allowed)
  + `head -20 crates/gui/src/views.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: clippy clean, fmt-clean:0, ZERO-BANNED, UNSAFE-FORBID-OK, naye helpers pure fns (koi indexing nahi, `.get()` only; saturating arithmetic)
- EVIDENCE (recorded): clippy `Finished` zero warnings; fmt-clean:0; banned zero; `forbid(unsafe_code)` kept.
- NOTE: bare `cargo fmt` nahi — `cargo fmt -p autocompare-gui` (package-scoped); check me sirf `views.rs` diffs the.

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-wrapnav-aw/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-wrapnav-aw/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`, size >= 8192, distinct > 64
- EVIDENCE (recorded round-1): `RESULT: PASS`; PNG 9624 bytes, magic `89504e470d0a1a0a` ok, distinct 256.
  Geo signals: `add_space []`, `Grid::7`, `text_style_height x4`, widths 24 calls;
  typo: `monospace x67`, `strong x19`, `small x80`, `.size ['14.0']` (koi naya size nahi), `row_height x4`;
  colors info-only 22 (koi add nahi — G2 NO-COLOR-ADD).

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-wrapnav-aw/REPORT.md checks/gui-wrapnav-aw/round-1/after.png checks/gui-wrapnav-aw/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views.rs checks/gui-wrapnav-aw/GATES.md checks/gui-wrapnav-aw/REPORT.md checks/gui-wrapnav-aw/round-1 && git commit -m "feat(gui): ..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`), sirf OWNS paths staged, working tree clean after commit, NO push
