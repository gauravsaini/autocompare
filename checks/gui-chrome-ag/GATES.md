# Wave AG-CHROME — GATES (Gap #18: header align / line-height / chrome hierarchy)

OWNS: `crates/gui/src/theme.rs` + `crates/gui/src/views.rs` + `checks/gui-chrome-ag/**` ONLY.
KEVL `app.rs` / `viewmodel.rs` / `tree.rs` / `views_table.rs` / `views_hex.rs` /
`gutter.rs` / `screenshot.rs` mat chhoona (read-only, grep-proof only).
Colors FROZEN (koi hue/token change nahi — sirf `theme::*_for` reuse, koi
`Color32::from_rgb` / `Visuals::` add nahi). Geometry first, typography second.
NO push (orchestrator pushes). Dusre agent ki files kabhi mat chhoona.
`git stash/push` kabhi nahi.

Scope: Gap #18 (PARTIAL, wave-p-gaps REPORT row 18; local-run-l GAP-04/07/11).
Bahari observation apne shabdon me: path fields global header bar me hain jabki
diff panes neeche split hain (header bar pane-split se offset); status/count
lines mono me hain jabki chrome hierarchy proportional honi chahiye (flat chrome);
line-height paint me shared measured mono row par hai par intent document nahi.
Is wave ka delta:
- `theme.rs`: chrome hierarchy tokens + `TypoRole::is_chrome` /
  `chrome_tab` / `chrome_section` / `chrome_status` + header bar/lead tokens
  (size/weight/spacing only, koi hue nahi).
- `views.rs`: `header_bar_height` / `header_content_lead` /
  `text_row_height_intent` / `chrome_*_role` pure helpers + paint_header /
  section header / status lines ka TypoRole routing (weak tone same, mono
  content rows untouched).

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: green baseline (339 passed, 0 failed, recorded 2026-11-20, `main@f05f0f1`)
- EVIDENCE: `round-1/baseline-tests.out` (copied pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/(theme|views).rs|checks/gui-chrome-ag/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "app.rs|viewmodel.rs|tree.rs|views_table.rs|views_hex|gutter.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/theme.rs crates/gui/src/views.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::)" || echo NO-COLOR-ADD`
- EXPECT: only `theme.rs` + `views.rs` + `checks/gui-chrome-ag/**`;
  KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-ADD (sirf existing frozen palette reuse;
  `.weak()` tone same rehta hai, family mono->proportional sirf status chrome par)

## G3 — Chrome hierarchy unit-tested (headless)

- CHECK: `cargo test -p autocompare-gui chrome 2>&1 | tail -n 5`
  + `cargo test -p autocompare-gui header 2>&1 | tail -n 5`
  + `cargo test -p autocompare-gui text_row 2>&1 | tail -n 5`
- EXPECT: green, kam se kam ye naye tests:
  + `chrome_sizes_match_typo_scale` (TAB==BODY 13, SECTION==HEADING 18,
    STATUS==SMALL 11; koi naya scale value nahi, sirf named alias)
  + `chrome_roles_carry_hierarchy` (`is_chrome` chrome par true / Mono par
    false; tab Body default, section Heading strong, status Small weak;
    poori scale ordering SMALL < MONO < BODY < HOMETITLE < HEADING)
  + `header_bar_tokens_align_with_paging_bars` (HEADER_BAR_H == HEADER_H 28;
    HEADER_LEAD == CONTENT_LEAD "  ")
  + `header_helpers_use_shared_tokens` (views: `header_bar_height`,
    `header_content_lead`, `chrome_*_role`, `indented_content` lead same)
  + `text_row_height_intent_is_measured_mono` (views helper
    `TYPO_LINE_HEIGHT_SOURCE` return karta hai; multipliers headless intent,
    paint shared measured row hi rehta hai)
  + purane typo/zoom/geometry tests same values par pass (koi scale drift nahi)

## G4 — MS Rust guidelines + fmt clean (sirf OWNS files)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `cargo fmt -- crates/gui/src/theme.rs crates/gui/src/views.rs -- --check; echo fmt:$?`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/theme.rs crates/gui/src/views.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -20 crates/gui/src/theme.rs crates/gui/src/views.rs | grep -c "forbid(unsafe_code)"`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/theme.rs crates/gui/src/views.rs | grep -nE "\[[a-zA-Z_][a-zA-Z0-9_]*\s*\]" | grep -v "Vec\|String\|Option\|Result" || echo NO-INDEXING`
- EXPECT: `clippy:0` zero warnings, `fmt:0` (sirf OWNS files par),
  ZERO-BANNED in non-test code (`unwrap_or`/`unwrap_or_default` allowed —
  banned sirf `unwrap()`/`expect()` hain), `forbid(unsafe_code)` dono files me,
  naye helpers pure fns (koi indexing nahi, `.get()` only; saturating/finite
  guards jahan float hai; `format!` only allocation)
- NOTE: bare `cargo fmt` mat chalao — sibling files untouched rahen

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-chrome-ag/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-chrome-ag/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-chrome-ag/REPORT.md checks/gui-chrome-ag/round-1/after.png checks/gui-chrome-ag/round-1/geo-typo-report.md`
  + `git add crates/gui/src/theme.rs crates/gui/src/views.rs checks/gui-chrome-ag/GATES.md checks/gui-chrome-ag/REPORT.md checks/gui-chrome-ag/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
