//! Home View session launcher model.
//!
//! Provides the 9 comparison session types matching the CLI commands.

#![forbid(unsafe_code)]
#![allow(dead_code)]

/// The nine comparison tabs plus the Home launcher view.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Home,
    Text,
    Folder,
    Hex,
    Merge,
    Table,
    Sync,
    Picture,
    FolderMerge,
    Audio,
}

impl Tab {
    /// Returns the human-readable display title for this tab.
    pub const fn title(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Text => "Text",
            Self::Folder => "Folder",
            Self::Hex => "Hex",
            Self::Merge => "Merge",
            Self::Table => "Table",
            Self::Sync => "Sync",
            Self::Picture => "Picture",
            Self::FolderMerge => "FolderMerge",
            Self::Audio => "Audio",
        }
    }
}

/// One session launcher card in the Home View.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeItem {
    pub title: &'static str,
    pub description: &'static str,
    pub tab: Tab,
}

/// Headless model for the Home View launcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeModel {
    pub items: Vec<HomeItem>,
}

/// Builds the pure Home View model with all 9 comparison session types.
pub fn home_model() -> HomeModel {
    HomeModel {
        items: vec![
            HomeItem {
                title: "Text Compare",
                description: "Side-by-side text comparison with inline difference highlighting",
                tab: Tab::Text,
            },
            HomeItem {
                title: "Folder Compare",
                description: "Compare folder hierarchies, timestamps, and file sizes",
                tab: Tab::Folder,
            },
            HomeItem {
                title: "Hex Compare",
                description: "Binary byte-by-byte comparison with hex offset layout",
                tab: Tab::Hex,
            },
            HomeItem {
                title: "Text Merge",
                description: "Three-way text reconciliation with conflict markers and Take resolution",
                tab: Tab::Merge,
            },
            HomeItem {
                title: "Table Compare",
                description: "Structured row-and-column comparison for delimited tables",
                tab: Tab::Table,
            },
            HomeItem {
                title: "Folder Sync",
                description: "Two-way directory synchronization with previewed action steps",
                tab: Tab::Sync,
            },
            HomeItem {
                title: "Picture Compare",
                description: "Visual image diffing with difference metrics and overlap heatmap",
                tab: Tab::Picture,
            },
            HomeItem {
                title: "Folder Merge",
                description: "Three-way folder merge with automatic and conflict resolution",
                tab: Tab::FolderMerge,
            },
            HomeItem {
                title: "Audio Compare",
                description: "Audio stream waveform and channel difference analysis",
                tab: Tab::Audio,
            },
        ],
    }
}

/// Home card typography (family, weight, size only; colors out of
/// scope). Matches `views::paint_home`: title strong 14, description
/// small weak, button body scale. GEO/integration wave applies these
/// in paint; this module stays headless so the contract is testable.
/// Sizes single-source from [`crate::theme::TypoRole`] (no literals).
pub const HOME_CARD_TITLE_SIZE: f32 = crate::theme::TypoRole::HomeTitle.size();
/// Home card title is strong.
pub const HOME_CARD_TITLE_STRONG: bool = true;
/// Home card description is small + weak.
pub const HOME_CARD_DESC_SMALL_WEAK: bool = true;
/// Home card button uses the body scale (default size, no small/strong).
/// Single-source from the button role (== [`crate::theme::TYPO_BODY_SIZE`]).
pub const HOME_CARD_BUTTON_BODY_SIZE: f32 = crate::theme::TypoRole::HomeButton.size();

/// Title point size for Home cards (via role, single source: 14.0).
pub const fn home_card_title_size() -> f32 {
    crate::theme::TypoRole::HomeTitle.size()
}

/// Whether the Home card description paints small + weak.
pub const fn home_card_desc_is_small_weak() -> bool {
    HOME_CARD_DESC_SMALL_WEAK
}

/// Button point size for Home cards (via role, single source: body 13.0).
pub const fn home_card_button_size() -> f32 {
    crate::theme::TypoRole::HomeButton.size()
}

/// Home card title role: strong proportional 14. Typography only;
/// paint geometry (card widths, spacing) lives elsewhere.
pub const fn home_card_title_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::HomeTitle
}

/// Home card description role: small weak proportional 11.
pub const fn home_card_desc_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::HomeDesc
}

/// Home card button role: default proportional body scale.
pub const fn home_card_button_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::HomeButton
}

// ----------------------------------------------------------------------------
// Home geometry contract (headless; paint adoption `views.rs` owner ke paas).
// Geometry first: widths, heights, margins, padding, alignment — session
// cards, recent list, buttons. Sab values maujooda paint/rhythm tokens se
// derive hain taaki contract paint se jhooth na bole; colors untouched.
// ----------------------------------------------------------------------------

/// Session-card grid columns: 9 cards / 3 = 3 poori rows, section rhythm barkarar.
pub const HOME_GRID_COLUMNS: usize = 3;
/// Card min width px: `views::CARD_W` parity (paint wahi rakhta hai).
pub const HOME_CARD_MIN_W: f32 = 240.0;
/// Card outer pad px: section rhythm ([`crate::theme::GEO_SECTION_GAP`] = 8).
pub const HOME_CARD_PAD: f32 = crate::theme::GEO_SECTION_GAP;
/// Card inner gap px: tight rhythm ([`crate::theme::GEO_TIGHT_GAP`] = 4, title/desc/button beech).
pub const HOME_CARD_INNER_GAP: f32 = crate::theme::GEO_TIGHT_GAP;
/// Card button min hit height px: standard hit row ([`crate::theme::CHROME_HEADER_BAR_H`] = 28
/// rhythm parity — paging bars/find bar wala dabane layak
/// row; purana 24px find-nav floor is primary CTA ke liye chhota tha —
/// taaki "Open …" report wale 120x22 dabav se bache).
pub const HOME_CARD_BUTTON_MIN_H: f32 = crate::theme::CHROME_HEADER_BAR_H;
/// Description wrap budget: do lines tak sab cards ek jaisi height rakhen.
pub const HOME_CARD_DESC_LINES: usize = 2;
/// Session-card grid gap px: section rhythm ([`crate::theme::GEO_SECTION_GAP`] = 8
/// parity) — cards aur rows dono isi beat par, koi naya gap nahi.
pub const HOME_GRID_GAP: f32 = crate::theme::GEO_SECTION_GAP;
/// Toolbar compact button width px: path button floor se 0.8× (96)
/// taaki wide text pills compact rahen; sabse lamba toolbar label
/// content-driven ab bhi poora dikhta hai (floor sirf chhote labels ko
/// pakadta hai, content ko crush nahi karta). Pure; headless-tested.
pub const HOME_TOOLBAR_BUTTON_W: f32 = HOME_CARD_BUTTON_MIN_W * 0.8;
/// Toolbar compact button height px: 24 exquisite hit floor (find-nav
/// precedent, `app::LAYOUT_FIND_HIT_MIN` mirror) taaki toolbar row 28 se
/// 24 par aaye. Pure; headless-tested.
pub const HOME_TOOLBAR_BUTTON_H: f32 = 24.0;
/// Card button min hit width px: card aadha (report ka 120-wide "Open …"
/// label budget; `HOME_CARD_MIN_W` se derive, koi magic number nahi).
pub const HOME_CARD_BUTTON_MIN_W: f32 = HOME_CARD_MIN_W * 0.5;
/// Card action ka hit area px²: width floor × height floor. Report ka
/// dabaa 120x22 (= 2640) isse chhota hai; polish isko upar rakhta hai.
pub fn home_card_button_hit_area() -> f32 {
    HOME_CARD_BUTTON_MIN_W * HOME_CARD_BUTTON_MIN_H
}

/// Card CTA single-line width px: card inner width (`HOME_CARD_MIN_W` minus
/// do taraf outer pad = 224). Purana `HOME_CARD_BUTTON_MIN_W` (120, card
/// aadha) untouched hai taaki paint-lock tests green rahen; ye naya budget
/// sabse lambe "Open Picture Compare" label (20 chars, body 13 ~7px advance
/// plus 24px button frame pad ≈ 164, kind-column precedent) ko single line me
/// deta hai. Font size change nahi (HomeButton body 13 reuse); paint adoption
/// paint-owner wave (`views.rs` `paint_home` `add_sized`) ka hai; colors untouched.
pub const HOME_CARD_CTA_W: f32 = HOME_CARD_MIN_W - HOME_CARD_PAD * 2.0;
/// CTA label advance px/char: body 13 proportional estimate
/// (kind-column precedent ~7, koi naya metric nahi).
pub const HOME_CARD_CTA_ADVANCE_PX: f32 = 7.0;
/// CTA button frame pad px: text pad + do taraf frame (16 + 8).
pub const HOME_CARD_CTA_FRAME_PAD: f32 = 24.0;

/// CTA width px (single source: [`HOME_CARD_CTA_W`]). Pure; headless-tested.
pub fn home_card_cta_width() -> f32 {
    HOME_CARD_CTA_W
}

/// CTA label text: hamesha "Open {title}" (paint parity — `views.rs`
/// `paint_home` wahi format karta hai). Koi naya size/role nahi; pure.
pub fn home_card_cta_text(title: &str) -> String {
    format!("Open {title}")
}

