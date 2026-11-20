# Gates: SYNCGUI — gui sync execution with mirror plus conflict nav

OWNS: crates/compare/**, crates/gui/src/tree.rs, checks/sync-gui/**

Scope: headless-first sync execution wiring — Update Left/Right/Both + Mirror, conflict nav, live log via compare ToolSpec, thin gui in tree.rs only, screenshot PNG plus edge fixture

- [x] G1: compare tests green (incl edge orphan/newer fixture + ToolSpec)
  CHECK: cargo test -p autocompare-compare 2>&1 | tail -5
  EXPECT: test result: ok
  EVIDENCE: shell=zsh cwd=/Users/ektasaini/autocompare exit=0; 11 suites ok (156 passed total, 0 failed), incl toolspec_gui_verdicts_match_engine_plans_on_orphan_newer_edge + toolspec_conflict_nav_and_live_log_cover_plan_and_report

- [x] G2: gui tests green (incl 4-mode + conflict nav + live log headless)
  CHECK: cargo test -p autocompare-gui 2>&1 | tail -5
  EXPECT: test result: ok
  EVIDENCE: shell=zsh cwd=/Users/ektasaini/autocompare exit=0; 787 passed, 0 failed, incl sync_preview_four_modes_cover_update_left_right + sync_conflict_nav_walks_preview_conflicts + sync_live_log_lines_cover_plan_and_report + session_conflict_cursor_and_live_log

- [x] G3: clippy zero warnings on owned crates
  CHECK: cargo clippy -p autocompare-compare -p autocompare-gui --all-targets -- -D warnings 2>&1 | tail -3
  EXPECT: Finished
  EVIDENCE: shell=zsh cwd=/Users/ektasaini/autocompare exit=0; Finished dev profile, no errors/warnings (fixed collapsible_if in sync_direction_for_row)

- [x] G4: fmt clean on owned files
  CHECK: cargo fmt --check 2>&1 | tail -3
  EXPECT: Finished
  EVIDENCE: shell=zsh cwd=/Users/ektasaini/autocompare exit=0; FMT_CLEAN (cargo fmt run over owned files, then --check clean)

- [x] G5: banned tokens zero in owned non-test code (comments, strings, test mods stripped; negative control in report)
  CHECK: python3 checks/sync-gui/round-1/check_banned.py
  EXPECT: BANNED_CLEAN
  EVIDENCE: shell=zsh cwd=/Users/ektasaini/autocompare exit=0; BANNED_CLEAN on sync.rs/lib.rs/tree.rs; negative control (dirty snippet) correctly FOUND tokens

- [x] G6: tree sync plan screenshot PNG exists and valid
  CHECK: sh -c 'python3 -c "from pathlib import Path; b=Path(\"checks/sync-gui/round-1/sync-tree-plan.png\").read_bytes(); assert b[:8]==bytes.fromhex(\"89504e470d0a1a0a\"), \"bad magic\"; assert len(b)>=8192, f\"too small {len(b)}\"; print(\"SCREENSHOT_OK\", len(b))"'
  EXPECT: SCREENSHOT_OK
  EVIDENCE: shell=zsh cwd=/Users/ektasaini/autocompare exit=0; SCREENSHOT_OK 11416 (PNG magic ok, distinct bytes 256)

- [x] G7: REPORT PASS recorded
  CHECK: sh -c 'grep -q "RESULT: PASS" checks/sync-gui/round-1/REPORT.md && echo REPORT_PASS'
  EXPECT: REPORT_PASS
  EVIDENCE: shell=zsh cwd=/Users/ektasaini/autocompare exit=0; REPORT_PASS
