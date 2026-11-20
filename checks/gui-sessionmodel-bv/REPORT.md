# BV-SES-MODEL Report — headless session store (Wave 1)

PLAN: `checks/gui-sessionplan-bu/PLAN.md` section 3 + 4. OWNS: sirf
`crates/gui/src/home.rs` + `checks/gui-sessionmodel-bv/`. Headless wave hai —
paint/app wiring baad ki waves me hogi. Colors FROZEN (is wave me koi color
touch nahi). Harness: `round-1/` me `RESULT: PASS`.

## Struct table

| Struct / fn | Role | Precedent reuse |
|---|---|---|
| `Session` | Per-instance khula compare (kind se alag; do Text ek saath ok) | `SessionSnapshot` fields mirror (trim/stamp/age) |
| `Session::new(id, kind, l, r, base, cursor)` | Trim + title derive + dirty=false + stamp 0 | `SessionSnapshot::new` trim semantics |
| `Session::with_saved_at / age_secs / age_label` | Stamp + graceful age | `SessionSnapshot` wahi, `recent_age_label` share |
| `session_title_for(kind, left, right, id)` | Label: left (na ho to right) ka stem/base; dono khaali par `"{Kind} {id}"` | Naya chhota pure helper (paint Wave 3 isko reuse karegi) |
| `SessionList` | Live store: `sessions` + `active: Option<u64>` + `next_id` | Cap `HOME_RECENT_MAX` (8) history se share |
| Getters `get / active / active_id / list / len / is_empty` | Read-only paint bridge | `SessionHistory` getter shape |

Title edge: trailing slash/backslash kat-te hain, extension utarti hai
(`left.txt` -> `left`), dotfile bachta hai (`.gitignore` poora), sirf-slash
path fallback par girta hai. Label kabhi khaali nahi.

## Transition truth table (T1-T7)

| # | Call | Result |
|---|---|---|
| T1 | `open(kind!=Home, ...)` | `Some(id)` (id=next_id, active=id, dirty=false) |
| T1 | `open(Home, ...)` | `None` no-op, next_id consume nahi |
| T1 cap | len 8 par `open` | sabse purani INACTIVE giri, active bachi, len 8 |
| T2 | `close(id)` maujood | true; sirf wahi hati |
| T2 neighbour | active band (beech) | left-neighbour active |
| T2 neighbour | active band (pehli) | right-neighbour active |
| T2 neighbour | aakhri band | active=None (launcher view) |
| T2 | `close(unknown)` | false, kuch nahi badla |
| T3 | `switch(id)` maujood | true, active=id |
| T3 | `switch(unknown)` | false, active untouched |
| T4 | `close_others(id)` maujood | true, sirf wahi bachi, active=id |
| T4 | `close_others(unknown)` | false no-op |
| T5 | `update_paths(id, ...)` | true, teeno trim, title untouched |
| T5 | `update_paths(unknown, ...)` | false |
| T6 | `update_cursor(id, n)` | true, cursor=n |
| T6 | `update_cursor(unknown, n)` | false |
| T7 | `rename(id, blank)` | false, title untouched |
| T7 | `rename(id, "  x  ")` | true, title="x" |
| T7 | `rename(unknown, ...)` | false |

Id kabhi reuse nahi hoti (band ke baad bhi next_id badhta hai).

## Test count

- `cargo test -p autocompare-gui session`: 25 passed (8 naye `session_*`,
  baaki pehle se maujood session-naam wale).
- Full suite: 642 passed, 0 failed.
- Clippy `--all-targets -D warnings`: zero. fmt: clean.
- Banned scan non-test: zero (hits sirf test-mod + line 5 `forbid`).
- Harness round-1: PASS (1 round, PNG 9765B magic-ok).

## Follow-up (is wave me nahi)

1. APP bridge (Wave 2): `App.sessions`, derived `current_tab`, T5/T6 bridge,
   `set_tab` compat. Faisla pending nahi — spec PLAN section 4 me ready hai.
2. T5 title policy: abhi title sirf open/rename par banta hai; path sync par
   label purana rehta hai. Wave 2 me tay karna: sync par re-derive ya rename
   ko priority (v1 me rename future hook hai, isliye paths-only rakha).
3. `dirty` flag v1 me set-on-open-false hai; stale-mark caller Wave 2/3 me.
4. Sibling dirt note: tree me `app.rs` / `views.rs` / `checks/gui-pathmeta-bu/`
   pehle se modified/untracked the (doosri wave ka kaam) — is commit me
   pathspec-only stage, unko haath nahi lagaya. `git pull` ki zaroorat nahi
   padi (local origin/main se ahead, fetch me naya kuch nahi).