/// CTA single-line budget theek hai: har 9 card ka label estimated need
/// (chars × advance + frame pad) [`HOME_CARD_CTA_W`] me fit. Pure;
/// headless-tested. Tang card par paint-owner wave width adopt karega.
pub fn home_card_cta_single_line_ok() -> bool {
    home_model().items.iter().all(|item| {
        home_card_cta_text(item.title).chars().count() as f32 * HOME_CARD_CTA_ADVANCE_PX
            + HOME_CARD_CTA_FRAME_PAD
            <= HOME_CARD_CTA_W
    })
}

/// Card CTA hit area px²: single-line width × hit floor (224×28 = 6272,
/// purane 120×28 = 3360 se dugna — paint adoption ke baad milega).
/// Pure; headless-tested.
pub fn home_card_cta_hit_area() -> f32 {
    HOME_CARD_CTA_W * HOME_CARD_BUTTON_MIN_H
}

/// Card title-vs-desc hierarchy theek hai: title desc se bada + strong,
/// desc small + weak, weight gap kam se kam ek kadam (100), dono chrome
/// proportional. Koi naya size nahi — sirf `TypoRole` ka antar pakka.
/// Pure; headless-tested.
pub fn home_card_hierarchy_ok() -> bool {
    use crate::theme::TypoRole;
    let title = TypoRole::HomeTitle;
    let desc = TypoRole::HomeDesc;
    title.size() > desc.size()
        && title.is_strong()
        && !desc.is_strong()
        && desc.is_small()
        && desc.is_weak()
        && title.weight_value() >= desc.weight_value() + 100
        && title.is_chrome()
        && desc.is_chrome()
        && title.family() == "proportional"
        && desc.family() == "proportional"
}

/// Session-start recent list cap: 8 entries, naya sabse upar.
pub const HOME_RECENT_MAX: usize = 8;

/// Grid rows for `item_count` cards at [`HOME_GRID_COLUMNS`] (ceil; khali par 0).
/// Pure; paint `Grid` row count isi se aani chahiye.
pub const fn home_grid_rows(item_count: usize) -> usize {
    item_count.div_ceil(HOME_GRID_COLUMNS)
}

/// Row-major cell of card `index`: (row, col). Pure; paint `Grid` order mirror.
pub const fn home_grid_cell(index: usize) -> (usize, usize) {
    (index / HOME_GRID_COLUMNS, index % HOME_GRID_COLUMNS)
}

/// Card min height px: title ek line + desc [`HOME_CARD_DESC_LINES`] lines
/// (dono apne role ke line-height px se) + button hit floor + do inner gaps +
/// upar-neeche pad. Koi magic number nahi — badlav token se aata hai.
/// Pure; headless-tested.
pub fn home_card_min_height() -> f32 {
    use crate::theme::TypoRole;
    TypoRole::HomeTitle.line_height_px()
        + TypoRole::HomeDesc.line_height_px() * HOME_CARD_DESC_LINES as f32
        + HOME_CARD_BUTTON_MIN_H
        + HOME_CARD_INNER_GAP * 2.0
        + HOME_CARD_PAD * 2.0
}

/// Session-start recent list: haal ke sessions, naya sabse upar, cap tak.
/// Pure headless model; paint (recent list UI) isko read-only dikhata hai.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecentSessions {
    entries: Vec<String>,
}

impl RecentSessions {
    /// Khali recent list.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Entry ko sabse upar rakhta hai: khali ignore, maujooda upar aata hai,
    /// cap se zyada girta hai. Pure state; kabhi fail nahi hota.
    pub fn push(&mut self, entry: impl Into<String>) {
        let text = entry.into();
        if text.trim().is_empty() {
            return;
        }
        self.entries.retain(|existing| existing != &text);
        self.entries.insert(0, text);
        self.entries.truncate(HOME_RECENT_MAX);
    }

    /// Naya-sabse-pehle order me entries.
    pub fn list(&self) -> &[String] {
        &self.entries
    }

    /// Entry ginti.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Khali hai ya nahi.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Sab entries hatata hai.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

// ----------------------------------------------------------------------------
// Session restore model (Wave AP-SESSION): recent list se purana session
// reopen — last state (tab, paths, cursor) snapshot me rehta hai.
// Geometry first (recent list rows), typography second, colors untouched.
// Paint (`views.rs` owner) isko read-only dikhata hai; record/restore pure
// headless hai taaki empty/missing paths graceful rahen, koi panic nahi.
// ----------------------------------------------------------------------------

/// Ek saved session ka last state: kaun sa tab, dono (ya teenon) paths,
/// us tab ka find cursor, aur save hone ka waqt (secs, 0 = unknown).
/// `base` two-path tabs par khali rehta hai. Paint isi se row + timestamp
/// dikhata hai; unknown waqt par timestamp chhupta hai, jhooth kabhi nahi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSnapshot {
    /// Kaun sa session tab tha.
    pub tab: Tab,
    /// Left path (trimmed, khali ho sakta hai).
    pub left: String,
    /// Right path (trimmed, khali ho sakta hai).
    pub right: String,
    /// Base path (three-way tabs par, warna khali).
    pub base: String,
    /// Us tab ka cursor (text/table find cursor, folder cursor, hex offset
    /// cursor ka `usize` roop; baaki tabs par 0).
    pub cursor: usize,
    /// Save hone ka waqt (caller ke secs, 0 = unknown). Dedupe isko
    /// ignore karta hai taaki purana waqt naya record na roke.
    pub saved_at_secs: u64,
}

impl SessionSnapshot {
    /// Snapshot banata hai; paths trim hote hain, cursor jaisa hai waisa.
    /// Waqt unknown (0) rehta hai; stamp ke liye `with_saved_at` lagao.
    pub fn new(
        tab: Tab,
        left: impl Into<String>,
        right: impl Into<String>,
        base: impl Into<String>,
        cursor: usize,
    ) -> Self {
        Self {
            tab,
            left: left.into().trim().to_string(),
            right: right.into().trim().to_string(),
            base: base.into().trim().to_string(),
            cursor,
            saved_at_secs: 0,
        }
    }

    /// Save waqt lagakar wapas deta hai (builder; purana nahi badalta).
    pub fn with_saved_at(mut self, now_secs: u64) -> Self {
        self.saved_at_secs = now_secs;
        self
    }

    /// Save ke baad guzra waqt (secs). Unknown waqt ya ulta clock par `None`,
    /// taaki paint jhootha "just now" kabhi na dikhaye.
    pub fn age_secs(&self, now_secs: u64) -> Option<u64> {
        if self.saved_at_secs == 0 || now_secs < self.saved_at_secs {
            return None;
        }
        Some(now_secs - self.saved_at_secs)
    }

    /// Row timestamp text: guzra waqt ka relative label, unknown par `None`.
    pub fn age_label(&self, now_secs: u64) -> Option<String> {
        self.age_secs(now_secs).map(recent_age_label)
    }

    /// Three-way session hai ya nahi (Merge / FolderMerge).
    pub const fn is_three_way(&self) -> bool {
        matches!(self.tab, Tab::Merge | Tab::FolderMerge)
    }

    /// Koi zaroori path khali hai ya nahi. Three-way par base bhi chahiye,
    /// two-path par sirf left/right. Paint isi se graceful hint dikhata hai.
    pub fn has_empty_paths(&self) -> bool {
        if self.left.trim().is_empty() || self.right.trim().is_empty() {
            return true;
        }
        if self.is_three_way() && self.base.trim().is_empty() {
            return true;
        }
        false
    }
}

/// Session-start recent history: snapshots, naya sabse upar, cap tak.
/// `RecentSessions` (titles) paint-compat ke liye rehta hai; ye full state
/// restore ke liye hai. Dono cap [`HOME_RECENT_MAX`] share karte hain.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionHistory {
    entries: Vec<SessionSnapshot>,
}

impl SessionHistory {
    /// Khali history.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Snapshot sabse upar rakhta hai: Home tab kabhi record nahi hota,
    /// same (tab, paths) dedupe hokar upar aata hai, cap se zyada girta hai.
    /// Waqt/cursor dedupe ko nahi rokta (sirf tab + paths compare hota hai).
    pub fn push(&mut self, snapshot: SessionSnapshot) {
        if snapshot.tab == Tab::Home {
            return;
        }
        self.entries.retain(|existing| {
            existing.tab != snapshot.tab
                || existing.left != snapshot.left
                || existing.right != snapshot.right
                || existing.base != snapshot.base
        });
        self.entries.insert(0, snapshot);
        self.entries.truncate(HOME_RECENT_MAX);
    }

    /// Waqt lagakar push karta hai: `now_secs` stamp hokar naya sabse upar.
    /// Home tab tab bhi skip hota hai. Pure state; kabhi fail nahi hota.
    pub fn push_stamped(&mut self, snapshot: SessionSnapshot, now_secs: u64) {
        self.push(snapshot.with_saved_at(now_secs));
    }

    /// Naya-sabse-pehle order me snapshots.
    pub fn list(&self) -> &[SessionSnapshot] {
        &self.entries
    }

    /// Entry ginti.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Khali hai ya nahi.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Sab entries hatata hai.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Index par snapshot, out-of-range par `None` (graceful, koi panic nahi).
    pub fn get(&self, index: usize) -> Option<&SessionSnapshot> {
        self.entries.get(index)
    }
}

