# P0-1 Session Tabs — Architecture Plan (BU-PLAN)

## 1. Summary

Round-1 REPORT (checks/gui-localrun-bs/REPORT.md P0-1) ke mutabiq: BC ek window me
multiple closable sessions rakhta hai (har open compare apne tab me, har tab par
close-x), jabki hamare paas single active `Tab` + 9 mode tabs hain. Ye planning-only
wave hai: crates me ZERO edits, sirf ye dir. Neeche design (model structs + state
transitions + paint fns) aur 3 disjoint follow-up wave specs hain.

Scope note: geometry + typography only. Colors FROZEN — is plan me koi palette spec
nahi, sirf section 7 me note hai. MS RUST N/A (is wave me koi code nahi).

## 2. BC behavior (apne shabdon me, observe-only)

BC ki window ke upar ek session wali patti hai, mode wali patti se alag:

- Har khula compare ek alag tab banta hai; label me mostly file/folder naam dikhta
  hai, poora path nahi (jagah bachane ke liye chhota naam).
- Har tab ke kone me chhota close-x hai; beech-click ya x se sirf wahi session
  band hota hai, baaki khule rehte hain.
- Ek waqt me ek hi session active dikhta hai (highlight alag), click se switch hota
  hai; switch par us session ke paths + cursor wapas aate hain (state alag rakhi
  lagti hai, global overwrite nahi).
- Naya compare kholne par nayi tab judti hai; aakhri tab band karne par khaali
  state (koi crash nahi, wapas launcher jaisa view).
- Bahut tabs khulne par patti scroll hoti hai (tabs sikudte nahi, overflow me teer
  ya scroll lagta hai).

Hamara current (crates/gui/src/app.rs, home.rs, views.rs, read-only study):

- `Tab` enum (home.rs:10) sirf session KIND hai (Home/Text/Folder/... — 10 variants),
  instance nahi. `App.tab: Tab` single active kind hai.
- Paths per-KIND hain, per-INSTANCE nahi: `text: Paths`, `folder: Paths`, ... (9
  fixed slots). Do Text compares ek saath khul hi nahi sakte.
- `paint_tabs` (views.rs:531) 10 fixed mode buttons paint karta hai, session strip
  nahi; close-x / switch / overflow ka koi concept nahi.
- `SessionSnapshot` + `SessionHistory` (home.rs:339/418) sirf recent-list record hai
  (reopen ke liye), live multi-session store nahi; cap HOME_RECENT_MAX=8 share karte
  hain.

## 3. Architecture — model structs

Sab structs headless + pure honi chahiye (paint sirf read-only dikhaye), existing
style (`RecentSessions`/`SessionHistory` precedent) par:

```rust
// home.rs (Wave 1 OWNS) — per-session instance, kind se alag
pub struct Session {
    pub id: u64,        // ever-increasing, reuse kabhi nahi
    pub kind: Tab,      // session type (Text/Folder/...; Home kabhi session nahi)
    pub title: String,  // tab label: chhota naam (file stem ya folder base), trimmed
    pub left: String,   // trimmed paths; three-way par base alag
    pub right: String,
    pub base: String,   // two-path kinds par khaali
    pub cursor: usize,  // us kind ka find/scroll cursor
    pub dirty: bool,    // compare ke baad model stale hua ya nahi (optional v1)
    pub saved_at_secs: u64, // 0 = unknown (SessionSnapshot precedent)
}

// home.rs — live multi-session store (history se alag: ye live hai, wo record hai)
pub struct SessionList {
    sessions: Vec<Session>,
    active: Option<u64>,  // active session id; None = khaali (launcher view)
    next_id: u64,         // id generator; 0 se start, kabhi reuse nahi
}
```

App-side (Wave 2 OWNS, app.rs):

```rust
// App me naya field; purane per-kind Paths/model fields migration tak rehte hain
pub struct App {
    // ...
    pub sessions: home::SessionList,
    // legacy: pub tab: Tab  -> derived banega: active session ka kind,
    //         khaali par Home. Direct field access dheere-dheere hatana hai.
}
```

Reuse decisions (ponytail — naya code minimum):

- `Session::new` trimming + `with_saved_at` + `age_label` wahi precedent reuse kare
  jo `SessionSnapshot` me hai; duplicate age logic nahi.
- `SessionList` cap `HOME_RECENT_MAX` reuse kare (8); 9th open par sabse purani
  INACTIVE session gire, active kabhi auto-close nahi.
- Title derivation: `left` path ka file-stem/folder-base; dono khaali par
  `"{Kind} N"` fallback (kabhi khaali label nahi).
- Per-kind models (text_model, folder_model, ...) v1 me per-KIND hi rahenge; sirf
  paths + cursor per-SESSION honge. Full per-session model Wave-4 (future) ka kaam
  hai — is plan ke 3 waves me nahi.

