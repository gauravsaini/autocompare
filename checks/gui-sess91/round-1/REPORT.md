# REPORT: sess91 round-1 — sessions persistence hardening

Base: 094f684. OWNS: crates/sessions/**, checks/gui-sess91/**. No GUI touch. No push.

## Kya badla (code)

1. `crates/sessions/src/lib.rs`: naya `pub(crate) fn atomic_write` — sibling `.tmp` file + rename; rename/write fail par tmp safai.
2. `crates/sessions/src/session.rs`:
   - `clamp_active_index` helper; v1 direct-deserialize path aur v0 object migration me dangling `active_tab_index` 0 par clamp (khali tabs + index 7, 1 tab + index 5/99 ab `active_tab()` kabhi None nahi jab tab hai).
   - `save_to_file_force` ab `atomic_write` use karta hai.
   - 3 naye unit tests: v1 clamp, v0 clamp, atomic save me koi `.tmp` sibling nahi bachta + reload barabar.
3. `crates/sessions/src/workspace.rs`, `settings.rs`: `save_to_file_force` ab `atomic_write` use karta hai.
4. `crates/sessions/tests/*.rs`: sirf rustfmt import-reorder (toolchain drift), koi logic change nahi.

## Skipped (YAGNI)

- `to_json` me schema stamp: constructors/migrate/deserialize already hamesha current version dete hai, kuch karne ko nahi tha.
- Duplicate tab-id reject: spec me uniqueness requirement nahi; `find_tab` pehla match deta hai.
- Workspace `active_session` dangling check: file baad me ban sakti hai, lenient rehna sahi hai.

## Verification (dobara mapa gaya)

| Gate | Natija |
|---|---|
| sess91:G1 tests | 34 + 11 + 20 + 12 = 77 passed, 0 failed (`tests.out`, 108 lines) |
| sess91:G2 fmt | FMT_OK |
| sess91:G3 clippy `-D warnings` | CLIPPY_OK, 0 warnings |
| sess91:G4 banned (unwrap/expect/panic/todo/unimplemented/unreachable/unsafe, non-test) | BANNED_0; control fixture (unwrap) -> HITS 1 exit 1 |
| sess91:G5 owns | OWNS_OK; control stray (`crates/gui/__STRAY_PROBE__`) -> HITS 1 exit 1, probe hataya |

## Blockers

Koi nahi. `checks/gui-sessionpaint-by/**` ko chhua nahi (koi stray nahi).
