# BY-SES-PAINT Gates (CHECK / EXPECT)

Source: `checks/gui-sessionplan-bu/PLAN.md` section 5 + section 6 Wave
BU-SES-PAINT (line 187+), dir `bu` se `by` me map (sibling paint run se
takrao nahi; `bw` precedent jaisa).
DEPENDENCY SATISFIED: MODEL landed (commit 334dee5, `home.rs`
Session/SessionList+T1-T7) + APP bridge landed (commit c950d03, `App.sessions`).
`home.rs` / `app.rs` READ-ONLY use, unme change MANA.

## G1 — label fallback tests green

- CHECK `cargo test -p autocompare-gui session_tab_label -- --nocapture`
- EXPECT ok: title normal par stem, blank/whitespace title par
  `"{Kind} {id}"` fallback (kabhi khaali label nahi), kind fallback me sahi.

## G2 — action enum tests green

- CHECK `cargo test -p autocompare-gui session_action -- --nocapture`
- EXPECT ok: default `None` hai, `Switch(id)` / `Close(id)` id carry karte
  hain, `Switch(4) != Close(4)`.

## G3 — strip empty-list test green

- CHECK `cargo test -p autocompare-gui session_strip -- --nocapture`
- EXPECT ok: khaali `SessionList` par empty gate true (hint branch, koi fake
  tab nahi), 1 open par false; 8-tab overflow math (8 tabs > 960 canvas,
  3 tabs fit).

## G4 — full suite green

- CHECK `cargo test -p autocompare-gui`
- EXPECT 0 failed (baseline + naye paint-contract tests).

## G5 — clippy + fmt + banned + colors zero

- CHECK `cargo clippy -p autocompare-gui --all-targets -- -D warnings`
- EXPECT zero warnings.
- CHECK `cargo fmt --check -p autocompare-gui`
- EXPECT clean.
- CHECK `rg 'unwrap|expect\(|panic!|todo!|unsafe' crates/gui/src/views.rs`
- EXPECT sirf `#[cfg(test)]` block + pehle se maujood
  `#![forbid(unsafe_code)]` line; non-test code me zero.
- CHECK `git diff --stat` + `rg 'Color32::from_rgb|from_rgba|Visuals::'`
  (views.rs diff me)
- EXPECT paint diff me zero naya color (existing selectable style reuse).

## G6 — fixture proof (2-3 tabs + 8-tab overflow + khaali)

- CHECK `tools/gui-polish/run-geo-typo.sh checks/gui-sessionpaint-by/round-1/`
  + scene PNGs (`session-2tabs.png`, `session-8tabs.png`,
  `session-empty.png`, test-painter se; `screenshot.rs` frozen/NO-TOUCH
  isliye scenes live label+geometry tokens se headless bante hain)
- EXPECT harness `RESULT: PASS`; har scene PNG magic-ok + size>=8192 +
  distinct>64; 8-tab bytes 2-tab se alag (overflow scroll proof); REPORT me
  shot map.

## G7 — OWNS-only dirt

- CHECK `git status --short`
- EXPECT sirf `M crates/gui/src/views.rs` + untracked
  `checks/gui-sessionpaint-by/`. KEVL NO-TOUCH: `app.rs`, `home.rs`,
  `viewmodel.rs`, `views_table.rs`, `theme.rs`, `gutter.rs`, `tree.rs`,
  `screenshot.rs`, `checks/opencode-waves/STATUS.md` (MANA), baaki checks
  dirs, scripts/, tools/.

## G8 — harness loop cap

- EXPECT max 6 rounds; 2 round tak koi file change nahi ya 10 checks fail
  to `checks/inbox/` me escalate (skill rule).
