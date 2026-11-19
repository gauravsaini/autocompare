# Wave AJ-HEXFIND — GATES (Gap #12 hex hissa: find box + Next/Prev + N/M count, paint)

OWNS: `crates/gui/src/views_hex.rs` + `checks/gui-hexfind-aj/**` ONLY.
KEVL `app.rs` / `views.rs` / `viewmodel.rs` / `views_table.rs` / `tree.rs` / `home.rs` / `gutter.rs` / `theme.rs` / `screenshot.rs` mat chhoona (read-only, grep-proof only; `app.rs` me `hex_find_needle` / `hex_find_cursor` pehle se hain — sirf semantics reference, koi edit nahi; hex find UI `views_hex` ke paint fn ke andar + wahi local state me rahe, caller signature same).
Colors FROZEN (koi hue change nahi — sirf existing small/weak typo, koi `Color32::from_rgb`/`from_rgba`/`Visuals::`/`SetVisuals`/`.size(` add nahi).
Geometry first, typography second. Clean-room: Beyond Compare bahar se observe only, apne shabdon me.
MS rust guidelines: `forbid(unsafe_code)` kept; non-test code me `unwrap`/`expect`/`panic`/`todo` mana hai (`.get()`/saturating/`unwrap_or` only).
NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo. `git stash/push` kabhi nahi.

Scope: Gap #12 ka hex hissa — hex window ke andar find row (table-find precedent `checks/gui-tablefind-ai/REPORT.md` reuse).
AE wave ka model (`hex_find_next`/`hex_find_prev` offset wrap, `hex_find_next/prev_position` grid hit, `hex_find_count` overlapping, `hex_find_count_label` N/M) verify-only, usme edit sirf tab jab test red ho.
Is wave ka delta paint layer hai, sab `views_hex.rs` me, local state ke saath:
`parse_hex_needle_text` (App `parse_hex_needle` ka views_hex-local mirror — views_hex app par depend nahi kar sakta, isliye same semantics yahin: separators hatakar hex-jode decode, warna raw utf-8),
`hex_find_match_offsets` (sab overlapping hits order me — N/M total + Next/Prev nav ke liye, `hex_find_count` ka poora-hit judvaan),
`paint_hex_find_bar(ui, needle: &mut String, badge: &str) -> (bool, bool)`
(table `paint_table_find_bar` ka geometry/typo mirror: bar `28.0` px, needle `120.0` px + hint, `small_button("◀ Prev")` / `small_button("Next ▶")`, badge right-end small weak + `4.0` pad; GEO `28.0/120.0/4.0` reuse, koi naya token/color nahi).
Paint fn caller ke locals (`&mut String` needle + badge `&str`) par chalti hai — koi App-field edit nahi; caller wiring (hex tab se call) KEVL hai isliye is scope se BAHAR (orchestrator/follow-up), nayi items file precedent `#[allow(dead_code)]` ke saath wiring-pending hain.

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `364 passed; 0 failed` (recorded 2026-11-20, `main@6ca1f98`, tablefind wave landed)
- EVIDENCE: `round-1/baseline-tests.out` (pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/views_hex.rs|checks/gui-hexfind-aj/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "app.rs|views.rs|viewmodel.rs|views_table.rs|tree.rs|home.rs|gutter.rs|theme.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/views_hex.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::|SetVisuals|\.size\()" || echo NO-COLOR-TYPO-ADD`
- EXPECT: only `views_hex.rs` + `checks/gui-hexfind-aj/**`;
  KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-TYPO-ADD

## G3 — Hex find + count kaam karta hai, unit-tested

- CHECK: `cargo test -p autocompare-gui hex_find`
- EXPECT: green, kam se kam ye tests:
  + `hex_find_wraps_forward_and_backward` (purana, same values pass)
  + `hex_find_positions_report_row_col_with_wrap` (purana AE, same values pass)
  + `hex_find_count_and_label_formats_n_of_m` (purana AE, same values pass)
  + `hex_find_needle_text_parses_hex_pairs_or_raw_bytes` (naya, views_hex.rs: `"14 1e"` hex decode, separators, odd/gair-hex raw fallback, khali khali)
  + `hex_find_match_offsets_collects_all_hits_in_order` (naya, views_hex.rs: overlapping order, khali needle/haystack khali, missing khali)
  + `hex_find_bar_geometry_reuses_table_precedent` (naya, views_hex.rs: `28.0/120.0/4.0` + badge label shape)
- WHAT: `views_hex.rs` me naye pure `parse_hex_needle_text(text: &str) -> Vec<u8>`
  (trim; `space/tab/,/;/_/-/:` skip; even-length sab-hex par jode-decode, warna raw utf-8; khali par khali; `.get()`/saturating only, koi indexing nahi)
  aur `hex_find_match_offsets(haystack: &[u8], needle: &[u8]) -> Vec<u64>`
  (ek-ek byte aage overlapping scan, har hit offset order me; khali/badi needle par khali; `hex_find_count` se `len()` barabar aata hai);
  nayi paint `paint_hex_find_bar(ui: &mut egui::Ui, needle: &mut String, badge: &str) -> (bool, bool)`
  (hex window ke andar find row: `horizontal_wrapped`, min-size `avail x 28.0`, `Find:` small weak label,
  needle `desired_width(120.0)` + hint `needle`, `small_button("◀ Prev")` / `small_button("Next ▶")`,
  badge right-end `small().weak()` + `4.0` pad; GEO table precedent reuse, koi naya token/color nahi;
  returns `(prev_clicked, next_clicked)` — cursor/badge advance caller ke locals me hota hai, App fields untouched);
  AE helpers verify-only (test red hua to hi edit)

## G4 — MS Rust guidelines + fmt clean (sirf OWNS files par fmt)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `cargo fmt -- crates/gui/src/views_hex.rs && rustfmt --edition 2024 --check crates/gui/src/views_hex.rs; echo fmt:$?`
  (sirf OWNS file; bare `cargo fmt` mat chalao taaki sibling files untouched rahen)
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views_hex.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -25 crates/gui/src/views_hex.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0`, ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naye helpers pure fns (`.get()`/saturating only, koi indexing/slicing nahi)

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-hexfind-aj/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-hexfind-aj/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64
  (paint fn caller-wired nahi hai is wave me — PNG bit-same rehna expected hai; harness headless proof hai)

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-hexfind-aj/REPORT.md checks/gui-hexfind-aj/round-1/after.png checks/gui-hexfind-aj/round-1/geo-typo-report.md`
  + `git add crates/gui/src/views_hex.rs checks/gui-hexfind-aj/GATES.md checks/gui-hexfind-aj/REPORT.md checks/gui-hexfind-aj/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
