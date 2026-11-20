# Wave EXQ-TYPO10 — REPORT (round-1)

Scope sirf OWNS: `crates/gui/src/theme.rs`, `crates/gui/src/viewmodel.rs`, `crates/gui/src/tree.rs`, `checks/gui-exq-theme10/**`.
Paralleled internally via `/swarm` (3 sub-tasks: typo-auditor, viewmodel-density, tree-alignment).
BC vs ours gap honest: BC PNG ~888K, ours `after.png` ~10K — paint wave ka kaam baaki hai; is wave me typography rhythm, viewmodel density tokens aur folder tree alignment focus.
Paint-neutral headless hai, isliye PNG byte same raha (10401 bytes); contract headless tests aur edge fixtures se locked hai.

## 1. TypoRole rhythm + line-height audit (`theme.rs`)

- **Line-height monotonic rhythm:**
  * Small (11px): 13.2px (`TYPO_LINE_HEIGHT_CHROME` 1.2)
  * Mono (12px): 15.6px (`TYPO_LINE_HEIGHT_MONO` 1.30)
  * Body (13px): 18.2px (`TYPO_LINE_HEIGHT_BODY` 1.40)
  * Heading (18px): 21.6px (`TYPO_LINE_HEIGHT_CHROME` 1.2)
  * Strict monotonic ordering: `Small < Mono < Body < Heading` eliminates line-height inversions.
- **Monospace consistency code panes:**
  * `TYPO_MONO_SIZE` 12.0px (MAX 13.0px), `TypoRole::Mono` is content-only (`!is_chrome()`), tabular (`is_tabular()`), regular weight 400.
  * Aliases single-sourced: `pane_body()`, `content_grid()`, `content_textcenter()`, `gutter_text()`.
- **Proportional chrome & weight hierarchy:**
  * App header / section: Heading 18 (medium/strong)
  * Tab / body: Body 13 (regular 400)
  * Status / meta: Small 11 (weak tone, semibold counts)
- **Letter spacing:**
  * `TYPO_LETTER_SPACING` = 0.0px runtime no-op (egui compatibility).
  * `TYPO_TRACKING_HEADING_EM` = 0.02em headless intent on Heading & HomeTitle only.
- **Gate:** `exq_typo10_roles_plus_rhythm_ok()` locks all rhythm, line-height, and typography constraints.

## 2. Viewmodel density + row-height tokens (`viewmodel.rs`)

- **RowDensity enum:** `Compact`, `Comfortable` (default), `Spacious`.
- **Text row-height tokens:**
  * `TEXT_ROW_HEIGHT_COMPACT` = 18.0px (table grid parity; 15.6px mono se 2.4px padding floor -> 0% clipping)
  * `TEXT_ROW_HEIGHT_COMFORTABLE` = 24.0px (`SPACE_XL`, screenshot/tree parity)
  * `TEXT_ROW_HEIGHT_SPACIOUS` = 28.0px (`CHROME_HEADER_BAR_H` parity)
- **Folder row-height tokens:**
  * `FOLDER_ROW_HEIGHT_COMPACT` = 20.0px (`SPACE_LG` 16.0 + `SPACE_XS` 4.0; body 18.2px se 1.8px safe padding floor -> 0% font ascender/descender clipping)
  * `FOLDER_ROW_HEIGHT_COMFORTABLE` = 24.0px (`tree::FOLDER_ROW_HEIGHT` parity)
  * `FOLDER_ROW_HEIGHT_SPACIOUS` = 28.0px (`CHROME_HEADER_BAR_H` parity)
- **Viewport density audit:**
  * `audit_viewmodel_density() -> ViewportDensityAudit` proves `zero_clipping_verified == true`.
  * Canonical 960x600 (panes_h = 476): Text capacities [26, 19, 17], Folder [23, 19, 17].
  * Fallback 720x450 (panes_h = 326): Text capacities [18, 13, 11], Folder [16, 13, 11].

## 3. Tree.rs folder-tree indent + alignment (`tree.rs`)

- **Folder-tree indent:** `tree_indent_step()` = 12.0px (`SPACE_MD`), `tree_indent_for_depth(depth) = depth as f32 * 12.0`.
- **Density integration:** `folder_row_height_for_density(density: FolderDensity) -> f32` maps density modes to 20/24/28px.
- **Expander & Columnar Alignment:**
  * Fixed expander cell: `FOLDER_EXPANDER_W` = 16.0px (`SPACE_LG`).
  * Hit target: `FOLDER_EXPANDER_HIT_H` = 24.0px (full row height).
  * Separated status mark layout: `TREE_STATUS_MARK_W` = 16.0px, `TREE_STATUS_GAP` = 8.0px, path origin `depth * 12.0 + 40.0` ensures file and directory entries align to the exact same column baseline.

## 4. Edge Fixture & Harness

- Edge fixture: `checks/gui-exq-theme10/round-1/comparison/empty-vs-nonempty.json`
- Headless test: `exq_typo10_empty_vs_nonempty_rhythm_and_density_edge` proves empty comparison (0 rows, 4 default gutter chars) and nonempty comparison (2 rows, line numbers, inline diff) with zero vertical/horizontal clipping.
- `tools/exquisite/run.sh checks/gui-exq-theme10/round-1 crates/gui/src`: **RESULT: PASS**
- Colors FROZEN: `Color32/Visuals` touches 33 — growth 0.
