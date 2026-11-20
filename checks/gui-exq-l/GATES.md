# GATES — SWARM-EXQ-L (exquisite hex/table parity, round-1)

| Gate | Result | Evidence |
|---|---|---|
| `cargo test -p autocompare-gui` green | PASS — 682 passed, 0 failed (baseline 678 + 4 naye) | `checks/gui-exq-l/round-1/tests.out` tail (`test result: ok. 682 passed`) |
| `cargo clippy -p autocompare-gui --all-targets -- -D warnings` zero | PASS — final window clean | clippy exit 0, zero warnings |
| `cargo fmt -p autocompare-gui -- --check` clean | PASS | fmt exit 0, zero diff |
| Banned zero (unwrap/expect/panic/todo/unimplemented/unreachable/indexing_slicing/unsafe, non-test) | PASS | added-lines grep zero matches (sirf `token` shabd wale false-positive nahi) |
| Colors zero new | PASS — 29 -> 29 | `round-1/geo-typo-report.md` (`Color32/Visuals touches 29`), `git diff` added-lines me `Color32::from_rgb`/`Visuals::` zero |
| Harness `RESULT: PASS` | PASS round-1 | `round-1/geo-typo-report.md` (`RESULT: PASS`), `round-1/after.png` 9765 B valid PNG |
| Geometry-first parity | PASS — goto/sheet bars find/key-bar rhythm reuse | `HEX_GOTO_BAR_H == HEX_FIND_BAR_H`, `TABLE_SHEET_BAR_H == TABLE_KEY_BAR_H` unit tests |
| Typography-second parity | PASS — label Body > value Mono > badge Small, koi naya size/token nahi | zone-role tests (`exq_l_zone_roles_*`, `table_section_label_*`) |
| OWNS-only diff | PASS — `views_hex.rs`, `views_table.rs`, `viewmodel.rs` + `checks/gui-exq-l/` sirf | `git status --porcelain -- <own pathspec>` |
| Dirty-tree rule v3 | PROCEED — M files me OWNS ke bahar ke naam sirf sibling (reports/sessions/sessionpaint) + untracked agyx dirs; STATUS.md/PLAN.md clean | `git status --porcelain` |

Before/after PNG: `round-1/after.png` (default scene; naye bars wiring-pending hain isliye raster bit-stable 9765 B, hierarchy proof unit tests + wired paint precedent se).