// ----------------------------------------------------------------------------
// Live multi-session store (Wave BV-SES-MODEL): khule compares, record nahi.
// `SessionSnapshot` / `SessionHistory` precedent reuse: trim/stamp/age wahi
// semantics; history record hai (reopen ke liye), ye live hai (tabs ke liye).
// Paint (views.rs, Wave 3) isko read-only dikhata hai; transitions pure
// headless hain, unknown id graceful no-op hai.
// ----------------------------------------------------------------------------

/// Ek khula compare session: kind se alag instance (do Text compares ek saath
/// khul sakte hain). `title` tab label hai (file stem ya folder base, trimmed);
/// khaali paths par `"{Kind} {id}"` fallback — label kabhi khaali nahi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    /// Ever-increasing id, reuse kabhi nahi (band ke baad bhi nahi).
    pub id: u64,
    /// Session type (Text/Folder/...; Home kabhi session nahi).
    pub kind: Tab,
    /// Tab label: file stem ya folder base, trimmed.
    pub title: String,
    /// Left path (trimmed).
    pub left: String,
    /// Right path (trimmed).
    pub right: String,
    /// Base path (three-way par, warna khaali; trimmed).
    pub base: String,
    /// Us session ka find/scroll cursor.
    pub cursor: usize,
    /// Compare ke baad model stale hua ya nahi (v1 me open par false).
    pub dirty: bool,
    /// Save waqt (secs, 0 = unknown; `SessionSnapshot` precedent).
    pub saved_at_secs: u64,
}

/// Tab label derive karta hai: `left` (khaali par `right`) ka file-stem ya
/// folder-base; dono khaali par `"{Kind} {id}"` fallback. Pure.
pub fn session_title_for(kind: Tab, left: &str, right: &str, id: u64) -> String {
    let picked = if !left.trim().is_empty() {
        left.trim()
    } else if !right.trim().is_empty() {
        right.trim()
    } else {
        return format!("{} {id}", kind.title());
    };
    let slashy = picked.replace('\\', "/");
    let no_trail = slashy.trim_end_matches('/');
    if no_trail.is_empty() {
        return format!("{} {id}", kind.title());
    }
    let seg = match no_trail.rsplit_once('/') {
        Some((_, tail)) => tail,
        None => no_trail,
    };
    let stem = match seg.rsplit_once('.') {
        Some((head, _)) if !head.is_empty() => head,
        _ => seg,
    };
    if stem.trim().is_empty() {
        format!("{} {id}", kind.title())
    } else {
        stem.to_string()
    }
}

impl Session {
    /// Session banata hai; paths trim hote hain, title derive hota hai,
    /// dirty false, waqt unknown (0). Stamp ke liye `with_saved_at` lagao.
    /// (`SessionSnapshot::new` precedent + id title fallback ke liye.)
    pub fn new(
        id: u64,
        kind: Tab,
        left: impl Into<String>,
        right: impl Into<String>,
        base: impl Into<String>,
        cursor: usize,
    ) -> Self {
        let left: String = left.into();
        let right: String = right.into();
        let title = session_title_for(kind, &left, &right, id);
        Self {
            id,
            kind,
            title,
            left: left.trim().to_string(),
            right: right.trim().to_string(),
            base: base.into().trim().to_string(),
            cursor,
            dirty: false,
            saved_at_secs: 0,
        }
    }

    /// Save waqt lagakar wapas deta hai (builder; purana nahi badalta).
    /// (`SessionSnapshot::with_saved_at` precedent.)
    pub fn with_saved_at(mut self, now_secs: u64) -> Self {
        self.saved_at_secs = now_secs;
        self
    }

    /// Save ke baad guzra waqt (secs). Unknown waqt ya ulta clock par `None`.
    /// (`SessionSnapshot::age_secs` precedent.)
    pub fn age_secs(&self, now_secs: u64) -> Option<u64> {
        if self.saved_at_secs == 0 || now_secs < self.saved_at_secs {
            return None;
        }
        Some(now_secs - self.saved_at_secs)
    }

    /// Row timestamp text: guzra waqt ka relative label, unknown par `None`.
    /// (`SessionSnapshot::age_label` precedent, wahi `recent_age_label`.)
    pub fn age_label(&self, now_secs: u64) -> Option<String> {
        self.age_secs(now_secs).map(recent_age_label)
    }
}

/// Live multi-session store (history se alag: ye live hai, wo record hai).
/// Transitions T1-T7 pure headless hain; unknown id par no-op false milta hai.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionList {
    sessions: Vec<Session>,
    active: Option<u64>,
    next_id: u64,
}

impl SessionList {
    /// Khaali store (koi session nahi, active `None` = launcher view).
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            active: None,
            next_id: 0,
        }
    }

    /// T1: session kholta hai — naya id, active wahi, dirty false. Home kind
    /// par no-op `None` (id consume nahi hota). Cap full par sabse purani
    /// INACTIVE session girti hai, active kabhi auto-close nahi hota.
    pub fn open(
        &mut self,
        kind: Tab,
        left: impl Into<String>,
        right: impl Into<String>,
        base: impl Into<String>,
    ) -> Option<u64> {
        if kind == Tab::Home {
            return None;
        }
        if self.sessions.len() >= HOME_RECENT_MAX {
            let drop_pos = self
                .sessions
                .iter()
                .position(|item| Some(item.id) != self.active);
            if let Some(pos) = drop_pos {
                self.sessions.remove(pos);
            }
        }
        let id = self.next_id;
        self.next_id = id.wrapping_add(1);
        self.sessions
            .push(Session::new(id, kind, left, right, base, 0));
        self.active = Some(id);
        Some(id)
    }

    /// T2: sirf wahi session band hota hai. Active band hua to naya active
    /// left-neighbour (na ho to right-neighbour); khaali par `None`.
    /// Unknown id par no-op false.
    pub fn close(&mut self, id: u64) -> bool {
        let pos = match self.sessions.iter().position(|item| item.id == id) {
            Some(pos) => pos,
            None => return false,
        };
        self.sessions.remove(pos);
        if self.active == Some(id) {
            self.active = if self.sessions.is_empty() {
                None
            } else if pos > 0 {
                self.sessions.get(pos - 1).map(|item| item.id)
            } else {
                self.sessions.first().map(|item| item.id)
            };
        }
        true
    }

    /// T3: active badalta hai; unknown id par no-op false.
    pub fn switch(&mut self, id: u64) -> bool {
        if self.sessions.iter().any(|item| item.id == id) {
            self.active = Some(id);
            true
        } else {
            false
        }
    }

    /// T4: sirf wahi session rehta hai, active wahi; unknown id par no-op.
    pub fn close_others(&mut self, id: u64) -> bool {
        if self.sessions.iter().any(|item| item.id == id) {
            self.sessions.retain(|item| item.id == id);
            self.active = Some(id);
            true
        } else {
            false
        }
    }

    /// T5: session ke paths sync karta hai (trimmed, title untouched).
    /// Switch bridge caller compare/type ke baad chalata hai. Unknown id
    /// par no-op false.
    pub fn update_paths(
        &mut self,
        id: u64,
        left: impl Into<String>,
        right: impl Into<String>,
        base: impl Into<String>,
    ) -> bool {
        match self.sessions.iter_mut().find(|item| item.id == id) {
            Some(item) => {
                item.left = left.into().trim().to_string();
                item.right = right.into().trim().to_string();
                item.base = base.into().trim().to_string();
                true
            }
            None => false,
        }
    }

    /// T6: session ka cursor save karta hai. Unknown id par no-op false.
    pub fn update_cursor(&mut self, id: u64, cursor: usize) -> bool {
        match self.sessions.iter_mut().find(|item| item.id == id) {
            Some(item) => {
                item.cursor = cursor;
                true
            }
            None => false,
        }
    }

    /// T7: tab label badalta hai (trimmed); blank par no-op false.
    /// Caller (`App::rename_session`) isi ko chalata hai.
    pub fn rename(&mut self, id: u64, title: impl Into<String>) -> bool {
        let title: String = title.into();
        if title.trim().is_empty() {
            return false;
        }
        match self.sessions.iter_mut().find(|item| item.id == id) {
            Some(item) => {
                item.title = title.trim().to_string();
                true
            }
            None => false,
        }
    }

    /// Session id par, na mile to `None`.
    pub fn get(&self, id: u64) -> Option<&Session> {
        self.sessions.iter().find(|item| item.id == id)
    }

    /// Active session, khaali par `None` (launcher view).
    pub fn active(&self) -> Option<&Session> {
        match self.active {
            Some(id) => self.get(id),
            None => None,
        }
    }

    /// Active session id, khaali par `None`.
    pub fn active_id(&self) -> Option<u64> {
        self.active
    }

    /// Sab sessions, khulne ke order me.
    pub fn list(&self) -> &[Session] {
        &self.sessions
    }

    /// Khule sessions ki ginti.
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    /// Koi session khula hai ya nahi.
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }
}

// ----------------------------------------------------------------------------
// Recent list geometry (first): rows, gaps, pad — sab maujooda rhythm tokens
// se derive, koi magic number nahi, colors untouched.
// ----------------------------------------------------------------------------

/// Recent row gap px: card inner gap (`HOME_CARD_INNER_GAP` = 4) reuse.
pub const RECENT_ROW_GAP: f32 = HOME_CARD_INNER_GAP;
/// Recent list outer pad px: card outer pad (`HOME_CARD_PAD` = 8) reuse.
pub const RECENT_LIST_PAD: f32 = HOME_CARD_PAD;

