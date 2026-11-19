# Wave U TYPO-EXQUISITE — GATES (write-first, one observable outcome per gate)

Scope: `crates/gui/src/theme.rs` + `checks/gui-typo-u/**` ONLY.
`views.rs` / `app.rs` read-only (role usage shape only).
Colors FROZEN, geometry FROZEN. No new deps.

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `231 passed; 0 failed`

## G2 — Exquisite tokens land, paint-neutral, no frozen-token drift

- CHECK: `git diff --stat && git diff crates/gui/src/theme.rs | grep -E "^[-+].*(SPACE_|GEO_|GUTTER_|Color32::from_rgb|DARK_|LIGHT_|DELETED_|INSERTED_|CHANGED_|ACCENT_|ERROR_)" || echo NO-FROZEN-DIFF`
- EXPECT: only `+` lines for new TYPO_* typo tokens + new `TypoRole` methods/tests;
  NO `-` lines on geometry consts, NO color-token lines, `rich()`/`rich_zoomed()` paint path unchanged
  (still `extra_letter_spacing(TYPO_LETTER_SPACING)` = 0.0; new per-role tracking is headless intent for future paint adoption).

## G3 — Zoom resolver still clamps every role MIN..MAX + new regression green

- CHECK: `cargo test -p autocompare-gui typo 2>&1 | tail -n 5 && cargo test -p autocompare-gui zoom 2>&1 | tail -n 5`
- EXPECT: all typo/zoom tests pass, including new
  `typo_exquisite_tokens_carry_hierarchy` + `typo_exquisite_zoom_clamps_every_role`
  (every role × every ZOOM_LEVELS × extreme ±100 stays within ZOOM_MIN_SIZE..ZOOM_MAX_SIZE;
  level 0 == base size).

## G4 — MS Rust guidelines + headless-first clean

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `cargo fmt -- --check; echo fmt:$?`
  + `grep -nE "unwrap|expect|panic|todo!" crates/gui/src/theme.rs | grep -v "^\s*//" | grep -v "#\[test\]" || echo ZERO-BANNED`
  (test module hits allowed only inside `#[cfg(test)]`; non-test code must print ZERO-BANNED)
- EXPECT: `clippy:0`, `fmt:0`, ZERO-BANNED in non-test code, `#![forbid(unsafe_code)]` kept.

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh checks/gui-typo-u/run-typo-u.sh checks/gui-typo-u/round-U-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-typo-u/round-U-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`, distinct > 64.

## G6 — Commit locally, NO push + REPORT done

- CHECK: `git log --oneline -1 && git status --short | head && git log origin/main..HEAD --oneline | head`
- EXPECT: one new `feat(gui): ...` English conventional commit on `main`,
  `checks/gui-typo-u/REPORT.md` present, working tree clean except commit,
  NO push (orchestrator verifies+pushes).
