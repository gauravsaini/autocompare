# GUI-SHELL P0 Shell Polish — REPORT (2026-11-20)

Mission: session toolbar, view toggles, honest status bar, empty-home guidance.
Scope: sirf owned files — `crates/gui/src/app.rs`, `theme.rs`, `screenshot.rs`, `main.rs`.

## Kya change hua

1. Session toolbar (`app.rs` ui):
   - `Swap Sides` button: active tab ke left/right paths swap karta, model clear, error clear, status me `swapped sides: <identity>`.
   - `Reload / Recompare` button: `run_active_tab()` dobara chalata, status me `comparing: <identity>`.
   - Dono Home par disabled rehte.
2. Tab titles carry session identity:
   - `session_identity()` — `left <--> right - Tab` format, empty side `(empty)` dikhta, kabhi jhooth nahi.
   - Har frame par `ViewportCommand::Title(window_title())` set hota; tabs ke upar identity label bhi paint hota.
3. View toggles wired + persist (`App` state):
   - `word_wrap`, `show_line_numbers`, `syntax_highlight` fields, `set_*` methods status ke saath.
   - Text panes par asar `view_text_for_paint()` se (owned file me, `views.rs` chhue bina):
     line-numbers off = numbers `None`, gutter 1; syntax off = accents clear; wrap on = pane 80 par cap.
   - Pure headless helpers: `effective_pane_chars()`, `view_text_for_paint()`, `text_kind_counts()`.
4. Status bar honest counts + har action par feedback:
   - `status_line()` har tab ke liye honest counts deta (text: equal/changed/deleted/inserted; folder: summary first line).
   - Compare start/done/fail, swap, toggle, drop, validation — sab `status` me likhte.
   - Neeche separator + status line hamesha paint hota.
5. Empty-home guidance:
   - Home par cards ke neeche hint: kaunsa session kholo, paths bharo, Compare dabao, Swap/Reload kya karta.
   - Har tab me `no <X> results yet` guidance jab model `None` hota — app kabhi blank nahi lagta.
6. Screenshot fixture (`screenshot.rs`):
   - Title ab session identity dikhata: `fixtures/left.rs <--> fixtures/right.rs - Text`.
   - Toolbar row add: `[Swap Sides] [Reload/Recompare] [x] Word Wrap [x] Line Numbers [x] Syntax`.
   - Status bar honest: `7 rows: 4 equal, 1 changed, 1 deleted, 1 inserted | text compare done`.

## Proof

Commands:
- `cargo build -p autocompare-gui`
- `./target/debug/autocompare-gui --screenshot checks/gui-shell/before.png` (change se pehle)
- `./target/debug/autocompare-gui --screenshot checks/gui-shell/after.png` (change ke baad)
- `cargo test -p autocompare-gui`
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`
- `rustfmt --edition 2024 --check` owned files par

Files:
- `checks/gui-shell/before.png` — size 9174 bytes, PNG magic OK
- `checks/gui-shell/after.png` — size 9846 bytes, PNG magic OK, before se DIFFERENT

Tests:
- `cargo test -p autocompare-gui`: 90 passed, 0 failed (is run me sibling agents ke 6 naye tests bhi shamil; shell ke 8 app tests: 2 purane + 6 naye — swap, toggles, view-prefs, identity, status-counts)

Clippy:
- Owned files (`app.rs`, `theme.rs`, `screenshot.rs`, `main.rs`) me zero clippy errors.
- Pura crate `clippy -D warnings` abhi fail hota — 21 errors sab `viewmodel.rs` / `tree.rs` me hai (TEXT/FOLDER agents ka WIP: dead `TextFilter`/`TextViewState`, collapsible-if). Owned scope ke bahar, isliye unhe chhua nahi.

Fmt:
- `rustfmt --check` owned files par clean (`app.rs`, `theme.rs`, `screenshot.rs` clean; `main.rs` ke through sirf sibling `viewmodel.rs` diff dikhta jo owned nahi).

MS rust rules:
- Owned files me `unsafe`, `unwrap()`/`expect()`, `panic!`/`todo!`/`unimplemented!`/`unreachable!` nahi; indexing/slicing nahi (sab `.get()`/`.first()`).

## Note (cross-agent)

- `views::paint_text` signature TEXT agent ke paas hai; full wrap/syntax ke liye wahan hook chahiye. Tab tak shell ne `view_text_for_paint()` clone-hook diya jo bina `views.rs` chhue asar dikhata. TEXT agent jab `paint_text(ui, model, &prefs)` jode to is helper ko seedha plug kar sakta.
