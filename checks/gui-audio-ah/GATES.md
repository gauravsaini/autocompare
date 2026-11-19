# Wave AH-AUDIO — GATES (Gap #8 audio hissa: Audio tab action UI)

OWNS: `crates/gui/src/app.rs` + `checks/gui-audio-ah/**` ONLY.
KEVL `views.rs`/`viewmodel.rs`/`tree.rs`/`views_table.rs`/`views_hex.rs`/
`gutter.rs`/`theme.rs`/`screenshot.rs` mat chhoona (read-only, model
`AudioDiff`/`AudioChecklist` read use ok). Colors FROZEN (koi hue/token
change nahi — sirf existing `.weak()` tone + frozen palette reuse, koi
`Color32::from_rgb` / `Visuals::` add nahi). Geometry first, typography
second (sirf existing tokens: `TIGHT_GAP` / `SECTION_GAP` + small weak /
monospace labels). NO push (orchestrator pushes). Dusre agent ki files
kabhi mat chhoona. `git stash/push` kabhi nahi.

Scope: Gap #8 ka audio hissa. Model half landed hai: `compare_audio_files`
-> `AudioChecklist::from_diff` (`run_audio` background job + result wiring
+ `views::paint_audio_checklist` paint sab kaam karta hai). Is wave ka delta
sirf `app.rs` me hai — Audio tab action UI:
- Play affordance stub-labeled: `Play (stub: no playback)` button; click par
  sirf status line set hoti hai, koi real playback nahi (label me stub saaf).
- Waveform/summary display App state se: `AudioChecklist` par windows +
  first-diff fields (paint-honest summary strip), summary line + textual
  waveform bar `app.rs` paint me (KEVL `views.rs` untouched).
- Sab naye helpers pure + headless-tested; koi real audio IO/playback nahi.

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: green baseline (344 passed, 0 failed, recorded 2026-11-20, `main@9cc197a`)
- EVIDENCE: `round-1/baseline-tests.out` (copied pre-edit tail)

## G2 — OWNS-only diff, KEVL + colors untouched

- CHECK: `git status --short && git diff --stat`
  + `git diff --name-only | grep -vE "^(crates/gui/src/app.rs|checks/gui-audio-ah/)" || echo OWNS-CLEAN`
  + `git diff --name-only | grep -E "views.rs|viewmodel.rs|tree.rs|views_table.rs|views_hex|gutter.rs|theme.rs|screenshot.rs" && echo KEVL-TOUCHED || echo KEVL-CLEAN`
  + `git diff -- crates/gui/src/app.rs | grep -E "^\+.*(Color32::from_rgb|from_rgba|Visuals::)" || echo NO-COLOR-ADD`
- EXPECT: only `app.rs` + `checks/gui-audio-ah/**`; KEVL-CLEAN; OWNS-CLEAN;
  NO-COLOR-ADD (frozen palette reuse, koi naya hue/token nahi)

## G3 — Audio action UI unit-tested (headless)

- CHECK: `cargo test -p autocompare-gui audio` + full `cargo test -p autocompare-gui`
- EXPECT: green, kam se kam ye naye tests (sab `app.rs` me, headless):
  + `audio_play_stub_label_names_stub` (label me "stub" + "no playback" saaf)
  + `audio_play_stub_request_sets_status_only` (click sirf status set karta hai,
    model untouched, koi playback state nahi)
  + `audio_waveform_bar_*` (empty/total-zero/all-same/partial share; width cap;
    koi panic nahi)
  + `audio_action_summary_*` (None model par honest "no results" line;
    Some model par `summary()` reuse)
  + `audio_checklist_carries_windows_for_waveform` (`from_diff` windows_total /
    windows_differing / first_diff_at preserve karta hai taaki bar App state se bane)
  + purane audio tests same values par pass (compat)

## G4 — MS Rust guidelines + fmt clean (sirf OWNS file)

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/app.rs; echo fmt:$?`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/app.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -20 crates/gui/src/app.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0` zero warnings, `fmt:0` (sirf `app.rs` par), ZERO-BANNED in non-test code
  (`unwrap_or`/`unwrap_or_default` allowed — banned sirf `unwrap()`/`expect()` hain),
  `forbid(unsafe_code)` kept, naye helpers pure fns (koi indexing nahi, `.get()` only;
  saturating/finite guards; `format!` only allocation)
- NOTE: bare `cargo fmt` mat chalao — sibling files untouched rahen

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-audio-ah/round-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-audio-ah/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64

## G6 — REPORT + round-1 artifacts, commit locally, NO push

- CHECK: `ls checks/gui-audio-ah/REPORT.md checks/gui-audio-ah/round-1/after.png checks/gui-audio-ah/round-1/geo-typo-report.md`
  + `git add crates/gui/src/app.rs checks/gui-audio-ah/GATES.md checks/gui-audio-ah/REPORT.md checks/gui-audio-ah/round-1 && git commit -m "..." && git log --oneline -1 && git status --short`
- EXPECT: conventional English commit (`feat(gui): ...`, Hinglish body allowed),
  sirf OWNS paths staged, working tree clean after commit, NO push
