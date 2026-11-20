# GATES — gui-darkfix-ce (option A startup visuals apply)

Scope: `main.rs:52` me 1-2 line startup fix. `app.rs` / `theme.rs` / `screenshot.rs` READ-ONLY.
Root-cause (gui-darkroot-cc sec-5): `App.dark_mode=true` kabhi live ctx par apply nahi hota
(`_cc` ignore), isliye live light + fixtures dark.

## Gate 1 — startup visuals test

- `main.rs:52` closure `_cc` ignore NAHI karta; `cc.egui_ctx.set_visuals(egui::Visuals::dark())`
  call karta hai `App::default()` se PEHLE.
- First frame se `ui.visuals().dark_mode == true`, `App.dark_mode == true` se match.

## Gate 2 — live-vs-fixture alignment note

- Fixture renderer frozen-dark hai (screenshot.rs `BG_DARK`, ctx bypass).
- Fix ke baad live bhi dark khulta hai (Light-Mode OS par bhi) → live Default + fixtures align.
- Header label check: `dark_mode=true` par `"Light Mode"` tabhi jab ctx bhi dark ho (desync khatm).

## Gate 3 — VISIBLE change document (expected, risk note)

- EXPECTED VISIBLE CHANGE: live window light→dark khulega. Ye fix ka maksad hai, regression nahi.
- Follow-up (alag wave, option C): persist + fixture mode-param + live re-capture proof.
- Is wave me proof = harness `RESULT: PASS` + PNG valid + tests green.

## Gate 4 — hygiene (har test/gate se PEHLE)

- `git status --short` me sirf OWNS dirty: `crates/gui/src/main.rs`, `checks/gui-darkfix-ce/`.
- Sibling `views.rs` edit chal raha hai — uska fail mera nahi; mera harness/gate result
  sibling dirt se bigde to `git stash` nahi — sirf apne OWNS verify karke report me note karo.
- Commit pathspec-only (`git add crates/gui/src/main.rs checks/gui-darkfix-ce`), `git add -A` MANA.
- `STATUS.md` MANA. PUSH MANA.

## Gate 5 — rust/MS gates

- `cargo test -p autocompare-gui` green.
- `cargo clippy -p autocompare-gui` zero warnings.
- `cargo fmt --check` clean (ya `fmt` ke baad diff sirf OWNS).
- Banned zero: non-test me `unwrap`/`expect`/`panic!`/`todo!`/`unsafe` — naya zero
  (`grep -rn` se verify; fix line me koi banned token nahi).
- Colors zero new: naya palette MANA; sirf existing `egui::Visuals::dark()` API reuse
  (views.rs:452 wala pattern).
- PNG valid: harness report me magic ok + >=8192B + non-blank.
