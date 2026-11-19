# Local-Run & Binary Early Feedback Report (Wave L LOCALBIN)

- **Date**: 2026-11-20
- **Workspace**: `/Users/ektasaini/autocompare`
- **Agent Role**: Wave L LOCALBIN agent (local run + binary early feedback)
- **Scope & Boundaries**: Strictly `checks/local-run-l/**` only. Zero source edits across `crates/**` (`views.rs`, `app.rs`, `theme.rs`, etc. untouched). `STATUS.md` untouched. Local git commit only, zero pushes.
- **Reference Comparator**: Beyond Compare (PID 23316, `/Applications/Beyond Compare.app`), window `t_left.txt <--> t_right.txt - Text Compare`. Clean-room observations only (own words, zero copied strings/assets/layout text).

---

## 1. Release Build & Runtime Metrics

| Metric | Target / Command | Result | Status |
| :--- | :--- | :--- | :--- |
| **Cargo Build** | `cargo build --release -p autocompare-gui` | Exit `0`, Wall: `55.48s`, Cargo: `55.43s` | PASS |
| **Binary Path** | `target/release/autocompare-gui` | Created successfully | PASS |
| **Binary Size** | `stat -f%z target/release/autocompare-gui` | `11,022,272` bytes (~10.51 MB) | PASS |
| **Headless Run** | `target/release/autocompare-gui --screenshot ...` | Exit `0`, clean headless execution | PASS |

---

## 2. Headless Screenshot PNG Proof

| Check | Expected | Observed | Status |
| :--- | :--- | :--- | :--- |
| **File Path** | `checks/local-run-l/after-L.png` | Exists | PASS |
| **File Size** | >= 8,192 bytes | `9,914` bytes | PASS |
| **Magic Header**| `89 50 4e 47 0d 0a 1a 0a` | `89504e470d0a1a0a` (PNG) | PASS |
| **Distinct Bytes**| > 64 (non-blank proof) | `256` unique byte values | PASS |
| **Resolution** | 960x600 RGBA | PNG image data, 960 x 600, 8-bit/color RGBA | PASS |
| **SHA256 Hash** | Consistent fixture render | `2f4e26839bd08de2` | PASS |

---

## 3. Test Suite Count

```text
running 193 tests
test app::tests::dropped_paths_populate_active_tab ... ok
test app::tests::folder_counts_line_uses_honest_summary ... ok
...
test screenshot::tests::dispatcher_routes_by_file_name ... ok

test result: ok. 193 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
```

- **Total Test Count**: `193` tests
- **Passed**: `193` (100% pass rate)
- **Failed / Ignored**: `0` failed, `0` ignored
- **Execution Time**: `0.31s`

---

## 4. Geo-Typo Exquisite Polish Harness (`round-L-1`)

Executed: `tools/gui-polish/run-geo-typo.sh checks/local-run-l/round-L-1`
Result: **RESULT: PASS** (Exit Code `0`)

- **PNG Validation**: 9,914 bytes (min 8,192: OK), magic OK, distinct 256 (OK)
- **Geometry AST Scan**:
  - `add_space` calls: `0` (clean removal in favor of structured geometry tokens)
  - `Grid::new` usage: `1`
  - `text_style_height`: `3` occurrences
  - `width` constraints: `9` width calls
- **Typography AST Scan**:
  - `monospace`: `33`
  - `strong`: `6`
  - `small`: `30`
  - Rich `.size(...)` values: `['14.0']` (single controlled usage for Home card title)
  - `row_height`: `3`
- **Color Boundaries Check**:
  - `Color32` / `Visuals` touches: `22` (unchanged, strictly out of scope)

---

## 5. Beyond Compare Clean-Room Gap Analysis

Clean-room external comparison against running reference instance (`/Applications/Beyond Compare.app`, PID 23316) comparing text files side-by-side. All observations expressed strictly in original terms without copying any layout assets, proprietary text, or copyrighted strings.

### Gap Summary Table

