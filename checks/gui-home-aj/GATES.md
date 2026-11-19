# Wave AJ-HOME — GATES (geometry first, typography second, colors frozen)

OWNS: `crates/gui/src/home.rs`, `crates/gui/src/app.rs`, `checks/gui-home-aj/**` ONLY.
KEVL (`views.rs`, `views_hex.rs`, `viewmodel.rs`, `tree.rs`, `views_table.rs`,
`gutter.rs`, `theme.rs`, `screenshot.rs`) read-only — paint adoption unke paas.
Colors FROZEN: koi `Color32`/`Visuals` edit nahi. No new deps. MS rust guidelines:
non-test code me `unsafe`/`unwrap`/`expect`/`panic!`/`todo!` mana hai.

Clean-room note: Beyond Compare sirf bahar se observe kiya (session launcher:
card grid + recent list + bada action button); sab contract apne shabdon me,
 paint tokens se derive kiya, koi copy-paste nahi.

## G1 — Baseline green (edit se pehle)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: sab pass, 0 failed
- EVIDENCE: 373 passed, 0 failed (6 naye: 5 home + 1 app)

## G2 — Home geometry contract headless (widths/heights/margins/padding/alignment)

- CHECK: `cargo test -p autocompare-gui home_ 2>&1 | tail -n 3`
- EXPECT: naye tests pass — `home_grid_columns_three`,
  `home_grid_rows_ceil_nine_to_three`, `home_grid_cell_row_major`,
  `home_card_min_height_fits_stack`, `home_recent_push_dedupes_and_caps`
- EVIDENCE: 10/10 `home_` + `home_session_open_records_recent` pass

## G3 — Typography second (TypoRole precedent reuse, koi naya size nahi)

- CHECK: `cargo test -p autocompare-gui home_card 2>&1 | tail -n 3`
- EXPECT: pass — title HomeTitle (14 strong proportional, tracking 0.02em,
  chrome lh 1.2), desc HomeDesc (11 small weak), button HomeButton (13 body);
  `home.rs` me koi `.size(` literal nahi, sab `TypoRole` se
- EVIDENCE: 4/4 `home_card*` pass; apne diff me 0 `.size(` literal, 0 color touch

## G4 — MS guidelines + clippy/fmt clean

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `cargo fmt -- home.rs app.rs` ke baad `cargo fmt --check` scope me clean
  + `grep -nE "unwrap|expect|panic!|todo!" crates/gui/src/home.rs crates/gui/src/app.rs | grep -v "cfg(test)" || echo ZERO-BANNED`
  + `#![forbid(unsafe_code)]` dono files me barkarar
- EXPECT: `clippy:0`, fmt clean, ZERO-BANNED (test module ke andar ki hits
  allowed nahi — tests me bhi banned tokens nahi honge)
- EVIDENCE: clippy exit 0; workspace `fmt --check` exit 0; apne diff me
  0 banned/unsafe + 0 Color32/Visuals; `forbid(unsafe_code)` barkarar

## G5 — Harness PASS + headless proof

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-home-aj/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-home-aj/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`, size >= 8192, distinct > 64
- EVIDENCE: harness exit 0, RESULT: PASS (9624B, magic ok, distinct 256)

## G6 — REPORT + commit, NO push

- CHECK: `git log --oneline -1 && git status --short | head && ls checks/gui-home-aj/REPORT.md checks/gui-home-aj/round-1/`
- EXPECT: ek naya `feat(gui): ...` English conventional commit (sirf OWNS paths
  staged), `REPORT.md` + `round-1/` maujood, NO push (orchestrator karega);
  `git stash` kabhi nahi, dusre agent ki files untouched
- EVIDENCE: commit `feat(gui): polish home session-start geometry and typography`;
  staged sirf OWNS paths; `views_hex.rs` (dusre agent) unstaged chhoda; no push
