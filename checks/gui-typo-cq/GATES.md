# Wave CQ-TYPO — GATES (gutter-text central alias, paint-neutral)

Scope: `crates/gui/src/theme.rs` + `checks/gui-typo-cq/**` ONLY.
`views.rs` / `views_table.rs` / `tree.rs` / `gutter.rs` / `home.rs` / `app.rs` /
`screenshot.rs` KO HAATH MAT LAGANA (parallel waves chal rahi hain).
`checks/opencode-waves/STATUS.md` MANA. Colors FROZEN (koi naya
`Color32` / `Visuals` touch nahi). Geometry FROZEN (koi margin/padding/width/
alignment change nahi — sirf headless typo alias + docs + test). Sibling diffs
KO HAATH NAHI LAGANA. NO push (orchestrator pushes). Repo ke bahar koi
copy/sandbox nahi.

WHY (live observation): BC process live hai (PID 23316, observe-ONLY — koi
click/edit nahi kiya). BC window screen par visible nahi tha (foreground me
Codex app tha), isliye hierarchy match recorded BC-dense ladder se kiya:
titles (strong-large) > body > status, aur gutter text diff-body mono me.
Central ramp me gutter pole ka koi named alias nahi tha — `gutter.rs` me local
`gutter_text_role()` helper hai jo `content_grid()` par bana hai, lekin
`theme.rs` me gutter ka koi first-class naam nahi. Ye wave wahi gap band
karti hai (paint-neutral, headless intent); paint adoption agli wave me.

## G1 — Baseline green (koi edit se pehle)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: green, `0 failed` (recorded 670 passed, 0 failed)
- EVIDENCE: pre-edit harness run (`round-1/before.png` + `round-1/tests.out`)

## G2 — OWNS-only diff, STATUS + colors + geometry untouched

- CHECK: `git status --short && git diff --name-only`
  + `git diff --name-only | grep -vE "^(crates/gui/src/theme.rs|checks/gui-typo-cq/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -iE "STATUS" || echo STATUS-UNTOUCHED`
  + `git diff -- crates/gui/src/theme.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::|add_space|set_width|desired_width|Grid::new)" || echo NO-COLOR-GEO-ADD`
- EXPECT: mera diff sirf `theme.rs` (+47/-1) + `checks/gui-typo-cq/**`;
  OWNS-CLEAN; STATUS-UNTOUCHED; NO-COLOR-GEO-ADD (koi naya size/color/
  geometry nahi — alias existing Mono role reuse karta hai)

## G3 — gutter_text alias lands, ramp locked, unit-tested

- CHECK: `cargo test -p autocompare-gui gutter_text`
- EXPECT: green, naya test `gutter_text_alias_matches_grid_mono`:
  + `TypoRole::gutter_text() == TypoRole::Mono == TypoRole::content_grid()`
    (size 12.0 == TYPO_MONO_SIZE, mono family, tabular, weight
    "regular-mono", line-height MONO multiplier, font_id monospace(12.0))
  + `gutter_text().spec()` ka size + font_id `Mono.spec()` se match
    (single-source delegation)
  + Ladder hold: caption 11 < gutter_text/grid 12 (BC ladder ka gutter pole)
  + Full suite: 671 passed (670 + 1 naya), 0 failed
- WHAT: `theme.rs` me 1 `const fn` alias (pure/headless, koi `Ui`/`Context`
  nahi) + ramp ladder doc touch (gutter pole naam se). Koi naya size/color/
  geometry nahi.

## G4 — Rust guidelines + fmt clean (sirf OWNS file par fmt)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/theme.rs; echo fmt:$?`
  (sirf OWNS file; bare `cargo fmt` mat chalao taaki sibling files untouched rahen)
  + `awk '/^#\[cfg\(test\)\]/{intest=1} !intest' crates/gui/src/theme.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -55 crates/gui/src/theme.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings (repo-wide, koi sibling diff nahi hai is
  baar), `fmt:0`, ZERO-BANNED non-test code me, `forbid(unsafe_code)` kept,
  naya helper pure const fn

## G5 — Harness PASS + PNG magic-ok + before/after byte-identical (paint-unchanged proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-typo-cq/round-1; echo harness:$?`
  + `shasum -a 256 checks/gui-typo-cq/round-1/before.png checks/gui-typo-cq/round-1/after.png`
  + `file checks/gui-typo-cq/round-1/before.png checks/gui-typo-cq/round-1/after.png`
- EXPECT: harness `RESULT: PASS`, dono PNG magic `89504e470d0a1a0a` (960x600
  RGBA), size >= 8192, distinct > 64, aur before/after sha256 IDENTICAL
  (headless-only wave: additive const fn + docs + test; `rich()` / `size()` /
  `text_style()` unchanged, isliye paint behavior same)

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-typo-cq/REPORT.md checks/gui-typo-cq/GATES.md checks/gui-typo-cq/round-1/before.png checks/gui-typo-cq/round-1/after.png checks/gui-typo-cq/round-1/geo-typo-report.md`
  + `git add crates/gui/src/theme.rs checks/gui-typo-cq && git commit -m "feat(gui): exquisite typography pass family/weight/line-height/spacing (CQ-TYPO)" && git log --oneline -1 && git status --short`
  + `ps aux | grep "[a]utocompare-gui" || echo NO-LIVE-PROCESS`
- EXPECT: conventional English commit message, sirf OWNS pathspec staged
  (kabhi `git add -A` nahi), working tree clean after commit, NO push,
  koi live process nahi chhoda
- REPORT (Hinglish-Roman ONLY, Devanagari MANA) must contain: family/weight/
  size/line-height table (har role), lines changed, fixture PNGs ka haal
  (byte-identical expected), follow-up EXACT wiring spec
  (`gutter::gutter_text_role` kab central `gutter_text()` par delegate karega).
