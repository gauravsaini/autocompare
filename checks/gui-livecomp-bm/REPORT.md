# LIVE visual compare vs Beyond Compare — gap REPORT (round-1)

Scope: geometry (width/margin/padding/alignment) + typography (family/weight/size/line-height). Colors FROZEN — is report me color par koi spec nahi, sirf note.

Inputs dono apps me SAME the (fixtures: t_left/t_right.txt, folder/left+right, tab_left/tab_right.csv). Shots: `round-1/live-*.png` (fullscreen) + `round-1/crop-*.png` (window crops). Sab `file` magic-verified PNG hain.

BC state honest note: BC pehle se khula tha text session ke saath; folder+table tabs `open -a` se khole, koi close nahi kiya. Humara app pid 1059 running chhoda hai.

## Shot map

| # | ours | BC | view |
|---|------|----|------|
| 1 | live-ours-home.png | — | Home cards |
| 2 | live-ours-text.png / crop-ours-text.png / live-ours-text-zoom.png | live-bc-text.png / crop-bc-text.png / live-bc-text-zoom.png | Text Compare, same fixtures |
| 3 | live-ours-folder.png / crop-ours-folder.png | live-bc-folder.png / crop-bc-folder.png | Folder Compare, same fixtures |
| 4 | live-ours-table.png / crop-ours-table.png | live-bc-table.png / crop-bc-table.png | Table Compare, same fixtures |
| 5 | — | live-bc-asfound.png | BC as-found (text session, user ka apna) |

## Gap table (geometry + typography only)

Rank: P0 = layout toot ta hai / debug artifact; P1 = BC parity ke liye zaroori spacing/type fix; P2 = chrome polish.

### TEXT view (ours: `crates/gui/src/views.rs` paint_text:2309 / paint_text_body:2328; BC: Text Compare)

| ID | rank | gap (observed) | exact follow-up spec (view/fn/lines, kya change, OWNS file) |
|----|------|----------------|---------------------------------------------------------------|
| G-T1 | P0 | Ours me ek hi combined row hai `Lnum Ltext \| Rnum Rtext` aur copy buttons (`→`/`←`) panes ke BEECH inline hain. BC me do independent panes + beech me patli center gutter hai jisme per-diff ek arrow hai. | views.rs `paint_text_body` (~2328+) ki row layout badlo: left pane column + fixed-width gutter column + right pane column. Copy buttons gutter me le jao. OWNS: `crates/gui/src/views.rs` (paint-only). |
| G-T2 | P1 | Diff highlight ours me tight text-span par hai (`beta` ke gird), BC me poori row-width band hai. (Color nahi — WIDTH ki baat hai.) | Highlight rect ko row ki full available width tak phailao, `paint_text_body` me. OWNS: `crates/gui/src/views.rs`. |
| G-T3 | P1 | Line-number gutter: ours me number text ke left chipka hai + sabse left me chhote `•` dots hain. BC me numbers right-aligned, fixed-width dedicated gutter, koi dots nahi, content se clear gap hai. | Number gutter ko fixed width do, numbers right-align karo, `•` dot column hatao. OWNS: `crates/gui/src/views.rs` (gutter paint) + `theme.rs` TypoRole sizes. |
| G-T4 | P1 | Status line ours me rows ke UPAR hai (`3 rows (3 shown): 1 diffs, 2 same \| no selection`). BC me status BOTTOM bar me hai (cursor `1:1`, format `Default text`, mode flags). | Status ko bottom bar me lao; cursor/format info add karo. OWNS: `crates/gui/src/views.rs` (`paint_text_body` status) + `app.rs` Text tab branch. |
| G-T5 | P2 | Ours title bar + breadcrumb me FULL absolute paths hain (truncate hokar `...` ). BC title me sirf basename pair hai (`t_left.txt <--> t_right.txt`), paths alag dedicated path bars me (tail-visible). | Window title basename-pair rakho; path fields tail-elide dikhao. OWNS: `app.rs` (viewport title) + `views.rs` (path row). |
| G-T6 | P2 | Toolbar order: BC me filter row (All/Diffs/Same...) + nav arrows path bars ke UPAR hain. Ours me Swap/Reload + check row upar hai par Find + Show-filters rows content ke beech bikhre hain. | Find + Show-filters ko ek compact toolbar row me path bars ke neeche lao. OWNS: `crates/gui/src/views.rs`. |
| G-T7 | P1 | Row leading/density: BC rows me zyada vertical padding hai (3 rows hawaadar), ours compact hain. Family note: code me content pehle se `.monospace()` hai (views.rs:4408/4427/4437 waghera) aur theme me `TYPO_MONO_STACK` + `TYPO_LINE_HEIGHT_SOURCE` defined hai (theme.rs:78,91) — isliye gap family ka nahi, SIZE/LEADING/WEIGHT ka hai. | `theme.rs` Mono role ka size/leading BC pitch tak badhao; font-probe harness se measured row-height verify karo. OWNS: `crates/gui/src/theme.rs` (paint-only). Devanagari nahi — sirf metrics. |

### FOLDER view (ours: `views.rs` paint_folder:4305, summary `tree.rs:328` / `viewmodel.rs:773`; BC: Folder Compare)

