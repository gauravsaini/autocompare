# Wave BS-TYPO — GATES (TypoRole alias paint adoption via gutter.rs+home.rs, views.rs NO-TOUCH)

Scope: `crates/gui/src/gutter.rs` + `crates/gui/src/home.rs` + `checks/gui-typo-bs/**`
ONLY (`theme.rs` read-mostly: BN aliases already landed, edit sirf zaroorat par —
ponytail smallest diff). `views.rs` / `views_table.rs` / `tree.rs` / `viewmodel.rs` /
`app.rs` / `screenshot.rs` KO HAATH MAT LAGANA (BR owns views.rs; bypass paint
call-sites sirf REPORT me EXACT wiring spec). `scripts/` + `checks/opencode-waves/
STATUS.md` MANA. Baaki `checks/` dirs NO-TOUCH. Colors FROZEN (koi
`Color32::from_rgb`/`from_rgba`/`Visuals::` add nahi). NO push (orchestrator pushes).
Repo ke bahar koi copy/sandbox nahi.

WHY (live observation): BN ne `TypoRole::content_grid()` (-> Mono 12) +
`caption()` (-> Small 11) central me define kiye, par paint-adjacent helpers ab bhi
direct roles/literals par hain: `gutter_text_role()` -> `Mono` (alias nahi),
`gutter_text_size()` -> `Mono.size()`, ruler/font calibration `TYPO_MONO_SIZE`
direct, ruler/segment consts literals (16.0/8.0/4.0), `HOME_CARD_TITLE_SIZE` literal
14.0, `HOME_CARD_BUTTON_BODY_SIZE` = `TYPO_BODY_SIZE` direct, card/pad/gap consts
literals (8.0/4.0/28.0), recent text/meta roles -> `HomeDesc` direct (caption alias
nahi). `views.rs` paint ALREADY in helpers se khata hai (`home_card_title_size()`,
`home_card_button_size()`, `recent_row_title_role().rich()`,
`recent_row_meta_role().rich()`, `gutter_ruler_width_for_font()` — views.rs:318,
334,499,515,1026), isliye gutter/home me alias adoption bina views.rs chhede paint
me flow hota hai. G-T7 (row leading) + G-C1 (tab/card padding) ka pixel tightening
views.rs paint literals me hai — is wave me sirf token single-sourcing (value-stable,
paint-neutral); pixel spec REPORT follow-up me.

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: green, `0 failed` (recorded 611 passed, 0 failed)
- EVIDENCE: `round-1/baseline-tests.out` (pre-edit tail)

## G2 — OWNS-only diff, KEVL + STATUS + colors + geometry-values untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/(theme|gutter|home).rs|checks/gui-typo-bs/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "views|views_table|screenshot|STATUS|app.rs|tree.rs|viewmodel|scripts/" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/ | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::)" || echo NO-COLOR-ADD`
  + `git diff -- checks/opencode-waves/STATUS.md | head -5; echo STATUS-UNTOUCHED`
- EXPECT: mera diff sirf `gutter.rs` + `home.rs` (+ `theme.rs` agar zaroori) +
  `checks/gui-typo-bs/**`; KEVL-CLEAN; OWNS-CLEAN; NO-COLOR-ADD; STATUS me zero diff.
  Geometry VALUES same (literals -> token reuse, koi number change nahi) taaki doosri
  waves ka paint parity na toote.

## G3 — Alias adoption lands, values locked, unit-tested

- CHECK: `cargo test -p autocompare-gui gutter 2>&1 | tail -n 3 && cargo test -p autocompare-gui home 2>&1 | tail -n 3 && cargo test -p autocompare-gui typo 2>&1 | tail -n 3`
- EXPECT: green. Locks:
  + `gutter_text_role() == TypoRole::content_grid()` (== Mono, size 12.0, mono family,
    tabular, weight "regular-mono", mono line-height)
  + `gutter_text_size() == TypoRole::content_grid().size()` (12.0, koi literal nahi)
  + `GUTTER_RULER_WIDE == SPACE_LG`, `GUTTER_RULER_NARROW == SPACE_SM`,
    `GUTTER_SEGMENT_MIN_H == SPACE_XS`, `GUTTER_HIT_SLOP == GUTTER_MERGE_GAP`
    (value-identical token reuse)
  + `HOME_CARD_TITLE_SIZE == HomeTitle.size()` (14.0), `HOME_CARD_BUTTON_BODY_SIZE ==
    HomeButton.size()` (13.0), `HOME_CARD_PAD/HOME_GRID_GAP/HOME_SECTION_GAP ==
    GEO_SECTION_GAP` (8.0), `HOME_CARD_INNER_GAP/HOME_PATH_ROW_GAP ==
    GEO_TIGHT_GAP` (4.0), `HOME_CARD_BUTTON_MIN_H/HOME_PATH_ROW_H ==
    CHROME_HEADER_BAR_H` (28.0, G-C1 hit-row rhythm)
  + `recent_row_text_role() == caption()` + `recent_row_meta_role() == caption()`
    (== HomeDesc == Small 11 weak proportional; purane `== HomeDesc` asserts bhi
    pass rahte hain kyunki Small == Small)
  + Naya/extended test: alias identity + token-equality locks gutter + home dono me.
    Koi naya point size/color/geometry-value nahi.

## G4 — MS Rust guidelines + fmt clean (sirf OWNS files par fmt)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/gutter.rs crates/gui/src/home.rs; echo fmt:$?`
  (sirf OWNS files; bare `cargo fmt` mat chalao taaki sibling files untouched rahen;
  `theme.rs` chheda to usko bhi check me jodo)
  + `awk '/^#\[cfg\(test\)\]/{intest=1} !intest' crates/gui/src/gutter.rs crates/gui/src/home.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -20 crates/gui/src/gutter.rs crates/gui/src/home.rs | grep -c "forbid(unsafe_code)"`
- EXPECT: `clippy:0` zero warnings, `fmt:0`, ZERO-BANNED in non-test code,
  `forbid(unsafe_code)` dono files me kept.

## G5 — Harness PASS + PNG magic-ok (value-stable proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-typo-bs/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-typo-bs/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64 (value-stable adoption -> paint behavior same).

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-typo-bs/REPORT.md checks/gui-typo-bs/round-1/after.png checks/gui-typo-bs/round-1/geo-typo-report.md`
  + `git add crates/gui/src/gutter.rs crates/gui/src/home.rs checks/gui-typo-bs/ && git commit -m "..." && git log --oneline -1 && git status --short`
  (theme.rs chheda to usko bhi stage karo; sibling untracked dirs stage mat karo)
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit (sibling untracked dirs
  chhede bina), NO push
- REPORT (Hinglish-Roman ONLY, Devanagari MANA) must contain: typo table
  before/after (family/weight/size/line-height per adopted role), exact lines changed,
  G-T7/G-C1 follow-up (EXACT views.rs wiring spec: kaun si line kaun sa alias adopt
  karegi + pixel tightening numbers).
