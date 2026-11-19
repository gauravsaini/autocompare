# Wave O VERIFY Report

HEAD: 971197f | Date: 2026-11-20 | Scope: release + metrics, report-only

## Metrics (baseline vs ab)

| Metric | Baseline | Ab | Delta | Gate |
|---|---|---|---|---|
| release binary size | 11022272 B | 11022272 B | 0 B | PASS |
| release build time (cached) | - | 0.42s | n/a (cache hit) | info |
| screenshot bytes | 9914 B | 9914 B | 0 B | PASS |
| screenshot magic | PNG 89504e47 | PNG 89504e47 | same | PASS |
| screenshot distinct bytes | - | 256 | ok | PASS |
| screenshot dims | - | 960x600 RGBA | ok | PASS |
| geo-typo harness | - | PASS | - | PASS |
| workspace test suites ok | 43 | 43 | 0 | PASS |
| workspace tests failed | 0 | 0 | 0 | PASS |
| workspace tests passed (total) | - | 572 | - | info |
| gui tests passed | 204 | 204 | 0 | PASS |
| clippy -D warnings | clean | clean (exit 0) | - | PASS |
| fmt --check | clean | clean (exit 0) | - | PASS |
| xtask layers | ok | ok | - | PASS |
| xtask secrets | ok | ok | - | PASS |

Binary size delta exactly 0 B batata hai Wave N/O paint wiring se release artifact me koi size regression nahi. Screenshot byte-identical (9914 B) matlab paint output stable hai.

## Gates (sab PASS)

- build-release: PASS (0.42s cached, size 11022272 B)
- screenshot: PASS (after-O.png, 9914 B, magic ok, distinct 256)
- geo-typo harness: PASS (round-O-1/geo-typo-report.md RESULT: PASS)
- tests workspace: PASS (43 suites ok, 572 passed, 0 failed; gui 204)
- clippy: PASS (tail: Finished dev profile, exit 0)
- fmt: PASS (exit 0, no diff)
- xtask layers: PASS (layers: ok)
- xtask secrets: PASS (secrets: ok)

Artifacts: after-O.png + round-O-1/ (after.png, geo-typo-report.md, tests.out, screenshot.out)

## Next-wave proposal (Wave P)

1. Pager paint wiring ka visual proof: paged scroll states ke screenshots lo (page 1 vs page 2) taaki pager consume hone ka pixel evidence mile, sirf byte-stability nahi.
2. Gutter-jump + find-box ke liye headless interaction test add karo (row jump + cell find ka assert), kyunki abhi screenshot me ye paths exercise nahi hote.
3. Release build time ka cold-build number lo (cargo clean ke baad) taaki incremental cache number ke saath real build cost bhi recorded ho.
4. Binary size stable hai, to next focus runtime perf (frame time) par rakho, size par nahi.
