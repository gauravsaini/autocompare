# exq-verify87 round-1 — verification REPORT (verify-only, koi code change nahi)

Base: `482d28c`. Scope OWNS: sirf `checks/gui-exq-verify87/**`. `crates/**` aur `tools/**` me ZERO edits.
Bhasha Hinglish-Roman hai, Devanagari use nahi hui.

## 1. Workspace tests (full `cargo test --workspace`)

- EXIT=0, 50 suites `test result: ok`, total 1786 passed, 0 failed.
- Raw output harness run se pehle `/tmp/workspace-tests-verify87.out` me bachaya tha (sha256 `40ea1df9...d4cf70`), REPORT me counts darj hain.
- Note: harness ne `tests.out` ko gui-only output se overwrite kiya; workspace counts upar wali line me safe hain.

## 2. Harness (`tools/exquisite/run.sh checks/gui-exq-verify87/round-1/`)

- harness-exit=0, `RESULT: PASS`.
- GUI tests (harness wala `tests.out`): 1162 passed, 0 failed.
- PNG (harness wala `after.png`, is REPORT ke baad ONLY-rule ke tahat hataya): 10946 bytes (min 8192 ok), magic ok (`89504e470d0a1a0a`), distinct bytes 256 (non-blank ok), sha256 `1c982b7e...a9fdedae`.
- `screenshot.out`: binary stdout, 2 lines (local-run notice + font families notice), koi error nahi.
- Exquisite signals (harness `exquisite-report.md` se, file hatane se pehle copy):
  - GEOMETRY: SPACE tokens [LG, MD, SM, XL, XS], margin/gap consts x776, Grid x16, row-h x380, width calls x85, gutter x2106.
  - TYPO: TypoRole x1194, monospace x26, strong x45, .size ['14.0'], weights [MEDIUM, REGULAR, SEMIBOLD], line-heights [BODY, CHROME, MONO, SOURCE], spacing [TYPO_LETTER_SPACING, extra_letter_spacing].
  - COLORS frozen: Color32/Visuals touches 42 (info-only, verify wave me koi color change nahi).

## 3. fmt check (report-only, fix MANA)

- `cargo fmt --check` exit=1. 4 diffs, sab `crates/sessions/**` me (lib.rs + tests corrupt_input.rs, migration.rs, round_trip.rs — import ordering).
- Ye pre-existing hai base par, verify scope se bahar hai, isliye fix nahi kiya.

## 4. Banned scan (report-only, fix MANA)

- Non-test code me `unwrap(`/`.expect(`/`panic!`/`todo!`/`unimplemented!`/`unreachable!` hits: 0.
- Method: har file me `mod tests {` boundary ke pehle wali lines scan ki; `crates/cli/src/bcomp.rs` jaise matches sab boundary ke baad (test mods) nikle.

## 5. Strays + file list

- Strays check: `git status` me `checks/gui-sessionpaint-by/**` aur `crates/sessions/**` me zero entries — revert ki zaroorat nahi padi.
- Round dir me ONLY: GATES.md (step-1 ledger) + REPORT.md + tests.out (harness gui-only) + screenshot.out. `after.png` aur `exquisite-report.md` ka data upar inline karke hataya.

## Verdict

- V1 GATES-first PASS, V2 workspace green PASS (1786/0), V3 harness PASS, V4 fmt FAIL-pre-existing (report-only), V5 banned 0 PASS, V6 outputs filed PASS, V7 commit no-push (neeche), V8 done-file (neeche).
- Koi code change nahi, koi push nahi.
