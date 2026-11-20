# REPORT — Wave BW-SES-APP (App bridge: sessions as source of truth)

## 1. Summary
`App.sessions: SessionList` field + bridge jud gaya. `current_tab` derived
hai (active kind, khaali=Home), `set_tab` compat hai (same-kind=switch, naya
kind=open), `open_home_session` live session kholta hai, T5/T6 sync switch par
paths load / type ke baad save karta hai. Full suite 648/648 green, clippy/fmt
clean, harness PASS, fixture md5 byte-same.

## 2. Bridge diagram (text)
- `set_tab(kind)` -> active same-kind ? `tab` sync : existing kind ?
  `switch_session(id)` : save-old + `sessions.open(kind, slot-paths)` + load.
- `switch_session(id)` -> save-old (T5/T6) + `sessions.switch(id)` + load-new
  + `tab` sync. Unknown id par false, kuch nahi badalta.
- `close_session(id)` -> (active ho to save) + `sessions.close(id)` ->
  naya active left-neighbour (na ho to right, model T2) + load + `tab` sync.
  Aakhri band par khaali, `current_tab`=Home.
- `open_home_session(tab)` -> save-old + `sessions.open(...)` + recent/history
  (pehle jaisa) + status `opened X` (pehle jaisa).
- Setters (`set_text/merge/folder_paths`), `apply_dropped_paths`,
  `swap_sides`, `reopen_session` (adopt), `stage_folder_double_click`
  (ab `set_tab` se) sab slots ko session me sync rakhte hain.
- Paint dispatch (`update` me `self.tab`, `paint_tabs`, `paint_home`) untouched
  — Wave 3 (views.rs) ka SessionAction wiring abhi baaki hai.

## 3. Compat table
| Caller | Pehle | Ab | Toota? |
|---|---|---|---|
| `current_tab()` | `self.tab` | derived (active kind / khaali=Home) | nahi (tests green) |
| `set_tab(kind)` | direct assign | switch-vs-open + save/load | nahi (`switching_tabs_preserves_paths` PASS) |
| `open_home_session` | tab+recent+history | + live `sessions.open`, status same | nahi |
| `reopen_session` | tab+slots+cursor | + adopt (same-kind update, warna open) | nahi (round-trip PASS) |
| `stage_folder_double_click` | `self.tab=target` | `set_tab(target)` | nahi (routing test PASS, yehi ek red tha, fix ho gaya) |
| paint (`&mut self.tab`) | direct | direct (unchanged) | nahi (fixture byte-same) |

## 4. Fixture stability note
`run-geo-typo.sh checks/gui-sessionapp-bw/round-1/`: RESULT: PASS (PNG 9765
bytes, magic ok, distinct 256). Determinism + stability: repeat run aur
`HEAD:app.rs` baseline se bana screenshot — teeno md5
`8ccf51e4824a03f84fb9e0d1b95fc521`, byte-same. Is wave me paint change nahi,
isliye stable rehna expected tha aur prove ho gaya.

## 5. Key metrics
- `cargo test -p autocompare-gui`: 648 passed, 0 failed (naye 6 bridge tests).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: zero.
- `cargo fmt --check -p autocompare-gui`: clean.
- Banned scan `rg 'unwrap\(|expect\(|panic!|todo!|unsafe' app.rs`: sirf
  `#![forbid(unsafe_code)]` line; non-test me zero (naya code `match` only).
- `git status --short`: sirf `M crates/gui/src/app.rs` + untracked
  `checks/gui-sessionapp-bw/` (STATUS untouched, `git push` nahi kiya).

## 6. Follow-up (Wave 3 + bahar)
- Mode strip / Home cards abhi bhi `&mut self.tab` direct likhte hain
  (legacy); unka SessionAction wiring (`paint_session_tab_strip`,
  Switch/Close) Wave BU-SES-PAINT me hoga.
- `Session.dirty` v1 me hamesha false rehta hai (compare ke baad stale flag
  Wave 4 ka kaam, PLAN section 3 ke mutabiq).
- `set_tab(Home)` sessions retain karke Home view dikhata hai; derived
  `current_tab` active kind deta hai — strip aane tak yehi contract hai.