| ID | Category | Gap Description | Severity | Suggested Owner |
| :--- | :--- | :--- | :---: | :---: |
| **GAP-01** | Geometry | Static 50/50 split vs Interactive Draggable Splitter | **P0** | **GEO** |
| **GAP-02** | Usability | Missing Toolbar Navigation Controls (Next/Prev Diff Buttons) | **P0** | **PARITY** |
| **GAP-03** | Usability | Missing 1-Click Margin Copy Arrows (Left-to-Right / Right-to-Left) | **P0** | **PARITY** |
| **GAP-04** | Geometry | Unaligned Header Path Inputs & Missing Per-Pane Actions | **P1** | **GEO** |
| **GAP-05** | Geometry | Minimap / Overview Gutter is Static (No Click-to-Jump Navigation) | **P1** | **GEO** |
| **GAP-06** | Typography | Fixed Monospace Font Scale & Missing Runtime Zoom (Cmd +/-) | **P1** | **TYPO** |
| **GAP-07** | Typography | Cramped Line-Height Vertical Rhythm vs Ergonomic Proportions | **P1** | **TYPO** |
| **GAP-08** | Usability | Missing In-Pane Interactive Search / Find Drawer | **P1** | **PARITY** |
| **GAP-09** | Geometry | Hex View Lacks Column Delimiters & Midline Octet Spacing | **P1** | **GEO** |
| **GAP-10** | Geometry | Table View Lacks Resizable Spreadsheet Headers & Frozen Header Bar | **P1** | **GEO** |
| **GAP-11** | Typography | Flat Monochromatic Contrast in Chrome vs Differentiated Hierarchy | **P2** | **TYPO** |
| **GAP-12** | Usability | Hard Truncation (`MAX_ROWS`) Without Pagination or Virtual Scroll | **P2** | **PARITY** |
| **GAP-13** | Usability | Folder Compare Lacks Dual Synchronized Trees & Action Menus | **P2** | **PARITY** |
| **GAP-14** | Usability | 3-Way Merge Lacks 3-Pane Top Layout with Bottom Output Editor | **P2** | **PARITY** |

---

### Detailed Findings by Category

#### A. Geometry Gaps (Widths, Margins, Alignment, Positioning)

1. **Static 50/50 Pane Split vs Resizable Center Divider (P0 - GEO)**:
   - *Observation*: Beyond Compare allows dynamic user adjustment of the central splitter dividing left and right panes via a draggable divider handle. Autocompare-gui statically allocates pane widths (`available_width / 2.0`), leaving wide lines clipped on one side while the opposite pane holds empty space.
   - *File Ref*: `crates/gui/src/views.rs:450-480`, `crates/gui/src/app.rs`.

2. **Unaligned Path Pickers & Header Margins (P1 - GEO)**:
   - *Observation*: Beyond Compare anchors separate path inputs directly on top of their corresponding panes with matching padding, browse buttons, and reload icons. Autocompare-gui renders path inputs in a global header bar that does not line up with the visual vertical split of the diff panes below.
   - *File Ref*: `crates/gui/src/app.rs:1200-1250`.

3. **Gutter Overview Minimap Alignment & Interactivity (P1 - GEO)**:
   - *Observation*: Beyond Compare places a full-height proportional mini-map strip on the right edge showing diff bands at exact vertical document positions with click-to-jump viewport scrolling. Autocompare-gui renders a narrow 6px/12px bar (`gutter.rs`), but it is visually disconnected from the scroll track and lacks click-event positioning.
   - *File Ref*: `crates/gui/src/gutter.rs:30-85`.

4. **Hex View Grid Layout (P1 - GEO)**:
   - *Observation*: Beyond Compare formats hex comparison into three distinct, rigid columns: 8-digit address, 16 hex bytes separated into two groups of 8 with a wider midline gutter, and 16 ASCII characters. Autocompare-gui computes cursor math but renders rows without distinct visual borders or midline spacing.
   - *File Ref*: `crates/gui/src/views_hex.rs:25-70`.

5. **Table Grid Alignment (P1 - GEO)**:
   - *Observation*: Beyond Compare displays structured table comparisons inside an authentic data grid with individual column widths, header separators, and cell border padding. Autocompare-gui renders table rows as formatted monospace strings with calculated padding rather than a native grid.
   - *File Ref*: `crates/gui/src/views_table.rs:300-360`.

---

#### B. Typography Gaps (Family, Weight, Line-Height, Spacing)

1. **Fixed Monospace Scale & Missing User Zoom (P1 - TYPO)**:
   - *Observation*: Beyond Compare allows dynamic font scaling via keyboard shortcuts (`Cmd +` / `Cmd -`) and font preferences (SF Mono, Menlo, JetBrains Mono, etc.). Autocompare-gui hardcodes font tokens to `12.0px` (`TYPO_MONO_SIZE`) with no runtime font-size scaling.
   - *File Ref*: `crates/gui/src/theme.rs:30-55`.

2. **Cramped Line-Height & Vertical Rhythm (P1 - TYPO)**:
   - *Observation*: Beyond Compare uses a comfortable line height (~1.3x to 1.4x font size) that cleanly separates lines of code during extensive scanning. Autocompare-gui relies on egui's default tight line box (`~14px` on `12px` font), creating dense visual clutter in diff-heavy sections.
   - *File Ref*: `crates/gui/src/theme.rs:25-32`, `crates/gui/src/views.rs`.

3. **Intra-Line Character Differentiation (P1 - TYPO)**:
   - *Observation*: When a line has internal edits, Beyond Compare isolates changed characters with distinct background shading, darker text weights, or subtle outlines while maintaining monospaced baseline alignment. Autocompare-gui relies on segmented accent text chunks which can feel abrupt.
   - *File Ref*: `crates/gui/src/views.rs:360-410`.