/// Recent row min height px: ek desc-role line + inner gap + outer pad.
/// Single-line row text kabhi clip na ho, button floor (24px) se upar rahe.
pub fn recent_row_min_height() -> f32 {
    use crate::theme::TypoRole;
    TypoRole::HomeDesc.line_height_px() + HOME_CARD_INNER_GAP + HOME_CARD_PAD
}

/// `count` rows wali recent list ki kul height px: rows + beech gaps +
/// upar-neeche pad. Khali par 0 (paint khali list chhupata hai).
pub fn recent_list_height(count: usize) -> f32 {
    if count == 0 {
        return 0.0;
    }
    let rows = count as f32 * recent_row_min_height();
    let gaps = count.saturating_sub(1) as f32 * RECENT_ROW_GAP;
    rows + gaps + RECENT_LIST_PAD * 2.0
}

/// Row `index` ka top offset px (list pad ke andar). Pure; paint row isi se lagaye.
pub fn recent_row_top(index: usize) -> f32 {
    RECENT_LIST_PAD + index as f32 * (recent_row_min_height() + RECENT_ROW_GAP)
}

// ----------------------------------------------------------------------------
// Recent timestamps (second): relative labels, pure, koi clock call nahi.
// Caller guzra waqt deta hai; paint sirf label dikhata hai. Colors untouched.
// ----------------------------------------------------------------------------

/// Guzre waqt ka relative row label: 60s se kam "just now", phir minute,
/// phir ghante, phir din. Pure; paint timestamp isi se likhta hai.
pub fn recent_age_label(elapsed_secs: u64) -> String {
    if elapsed_secs < 60 {
        String::from("just now")
    } else if elapsed_secs < 3_600 {
        format!("{}m ago", elapsed_secs / 60)
    } else if elapsed_secs < 86_400 {
        format!("{}h ago", elapsed_secs / 3_600)
    } else {
        format!("{}d ago", elapsed_secs / 86_400)
    }
}

// ----------------------------------------------------------------------------
// Recent two-line row geometry (first): title + timestamp/meta stack.
// Single-line helpers upar paint-compat ke liye untouched rehte hain; nayi
// polish do-line row par banti hai. Sab rhythm tokens se derive.
// ----------------------------------------------------------------------------

/// Title/meta beech gap px: card inner gap (4) reuse, koi naya number nahi.
pub const RECENT_ROW_TITLE_META_GAP: f32 = HOME_CARD_INNER_GAP;

/// Do-line row min height px: title line (body) + meta line (desc) + beech
/// gap + row pad. Single-line row se uncha, button floor (24px) se upar.
pub fn recent_row_two_line_height() -> f32 {
    use crate::theme::TypoRole;
    TypoRole::HomeButton.line_height_px()
        + TypoRole::HomeDesc.line_height_px()
        + RECENT_ROW_TITLE_META_GAP
        + RECENT_LIST_PAD
}

/// `count` rows wali do-line recent list ki kul height px: rows + beech
/// gaps + upar-neeche pad. Khali par 0 (paint khali list chhupata hai).
pub fn recent_list_height_two_line(count: usize) -> f32 {
    if count == 0 {
        return 0.0;
    }
    let rows = count as f32 * recent_row_two_line_height();
    let gaps = count.saturating_sub(1) as f32 * RECENT_ROW_GAP;
    rows + gaps + RECENT_LIST_PAD * 2.0
}

/// Do-line row `index` ka top offset px (list pad ke andar). Pure.
pub fn recent_row_top_two_line(index: usize) -> f32 {
    RECENT_LIST_PAD + index as f32 * (recent_row_two_line_height() + RECENT_ROW_GAP)
}

/// Do-line list ki capped height px: sirf `max_visible` rows tak jagah,
/// baaki paint scroll karta hai. `max_visible` 0 par 0.
pub fn recent_list_height_capped(count: usize, max_visible: usize) -> f32 {
    recent_list_height_two_line(count.min(max_visible))
}

/// List ko scroll chahiye ya nahi: rows visible budget se zyada par true.
pub fn recent_list_needs_scroll(count: usize, max_visible: usize) -> bool {
    count > max_visible
}

// ----------------------------------------------------------------------------
// Recent list columnar alignment (first): title flex + kind fixed + age
// fixed. Single-line legacy helpers upar untouched; ye teen-column row ka
// headless column math hai (paint adoption paint-owner wave ka). Sab rhythm
// tokens se derive; fixed widths me derivation comment hai. Colors untouched.
// ----------------------------------------------------------------------------

/// Column beech gap px: card inner gap (4) reuse, koi naya number nahi.
pub const RECENT_COL_GAP: f32 = HOME_CARD_INNER_GAP;
/// Kind column width px: sabse lamba kind "FolderMerge" (11 chars, body 13
/// ~7px advance = 77 + 16 pad = 93) ka wrap budget, 96 par goal.
pub const RECENT_COL_KIND_W: f32 = 96.0;
/// Age column width px: "just now" / "23h ago" (desc 11 + bullet + pad)
/// ka wrap budget, 72 par goal.
pub const RECENT_COL_AGE_W: f32 = 72.0;

/// `total_w` me title column ki flex width px: kul me se list pad (do taraf),
/// kind + age columns aur do column gaps ghatao; tang par 0 (clip ka jhooth
/// nahi). Non-finite input passthrough taaki paint kabhi NaN na khaye.
/// Pure; headless-tested.
pub fn recent_row_title_width(total_w: f32) -> f32 {
    if !total_w.is_finite() {
        return total_w;
    }
    (total_w - RECENT_LIST_PAD * 2.0 - RECENT_COL_KIND_W - RECENT_COL_AGE_W - RECENT_COL_GAP * 2.0)
        .max(0.0)
}

/// `total_w` ke teen column (title, kind, age) widths px: title flex, kind
/// aur age fixed. Chaudi list par teeno + pad + gap kul ke barabar; tang
/// par title 0, kind/age apne budget par. Pure; headless-tested.
pub fn recent_row_columns(total_w: f32) -> (f32, f32, f32) {
    (
        recent_row_title_width(total_w),
        RECENT_COL_KIND_W,
        RECENT_COL_AGE_W,
    )
}

// ----------------------------------------------------------------------------
// Home chrome vertical rhythm (first): launcher par path picker nahi —
// Compare tabs apne picker khud rakhte hain. Ye contract batata hai kitna
// chrome bachta hai aur section kahan se shuru hota hai. Sab maujooda
// rhythm tokens se derive; colors untouched.
// ----------------------------------------------------------------------------

/// Home launcher path picker dikhata hai ya nahi: kabhi nahi (false).
/// Report section 2: Home par path row 82px chrome waste karta hai.
pub const HOME_SHOWS_PATH_PICKER: bool = false;
/// Path row height px: standard hit row ([`crate::theme::CHROME_HEADER_BAR_H`] = 28 parity).
pub const HOME_PATH_ROW_H: f32 = crate::theme::CHROME_HEADER_BAR_H;
/// Path row neeche gap px: tight rhythm ([`crate::theme::GEO_TIGHT_GAP`] = 4 parity).
pub const HOME_PATH_ROW_GAP: f32 = crate::theme::GEO_TIGHT_GAP;
/// Section beech gap px: section rhythm ([`crate::theme::GEO_SECTION_GAP`] = 8 parity) —
/// card grid aur recent list isi beat par alag hote hain.
pub const HOME_SECTION_GAP: f32 = crate::theme::GEO_SECTION_GAP;

/// Path picker chhupane se bachta chrome px: row + uska gap = 32.
pub fn home_saved_chrome_height() -> f32 {
    HOME_PATH_ROW_H + HOME_PATH_ROW_GAP
}

/// Content ka top offset px: picker dikhe to row + gap, Home par 0.
/// Pure; paint section isi se lagaye.
pub fn home_content_top_offset(shows_picker: bool) -> f32 {
    if shows_picker {
        home_saved_chrome_height()
    } else {
        0.0
    }
}

/// Card grid (`cards_h` px) ke baad recent section ka top offset px:
/// grid + section gap. Pure; paint recent list isi se lagaye.
pub fn home_recent_section_top(cards_h: f32) -> f32 {
    if !cards_h.is_finite() {
        return cards_h;
    }
    (cards_h + HOME_SECTION_GAP).max(0.0)
}

// ----------------------------------------------------------------------------
// Recent list typography (second): row text role reuse, koi naya size nahi.
// ----------------------------------------------------------------------------

/// Recent row text role: chhota kamzor proportional (secondary list text).
/// Adopts the [`crate::theme::TypoRole::caption`] alias (== HomeDesc, value-stable).
pub const fn recent_row_text_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::caption()
}

/// Recent row text size px (single source: caption alias, 11.0).
pub fn recent_row_text_size() -> f32 {
    crate::theme::TypoRole::caption().size()
}

/// Recent row text line-height px (body 1.40 x desc size): row stack isi se.
/// HomeDesc par rehta hai taaki row numbers value-stable rahen (caption alias ka
/// multiplier alag hai — G-T7 pixel tightening views.rs follow-up me).
pub fn recent_row_text_line_height_px() -> f32 {
    crate::theme::TypoRole::HomeDesc.line_height_px()
}

