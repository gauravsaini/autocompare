# Wave P RELEASE Report (post-paint verify, report-only)

HEAD: `0b922ff` (docs(gaps): wave P recheck post-paint deltas)
Scope: `checks/wave-p-release/**` only. Zero source edits. STATUS.md untouched.

## 1. Metrics + deltas vs baseline

| Metric | Baseline | Actual (P) | Delta | Gate |
|---|---|---|---|---|
| release binary `target/release/autocompare-gui` | 11022272 B | 11056624 B | +34352 B (+0.31%) | PASS (build ok, size drift <1%) |
| release build wall time | — | 49.37s (`Finished release in 49.37s`) | info only | PASS |
| screenshot `after-P.png` bytes | 9914 B | 9914 B | 0 B exact match | PASS |
| screenshot magic | PNG `89504e470d0a1a0a` | `89504e470d0a1a0a` ok | — | PASS |
| screenshot geometry | 960x600 RGBA | 960x600 RGBA, 8-bit/color RGBA non-interlaced | — | PASS |
| screenshot distinct bytes | — | 256 (>64, not blank) | — | PASS |
| geo-typo round `round-P-1/after.png` | — | 9914 B, magic ok, distinct 256 | matches release png | PASS |
| `cargo test --workspace` suites | 43 suites 0 failed | 43 `test result: ok`, 0 failed | 0 | PASS |
| `cargo test` total passed | — | 579 passed, 0 failed | — | PASS |
| `gui` unittests | 211 | 211 passed, 0 failed | 0 | PASS |
| clippy `--all-targets -- -D warnings` | clean | clean (`Finished dev in 0.14s`, no warnings) | — | PASS |
| `cargo fmt --all --check` | clean | clean (exit 0, no diff) | — | PASS |
| `xtask layers` | ok | `layers: ok` | — | PASS |
| `xtask secrets` | ok | `secrets: ok` | — | PASS |

Artifacts:
- `checks/wave-p-release/after-P.png` (9914 B, sha256 prefix `2f4e26839bd08de2`)
- `checks/wave-p-release/round-P-1/after.png` (9914 B), `geo-typo-report.md` (RESULT: PASS), `screenshot.out` (empty), `tests.out`

Geo-typo signals (round-P-1):
- `RESULT: PASS`
- geometry: `add_space []`, `Grid::1`, `text_style_height x3`, 12 width calls
- typography: `monospace x33`, `strong x6`, `small x37`, `.size [14.0]`, `row_height x3`
- colors: `Color32/Visuals touches 22` (info only, out of scope)

## 2. Gates summary

- BUILD: PASS
- SCREENSHOT: PASS (byte-identical to 9914B baseline)
- GEO-TYPO: PASS
- TEST: PASS (43/43 ok, gui 211/211)
- CLIPPY: PASS
- FMT: PASS
- LAYERS: PASS
- SECRETS: PASS
- SCOPE: PASS (only `checks/wave-p-release/**`, no source, no STATUS.md, no push)

Overall: **ALL GREEN — release-ready post-paint.**

## 3. Next-wave proposal (Wave Q)

Report-only, no code changed isliye fix list sirf proposal hai, owner hint ke saath:

1. GUI: release binary +0.31% drift track karo; agar next paint me >1% badhe to bloat check (owner GUI).
2. PARITY: gaprecheck OPEN list se top-1 parity gap ko ek wave me close karo, BCompare se re-verify (owner PARITY).
3. GEO: `add_space []` signal hai — spacing abhi constants me nahi dikh raha, geometry tokens audit karo (owner GEO).
4. TYPO: `.size [14.0]` single value hai — scale/row-height matrix ek round me confirm karo (owner TYPO).
5. RELEASE: screenshot byte-identical hai (good), par flaky-guard ke liye 2-run stability check add karo (owner RELEASE).

Risk: koi nahi, sab gates green. Push nahi kiya, local commit only.
