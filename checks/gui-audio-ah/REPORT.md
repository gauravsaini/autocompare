# Wave AH-AUDIO — REPORT (Gap #8 audio hissa: Audio tab action UI)

OWNS: `crates/gui/src/app.rs` + `checks/gui-audio-ah/**` ONLY.
KEVL files untouched (sibling `splitpaint-ah` wave ke uncommitted
`views.rs` changes working tree me the — unko haath nahi lagaya; mera
commit sirf OWNS paths stage karta hai).

## Delta (sirf `app.rs`)

- `AudioChecklist` par 3 naye fields: `windows_total`, `windows_differing`,
  `first_diff_at` — `from_diff` engine diff se bharta hai, taaki waveform
  strip App state se bane (paint-side guess nahi).
- Naye pure helpers: `AUDIO_WAVEFORM_WIDTH` (40), `audio_play_stub_label()`
  (`Play (stub: no playback)` — stub label me saaf), `audio_waveform_bar()`
  (differing share `#` over `-`; zero-total honest all-`-`; clamp + finite
  guards), `audio_first_diff_text()` (non-finite/missing par `-`).
- Naye App methods: `audio_action_summary()` (model verdict, warna honest
  no-results note), `request_audio_play_stub()` (sirf status line, model
  untouched, koi playback path nahi).
- Audio tab paint: checklist ke neeche `TIGHT_GAP` + separator, phir summary
  line (small weak), waveform strip (monospace small weak), first-diff line
  (small weak), aur stub Play button (model na ho to disabled). Koi naya
  color/token nahi — sirf existing geo/typo tokens.

## G1 — Baseline green

- `round-1/baseline-tests.out`: 344 passed, 0 failed (`main@9cc197a`).

## G2 — OWNS-only commit, KEVL + colors untouched

- `git add` sirf `crates/gui/src/app.rs` + `checks/gui-audio-ah/**`.
- KEVL grep: mere diff me `views.rs`/`viewmodel.rs`/`tree.rs`/`views_table.rs`/
  `views_hex`/`gutter.rs`/`theme.rs`/`screenshot.rs` nahi hain.
- Color grep (`Color32::from_rgb|from_rgba|Visuals::`): NO-COLOR-ADD.
- NOTE: sibling wave ke uncommitted `views.rs` + `checks/gui-splitpaint-ah/`
  changes commit ke baad bhi working tree me rahenge (unki files, unka commit).

## G3 — Tests green

- `cargo test -p autocompare-gui`: 355 passed, 0 failed (344 baseline + 6
  naye audio action-UI tests + 5 sibling-wave tests jo beech me land hue;
  plan commit `63e8051` docs-only tha, count delta sibling ke uncommitted
  `views.rs` tests se aaya jo harness run me sath chale).
- Naye tests: `audio_checklist_carries_windows_for_waveform`,
  `audio_play_stub_label_names_stub`, `audio_play_stub_request_sets_status_only`,
  `audio_waveform_bar_covers_empty_and_shares`, `audio_first_diff_text_names_stamp_or_dash`,
  `audio_action_summary_follows_app_state`. Purane audio tests same values par pass.

## G4 — Guidelines + fmt

- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: exit 0, zero warnings.
- `rustfmt --edition 2024 --check crates/gui/src/app.rs`: exit 0.
- Banned grep (non-test `unwrap(|expect(|panic!|todo!|unimplemented!|unreachable!`): ZERO-BANNED.
- `forbid(unsafe_code)` kept. Naye helpers pure fns (`.get()`/saturating/finite guards).

## G5 — Harness PASS

- `zsh tools/gui-polish/run-geo-typo.sh checks/gui-audio-ah/round-1`: exit 0,
  `RESULT: PASS`; `after.png` 9624 bytes, magic `89504e470d0a1a0a`, distinct 256.

## G6 — Commit, NO push

- Conventional English commit, sirf OWNS paths, NO push (orchestrator pushes).
