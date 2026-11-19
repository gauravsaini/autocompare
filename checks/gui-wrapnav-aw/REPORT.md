# Wave AW-WRAPNAV report (text wrap-continuation navigation parity)

## Scope
- OWNS: `crates/gui/src/views.rs` + `checks/gui-wrapnav-aw/**`.
- KEVL `gutter.rs` / `views_table.rs` / `views_hex.rs` / `tree.rs` / `app.rs` /
  `home.rs` / `viewmodel.rs` untouched. `gutter.rs` me sibling wave ka
  in-progress diff tha (beech me `ae79a1a` commit hua) — haath nahi lagaya.
  `checks/opencode-waves/STATUS.md` untouched (orchestrator-owned).
- Colors FROZEN: koi naya hue/token nahi — sirf row ke apne `fg/bg` +
  `span_text_format` / `find_segment_format` reuse.
- Geometry first (wrap row mapping, continuation offsets, viewport math),
  typography second (same mono `FontId`, koi naya size/weight nahi).

## Problem
- Wrap-on list har model row ko `wrap_pane_pair_count` sub-rows me paint
  karti thi, par current-row scroll hamesha row head (sub 0) par hota tha —
  lambi wrapped row me find match continuation (sub > 0) me ho to viewport
  match se door rehta tha.
- Find matches continuations me highlight to hote the (`wrap_chunk_spans`
  mapping), par kaunsa sub-row hit hai iska koi headless contract nahi tha —
  next/prev-match jump wrap rows ko flat visual offset par sahi map karne ka
  koi test-locked helper nahi tha.

## Delta (views.rs, +378/-3)
- Pure helpers (koi egui ctx nahi, koi indexing nahi `.get()` only, saturating arithmetic):
  + `wrap_find_sub_rows(line, needle, ignore_case, width)` — per-sub hit list;
    `find_line_spans` + `wrap_chunks` + `wrap_chunk_spans` intersect se, isliye
    continuations par hits sahi sub me track rehte hain (split matches dono
    sides me dikhte hain). Khali needle par khali.
  + `wrap_row_find_sub(row, needle, ignore_case, width)` — dono panes ka
    earliest hit sub; koi hit nahi par `None`.
  + `wrap_find_scroll_sub(row, needle, ignore_case, width, pair_count)` —
    clamped scroll target sub; bina hit/khali needle par 0 (row head).
  + `wrap_visual_offset(counts, vis_pos, sub)` — flat visual offset; paint ka
    `want_scroll` guard isi se (exact-sub validation).
  + `wrap_vis_pos_for_model(visible, model_row)` — jump mapping
    (wiring-pending `allow(dead_code)`, App jump wiring orchestrator ke paas).
  + `wrap_find_jump_visual(counts, visible, rows, model_row, needle, ignore_case, width)` —
    next/prev-match jump ka flat visual target: visible start + matched sub
    (clamped); miss par row head; filtered-out/OOB par `None` (wiring-pending).
  + `wrap_viewport_window(total, top, viewport)` — clamped viewport window
    (contract-only).
  + `wrap_sub_indent_chars(line, width, sub)` — continuation indent contract
    (contract-only).
- Paint wiring (wrap-on loop): har selected row ke liye `scroll_sub` nikalta hai
  (find active + needle non-empty par `wrap_find_scroll_sub`, warna 0) aur
  `want_scroll` exact sub par `wrap_visual_offset(...).is_some()` se guard hota
  hai — scroll head par nahi, hit wale sub par hota hai. Head + continuation
  dono branches me scroll check hai. Wrap-off path byte-identical hai.
- Typography: continuations plain mono, wahi `FontId` — koi naya `.size`,
  weight, ya token nahi (harness: `.size ['14.0']` unchanged, `monospace x67`).

## Tests (G3)
- Naye (8, sab `wrapnav_` prefix): head+continuation tracking, earliest-pane
  pick, scroll clamp, visual-offset/vis-pos mapping, jump-lands-on-continuation
  (miss par head, filtered-out/OOB None), viewport clamp, indent contract,
  continuation find-format mono-size lock.
- Self-review fix: 1 test assumption (`alpha` repeated fixture me do subs me
  milta hai — `vec![0]` se `contains(&0)` kiya); clippy `indexing_slicing`
  (test me `rows[i]` se `.get()` map par); `want_scroll` ko exact-sub guard
  (`wrap_visual_offset`) par; 4 wiring-pending/contract helpers par
  `allow(dead_code)` (repo precedent).
- Full: `cargo test -p autocompare-gui` 537 passed, 0 failed.

## Gates
- G1: full suite 537 passed / 0 failed (post-change; `round-1/tests.out`).
- G2 OWNS: tracked diff sirf `views.rs`; untracked sirf `checks/gui-wrapnav-aw/`;
  OWNS-CLEAN; NO-COLOR-ADD.
- G4 MS rust: clippy `--all-targets` zero warnings; `cargo fmt -p` check clean;
  non-test me ZERO unwrap/expect/panic/todo; `forbid(unsafe_code)` kept.
- G5 harness: `run-geo-typo.sh checks/gui-wrapnav-aw/round-1` RESULT PASS;
  after.png 9624 bytes, magic `89504e470d0a1a0a` ok, distinct 256.
- Geo signals: `add_space []`, `Grid::7`, `text_style_height x4`; typo: mono
  same, koi naya size nahi. Colors info-only, zero add.

## Commit
- Local only, no push. Sirf OWNS paths staged.