## 4. Architecture — state transitions

`SessionList` par pure methods (headless-tested, koi panic nahi, out-of-range
graceful):

| # | Transition | Input | Rule |
|---|---|---|---|
| T1 | `open(kind, left, right, base)` | kind != Home | naya Session id=next_id, active=id, dirty=false; Home par no-op false. Cap full par oldest inactive drop. |
| T2 | `close(id)` | session id | sirf wahi hate; active band hua to naya active = left-neighbour (na ho to right-neighbour); list khaali par active=None. Unknown id par no-op false. |
| T3 | `switch(id)` | session id | active=id; unknown id par no-op false. Paths/cursor paint active se padhe. |
| T4 | `close_others(id)` | session id | sirf wahi rahe, active=id; unknown id par no-op. (v1 me optional — Wave 2 decide kare.) |
| T5 | `update_paths(id, left, right, base)` | trimmed | active session ke paths sync (compare/type ke baad caller chalaye). |
| T6 | `update_cursor(id, cursor)` | usize | switch se pehle caller purana cursor save kare, switch ke baad naya lagaye. |
| T7 | `rename(id, title)` | non-blank | blank trim-khaali par no-op; future double-click rename ke liye hook. |

App-level mapping (Wave 2):

- `App::current_tab()` derived: active session ka kind, khaali par `Tab::Home`.
- `App::set_tab(kind)` legacy compat: same-kind session ho to switch, na ho to
  khaali paths se open (purane callers nahi tootenge).
- `open_home_session(tab)` ab `sessions.open(...)` chalaye + recent/history record
  pehle jaisa (dono systems parallel chalenge migration tak).
- Switch par per-kind `Paths` struct me active session ke paths load hon (paint ke
  `paint_paths(&mut left, &mut right)` signature bina tode), type ke baad wapas
  session me save hon (T5). Ye bridge Wave 2 ka core hai.

## 5. Architecture — paint fns (views.rs, Wave 3 OWNS)

Existing `paint_tabs` (mode patti, 10 fixed buttons) REHTI hai; uske UPAR nayi
session patti aati hai. Naye fns:

- `paint_session_tab_strip(ui, list: &SessionList, action: &mut SessionAction)` —
  poori patti: `horizontal` + `ScrollArea::horizontal` (overflow scroll, tabs sikudte
  nahi). Khaali list par khaali launcher hint (koi fake tab nahi). Geometry tokens
  reuse (TIGHT_GAP rhythm, TAB_BOTTOM_GAP neeche).
- `paint_session_tab(ui, session: &Session, is_active: bool) -> TabHit` — single
  tab: label (title, fixed max width + clip, do-line kabhi nahi) + close-x button
  (`"x"`, chhota hit box). Active par highlight (existing selected-value style,
  koi naya color nahi). Return enum: `None | Switch | Close`.
- `session_tab_label(session) -> String` — pure helper: title ya fallback
  `"{Kind} {id}"`; paint isi se label le (logic paint me nahi).
- `SESSION_TAB_MAX_W: f32` const — token (existing CARD_W precedent jaisa), magic
  number paint me nahi. Close-x size existing browse-button size token reuse.
- `SessionAction` enum (views.rs ya viewmodel.rs — Wave 3 decide kare, ek jagah):
  `None | Switch(u64) | Close(u64)`. Paint sirf action emit kare, mutate App
  caller me ho (egui borrow pattern: `paint_paths` precedent).

Chrome typography: tab label `chrome_tab_role` (Body 13 proportional) — diff/tab
text mono nahi (mono-heavy UI note round-1 se carried: chrome proportional rahe).

## 6. Follow-up wave specs (3 disjoint)

### Wave BU-SES-MODEL — headless session store

- OWNS: `crates/gui/src/home.rs`, `checks/gui-sessionmodel-bu/**`.
- KEVL NO-TOUCH: `crates/gui/src/app.rs`, `views.rs`, `viewmodel.rs`,
  `screenshot.rs`, baaki checks dirs, scripts/, tools/, STATUS.
- Kaam: section 3 ke `Session` + `SessionList` + T1-T7 transitions + title
  derivation + cap rule; `SessionSnapshot` precedent reuse (trim/stamp/age).
- GATES (CHECK / EXPECT):
  - CHECK `cargo test -p autocompare-gui session` / EXPECT green (naye tests:
    open/close/switch/neighbour/cap/drop-oldest/unknown-id no-op/blank rename
    no-op/Home never-session/title fallback).
  - CHECK `cargo clippy -p autocompare-gui -- -D warnings` / EXPECT zero.
  - CHECK `cargo fmt --check -p autocompare-gui` / EXPECT clean.
  - CHECK `rg 'unwrap|expect\(|panic!|todo!|unsafe' crates/gui/src/home.rs` /
    EXPECT sirf test-mod hits (non-test zero).
  - CHECK `git status --short` / EXPECT sirf OWNS files (M home.rs + untracked
    own checks dir).