/// Recent row text tracking em (headers wala halka failao nahi, zero).
pub fn recent_row_text_tracking_em() -> f32 {
    crate::theme::TypoRole::HomeDesc.letter_spacing_em()
}

/// Recent row text numeric weight (regular 400, desc role se).
pub fn recent_row_text_weight_value() -> u16 {
    crate::theme::TypoRole::HomeDesc.weight_value()
}

// ----------------------------------------------------------------------------
// Recent two-line row typography (second): title ek kadam upar (body),
// meta chhota kamzor (desc). Dono `TypoRole` reuse, koi naya size nahi.
// ----------------------------------------------------------------------------

/// Row title role: primary session naam (body scale, medium).
pub const fn recent_row_title_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::HomeButton
}

/// Row meta/timestamp role: secondary line (small weak proportional).
/// Adopts the [`crate::theme::TypoRole::caption`] alias (== HomeDesc, paint-identical:
/// dono Small style 11 weak). Single-line legacy row text ke barabar, taaki purana
/// paint na jhoothe. Numbers HomeDesc par rehte hain (value-stable).
pub const fn recent_row_meta_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::caption()
}

/// Row title size px (single source: body role).
pub fn recent_row_title_size() -> f32 {
    crate::theme::TypoRole::HomeButton.size()
}

/// Row title line-height px: do-line stack ki pehli line isi se.
pub fn recent_row_title_line_height_px() -> f32 {
    crate::theme::TypoRole::HomeButton.line_height_px()
}

/// Row title tracking em (body par zero).
pub fn recent_row_title_tracking_em() -> f32 {
    crate::theme::TypoRole::HomeButton.letter_spacing_em()
}

/// Row title numeric weight (medium 500).
pub fn recent_row_title_weight_value() -> u16 {
    crate::theme::TypoRole::HomeButton.weight_value()
}

/// Row meta size px (single source: desc role, legacy row text barabar).
pub fn recent_row_meta_size() -> f32 {
    crate::theme::TypoRole::HomeDesc.size()
}

/// Row meta line-height px: do-line stack ki dusri line isi se.
pub fn recent_row_meta_line_height_px() -> f32 {
    crate::theme::TypoRole::HomeDesc.line_height_px()
}

/// Row meta tracking em (zero).
pub fn recent_row_meta_tracking_em() -> f32 {
    crate::theme::TypoRole::HomeDesc.letter_spacing_em()
}

/// Row meta numeric weight (regular 400).
pub fn recent_row_meta_weight_value() -> u16 {
    crate::theme::TypoRole::HomeDesc.weight_value()
}

/// Title/meta hierarchy theek hai: title meta se bada, meta small + weak,
/// dono chrome proportional. Pure; headless-tested.
pub fn recent_row_hierarchy_ok() -> bool {
    use crate::theme::TypoRole;
    let title = TypoRole::HomeButton;
    let meta = TypoRole::HomeDesc;
    title.size() > meta.size()
        && meta.is_small()
        && meta.is_weak()
        && title.is_chrome()
        && meta.is_chrome()
        && title.family() == "proportional"
        && meta.family() == "proportional"
}

// ----------------------------------------------------------------------------
// Recent empty/missing states (honest): view kabhi blank na lage, row kabhi
// stale paths par "ready" ka jhooth na bole. Sab pure `&str`/`String`.
// ----------------------------------------------------------------------------

/// Khali recent list ka guidance: fresh start vs cleared. Rows win nahi
/// (list hi khali hai), isliye hamesha ek imaandaar hint milta hai.
pub const fn recent_list_empty_hint(was_cleared: bool) -> &'static str {
    if was_cleared {
        "recent cleared: open any compare above to pin it here"
    } else {
        "no recent sessions yet: open any compare above to pin it here"
    }
}

/// Row status label: missing paths par graceful, warna refresh reminder.
pub const fn recent_row_status_label(has_empty_paths: bool) -> &'static str {
    if has_empty_paths {
        "paths missing: type both paths then Compare"
    } else {
        "paths restored: Compare to refresh"
    }
}

/// Row meta line: status + timestamp, dono imaandaar. Unknown waqt par
/// timestamp chhupta hai (koi "just now" ka jhooth nahi).
pub fn recent_row_meta_text(snapshot: &SessionSnapshot, now_secs: u64) -> String {
    let status = recent_row_status_label(snapshot.has_empty_paths());
    match snapshot.age_label(now_secs) {
        Some(age) => format!("{status} \u{2022} {age}"),
        None => String::from(status),
    }
}

// ----------------------------------------------------------------------------
// Home typography layer-2 (second): family/weight/line-height/tracking —
// sab `TypoRole` precedent reuse, koi naya point size nahi, colors untouched.
// ----------------------------------------------------------------------------

/// Title line-height px (chrome 1.2 x 14): card stack isi se banta hai.
pub const fn home_card_title_line_height_px() -> f32 {
    crate::theme::TypoRole::HomeTitle.line_height_px()
}

/// Description line-height px (body 1.40 x 11): do-line budget isi se.
pub const fn home_card_desc_line_height_px() -> f32 {
    crate::theme::TypoRole::HomeDesc.line_height_px()
}

/// Title tracking em (0.02): headers wala halka failao, baaki zero.
pub const fn home_card_title_tracking_em() -> f32 {
    crate::theme::TypoRole::HomeTitle.letter_spacing_em()
}

/// Title numeric weight (medium 500).
pub const fn home_card_title_weight_value() -> u16 {
    crate::theme::TypoRole::HomeTitle.weight_value()
}

/// Description numeric weight (regular 400).
pub const fn home_card_desc_weight_value() -> u16 {
    crate::theme::TypoRole::HomeDesc.weight_value()
}

/// Button numeric weight (medium 500).
pub const fn home_card_button_weight_value() -> u16 {
    crate::theme::TypoRole::HomeButton.weight_value()
}

/// Teenon card roles chrome (proportional UI) hain, mono nahi: card text
/// code bankar nahi, launcher bankar dikhta hai. Pure; headless-tested.
pub fn home_card_roles_are_chrome_proportional() -> bool {
    use crate::theme::TypoRole;
    let roles = [
        TypoRole::HomeTitle,
        TypoRole::HomeDesc,
        TypoRole::HomeButton,
    ];
    roles
        .iter()
        .all(|role| role.is_chrome() && role.family() == "proportional")
}

/// Empty-state guidance for the Folder tab so the view never looks
/// blank: rows win, then errors, then the idle hint.
pub const fn folder_empty_hint(has_rows: bool, has_error: bool) -> &'static str {
    if has_rows {
        ""
    } else if has_error {
        "compare failed: fix the message above, then Compare again"
    } else {
        "type both folder paths, press Compare, then expand a folder"
    }
}

/// Routes a staged folder double-click to the tab that opens it. A
/// staged file path always lands in the Text session; with no staging
/// the host stays on Folder.
pub fn resolve_compare_tab(staged: &Option<String>) -> Tab {
    if staged.is_some() {
        Tab::Text
    } else {
        Tab::Folder
    }
}

