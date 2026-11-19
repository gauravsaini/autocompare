# Parity Cleanup — Wave J Agent A Report

Scope sirf owned files: `crates/gui/src/views_hex.rs`, `crates/gui/src/views_table.rs`,
`checks/parity-cleanup/**`. `app.rs`, `views.rs`, `main.rs`, `STATUS.md` ko chhua nahi.

## 1. Kaam (2 slice, 2 commit)

- Slice 1 (`10c8928`): hex wiring — `status_line` ab selection `selected_hunk` se
  validate karta hai; `from_comparison` engine hunk ko `chunk_run` se kaat-ta hai
  (`HEX_HUNK_LEN` cap + honest truncation); slice path cap check `is_over_cap` se.
- Slice 2 (ye commit): table wiring — `to_key_options` ab `to_compare_options` se
  banta hai (single source); baaki Table/slice/label helpers test-only hain to
  targeted `#[allow(dead_code)]` + justification comment ke saath rakhe hain.

## 2. Per-item (used-where ya allow-kyun)

| Item | Natija | File:line |
|---|---|---|
| `selected_hunk` | REAL USE — `status_line` guard me | views_hex.rs:326 |
| `chunk_run` | REAL USE — `from_comparison` se call | views_hex.rs:150,339 |
| `HEX_HUNK_LEN` | REAL USE — `chunk_run:350` me `min` cap | views_hex.rs:30,350 |
| `build_from_slices` | test-only allow — binary file engine + `from_comparison` chalata hai, pure slice path sirf unit tests ke liye | views_hex.rs:178 |
| `from_label` | test-only allow — selector typed enum (`all()`) chalata hai, label parse ka runtime caller nahi | views_hex.rs:78 |
| `is_over_cap` | pehle se live, no change (ab `build_from_slices` bhi isko call karta hai) | views_hex.rs:222 |
| `to_compare_options` | REAL USE — `to_key_options` isko call karta hai | views_table.rs:159,173 |
| `suggest_for` | test-only allow — file wiring me parsed `Table` kabhi nahi milta | views_table.rs:134 |
| `apply_suggestion` | test-only allow — upar wali wajah | views_table.rs:141 |
| `cells_equal` | test-only allow — runtime cell compare engine diff ke andar hota hai | views_table.rs:181 |
| `refresh` | test-only allow — file wiring `summarize` se summary banata hai | views_table.rs:190 |
| `total()` / `line()` | pehle se live (Wave I), no change | views_table.rs:52,61 |

Kuch remove nahi kiya — har item ke paas ya live caller hai ya tests + allow hai, stub kuch nahi.

## 3. Gates

- `cargo test -p autocompare-gui`: full crate **block hai** — sibling Wave J B mid-edit hai,
  `app.rs` (uncommitted, B ka) 4 missing paint fn mangta hai (`paint_sync_checklist`,
  `paint_picture_compare`, `paint_folder_merge_checklist`, `paint_audio_checklist`).
  B files ko chhua nahi. Owned files isolated harness (`/tmp/owned-gate`, real crate wale
  roots mirror karta hai) me **25/25 green**: baseline 22 (hex 12 + table 10) se +3
  (hex me 2 naye: overlong-hunk chunking, stale selection; table me 1 naya: key/compare
  options parity). Full-crate delta 172 → 175 (B green hote hi verify hoga).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: full crate B ki wajah se
  compile nahi hota; faithful-roots harness me owned files par **ZERO dead-code errors**
  (bache allows sirf test-only hain).
- `cargo fmt`: owned files clean (`rustfmt --check` green); `cargo fmt --check` ka ekmatra
  diff B wale `app.rs` me hai, usko chhua nahi.

## 4. 6 limitations (honest)

1. `suggest_for`/`apply_suggestion`/`refresh` runtime me abhi bhi test-only hain —
   file-driven wiring parsed `Table` deta nahi (Wave I limitation #1 abhi bhi lagu).
2. `build_from_slices`/`from_label` sirf unit-test fixture hain; binary file engine path use karta hai.
3. `from_comparison` ka 64B chunking defensive hai — engine contract par assert nahi karta.
4. Full-crate test/clippy gates sibling B ke green hone par dobara chalane hain; abhi isolated harness hi saboot hai.
5. Naye tests headless hain (paint-driven nahi) kyunki paint `views.rs` me hai jo scope se bahar hai.
6. Behavior change sirf ek: over-long/over-budget engine hunk ab honestly truncate report karta hai; paint flow me koi change nahi.