4. **Chrome Typographic Hierarchy (P2 - TYPO)**:
   - *Observation*: Beyond Compare differentiates session tabs, breadcrumbs, status counts, and encoding labels using subtle proportional styling and tonal hierarchy. Autocompare-gui relies heavily on monospace labels across status bars and secondary headers, giving the app an unpolished developer-tool feel.
   - *File Ref*: `crates/gui/src/theme.rs:18-35`.

---

#### C. Feature & Usability Gaps (Parity Blockers)

1. **Missing Next/Previous Diff Toolbar Controls (P0 - PARITY)**:
   - *Observation*: The core workflow of diff tools is stepping through changes. Beyond Compare features prominent, large toolbar buttons for Next Diff, Prev Diff, and First Diff with active keyboard shortcuts. In autocompare-gui, diff navigation is hidden behind F3 keyboard bindings without visible toolbar buttons.
   - *File Ref*: `crates/gui/src/app.rs`, `crates/gui/src/views.rs`.

2. **Missing 1-Click Gutter Merge / Copy Buttons (P0 - PARITY)**:
   - *Observation*: Beyond Compare allows users to copy an individual difference hunk or line from Left to Right (or Right to Left) with a single click on directional arrows located directly inside the divider margin. Autocompare-gui has backend copy models (`copy.rs`) but lacks interactive visual buttons on each hunk line in the GUI.
   - *File Ref*: `crates/gui/src/copy.rs`, `crates/gui/src/views.rs:440-510`.

3. **Missing Find / Search Drawer (P1 - PARITY)**:
   - *Observation*: Beyond Compare provides an in-editor search bar showing total match counts, current match index, and jump controls. Autocompare-gui has no search UI in the text view.
   - *File Ref*: `crates/gui/src/views.rs`.

4. **Truncation Cap Without Virtual Scroll / Paging (P2 - PARITY)**:
   - *Observation*: Beyond Compare handles multi-megabyte files smoothly with high-performance virtualized scrolling. Autocompare-gui enforces a hard `MAX_ROWS` limit (e.g., 5,000 lines), showing a static truncation message without pagination or on-demand streaming.
   - *File Ref*: `crates/gui/src/viewmodel.rs`, `crates/gui/src/views.rs:465-475`.

5. **3-Way Merge Structure (P2 - PARITY)**:
   - *Observation*: Beyond Compare provides a true 3-way merge window: three comparison panes at the top (Left, Base/Ancestor, Right) and an editable result preview pane at the bottom with hunk conflict pickers. Autocompare-gui renders a single linear merged output list with coarse file-level resolution.
   - *File Ref*: `crates/gui/src/views.rs:315-345`.

---

## 6. Top-5 Prioritized Fix Proposals for Next Waves

1. **Interactive Draggable Center Splitter (Wave M - GEO)**:
   - Implement an egui draggable separator widget between left and right text panes, storing the split fraction in `App` state (defaulting to 0.50).
   - Ensure resize clamps within `0.20..=0.80` to prevent pane collapse.

2. **Next/Previous Diff Toolbar Navigation Controls (Wave M - PARITY)**:
   - Add a dedicated navigation toolbar directly above the diff view containing: `[ |< First ]`, `[ < Prev Diff ]`, `[ Next Diff > ]`, and diff count badge `[ N / Total ]`.
   - Wire buttons directly to `select_previous_difference` and `select_next_difference`.

3. **1-Click Gutter Copy/Merge Glyphs (Wave N - PARITY)**:
   - In the center margin between panes, render clickable directional arrow buttons (`->` and `<-`) aligned with each difference hunk.
   - Wire clicks to trigger `apply_copy_section` or `copy_line` to achieve real interactive merging parity.

4. **Configurable Typographic Zoom & Line-Spacing (Wave N - TYPO)**:
   - Introduce dynamic font scaling (`zoom_level: f32`) with keyboard bindings `Cmd+=` (zoom in), `Cmd+-` (zoom out), and `Cmd+0` (reset).
   - Increase default line height ratio from dense 1.15x to comfortable 1.35x for improved readability.

5. **Click-to-Jump Navigation on Overview Minimap Gutter (Wave O - GEO/PARITY)**:
   - Make the `gutter.rs` overview bar interactive: register mouse click/drag events on the minimap rect, map the vertical click fraction to the row index, and update scroll position.

---

## 7. Governance & Verification Summary

- **Source Code Integrity**: Zero modifications outside `checks/local-run-l/**`.
- **Status Files**: `STATUS.md` untouched.
- **Git Discipline**: Local commit only, zero remote push operations.
- **Harness Verification**: Passed headless checks and full 193-test suite.