/// Consent hint for the sync Apply gate.
pub const fn sync_consent_hint(consented: bool, steps: usize) -> &'static str {
    if steps == 0 {
        "nothing to apply: the preview is empty"
    } else if consented {
        "consented: Apply will run the planned steps"
    } else {
        "preview only: tick consent to enable Apply"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_model_covers_all_nine_session_types() {
        let model = home_model();
        assert_eq!(model.items.len(), 9);
        assert!(model.items.iter().any(|i| i.tab == Tab::Text));
        assert!(model.items.iter().any(|i| i.tab == Tab::Folder));
        assert!(model.items.iter().any(|i| i.tab == Tab::Hex));
        assert!(model.items.iter().any(|i| i.tab == Tab::Merge));
        assert!(model.items.iter().any(|i| i.tab == Tab::Table));
        assert!(model.items.iter().any(|i| i.tab == Tab::Sync));
        assert!(model.items.iter().any(|i| i.tab == Tab::Picture));
        assert!(model.items.iter().any(|i| i.tab == Tab::FolderMerge));
        assert!(model.items.iter().any(|i| i.tab == Tab::Audio));
    }

    #[test]
    fn folder_empty_hint_never_leaves_view_blank() {
        assert_eq!(folder_empty_hint(true, false), "");
        assert!(folder_empty_hint(false, true).contains("Compare again"));
        assert!(folder_empty_hint(false, false).contains("type both folder paths"));
    }

    #[test]
    fn sync_consent_hint_tracks_gate() {
        assert!(sync_consent_hint(false, 0).contains("nothing to apply"));
        assert!(sync_consent_hint(false, 3).contains("preview only"));
        assert!(sync_consent_hint(true, 3).contains("consented"));
    }

    #[test]
    fn staged_file_routes_to_text_otherwise_folder() {
        assert_eq!(
            resolve_compare_tab(&Some("src/a.txt".to_string())),
            Tab::Text
        );
        assert_eq!(resolve_compare_tab(&None), Tab::Folder);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn home_card_roles_carry_pane_hierarchy() {
        use crate::theme::{TYPO_LINE_HEIGHT_SOURCE, TypoRole};
        assert_eq!(home_card_title_role(), TypoRole::HomeTitle);
        assert_eq!(home_card_desc_role(), TypoRole::HomeDesc);
        assert_eq!(home_card_button_role(), TypoRole::HomeButton);
        assert_eq!(home_card_title_role().size(), HOME_CARD_TITLE_SIZE);
        assert!(home_card_title_role().is_strong());
        assert_eq!(home_card_title_role().family(), "proportional");
        assert!(home_card_desc_role().is_small());
        assert!(home_card_desc_role().is_weak());
        assert_eq!(home_card_button_role().size(), HOME_CARD_BUTTON_BODY_SIZE);
        assert!(!home_card_button_role().is_strong());
        assert_eq!(
            home_card_title_role().line_height_source(),
            TYPO_LINE_HEIGHT_SOURCE
        );
    }

    #[test]
    fn home_grid_columns_three_rows_ceil() {
        assert_eq!(super::HOME_GRID_COLUMNS, 3);
        assert_eq!(super::home_grid_rows(0), 0);
        assert_eq!(super::home_grid_rows(1), 1);
        assert_eq!(super::home_grid_rows(9), 3);
        assert_eq!(super::home_grid_rows(10), 4);
        assert_eq!(super::home_grid_rows(super::home_model().items.len()), 3);
    }

    #[test]
    fn home_grid_cell_is_row_major() {
        assert_eq!(super::home_grid_cell(0), (0, 0));
        assert_eq!(super::home_grid_cell(2), (0, 2));
        assert_eq!(super::home_grid_cell(3), (1, 0));
        assert_eq!(super::home_grid_cell(8), (2, 2));
    }

    #[test]
    fn home_card_min_height_fits_stack() {
        use crate::theme::TypoRole;
        let height = super::home_card_min_height();
        let expected = TypoRole::HomeTitle.line_height_px()
            + TypoRole::HomeDesc.line_height_px() * super::HOME_CARD_DESC_LINES as f32
            + super::HOME_CARD_BUTTON_MIN_H
            + super::HOME_CARD_INNER_GAP * 2.0
            + super::HOME_CARD_PAD * 2.0;
        assert!((height - expected).abs() < 0.001);
        // 14*1.2 + 2*11*1.40 + 28 + 8 + 16 = 99.6: hit-row floor se upar,
        // purane 75px clip se mehfooz.
        assert!((height - 99.6).abs() < 0.01);
        assert!(height > super::HOME_CARD_BUTTON_MIN_H);
        assert_eq!(super::HOME_CARD_MIN_W, 240.0);
    }

    #[test]
    fn home_recent_push_dedupes_and_caps() {
        let mut recent = super::RecentSessions::new();
        assert!(recent.is_empty());
        assert_eq!(recent.len(), 0);
        recent.push("   ");
        assert!(recent.is_empty());
        for idx in 0..10 {
            recent.push(format!("session {idx}"));
        }
        assert_eq!(recent.len(), super::HOME_RECENT_MAX);
        assert_eq!(recent.list().first().map(String::as_str), Some("session 9"));
        recent.push("session 5");
        assert_eq!(recent.len(), super::HOME_RECENT_MAX);
        assert_eq!(recent.list().first().map(String::as_str), Some("session 5"));
        let dupes = recent
            .list()
            .iter()
            .filter(|entry| entry.as_str() == "session 5")
            .count();
        assert_eq!(dupes, 1);
        recent.clear();
        assert!(recent.is_empty());
    }

    #[test]
    fn home_card_typo_second_layer_reuses_roles() {
        use crate::theme::{
            TYPO_LINE_HEIGHT_BODY, TYPO_LINE_HEIGHT_CHROME, TYPO_TRACKING_HEADING_EM,
            TYPO_WEIGHT_MEDIUM, TYPO_WEIGHT_REGULAR, TypoRole,
        };
        assert_eq!(
            super::home_card_title_line_height_px(),
            14.0 * TYPO_LINE_HEIGHT_CHROME
        );
        assert_eq!(
            super::home_card_desc_line_height_px(),
            11.0 * TYPO_LINE_HEIGHT_BODY
        );
        assert_eq!(
            super::home_card_title_tracking_em(),
            TYPO_TRACKING_HEADING_EM
        );
        assert_eq!(super::home_card_title_weight_value(), TYPO_WEIGHT_MEDIUM);
        assert_eq!(super::home_card_desc_weight_value(), TYPO_WEIGHT_REGULAR);
        assert_eq!(super::home_card_button_weight_value(), TYPO_WEIGHT_MEDIUM);
        assert_eq!(TypoRole::HomeButton.family(), "proportional");
        assert!(super::home_card_roles_are_chrome_proportional());
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn home_card_typo_matches_paint_contract() {
        use crate::theme::{TYPO_BODY_SIZE, TYPO_SMALL_SIZE, TypoRole};
        // Title strong 14 = the single `.size(14.0)` in paint_home.
        assert_eq!(home_card_title_size(), 14.0);
        assert_eq!(HOME_CARD_TITLE_SIZE, 14.0);
        assert!(HOME_CARD_TITLE_STRONG);
        assert_eq!(TypoRole::HomeTitle.size(), 14.0);
        assert!(TypoRole::HomeTitle.is_strong());
        // Description small weak.
        assert!(home_card_desc_is_small_weak());
        assert!(HOME_CARD_DESC_SMALL_WEAK);
        assert_eq!(TypoRole::HomeDesc.size(), TYPO_SMALL_SIZE);
        assert!(TypoRole::HomeDesc.is_small());
        assert!(TypoRole::HomeDesc.is_weak());
        // Button body scale.
        assert_eq!(home_card_button_size(), TYPO_BODY_SIZE);
        assert_eq!(TypoRole::HomeButton.size(), TYPO_BODY_SIZE);
    }

    #[test]
    fn session_snapshot_trims_and_flags_empty_paths() {
        let full = super::SessionSnapshot::new(super::Tab::Text, "  left.txt ", "right.txt", "", 2);
        assert_eq!(full.left.as_str(), "left.txt");
        assert!(!full.has_empty_paths());
        assert!(!full.is_three_way());
        let empty = super::SessionSnapshot::new(super::Tab::Text, "", "right.txt", "", 0);
        assert!(empty.has_empty_paths());
        let no_base = super::SessionSnapshot::new(super::Tab::Merge, "l", "r", "   ", 0);
        assert!(no_base.is_three_way());
        assert!(no_base.has_empty_paths());
    }

    #[test]
    fn session_history_push_dedupes_caps_and_skips_home() {
        let mut history = super::SessionHistory::new();
        assert!(history.is_empty());
        history.push(super::SessionSnapshot::new(
            super::Tab::Home,
            "a",
            "b",
            "",
            0,
        ));
        assert!(history.is_empty());
        for idx in 0..10 {
            history.push(super::SessionSnapshot::new(
                super::Tab::Text,
                format!("left {idx}"),
                "right",
                "",
                idx,
            ));
        }
        assert_eq!(history.len(), super::HOME_RECENT_MAX);
        assert_eq!(history.list().first().map(|s| s.cursor), Some(9));
        assert!(history.get(100).is_none());
        history.push(super::SessionSnapshot::new(
            super::Tab::Text,
            "left 5",
            "right",
            "",
            55,
        ));
        assert_eq!(history.len(), super::HOME_RECENT_MAX);
        assert_eq!(history.list().first().map(|s| s.cursor), Some(55));
        history.clear();
        assert!(history.is_empty());
    }

    #[test]
    fn recent_list_geometry_stacks_rows_with_pad_and_gap() {
        assert_eq!(super::RECENT_ROW_GAP, super::HOME_CARD_INNER_GAP);
        assert_eq!(super::RECENT_LIST_PAD, super::HOME_CARD_PAD);
        let row = super::recent_row_min_height();
        // Text row: desc line kabhi clip na ho (button floor ka daava nahi —
        // row me button nahi, text hai; hit floor sirf card CTA par hai).
        assert!(row > crate::theme::TypoRole::HomeDesc.line_height_px());
        assert_eq!(super::recent_list_height(0), 0.0);
        let one = super::recent_list_height(1);
        assert!((one - (row + super::RECENT_LIST_PAD * 2.0)).abs() < 0.001);
        let two = super::recent_list_height(2);
        assert!(
            (two - (row * 2.0 + super::RECENT_ROW_GAP + super::RECENT_LIST_PAD * 2.0)).abs()
                < 0.001
        );
        assert_eq!(super::recent_row_top(0), super::RECENT_LIST_PAD);
        assert!(
            (super::recent_row_top(1) - (super::RECENT_LIST_PAD + row + super::RECENT_ROW_GAP))
                .abs()
                < 0.001
        );
    }

    #[test]
    fn recent_row_typo_reuses_desc_role() {
        use crate::theme::TypoRole;
        // caption() == Small; HomeDesc se variant alag par paint-identical
        // (Small.rich == HomeDesc.rich, dono 11 weak proportional).
        assert_eq!(super::recent_row_text_role(), TypoRole::caption());
        assert_eq!(super::recent_row_text_size(), TypoRole::caption().size());
        assert_eq!(super::recent_row_text_size(), TypoRole::HomeDesc.size());
        assert_eq!(
            super::recent_row_text_line_height_px(),
            TypoRole::HomeDesc.line_height_px()
        );
        assert_eq!(
            super::recent_row_text_tracking_em(),
            TypoRole::HomeDesc.letter_spacing_em()
        );
        assert_eq!(
            super::recent_row_text_weight_value(),
            TypoRole::HomeDesc.weight_value()
        );
        assert!(super::recent_row_text_role().is_small());
        assert!(super::recent_row_text_role().is_weak());
        assert_eq!(super::recent_row_text_role().family(), "proportional");
    }

    #[test]
    fn homerecent_age_label_buckets_are_honest() {
        assert_eq!(super::recent_age_label(0), "just now");
        assert_eq!(super::recent_age_label(59), "just now");
        assert_eq!(super::recent_age_label(60), "1m ago");
        assert_eq!(super::recent_age_label(3_599), "59m ago");
        assert_eq!(super::recent_age_label(3_600), "1h ago");
        assert_eq!(super::recent_age_label(86_399), "23h ago");
        assert_eq!(super::recent_age_label(86_400), "1d ago");
        assert_eq!(super::recent_age_label(172_800), "2d ago");
    }

    #[test]
    fn homerecent_snapshot_stamp_and_age_stay_graceful() {
        let plain = super::SessionSnapshot::new(super::Tab::Text, "l", "r", "", 0);
        assert_eq!(plain.saved_at_secs, 0);
        assert!(plain.age_secs(1_000).is_none());
        assert!(plain.age_label(1_000).is_none());
        let stamped = plain.with_saved_at(1_000);
        assert_eq!(stamped.saved_at_secs, 1_000);
        assert_eq!(stamped.age_secs(999), None);
        assert_eq!(stamped.age_secs(1_000), Some(0));
        assert_eq!(stamped.age_secs(1_059), Some(59));
        assert_eq!(stamped.age_label(1_000), Some(String::from("just now")));
        assert_eq!(stamped.age_label(4_600), Some(String::from("1h ago")));
    }

    #[test]
    fn homerecent_history_push_stamped_dedupes_ignoring_time() {
        let mut history = super::SessionHistory::new();
        history.push_stamped(
            super::SessionSnapshot::new(super::Tab::Text, "l", "r", "", 1),
            100,
        );
        history.push_stamped(
            super::SessionSnapshot::new(super::Tab::Folder, "a", "b", "", 0),
            200,
        );
        assert_eq!(history.len(), 2);
        history.push_stamped(
            super::SessionSnapshot::new(super::Tab::Text, "l", "r", "", 9),
            9_999,
        );
        assert_eq!(history.len(), 2);
        let first = history.list().first().map(|s| (s.cursor, s.saved_at_secs));
        assert_eq!(first, Some((9, 9_999)));
        for idx in 0..10 {
            history.push_stamped(
                super::SessionSnapshot::new(super::Tab::Table, format!("l{idx}"), "r", "", idx),
                50_000 + idx as u64,
            );
        }
        assert_eq!(history.len(), super::HOME_RECENT_MAX);
    }

    #[test]
    fn homerecent_two_line_geometry_stacks_with_rhythm() {
        use crate::theme::TypoRole;
        assert_eq!(super::RECENT_ROW_TITLE_META_GAP, super::HOME_CARD_INNER_GAP);
        let two = super::recent_row_two_line_height();
        let expected = TypoRole::HomeButton.line_height_px()
            + TypoRole::HomeDesc.line_height_px()
            + super::RECENT_ROW_TITLE_META_GAP
            + super::RECENT_LIST_PAD;
        assert!((two - expected).abs() < 0.001);
        assert!(two > super::recent_row_min_height());
        assert_eq!(super::recent_list_height_two_line(0), 0.0);
        let one = super::recent_list_height_two_line(1);
        assert!((one - (two + super::RECENT_LIST_PAD * 2.0)).abs() < 0.001);
        assert_eq!(super::recent_row_top_two_line(0), super::RECENT_LIST_PAD);
        assert!(
            (super::recent_row_top_two_line(1)
                - (super::RECENT_LIST_PAD + two + super::RECENT_ROW_GAP))
                .abs()
                < 0.001
        );
        assert_eq!(super::recent_list_height_capped(0, 5), 0.0);
        assert_eq!(
            super::recent_list_height_capped(3, 5),
            super::recent_list_height_two_line(3)
        );
        assert_eq!(
            super::recent_list_height_capped(9, 5),
            super::recent_list_height_two_line(5)
        );
        assert!(!super::recent_list_needs_scroll(5, 5));
        assert!(super::recent_list_needs_scroll(6, 5));
    }

    #[test]
    fn homerecent_title_meta_typo_reuses_roles() {
        use crate::theme::TypoRole;
        assert_eq!(super::recent_row_title_role(), TypoRole::HomeButton);
        // caption() == Small; HomeDesc se variant alag par paint-identical.
        assert_eq!(super::recent_row_meta_role(), TypoRole::caption());
        assert_eq!(super::recent_row_title_size(), TypoRole::HomeButton.size());
        assert_eq!(
            super::recent_row_title_line_height_px(),
            TypoRole::HomeButton.line_height_px()
        );
        assert_eq!(
            super::recent_row_meta_line_height_px(),
            TypoRole::HomeDesc.line_height_px()
        );
        assert_eq!(
            super::recent_row_title_weight_value(),
            TypoRole::HomeButton.weight_value()
        );
        assert_eq!(
            super::recent_row_meta_weight_value(),
            TypoRole::HomeDesc.weight_value()
        );
        assert!(super::recent_row_hierarchy_ok());
    }

    #[test]
    fn homerecent_empty_and_missing_states_stay_honest() {
        assert!(super::recent_list_empty_hint(false).contains("no recent sessions yet"));
        assert!(super::recent_list_empty_hint(true).contains("recent cleared"));
        assert!(super::recent_row_status_label(true).contains("paths missing"));
        assert!(super::recent_row_status_label(false).contains("Compare to refresh"));
        let missing = super::SessionSnapshot::new(super::Tab::Text, "", "r", "", 0);
        let meta_unknown = super::recent_row_meta_text(&missing, 5_000);
        assert!(meta_unknown.contains("paths missing"));
        assert!(!meta_unknown.contains("ago"));
        assert!(!meta_unknown.contains("just now"));
        let ready =
            super::SessionSnapshot::new(super::Tab::Text, "l", "r", "", 0).with_saved_at(1_000);
        let meta_ready = super::recent_row_meta_text(&ready, 1_060);
        assert!(meta_ready.contains("Compare to refresh"));
        assert!(meta_ready.contains("1m ago"));
        assert!(meta_ready.contains('\u{2022}'));
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn homegeo_card_hit_target_beats_cramped_button() {
        use crate::theme::{CHROME_HEADER_BAR_H, GEO_SECTION_GAP, GEO_TIGHT_GAP, TypoRole};
        // G-C1: card/pad/gap/hit-row consts theme tokens se single-source
        // (values wahi: pad/grid/section 8, inner/path-gap 4, hit-row 28).
        assert_eq!(super::HOME_CARD_PAD, GEO_SECTION_GAP);
        assert_eq!(super::HOME_GRID_GAP, GEO_SECTION_GAP);
        assert_eq!(super::HOME_CARD_INNER_GAP, GEO_TIGHT_GAP);
        assert_eq!(super::HOME_PATH_ROW_GAP, GEO_TIGHT_GAP);
        assert_eq!(super::HOME_CARD_BUTTON_MIN_H, CHROME_HEADER_BAR_H);
        assert_eq!(super::HOME_PATH_ROW_H, CHROME_HEADER_BAR_H);
        assert_eq!(super::HOME_CARD_TITLE_SIZE, TypoRole::HomeTitle.size());
        assert_eq!(
            super::HOME_CARD_BUTTON_BODY_SIZE,
            TypoRole::HomeButton.size()
        );
        assert_eq!(super::HOME_CARD_BUTTON_MIN_H, 28.0);
        assert_eq!(super::HOME_GRID_GAP, 8.0);
        assert_eq!(super::HOME_CARD_BUTTON_MIN_W, 120.0);
        // Toolbar compact floor: 96x24 (area 2304), path floor se tang.
        assert_eq!(super::HOME_TOOLBAR_BUTTON_W, 96.0);
        assert_eq!(
            super::HOME_TOOLBAR_BUTTON_W,
            super::HOME_CARD_BUTTON_MIN_W * 0.8
        );
        assert_eq!(super::HOME_TOOLBAR_BUTTON_H, 24.0);
        assert!(super::HOME_TOOLBAR_BUTTON_W < super::HOME_CARD_BUTTON_MIN_W);
        // Report ka dabaa 120x22 = 2640; naya floor 120x28 = 3360.
        assert!((super::home_card_button_hit_area() - 3_360.0).abs() < 0.001);
        assert!(super::home_card_button_hit_area() > 2_640.0);
        assert!(super::home_card_min_height() > 75.0);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn homecta_single_line_budget_fits_all_nine_labels() {
        // G-C2: CTA width full inner (240 - 2*8 = 224), purana 120 floor untouched.
        assert_eq!(super::HOME_CARD_CTA_W, 224.0);
        assert_eq!(
            super::HOME_CARD_CTA_W,
            super::HOME_CARD_MIN_W - super::HOME_CARD_PAD * 2.0
        );
        assert!(super::HOME_CARD_CTA_W > super::HOME_CARD_BUTTON_MIN_W);
        assert_eq!(super::home_card_cta_width(), super::HOME_CARD_CTA_W);
        assert_eq!(
            super::home_card_cta_text("Text Compare"),
            "Open Text Compare"
        );
        // Sabse lamba "Open Picture Compare" (20 chars): 20*7+24 = 164 <= 224.
        assert!(super::home_card_cta_single_line_ok());
        let longest = super::home_model()
            .items
            .iter()
            .map(|item| super::home_card_cta_text(item.title).chars().count())
            .max()
            .unwrap_or(0);
        assert_eq!(longest, 20);
        // Hit area 224x28 = 6272, purane 3360 se dugna.
        assert!((super::home_card_cta_hit_area() - 6_272.0).abs() < 0.001);
        assert!(super::home_card_cta_hit_area() > super::home_card_button_hit_area());
    }

    #[test]
    fn homegeo_card_title_desc_contrast_reads_title_first() {
        assert!(super::home_card_hierarchy_ok());
        assert!(
            super::home_card_title_weight_value() >= super::home_card_desc_weight_value() + 100
        );
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn homegeo_recent_columns_tile_without_clip_lies() {
        assert_eq!(super::RECENT_COL_GAP, super::HOME_CARD_INNER_GAP);
        let total = 460.0;
        let (title, kind, age) = super::recent_row_columns(total);
        assert_eq!(kind, super::RECENT_COL_KIND_W);
        assert_eq!(age, super::RECENT_COL_AGE_W);
        assert!(
            (title + kind + age + super::RECENT_LIST_PAD * 2.0 + super::RECENT_COL_GAP * 2.0
                - total)
                .abs()
                < 0.001
        );
        assert_eq!(super::recent_row_title_width(100.0), 0.0);
        assert!(super::recent_row_title_width(f32::NAN).is_nan());
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn homegeo_home_hides_picker_and_keeps_section_rhythm() {
        use crate::theme::GEO_SECTION_GAP;
        assert!(!super::HOME_SHOWS_PATH_PICKER);
        assert_eq!(super::home_saved_chrome_height(), 32.0);
        assert_eq!(super::home_content_top_offset(false), 0.0);
        assert_eq!(super::home_content_top_offset(true), 32.0);
        assert_eq!(super::HOME_SECTION_GAP, 8.0);
        assert_eq!(super::HOME_SECTION_GAP, GEO_SECTION_GAP);
        assert!((super::home_recent_section_top(200.0) - 208.0).abs() < 0.001);
        assert!(super::home_recent_section_top(f32::NAN).is_nan());
    }

    #[test]
    fn session_open_activates_and_home_is_never_a_session() {
        let mut list = super::SessionList::new();
        assert!(list.is_empty());
        assert_eq!(list.active_id(), None);
        let first = list.open(super::Tab::Text, "a.txt", "b.txt", "");
        assert_eq!(first, Some(0));
        assert_eq!(list.active_id(), Some(0));
        assert_eq!(list.len(), 1);
        // Home no-op: na session banta hai, na id consume hota hai.
        assert_eq!(list.open(super::Tab::Home, "a", "b", ""), None);
        assert_eq!(list.len(), 1);
        assert_eq!(list.active_id(), Some(0));
        let second = list.open(super::Tab::Folder, "l", "r", "");
        assert_eq!(second, Some(1));
        assert_eq!(list.active_id(), Some(1));
    }

    #[test]
    fn session_title_derives_stem_and_falls_back() {
        assert_eq!(
            super::session_title_for(super::Tab::Text, "a/b/left.txt", "", 3),
            "left"
        );
        assert_eq!(
            super::session_title_for(super::Tab::Folder, "a/b/mydir/", "", 4),
            "mydir"
        );
        assert_eq!(
            super::session_title_for(super::Tab::Text, "", "r.txt", 5),
            "r"
        );
        assert_eq!(
            super::session_title_for(super::Tab::Text, "", "   ", 6),
            "Text 6"
        );
        assert_eq!(
            super::session_title_for(super::Tab::FolderMerge, "", "", 7),
            "FolderMerge 7"
        );
        let mut list = super::SessionList::new();
        let id = list
            .open(super::Tab::Text, "  spaced.txt ", "b", "")
            .unwrap_or(99);
        assert_eq!(list.get(id).map(|s| s.title.as_str()), Some("spaced"));
        assert_eq!(list.get(id).map(|s| s.left.as_str()), Some("spaced.txt"));
    }

    #[test]
    fn session_close_picks_neighbour_and_unknown_is_noop() {
        let mut list = super::SessionList::new();
        assert!(!list.close(42));
        let a = list.open(super::Tab::Text, "a", "b", "").unwrap_or(99);
        let b = list.open(super::Tab::Folder, "c", "d", "").unwrap_or(99);
        let c = list.open(super::Tab::Hex, "e", "f", "").unwrap_or(99);
        // Beech wala band: active (c) untouched.
        assert!(list.close(b));
        assert_eq!(list.active_id(), Some(c));
        assert_eq!(list.len(), 2);
        // Active aakhri band: left-neighbour (a) active.
        assert!(list.close(c));
        assert_eq!(list.active_id(), Some(a));
        // Active pehla band: right-neighbour active hota, yahan khaali.
        assert!(list.close(a));
        assert_eq!(list.active_id(), None);
        assert!(list.is_empty());
        assert!(!list.close(a));
    }

    #[test]
    fn session_close_first_active_picks_right_neighbour() {
        let mut list = super::SessionList::new();
        let a = list.open(super::Tab::Text, "a", "b", "").unwrap_or(99);
        let b = list.open(super::Tab::Folder, "c", "d", "").unwrap_or(99);
        assert!(list.switch(a));
        assert!(list.close(a));
        assert_eq!(list.active_id(), Some(b));
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn session_switch_and_close_others_gate_unknown() {
        let mut list = super::SessionList::new();
        let a = list.open(super::Tab::Text, "a", "b", "").unwrap_or(99);
        let b = list.open(super::Tab::Folder, "c", "d", "").unwrap_or(99);
        assert_eq!(list.active_id(), Some(b));
        assert!(list.switch(a));
        assert_eq!(list.active().map(|s| s.id), Some(a));
        assert!(!list.switch(999));
        assert_eq!(list.active_id(), Some(a));
        assert!(!list.close_others(999));
        assert_eq!(list.len(), 2);
        assert!(list.close_others(b));
        assert_eq!(list.len(), 1);
        assert_eq!(list.active_id(), Some(b));
    }

    #[test]
    fn session_cap_drops_oldest_inactive_never_active() {
        let mut list = super::SessionList::new();
        for idx in 0..super::HOME_RECENT_MAX {
            let id = list
                .open(super::Tab::Text, format!("l{idx}"), "r", "")
                .unwrap_or(99);
            assert_eq!(id, idx as u64);
        }
        assert_eq!(list.len(), super::HOME_RECENT_MAX);
        // Sabse purana (id 0) active rakho, phir 9th kholo.
        assert!(list.switch(0));
        let ninth = list.open(super::Tab::Text, "new", "r", "");
        assert_eq!(ninth, Some(super::HOME_RECENT_MAX as u64));
        assert_eq!(list.len(), super::HOME_RECENT_MAX);
        // Oldest inactive (id 1) gira, active (id 0) bacha.
        assert!(list.get(1).is_none());
        assert_eq!(list.get(0).map(|s| s.id), Some(0));
        assert_eq!(list.active_id(), ninth);
        // Id reuse kabhi nahi: band ke baad bhi next_id badhta hai.
        assert!(list.close(ninth.unwrap_or(99)));
        let reopened = list.open(super::Tab::Text, "x", "y", "");
        assert_eq!(reopened, Some(super::HOME_RECENT_MAX as u64 + 1));
    }

    #[test]
    fn session_update_paths_cursor_and_rename_trim() {
        let mut list = super::SessionList::new();
        let id = list.open(super::Tab::Text, "a", "b", "").unwrap_or(99);
        assert!(list.update_paths(id, "  l.txt ", "r.txt", " "));
        assert_eq!(list.get(id).map(|s| s.left.as_str()), Some("l.txt"));
        assert_eq!(list.get(id).map(|s| s.base.as_str()), Some(""));
        // Title T5 par untouched (rename hook alag hai).
        assert_eq!(list.get(id).map(|s| s.title.as_str()), Some("a"));
        assert!(list.update_cursor(id, 7));
        assert_eq!(list.get(id).map(|s| s.cursor), Some(7));
        assert!(!list.update_paths(999, "x", "y", ""));
        assert!(!list.update_cursor(999, 1));
        // Blank rename no-op, unknown no-op, trim rename ok.
        assert!(!list.rename(id, "   "));
        assert_eq!(list.get(id).map(|s| s.title.as_str()), Some("a"));
        assert!(!list.rename(999, "zed"));
        assert!(list.rename(id, "  custom  "));
        assert_eq!(list.get(id).map(|s| s.title.as_str()), Some("custom"));
    }

    #[test]
    fn session_stamp_and_age_mirror_snapshot_precedent() {
        let plain = super::Session::new(3, super::Tab::Text, "l", "r", "", 0);
        assert_eq!(plain.saved_at_secs, 0);
        assert!(!plain.dirty);
        assert!(plain.age_secs(1_000).is_none());
        assert!(plain.age_label(1_000).is_none());
        let stamped = plain.with_saved_at(1_000);
        assert_eq!(stamped.age_secs(999), None);
        assert_eq!(stamped.age_secs(1_060), Some(60));
        assert_eq!(stamped.age_label(1_060), Some(String::from("1m ago")));
    }
}
