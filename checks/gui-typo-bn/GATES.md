# Wave BN-TYPO — GATES (central type ramp tighten: header/tab/grid/hint, paint-neutral)

Scope: `crates/gui/src/theme.rs` + `checks/gui-typo-bn/**` ONLY (gutter.rs/home.rs
read-only: already Mono/Small-complete, no edit needed — ponytail smallest diff).
`views.rs` / `views_table.rs` / `tree.rs` / `app.rs` / `screenshot.rs` KO HAATH MAT
LAGANA (parallel waves chal rahi hain; bypass `.monospace()` call-sites sirf REPORT
me EXACT wiring spec — paint calls ko mat badlo). `checks/opencode-waves/STATUS.md`
MANA. Colors FROZEN (koi `Color32::from_rgb`/`from_rgba`/`Visuals::` add nahi).
Geometry NO-TOUCH (koi `add_space`/padding/width/Grid-structure change MANA —
sirf font/weight/size/line-height headless tokens). Sibling diffs KO HAATH NAHI
LAGANA. NO push (orchestrator pushes). Repo ke bahar koi copy/sandbox nahi.

WHY (live observation): baaki paint `.monospace()` direct calls par default size
me hai (folder grid paths 13 / sides 12 via tree.rs, sync badges, table cells) —
central ramp (Heading 18 strong / tab Body 13 medium-step / Mono 12 fixed /
Small 11 hints) adhoora naam tha: grid + hint/caption ke liye koi named central
alias nahi, `tree.rs` `TypoRole::Caption` precedent mangta hai jo exist nahi
karta. Ye wave central aliases define karti hai (paint-neutral, headless intent);
paint adoption agli wave me EXACT wiring spec se.

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: green, `0 failed` (recorded 606 passed, 0 failed)
- EVIDENCE: `round-1/baseline-tests.out` (pre-edit tail)

## G2 — OWNS-only diff, KEVL + STATUS + colors + geometry untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/theme.rs|checks/gui-typo-bn/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "views|views_table|screenshot|STATUS|app.rs|tree.rs|gutter.rs|home.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/theme.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::|add_space|set_width|desired_width|Grid::new)" || echo NO-COLOR-GEO-ADD`
- EXPECT: mera diff sirf `theme.rs` + `checks/gui-typo-bn/**`; KEVL-CLEAN;
  OWNS-CLEAN; NO-COLOR-GEO-ADD (koi naya point size nahi — aliases existing
  roles reuse karte hain)

## G3 — Grid + caption aliases land, ramp locked, unit-tested

- CHECK: `cargo test -p autocompare-gui typo`
- EXPECT: green, naya test `content_grid_and_caption_aliases_carry_ramp`:
  + `TypoRole::content_grid() == TypoRole::Mono` (size 12.0 == TYPO_MONO_SIZE,
    mono family, tabular, weight "regular-mono", line-height MONO multiplier)
  + `TypoRole::caption() == TypoRole::Small` (size 11.0 == TYPO_SMALL_SIZE,
    proportional chrome, weak+small, tabular counts, line-height CHROME multiplier)
  + Ramp order: caption 11 < grid 12 < tab 13 < card-title 14 < section 18
    (hint < grid < tab < card < header — BC-dense ladder)
  + `content_grid().spec().size == Mono.spec().size`,
    `caption().spec().size == Small.spec().size` (single-source delegation)
  + `tree.rs` ka manga `Caption` precedent ab central me defined hai
    (caption-regular = Small; caption-strong paint-side composition, follow-up)
- WHAT: `theme.rs` me 2 `const fn` aliases (pure/headless, koi `Ui`/`Context`
  nahi) + ramp ladder doc touch (4 poles: header strong-large / tab
  medium-step / grid mono-fixed / hints small). Koi naya size/color/geometry nahi.

## G4 — MS Rust guidelines + fmt clean (sirf OWNS file par fmt)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/theme.rs; echo fmt:$?`
  (sirf OWNS file; bare `cargo fmt` mat chalao taaki sibling files untouched rahen)
  + `awk '/^#\[cfg\(test\)\]/{intest=1} !intest' crates/gui/src/theme.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -55 crates/gui/src/theme.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0`, ZERO-BANNED in non-test code,
  `forbid(unsafe_code)` kept, naye helpers pure const fns

## G5 — Harness PASS + PNG magic-ok (paint-unchanged proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-typo-bn/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-typo-bn/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64 (headless-only wave → PNG bit-stable vs baseline)

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-typo-bn/REPORT.md checks/gui-typo-bn/round-1/after.png checks/gui-typo-bn/round-1/geo-typo-report.md`
  + `git add crates/gui/src/theme.rs checks/gui-typo-bn/GATES.md checks/gui-typo-bn/REPORT.md checks/gui-typo-bn/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
- REPORT (Hinglish-Roman ONLY, Devanagari MANA) must contain: family/weight/
  size/line-height table (har role), lines changed, follow-up (EXACT wiring spec:
  kaun si views.rs line kaun sa alias adopt karegi + tree.rs FOLDER_SIDE 12 vs
  Small 11 reconcile note).
