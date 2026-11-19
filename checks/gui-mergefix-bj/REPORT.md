# REPORT — Wave BJ-MERGEFIX (BI-M4 take-row fixture + BI-S4 sync dispatcher)

Scope: BI audit (`checks/gui-mergesync-bi/REPORT.md` §spec table) ke do P2 gaps ka fixture-side
fix — koi paint/model edit nahi, sirf `crates/gui/src/screenshot.rs` (fixture + dispatcher + tests)
aur `checks/gui-mergefix-bj/**`. Live take-row aur sync Apply-gate ka headless proof ab PNG me hai.

## Kya badla (screenshot.rs only, 738 insertions, 6 deletions)

1. **BI-M4 — merge fixture me take-row strip + decision line** (`render_merge_fixture_png`):
   rows ke neeche 24px strip (`Take:` + `Take Left/Right/Base/Both` boxes, live paint labels),
   uske neeche live `format_merge_hunk_title(0, 1)` title line aur live
   `format_merge_hunk_decision_with_pick(0, Some(Left))` decision line
   (`hunk 1: take left`). Status line ab `| hunk 1 of 1: take left` ke saath.
   Hunk data live `merge_conflict_hunks` se (marker model par 1 hunk), summary
   `merge_pick_summary` se (total 1, resolved 1, pending 0).
2. **BI-M4 — alag merge-take fixture route** (`render_merge_take_fixture_png`, naya):
   focused 1-hunk card — title + `merge_hunk_sides` ki teen sides (changed band) +
   take strip + decision + `apply_merge_picks` ka resolved output
   (`resolved (take left): let y = 20;`). Merge fixture se byte-alag scene.
3. **BI-S4 — dispatcher me `sync` + `merge-take` branch** (`render_fixture_png:82-120`):
   `merge-take` check `merge` se pehle (substring order), `sync` branch naya.
   AW2 picture-precedent (fallback identical twin) ab sync par lagu nahi — dedicated scene hai.
4. **BI-S4 — `render_sync_fixture_png` (naya)**: checklist rows + dry-run + confirm + Apply,
   sab live headless truth se — `SyncChecklist::from_plan` (2 CopyToRight steps + 1 skip),
   `summary`/`checked_count`/`copy_right_plan`, `sync_dry_run_summary`,
   `sync_apply_enabled(true, 2)`, row strings `glyph`/`display_line`/`action_label` se,
   skip line `views::sync_skip_text` se. Gate order live Sync-tab jaisa
   (summary -> rows -> skips -> note -> dry-run -> confirm -> dry-line -> count -> Apply).
5. **Sync tab chrome**: `paint_chrome` me `active=6` highlight box `(348,40,56,24)` +
   Sync label highlight; doc comment me route/active list update.

## Live-proof (koi mock string nahi)

- Take side labels paint wale hain (`Take Left/Right/Base/Both` — `views.rs` button strings).
- Decision `hunk 1: take left` live formatter ka output hai.
- Resolved line live `apply_merge_picks` ka output hai (`let y = 20;`).
- Sync counts/gate live functions ke return hain (`2 step(s) planned, 1 skipped`,
  `dry-run: 2 of 2 checked step(s) would copy left->right (no writes)`, gate open).

## Harness evidence (round-1/)

- `bash tools/gui-polish/run-geo-typo.sh` → EXIT 0, `RESULT: PASS`
  (PNG 9695B, magic ok, distinct 255; Grid x10, monospace x53, strong x23, small x50,
  `.size` sirf `['14.0']` — koi typo/color change nahi).
- `cargo test -p autocompare-gui` → 595 passed, 0 failed (`tests.out` tail).
- `cargo clippy --all-targets -- -D warnings` → zero warnings; `cargo fmt --check` → clean.
- PNGs (magic `89504e470d0a1a0a` sab me ok, sab >= 8192B, distinct > 64):
  - `after.png` (9695B, md5 `35d69828f03d`) — harness default (text scene)
  - `after-merge.png` (9963B, md5 `5396bf95b0fb`) — take-strip wali merge fixture
  - `after-merge-take.png` (8848B, md5 `bbdabeb8d71e`) — dedicated take scene (merge se alag)
  - `after-sync.png` (10732B, md5 `1d937616912d`) — dedicated sync scene (sab se alag)
- Naye unit tests: `merge_take_scene_proves_live_take_row`,
  `sync_scene_matches_checklist_gate_contract`, `dispatcher_routes_sync_scene`
  (twin-guard: sync/take purane 5 scenes se byte-alag; foldermerge==merge compat intact).

## Scope / DoD

- OWNS only: `crates/gui/src/screenshot.rs` + `checks/gui-mergefix-bj/**`.
  Working tree ke `app.rs`/`views.rs` diffs parallel agent ke hain — is commit me nahi.
- `views.rs`/`app.rs`/`tree.rs` me ZERO paint edit; `checks/opencode-waves/STATUS.md` untouched.
- Colors FROZEN — sirf frozen palette + chrome grays reuse, koi naya hue nahi.
- DoD: GATES.md ke G1–G6 sab green; commit local only, NO push.

## Files (sirf OWNS)

- `crates/gui/src/screenshot.rs` (dispatcher + merge strip + merge-take + sync + 3 tests)
- `checks/gui-mergefix-bj/GATES.md` (yeh gates)
- `checks/gui-mergefix-bj/REPORT.md` (yeh file)
- `checks/gui-mergefix-bj/round-1/` (`after.png`, `after-merge.png`, `after-merge-take.png`,
  `after-sync.png`, `geo-typo-report.md`, `tests.out`, `screenshot*.out`)