| ID | rank | gap (observed) | exact follow-up spec |
|----|------|----------------|----------------------|
| G-F1 | P0 | Red debug overlay `First use of ScrollArea ID D92C` row 1 (`R extra.txt`) ke upar paint hua hai. Ye string humare crates me kahin nahi (grep zero) — egui framework ka emitted text hai jo hamare ScrollArea usage se trigger hua. | Folder paint path me `ScrollArea` ko stable id do (har frame same id, single `show` call); overlay gayab hona + rows clear dikhna acceptance hai. OWNS: `crates/gui/src/views.rs` `paint_folder` (~4305–4420) + `app.rs` Folder branch. Koi framework edit nahi. |
| G-F2 | P0 | Ours rows plain text hain (`R extra.txt`, `< f.txt`) — Name/Size/Modified COLUMNS nahi hain. BC me dono sides me 3 aligned columns hain (Name/Size/Modified headers ke saath). | Folder rows ko grid columns (Name/Size/Modified per side) me paint karo. OWNS: `crates/gui/src/views.rs` `paint_folder` + `tree.rs` FolderDisplay model. |
| G-F3 | P1 | Summary line DUPLICATE hai — top (`2 entries: 0 same...`) aur bottom me same string dobara. BC me bottom me per-side counts hain. | Ek hi summary rakho (bottom status pattern), doosri hatao. OWNS: `crates/gui/src/views.rs` / `app.rs` Folder branch. |
| G-F4 | P1 | Orphan row (`extra.txt` right-only) full-row band highlight nahi hai; BC me poori width band hai. (Sirf geometry — color frozen.) | Status rows ka highlight rect full row width tak. OWNS: `crates/gui/src/views.rs`. |
| G-F5 | P2 | Ours status ek hi lambi string hai; BC me left/right alag counts + neeche log panel hai. | Summary ko per-side counts me todo (log panel is scope se bahar — alag wave). OWNS: `tree.rs:328` summary builder + `views.rs`. |

### TABLE view (ours: `views.rs` paint_table:1986 / paint_table_keyed:1490, note `views_table.rs:29-31`; BC: Table Compare)

| ID | rank | gap (observed) | exact follow-up spec |
|----|------|----------------|----------------------|
| G-B1 | P0 | Header row DATA row ki tarah paint hui hai (`= id name role` + neeche dashed `----` separator). BC me asli header hai: column letters (A/B/C) + styled header row + row numbers. | Header ko grid header banao; `=` marker aur dashed separator row hatao. OWNS: `crates/gui/src/views.rs` `paint_table` (1986) / `paint_table_keyed` (1490). |
| G-B2 | P0 | Duplicate control rows: 2x Find bars, 2x `no key columns...` notes, 2x Prev/Next page rows (summary ke upar ek set, neeche ek set). BC me single detail pane hai. | Single Find + single nav + single note rakho, baaki hatao. OWNS: `crates/gui/src/views.rs` `paint_table*` + `app.rs` Table branch. |
| G-B3 | P0 | Cells space-padded text flow me hain (`[ops => lead]` inline). BC me aligned grid cells hain, diff cell alag (geometry: cell rect). | Table ko `Grid::new` columns me paint karo, diff per-cell rect me. OWNS: `crates/gui/src/views.rs` + `views_table.rs` cell helpers. |
| G-B4 | P1 | Row numbers + column letters (A/B/C) missing hain; BC me dono hain. | Row-number gutter + column-letter header row add karo. OWNS: `crates/gui/src/views.rs`. |
| G-B5 | P2 | Summary wording (`table keyed: 1 matching, 0 modified...`) vs BC (`1 difference row(s) \| Same`). | Wording/layout BC pattern par lao. OWNS: `viewmodel.rs` / `views.rs`. |

### CHROME — all tabs (tab strip `app.rs`, path rows `views.rs`)

| ID | rank | gap (observed) | exact follow-up spec |
|----|------|----------------|----------------------|
| G-C1 | P2 | Tab strip: ours checkbox-pills ek line me hain (theek), par active-pill + spacing BC session tabs (icon + label + x, top strip) se halki hai. Koi toot nahi hai. | Spacing/padding tokens se polish karo. OWNS: `app.rs` tab strip + `theme.rs`. |
| G-C2 | P2 | Path fields lamba path head-se truncate karte hain (beech ka hissa gayab). BC path bar tail dikhata hai (basename visible). | Path display tail-elide karo taaki filename dikhe. OWNS: `crates/gui/src/views.rs` path rows. |

## Colors — FROZEN (note only, koi spec nahi)
BC diff bands (lavender/blue), orphan purple, table diff pink — ye sab color wave ke liye parked hain. Upar ke specs me jahan "band/rect" likha hai wahan sirf GEOMETRY (full-width rect, grid cell) matlab hai, color nahi.

## Kya verify ho gaya
- 9 fullscreen live PNGs + 6 window crops + 2 zoom crops, sab PNG magic-verified (GATES.md dekho).
- Har gap ke saath exact fn/line + OWNS file di hai. Koi `.rs` edit nahi kiya gaya (git status me sirf `checks/gui-livecomp-bm/` naya hoga).