- REPORT: `checks/gui-sessionmodel-bu/REPORT.md` Hinglish-Roman (struct table +
  transition truth table + test count).

### Wave BU-SES-APP — App bridge (sessions as source of truth)

- OWNS: `crates/gui/src/app.rs`, `checks/gui-sessionapp-bu/**`.
- KEVL NO-TOUCH: `home.rs` (model API read-only use), `views.rs`, `viewmodel.rs`,
  baaki checks dirs, scripts/, tools/, STATUS.
- Kaam: `App.sessions: SessionList` field + Default init; `current_tab` derived;
  `set_tab` compat; `open_home_session` -> `sessions.open`; T5/T6 bridge (switch
  par paths load, type ke baad save); cap/drop rule section 4 ke mutabiq.
  DEPENDENCY: MODEL wave landed hona chahiye (SessionList API usi se aayegi).
- GATES (CHECK / EXPECT):
  - CHECK `cargo test -p autocompare-gui` / EXPECT green (baseline + naye:
    derived tab khaali=Home, set_tab switch-vs-open, close neighbour rule,
    legacy callers compat).
  - CHECK clippy `-D warnings` / EXPECT zero; fmt clean; banned-pattern scan
    non-test zero.
  - CHECK paint behavior unchanged: `cargo run -p autocompare-gui -- --screenshot
    ...` fixtures byte-same ya REPORT me diff justify (is wave me paint change
    nahi, isliye fixtures stable rehne chahiye).
  - CHECK `git status --short` / EXPECT sirf OWNS files.
- REPORT: `checks/gui-sessionapp-bu/REPORT.md` Hinglish-Roman (bridge diagram +
  compat table + fixture stability note).

### Wave BU-SES-PAINT — session tab strip paint

- OWNS: `crates/gui/src/views.rs`, `checks/gui-sessionpaint-bu/**`.
- KEVL NO-TOUCH: `app.rs`, `home.rs`, `viewmodel.rs`, baaki checks dirs,
  scripts/, tools/, STATUS.
- Kaam: section 5 ke `paint_session_tab_strip` + `paint_session_tab` +
  `session_tab_label` + `SESSION_TAB_MAX_W` + `SessionAction`; mode wali
  `paint_tabs` untouched; overflow ScrollArea; khaali-list hint; chrome role
  typography; colors FROZEN (existing selected style reuse).
  DEPENDENCY: MODEL + APP waves landed (list + action wiring App caller me tabhi
  jud payegi; is wave me paint fns + headless label tests + fixture proof).
- GATES (CHECK / EXPECT):
  - CHECK `cargo test -p autocompare-gui` / EXPECT green (label fallback tests,
    action enum tests, strip empty-list test).
  - CHECK clippy/fmt clean; banned-pattern non-test zero.
  - CHECK headless fixture `--screenshot` session scenes (2-3 tabs, 8-tab
    overflow, khaali) / EXPECT PNG magic-ok + size>=8192 + distinct>64, REPORT
    me shot map.
  - CHECK `git status --short` / EXPECT sirf OWNS files.
- REPORT: `checks/gui-sessionpaint-bu/REPORT.md` Hinglish-Roman (paint fn table +
  geometry tokens + shot map + overflow behavior note).

Wave order: MODEL -> APP -> PAINT (serial dependency, har wave pichhli ke landed
API par). Teenon ke OWNS disjoint hain (home.rs / app.rs / views.rs + alag checks
dirs), isliye spec parallel likhi gayi hai par land serial hogi.

## 7. Colors note (scope bahar, spec nahi)

Active-tab highlight aur close-x hover existing selected/button style reuse karenge;
koi naya hue/palette is plan me propose nahi ho raha. Round-1 ka Light-vs-dark
mismatch open item (app.rs Default `dark_mode=true` vs live Light Mode) is plan ko
block nahi karta; uska root-cause follow-up `app.rs` + `theme.rs` me rahega.

## 8. Constraints (teeno waves par lagu)

- Hinglish-Roman only (Devanagari MANA), REPORT + GATES + commit message sab me.
- Conventional commit, NO PUSH (orchestrator verifies then pushes).
- Pathspec-only commits (`git add <owns-paths>`; `git add -A` MANA — BT sweep
  lesson).
- MS rust guidelines (forbid unsafe, no unwrap/expect/panic/todo non-test).
- Headless-first: paint se pehle model tests; fixture proof har paint change par.
- STATUS (`checks/opencode-waves/STATUS.md`) MANA — sirf orchestrator.
