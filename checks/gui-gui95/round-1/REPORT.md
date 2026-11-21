# REPORT: leaf-3.2 gui95 round-1 GUI verification

Base: 11aabc9. Worktree: /tmp/wt-gui95. Date: 2026-11-21.
Mode: checks-only (zero edits under crates/gui/**; polished theme/views untouched).

## Outcome

- G1 met: `cargo test -p autocompare-gui --lib` → 1163 passed, 0 failed.
- G2 met: `cargo test -p autocompare-gui --test real_egui_shot` → 2 passed (dark + light text-compare shots).
- G3 met: `cargo fmt -p autocompare-gui -- --check` → clean.
- G4 ABANDONED (see below), replaced by G7.
- G5 met: banned_scan.py → GUI_BANNED_OK, 0 non-test hits (12 test-only unwrap/expect/panic hits, all inside #[cfg(test)] mods).
- G6 met: owns_scan.py → GUI_OWNS_OK (only checks/gui-gui95/** untracked; crates/gui/** untouched).
- G7 met: clippy_debt.py → GUI_CLIPPY_DEBT_OK warnings=14 errors=0 (no growth vs base snapshot).
- Full suite (tests.out): 1163 lib + 2 egui shots green, exit 0.
- Total: 6 met, 0 unmet, 1 abandoned (G4, terminal handoff for polish wave, not a leaf blocker).

## Coverage verified (PLAN leaf-3.2:C10 spot-check, no code churn needed)

- Home/tabs: home_model, card geometry, tab strip label-fallback + overflow scroll (prior wave) — lib tests green.
- Gutter: gutter_geometry, minimap marker bands, split-handle math — lib tests green.
- Viewmodel: build_text_rows, filters (All/Diffs/Same/Context), diff-nav badge, status zones, cursor strip, prefs — lib tests green.
- Dark/Light: theme_contrast_ok + real_egui dark/light shots — green.

## Findings for next wave (not fixed, out of checks-only scope)

- R1: Pre-existing clippy debt at clean base 11aabc9: 14 lib warnings (doc-indent x7, too-many-args x3, unwrap_or_default x2, is_multiple_of x1, empty if-branch x1) + 75 test-target style hits under --all-targets (clippy lints apply to test code; no test-target allow config). Polish wave should fix or add test-target lint config. G4 abandoned for this reason.
- R2: Running `cargo test -p autocompare-gui` byte-regenerates checks/gui-sessionpaint-by/round-1/session-{2,8}tabs.png (kittest snapshots owned by another leaf). Reverted via `git checkout -- checks/gui-sessionpaint-by/` after every test run; that dir is clean at commit. Any future test run will re-dirty it — consider moving snapshot outputs under the owning leaf's control or .gitignore-ing regenerated PNGs.

## Files written (OWNS: crates/gui/** + checks/gui-gui95/** ONLY)

- checks/gui-gui95/round-1/GATES.md (ledger, G4 abandoned + G7 added with reason)
- checks/gui-gui95/round-1/REPORT.md (this file)
- checks/gui-gui95/round-1/tests.out (full `cargo test -p autocompare-gui` log)
- checks/gui-gui95/round-1/banned_scan.py (G5 oracle)
- checks/gui-gui95/round-1/owns_scan.py (G6 oracle)
- checks/gui-gui95/round-1/clippy_debt.py (G7 oracle)
