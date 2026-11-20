# BY-SES-PAINT Report — session tab strip (views.rs)

 wave: BU-SES-PAINT. DEPENDENCY SATISFIED: MODEL (334dee5) + APP bridge
 (c950d03) landed; `home.rs` / `app.rs` READ-ONLY use hue, unme zero change.
 Mode wali `paint_tabs` untouched hai; nayi session patti uske UPAR aati hai.

## Paint fn table

| Fn / item | Kaam | Notes |
|---|---|---|
| `SESSION_TAB_MAX_W: f32 = 160.0` | Tab cell fixed width (clip ke saath) | `CARD_W` precedent jaisa named token, card se tang; magic number paint me nahi |
| `SessionAction` | `None` / `Switch(u64)` / `Close(u64)` | Per-tab hit + strip out-param ek hi enum (do same-shape enum nahi); caller `App` agle wave me is par switch/close chalayega |
| `session_tab_label(session)` | Title, blank/whitespace par `"{Kind} {id}"` fallback | Pure; paint isi se label leta hai; `session_title_for` fallback mirror |
| `session_strip_empty_hint()` | Khaali-list honest hint | Koi fake tab nahi: `"no open sessions: open any compare from Home"` |
| `session_strip_is_empty(list)` | Hint vs tabs branch ka gate | Pure |
| `session_tab_close_edge()` | Close-x square edge 28.0 | `path_browse_button_size` height reuse (120x28 floor), koi naya size nahi |
| `paint_session_tab(ui, session, is_active)` | Label (fixed max + truncate, single line) + chhota `x`; active par existing selectable style | Close pehle check hota hai; contract-only (`#[allow(dead_code)]`, App wiring agle wave ka hai) |
| `paint_session_tab_strip(ui, list, action)` | Poori patti: `ScrollArea::horizontal` + neeche `TAB_BOTTOM_GAP` | Khaali par hint; first-hit-wins; contract-only |

## Geometry tokens

- Rhythm: `TIGHT_GAP` (item spacing + tab-bottom gap se pehle ka spacing),
  `TAB_BOTTOM_GAP` (patti ke neeche).
- Tab cell: `SESSION_TAB_MAX_W` (160) x close edge (28); close `28x28`
  square browse-button floor se.
- Overflow: tabs sikudte nahi — `ScrollArea::horizontal` scroll karta hai.
  Math: ek cell `160 + 28 + 4 = 192`; 3 tabs `576 < 960` (fit), 8 tabs
  `1536 > 960` (scroll). Cap `HOME_RECENT_MAX` (8) par bhi strip scroll me
  rehta hai, active tab kabhi auto-close nahi hota (MODEL rule reuse).

## Typo

- Tab label: `chrome_tab_role` (Body 13 proportional) — diff/tab text mono
  nahi (round-1 mono-heavy note se carried).
- Khaali hint: `chrome_status_role` (Small weak proportional).

## Colors

- FROZEN: zero naya hue. Active tab + close-x existing selectable/button
  style reuse karte hain (`paint_tabs` wali highlight). Diff me
  `Color32::from_rgb` / `Visuals::` touch zero.

## Shot map (round-1)

| Shot | Scene | Bytes | Check |
|---|---|---|---|
| `after.png` | Live Default binary (strip wiring abhi App me nahi, isliye purana UI — no-regression proof) | 9765 | magic ok, >=8192, distinct 256 |
| `session-2tabs.png` | 2 live sessions (Text, Folder), pehla active | 245369 | magic ok, >=8192, distinct 256 |
| `session-8tabs.png` | 8 sessions (cap full), overflow arrow + scrollbar track | 245372 | magic ok, >=8192, distinct 256, 2-tab se alag bytes |
| `session-empty.png` | Khaali list, hint bar, koi fake tab nahi | 245316 | magic ok, >=8192, distinct 256 |

 Note: `screenshot.rs` is wave me NO-TOUCH tha, isliye scenes live
 binary se nahi aaye — test-painter (`views.rs` `#[cfg(test)]`, live
 `session_tab_label` + live geometry tokens se 960x600 canvas, frozen
 hues) se bane hain. Exact label text unit tests me lock hai; PNG geometry
 proof hai. Harness `RESULT: PASS` (round-1, repeat nahi laga).

## Overflow behavior note

- 4-5 tabs tak sab dikhte hain; uske baad right-end arrow + scrollbar
  track aata hai, tabs ka width fix rehta hai (clip via truncate).
- Switch click label par, close `x` par; ek frame me ek hit (first-wins).
- Khaali hone par patti gayab nahi hoti — hint line rehti hai taaki layout
  jump na kare (`TAB_BOTTOM_GAP` dono branches me).

## Gates

- `cargo test -p autocompare-gui`: 654 passed, 0 failed (naye: label
  fallback, action enum, strip empty gate, geometry reuse, overflow math,
  3-scene PNG proof).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: zero.
- `cargo fmt --check -p autocompare-gui`: clean.
- Banned strict (`unwrap(`/`expect(`/`panic!`/`todo!`/unsafe) non-test:
  zero. Colors diff me zero.
- `git status --short`: sirf `M crates/gui/src/views.rs` + untracked
  `checks/gui-sessionpaint-by/`. STATUS untouched.

## Follow-up

- App caller wiring (strip paint + `SessionAction` match + T5/T6 bridge)
  agle wave ka hai — is wave me paint fns contract-only hain.
- Future: `rename` hook (double-click rename) T7 ready hai, paint me nahi
  joda (scope bahar).
