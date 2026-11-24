import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { textApplyEdit, textApplyHunk, textCompare, textCompareDemo, textFind, textGetOverview, textGetRows, textSaveSide } from "../../ipc/text";
import type { OverviewChunk, TextFileMeta, TextRowDto, TextSide } from "../../ipc/text";
import { arrayField, errString, isRecord } from "./types";
import type { ViewPaths } from "./types";
import { FindOverlay, FormatMenu, GotoOverlay, IgnoreMenu, OverviewStrip, TextToolbar } from "./text/controls";
import { VIEW_FORMAT_EVENT } from "./text/controls/FormatMenu";
import { Button } from "../ui";
import type { OverviewStripHunk } from "./text/controls/OverviewStrip";
import { RowLine } from "./text/RowLine";
// Wave-161: bus event names shared with the top Toolbar (leaf module —
// no chrome barrel import, so no view/chrome cycle).
import { VIEW_COMMAND_EVENT, VIEW_MINOR_EVENT } from "../chrome/ViewCommandBus";

/**
 * Side-by-side text comparison view (Wave 2B, real-data first paint Wave 5B).
 *
 * IPC: `text_compare` (summary + session) then paged `text_get_rows`
 * windows of `PAGE_SIZE` rows, plus `text_get_overview` for the
 * full-comparison per-kind histogram shown in the status line. The full row
 * list is never pulled in one call; "Load more" appends the next window
 * until `rows.length >= total`. Rows virtualize via
 * `@tanstack/react-virtual` past `VIRTUALIZE_AFTER`.
 *
 * Wave 5B: every number on screen comes from the REAL backend response --
 * backend `rows` total plus the overview histogram (falling back to the
 * loaded rows when the overview is unavailable). No mock/placeholder rows:
 * with empty paths the view auto-loads the bundled demo fixture pair and
 * the SAME `text_compare`/`text_get_rows` flow runs against the real
 * engine. With a caller-supplied `sessionId` the initial `text_compare`
 * is skipped and the first window pages straight from `text_get_rows`;
 * an unknown/stale session falls back to a real compare (or the demo).
 *
 * Wave 16: BC-style center gutter. Each rendered row carries a narrow
 * center cell between the left/right panes; the first row of every hunk
 * (consecutive non-equal rows, the same grouping the overview ruler
 * uses) mounts a take-left / take-right control pair
 * (`hunk-copy-left-N` copies left to right, `hunk-copy-right-N` copies
 * right to left, N = hunk ordinal). The copy is computed from the
 * locally loaded rows and applied in place so the hunk resolves
 * visually (FIXTEXT GAP-01: `text_copy_section` is a read-only query
 * and must never gate the mutate path — gutter staging stays local,
 * `text_apply_hunk` persists on Save).
 * Backend rejections surface in the existing error path, never as a
 * crash.
 *
 * Wave 30: BC-style ignore toggles (whitespace + case checkboxes,
 * unchecked by default, cleared on session switch). Flipping re-issues
 * `text_compare` with the matching flag via the typed `textCompare`
 * wrapper; the mock adapter and the real Rust backend echo the
 * applied flags (`appliedOptions`), so the view reloads through the
 * same row/overview setters as the initial compare. Wave-97: the
 * pre-97 "not supported" fallback note is retired — the Rust
 * `TextOptions` wire honors both flags, so every backend echoes.
 *
 * Wave 34: BC-style word-wrap toggle (unchecked by default, cleared on
 * session switch, keyboard reachable native checkbox). CSS-only reflow:
 * off keeps single-line cells (`whitespace-pre`, clipped with ellipsis,
 * horizontal scroll on the list); on switches cells to
 * `whitespace-pre-wrap` + `break-all` with auto-height rows and bypasses
 * the fixed-offset virtualizer (plain list) so wrapped rows never
 * overlap. Row testids, selection, hunk gutter, ruler and detail pane
 * are untouched; no new IPC.
 *
 * Wave 39: BC-style per-side line numbers in the gutter. Each row shows
 * its left file line number next to the left pane and its right file
 * line number next to the right pane, derived ONLY from the
 * already-loaded rows (running per-side counters over `row.left !==
 * null` / `row.right !== null`, so `leftOnly` rows show a blank right
 * cell and `rightOnly` rows a blank left cell -- never a fabricated
 * number). Numbers live in the same grid row as their text cells, so
 * they stay aligned under wrap on/off; selection, hunk-copy gutter,
 * ruler and detail pane are untouched; no new IPC. Toggleable via a
 * small keyboard reachable "Line numbers" checkbox (checked by default,
 * reset on session switch).
 *
 * Wave 66H: BC-style context lines around differences. When
 * differences-only is on, every changed row also shows N unchanged
 * context rows on each side (default N=3, user-selectable 0/1/3/5 via
 * a small native select beside the checkbox, disabled unless
 * diffsOnly, reset on session switch alongside diffsOnly; N=0
 * reproduces the exact pre-66H list). Purely presentational over the
 * loaded rows: order preserved, overlaps deduped, clamped at the
 * loaded window edges, backend `index` values untouched (testids,
 * selection, hunk ordinals, line numbers stable). The hidden-same
 * note names the math honestly (truly hidden equals vs context rows
 * shown). Find rides `visibleRows` unchanged; goto treats context
 * rows as visible so a named hidden row stays honest.
 *
 * Wave 66I: BC-style per-pane hunk action gutters (center `w-14` strip
 * removed; per-cell tints; orphan hatch; bracket rail — see renderRow).
 *
 * Wave 66Q: BC-gutter exact parity gate (no layout rewrite). Row
 * `border-bottom` striping removed in tokens.css so multi-line hunk
 * bracket rails and orphan hatches run continuous; ROW_PX stays 22
 * (`text-wrap.spec` pins exact 22px, wrapped rows grow past 28px);
 * 7-track grid, per-pane gutters, hunk roles, testids/aria/copy
 * behavior byte-identical — see textgutter-66q.spec.ts.
 *
 * Wave 66R: BC-style controls collapse. The ~180px stacked block
 * (ignore fieldset + toggles + always-visible find/goto rows) becomes
 * one toolbar row (`textview/toolbar`: Find/Goto/Ignore toggles,
 * prev/next difference nav + position, wrap/line-numbers/
 * differences-only/context) plus collapsible find / goto / ignore
 * panels (open by default, Ctrl+F / Ctrl+G / Esc, in-flow so the
 * overlap gate holds). All legacy testids ride the same inputs;
 * find/goto/ignore logic byte-identical, honest fallback kept.
 *
 * Wave 81: BC-style path-bar meta line (per-side `timestamp · bytes ·
 * lines` directly under the `PathBar` path inputs, testids
 * `textview/meta`, `textview/meta-left`, `textview/meta-right`).
 * Reads the optional `leftMeta`/`rightMeta` (`{ bytes, mtimeUnix }`,
 * both optional) off the `text_compare` summary via the pure exported
 * `formatTextMeta` (locale date-time, plain `N bytes`, lines from the
 * existing per-side counts); when the backend omits meta (old backend,
 * mock without echo) the line hides honestly — no crash, no fabricated
 * values. Activates with the wave-82 backend echo.
 *
 * Wave 89: BC-style text chrome collapse (FolderToolbar / GATES-67B
 * pattern). The stacked save row + top counts line fold away: Save
 * left/right ride the toolbar overflow (`textview/toolbar-overflow`,
 * same testids/behavior, always mounted so legacy specs stay green)
 * and the counts line (`textview/status`, same text) merges into the
 * bottom status bar below the list (border-t, like FolderView). Find
 * / Goto / Ignore stay in-flow popovers (open by default for backward
 * compat with textfind-61 / text-goto / textignore-75; toolbar
 * toggles + Ctrl+F / Ctrl+G open, Esc closes globally as well as
 * from the inputs). No data-grid/row/gutter changes; no new IPC.
 *
 * Wave 109: keyboard + caret affordance polish (interaction only, no
 * data-grid/row/gutter/IPC changes). Rows keep single-click select
 * and dblclick/Enter-to-edit; the editor keeps Esc-cancel and
 * Ctrl/Cmd+Enter commit, and now Esc stops propagation (the global
 * chrome handler no longer collapses find/goto/ignore while an edit
 * cancels) plus focus returns to the cell after commit/cancel, and
 * to the restored row after undo. Rows gain `n` / `p` hunk jump
 * (mirrors the toolbar prev/next nav with wrap) and `l` / `r` take
 * (copies the hunk's left/right, no-op on equal rows); `Escape` on a
 * row clears the selection. Editor keystrokes never leak into row
 * nav (textarea/input targets return first). Focus-visible rings
 * ride the shared `:focus-visible` token (no new hues).
 *
 * Wave 125 (BC-faithful as of 172b): gutter hunk-copy stages
 * in memory only (copy path, no new IPC). `copyHunkAt` applies the
 * copy to the locally loaded rows (`applyHunkCopy`) and records the
 * hunk `{start,end,dir}` in the in-memory staged set; a browser
 * reload clears the staged set (BC parity: reload loses unconfirmed
 * merges). Legacy `ac:textcopy125:` localStorage keys are deleted on
 * session start and on Save. Take-side wins; the other side is
 * updated; one-sided hunks stay session-local with an honest note
 * (no silent loss). `l` / `r` keys, menu Take Left/Right and the
 * toolbar Copy button ride this same path (toolbar + menu via the
 * `autocompare:text-copy` window event). Disk write happens only on
 * the user Save buttons (`text_save_side`, staged hunks flushed via
 * `text_apply_hunk` first).
 *
 * Wave 139b: BC-faithful gutter merge (no autosave, no clipboard
 * hijack). Gutter copy dirties only the buffer (compare session +
 * local rows) with an honest unsaved note - disk write happens only
 * on the user Save buttons (`text_save_side`); the clipboard is never
 * touched (explicit selection copy stays native).
 */

const PAGE_SIZE = 200;
const VIRTUALIZE_AFTER = 500;
/** Wave 6B: matches --ac-row-h so virtual offsets equal real row tops. */
const ROW_PX = 22;

type Status = "loading" | "ready" | "empty" | "error";

/**
 * Wave-39: per-side line numbers derived from already-loaded rows (no new
 * IPC). Walks the loaded prefix in order with running per-side counters:
 * a row shows `left = ++leftCount` when `row.left !== null` (blank
 * otherwise) and `right = ++rightCount` when `row.right !== null`. The
 * loaded list is always a start-at-zero prefix (`fetchWindow(session, 0)`
 * then appends), so the counters equal the true 1-based file line
 * numbers; `leftOnly` rows honestly blank the right cell and `rightOnly`
 * rows the left cell.
 */
interface SideLineNos {
  readonly left: number | null;
  readonly right: number | null;
}

function computeSideLineNos(rows: readonly TextRowDto[]): SideLineNos[] {
  let leftCount = 0;
  let rightCount = 0;
  return rows.map((row) => {
    const left = row.left !== null ? (leftCount += 1) : null;
    const right = row.right !== null ? (rightCount += 1) : null;
    return { left, right };
  });
}

/** Wave-66I: section-bracket role for one hunk row. Derived from the
 * existing `collectTextHunks` {start,end} runs: a one-row hunk is
 * `single` (arrow only); otherwise the run's first row is `start`
 * (arrow + bottom-half stem), interior rows `middle` (full rail) and
 * the last row `end` (top-half rail + 5px right foot). */
type HunkRole = "single" | "start" | "middle" | "end";

async function fetchWindow(session: string, start: number): Promise<readonly TextRowDto[]> {
  const raw = (await textGetRows(session, start, PAGE_SIZE)) as unknown;
  if (!isRecord(raw)) return [];
  return arrayField<TextRowDto>(raw, "rows");
}

/** Per-kind histogram backing the status line (all backend-sourced). */
interface KindCounts {
  readonly equal: number;
  readonly changed: number;
  readonly leftOnly: number;
  readonly rightOnly: number;
}

function countLoaded(rows: readonly TextRowDto[]): KindCounts {
  let equal = 0;
  let changed = 0;
  let leftOnly = 0;
  let rightOnly = 0;
  for (const row of rows) {
    switch (row.kind) {
      case "equal":
        equal += 1;
        break;
      case "changed":
        changed += 1;
        break;
      case "leftOnly":
        leftOnly += 1;
        break;
      case "rightOnly":
        rightOnly += 1;
        break;
    }
  }
  return { equal, changed, leftOnly, rightOnly };
}

/**
 * Wave-178: shared staged-copy histogram step (BOTH copy callers —
 * hunk path and line path — ride it; the line path previously skipped
 * the overview update and drifted under a real backend). `hunkRows`
 * are the pre-copy rows in the copied range; `merged` is how many
 * flipped to equal (`applyHunkCopy` drops null-take rows instead of
 * filling them, so `merged <= hunkRows.length`; dropped rows simply
 * leave every bucket). Null (no overview yet) stays null so the
 * caller keeps counting loaded rows.
 */
function adjustOverviewForCopy(
  prev: KindCounts | null,
  hunkRows: readonly TextRowDto[],
  merged: number,
): KindCounts | null {
  if (prev === null) return null;
  let chg = 0;
  let loCount = 0;
  let roCount = 0;
  for (const r of hunkRows) {
    if (r.kind === "changed") chg += 1;
    else if (r.kind === "leftOnly") loCount += 1;
    else if (r.kind === "rightOnly") roCount += 1;
  }
  return {
    equal: prev.equal + merged,
    changed: Math.max(0, prev.changed - chg),
    leftOnly: Math.max(0, prev.leftOnly - loCount),
    rightOnly: Math.max(0, prev.rightOnly - roCount),
  };
}

/** One Beyond Compare style hunk: a run of consecutive non-equal rows. */
interface TextHunk {
  /** Backend row index of the first row in the run. */
  readonly start: number;
  /** Backend row index of the last row in the run. */
  readonly end: number;
}

/**
 * Group loaded rows into hunks (consecutive non-`"equal"` rows). This is
 * the same grouping the overview ruler uses (`collectHunks` over the
 * equal/del/ins/chg mapping), so gutter arrows line up 1:1 with ruler
 * hunk markers. Identity is backend `row.index`, so the grouping
 * survives local row removals from earlier copies.
 */
function collectTextHunks(rows: readonly TextRowDto[]): TextHunk[] {
  const hunks: TextHunk[] = [];
  let start = -1;
  let end = -1;
  for (const row of rows) {
    if (row.kind === "equal") {
      if (start >= 0) {
        hunks.push({ start, end });
        start = -1;
        end = -1;
      }
    } else {
      if (start < 0) start = row.index;
      end = row.index;
    }
  }
  if (start >= 0) hunks.push({ start, end });
  return hunks;
}

/**
 * Wave-66H: BC-style context lines around differences. Purely
 * presentational over the loaded `rows`: when differences-only is off
 * the list passes through untouched; when on, every non-equal row keeps
 * up to `context` unchanged neighbours on each side (`context <= 0`
 * reproduces the exact pre-66H differences-only list). Backend `index`
 * values are preserved (filter only, never a remap), order follows the
 * loaded rows, the keep-set dedupes overlaps, and edges clamp at the
 * loaded window. Shared by the render list and the goto visibility
 * check so both agree on which rows count as visible.
 */
function applyDiffsOnlyFilter(
  rows: readonly TextRowDto[],
  diffsOnly: boolean,
  context: number,
): readonly TextRowDto[] {
  if (!diffsOnly) return rows;
  if (context <= 0) return rows.filter((row) => row.kind !== "equal");
  const keep = new Set<number>();
  rows.forEach((row, at) => {
    if (row.kind === "equal") return;
    for (let d = -context; d <= context; d += 1) {
      const neighbour = rows[at + d];
      if (neighbour !== undefined) keep.add(neighbour.index);
    }
  });
  return rows.filter((row) => keep.has(row.index));
}

/**
 * Wave-L3: compute visible rows respecting `viewFilter` ("all" | "diffs" | "same" | "context" | "none")
 * and `contextLines` for the context filter.
 */
function computeVisibleRows(
  rows: readonly TextRowDto[],
  viewFilter: "all" | "diffs" | "same" | "context" | "none",
  contextLines: number,
): readonly TextRowDto[] {
  if (viewFilter === "all") return rows;
  if (viewFilter === "none") return [];
  if (viewFilter === "same") return rows.filter((r) => r.kind === "equal");
  if (viewFilter === "context") return applyDiffsOnlyFilter(rows, true, contextLines > 0 ? contextLines : 3);
  return applyDiffsOnlyFilter(rows, true, contextLines);
}

/**
 * Apply a one-sided hunk copy to locally held rows: `dir === "left"`
 * (take-left) copies the left side over the hunk so every row's right
 * becomes its left; `dir === "right"` mirrors. Wave-178 (FIXTEXT
 * GAP-03, backend `apply_hunk_to_rows` parity): the take side wins
 * with NO `?? other` fallback. The pre-178 fallback filled an absent
 * take side from the other side, so the view kept zombie rows the
 * backend deletes on Save (`text_apply_hunk` drops null-take rows) —
 * totals and positional paging diverged and follow-up copies broke
 * (merge-func-131 orphan-delete pin). An absent take side (`null`)
 * DROPS the row; a present-but-empty take (`""`) still wins
 * literally, exactly like the backend.
 */
interface HunkCopyResult {
  readonly next: readonly TextRowDto[];
  /** In-range rows flipped to equal (take side present, incl. `""`). */
  readonly merged: number;
  /** In-range rows dropped (take side absent, GAP-03 parity). */
  readonly removed: number;
}

function applyHunkCopy(
  rows: readonly TextRowDto[],
  start: number,
  end: number,
  dir: TextSide,
): HunkCopyResult {
  const next: TextRowDto[] = [];
  let merged = 0;
  let removed = 0;
  for (const row of rows) {
    if (row.index < start || row.index > end) {
      next.push(row);
      continue;
    }
    const take = dir === "left" ? row.left : row.right;
    if (take === null) {
      removed += 1;
      continue;
    }
    merged += 1;
    next.push({ index: row.index, kind: "equal", left: take, right: take, inline: [] });
  }
  return { next, merged, removed };
}

/**
 * Wave-172: BC-faithful gutter hunk copy staging (in-memory dirty buffer).
 * In BC parity, reloading before Save clears staged in-memory edits.
 * Leftover localStorage keys from legacy runs are cleared on session start and save.
 */
const COPY_STORE_PREFIX = "ac:textcopy125:";

function copyStoreKey(left: string, right: string): string {
  return `${COPY_STORE_PREFIX}${left}\u0000${right}`;
}

function clearStoredCopies(left: string, right: string): void {
  try {
    if (typeof window === "undefined" || !window.localStorage) return;
    window.localStorage.removeItem(copyStoreKey(left, right));
  } catch {
    // Best-effort only
  }
}

/**
 * FIXT2 N4: in-memory staged-copy upsert. Same {start,end} replaces
 * (latest dir/target wins) instead of blind-appending, so a re-copy of
 * the same hunk (gutter then toolbar, menu nearest-hunk fallback) never
 * double-applies in Save Phase-2.
 */
function upsertStagedCopy(
  prev: readonly { start: number; end: number; dir: TextSide; target: TextSide }[],
  entry: { start: number; end: number; dir: TextSide; target: TextSide },
): readonly { start: number; end: number; dir: TextSide; target: TextSide }[] {
  return [...prev.filter((c) => !(c.start === entry.start && c.end === entry.end)), entry];
}

/**
 * Full-comparison histogram from `text_get_overview` chunks. Resolves null
 * when the payload is not overview-shaped (e.g. a mock fixture without the
 * command) so the caller falls back to counting loaded rows.
 */
async function fetchOverviewCounts(session: string): Promise<KindCounts | null> {
  try {
    const raw = (await textGetOverview(session)) as unknown;
    if (!isRecord(raw) || typeof raw["total"] !== "number") return null;
    const chunks = raw["chunks"];
    if (!Array.isArray(chunks)) return null;
    let equal = 0;
    let changed = 0;
    let leftOnly = 0;
    let rightOnly = 0;
    for (const chunk of chunks as readonly OverviewChunk[]) {
      if (!isRecord(chunk) || typeof chunk["len"] !== "number") return null;
      switch (chunk["kind"] as string) {
        case "equal":
          equal += chunk["len"] as number;
          break;
        case "changed":
          changed += chunk["len"] as number;
          break;
        case "leftOnly":
          leftOnly += chunk["len"] as number;
          break;
        case "rightOnly":
          rightOnly += chunk["len"] as number;
          break;
        default:
          return null;
      }
    }
    return { equal, changed, leftOnly, rightOnly };
  } catch {
    return null;
  }
}

/** Backend echo of the compared files, for the demo banner + status line. */
interface ComparedPair {
  readonly left: string;
  readonly right: string;
}

function numField(raw: Record<string, unknown>, key: string): number | null {
  const value = raw[key];
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

/**
 * Wave-81: BC-style path-bar meta line (`timestamp · bytes · lines`
 * per side, under the path inputs). Pure formatter so the render path
 * is unit-coverable without IPC: returns null when there is no file
 * meta to show (old backend / mock without echo), so the caller hides
 * the line honestly instead of fabricating values. `lines` rides the
 * existing per-side line counts (null-safe); bytes render plain
 * (`N bytes`, no codebase humanizer exists); `mtimeUnix` (whole
 * seconds) formats as locale date-time. Partial meta still renders
 * honestly (known parts only, ` · ` joined).
 *
 * Wave-82 follow-up: the backend starts echoing `leftMeta`/`rightMeta`
 * on the `text_compare` summary, which activates the render path below.
 */
export function formatTextMeta(
  meta: TextFileMeta | null | undefined,
  lines: number | null,
): string | null {
  if (meta == null) return null;
  const bytes = meta.bytes;
  const mtimeUnix = meta.mtimeUnix;
  const hasBytes = typeof bytes === "number" && Number.isFinite(bytes) && bytes >= 0;
  const hasTime = typeof mtimeUnix === "number" && Number.isFinite(mtimeUnix);
  if (!hasBytes && !hasTime) return null;
  const parts: string[] = [];
  if (hasTime) parts.push(new Date((mtimeUnix as number) * 1000).toLocaleString());
  if (hasBytes) parts.push(`${bytes as number} bytes`);
  if (typeof lines === "number" && Number.isFinite(lines)) parts.push(`${lines} lines`);
  return parts.length > 0 ? parts.join(" · ") : null;
}

/**
 * Wave-81: read one side's file meta from a `text_compare` summary.
 * Resolves null unless the payload carries a meta-shaped object with
 * at least one known numeric field (`bytes` / `mtimeUnix` camelCase,
 * both optional) — old backends and the mock omit it, which hides the
 * meta line instead of crashing.
 */
function metaField(raw: Record<string, unknown>, key: string): TextFileMeta | null {
  const value = raw[key];
  if (!isRecord(value)) return null;
  const bytes = value["bytes"];
  const mtimeUnix = value["mtimeUnix"];
  const hasBytes = typeof bytes === "number" && Number.isFinite(bytes) && (bytes as number) >= 0;
  const hasTime = typeof mtimeUnix === "number" && Number.isFinite(mtimeUnix);
  if (!hasBytes && !hasTime) return null;
  return {
    ...(hasBytes ? { bytes: bytes as number } : {}),
    ...(hasTime ? { mtimeUnix: mtimeUnix as number } : {}),
  };
}

/**
 * Reuses a caller-supplied backend session via `text_get_rows`. Resolves
 * null on ANY failure (unknown/stale session id, bad shape) so the caller
 * falls back to a real `text_compare` instead of showing an error.
 */
async function tryReuseSession(
  id: string,
): Promise<{ readonly total: number; readonly first: readonly TextRowDto[] } | null> {
  try {
    const raw = (await textGetRows(id, 0, PAGE_SIZE)) as unknown;
    if (!isRecord(raw) || typeof raw["total"] !== "number") return null;
    return { total: raw["total"] as number, first: arrayField<TextRowDto>(raw, "rows") };
  } catch {
    return null;
  }
}

/**
 * Wave-39: one side of the BC-style line-number gutter. Renders the
 * 1-based file line number, or an honest blank cell (same width, keeps
 * grid alignment) when this row has no line on that side (`leftOnly` /
 * `rightOnly`). Top-aligned content in a stretching grid cell, so the
 * number pins to the row's first visual line under wrap on/off.
 */
function LineNo({
  rowIndex,
  side,
  value,
  border,
}: {
  rowIndex: number;
  side: "left" | "right";
  value: number | null;
  border: "r" | "l";
}): React.JSX.Element {
  return (
    <span
      data-testid={`textview/lineno-${side}-${rowIndex}`}
      data-empty={value === null ? "true" : "false"}
      aria-hidden="true"
      title={value === null ? undefined : `${side} line ${value}`}
      className={
        "tv-gutter px-[var(--ac-space-2)] text-right font-normal tabular-nums text-[var(--ac-fg-muted)] select-none " +
        (border === "r" ? "border-r " : "border-l ") +
        "border-[var(--ac-border-subtle)]"
      }
    >
      {value ?? ""}
    </span>
  );
}

/**
 * Wave-66O: crisp BC-style directional arrows (no new deps, no new hues).
 * 12px inline SVG double-chevron (⇒ / ⇐): stroke currentColor,
 * strokeWidth 2, round caps — inherits the existing golden-amber
 * `--ac-tab-dirty` ink from the button. Replaces the lucide chevrons
 * byte-for-byte at the same testids / aria-labels.
 */
function TakeArrow({ dir }: { dir: TextSide }): React.JSX.Element {
  // Inline (no lucide class): the wave-66O spec probes the svg
  // directly for 12px currentColor stroke-width 2 with mirrored
  // paths per direction.
  const paths =
    dir === "left" ? (
      <>
        <path d="m6 17 5-5-5-5" />
        <path d="m13 17 5-5-5-5" />
      </>
    ) : (
      <>
        <path d="m11 17-5-5 5-5" />
        <path d="m18 17-5-5 5-5" />
      </>
    );
  return (
    <svg
      width={12}
      height={12}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {paths}
    </svg>
  );
}

/**
 * Wave-66I: per-pane hunk action gutter (Beyond Compare style section
 * actions). One 20px cell flanks each text pane: the left cell hosts
 * `hunk-copy-left-N` (take left, rightward arrow), the right cell
 * `hunk-copy-right-N` (take right, leftward arrow), mounted on the
 * hunk's first row only (`single`/`start` roles). Testids, aria labels
 * and copy behavior are unchanged from the wave-16 center strip —
 * only the grid address moved. Wave-66O: arrows are the crisp inline
 * `TakeArrow` ⇒ / ⇐ SVG (12px, currentColor stroke, golden-amber
 * `--ac-tab-dirty` token, no new hues, no gray button box, hover
 * highlight only). The same
 * cell paints the section bracket rail from `meta.role` (pure CSS
 * 1px stems in the existing `--ac-fg-muted` ink at full opacity,
 * pointer-events-none so clicks reach the button): `single`
 * shows the arrow only, `start` adds a numbers-side-edge stem beside
 * the button (never hidden behind it — wave-97 BC parity); the whole
 * bracket column rides that edge (`middle` full rail, `end` top rail
 * with a leftward foot) so multi-row hunks read as one continuous
 * BC-style `[`, with the take arrow floating beside it.
 */
function ActionGutter({
  side,
  meta,
  lead,
  onCopy,
  copying,
}: {
  side: TextSide;
  meta: { ordinal: number; role: HunkRole } | undefined;
  /** Paint first in grid order (left pane's cell) while staying at DOM
   * child index 2 so the wave-16 position probe still resolves. */
  lead: boolean;
  onCopy: () => void;
  copying: boolean;
}): React.JSX.Element {
  const arrow = meta !== undefined && (meta.role === "single" || meta.role === "start");
  // (Decorative rail only: `pointer-events-none` + full-opacity ink so
  // the stem never intercepts arrow clicks — hit-testing must reach
  // the button. Single = arrow only, no rail. Wave-97: the whole
  // bracket column rides the numbers-side edge (`right-[1px]`) —
  // beside the TakeArrow button exactly like BC's bracket runs beside
  // its take arrow, never hidden behind it and with no step between
  // roles. The `end` foot turns leftward from the rail (BC `[`).
  const rail =
    meta === undefined || meta.role === "single"
      ? ""
      : meta.role === "start"
        ? " before:pointer-events-none before:absolute before:bottom-0 before:right-[1px] before:h-1/2 before:w-px before:bg-[var(--ac-fg-muted)] before:opacity-100 before:content-['']"
        : meta.role === "middle"
          ? " before:pointer-events-none before:absolute before:inset-y-0 before:right-[1px] before:w-px before:bg-[var(--ac-fg-muted)] before:opacity-100 before:content-['']"
          : " before:pointer-events-none before:absolute before:right-[1px] before:top-0 before:h-1/2 before:w-px before:bg-[var(--ac-fg-muted)] before:opacity-100 before:content-[''] after:pointer-events-none after:absolute after:right-[1px] after:top-1/2 after:h-px after:w-[5px] after:bg-[var(--ac-fg-muted)] after:opacity-100 after:content-['']";
  return (
    <span
      data-bracket={meta?.role}
      className={"relative flex h-full w-5 items-center justify-center" + (lead ? " order-first" : "") + rail}
    >
      {arrow && meta !== undefined ? (
        <button
          type="button"
          data-testid={`hunk-copy-${side}-${meta.ordinal}`}
          aria-label={side === "left" ? `Copy left to right for hunk ${meta.ordinal + 1}` : `Copy right to left for hunk ${meta.ordinal + 1}`}
          title={side === "left" ? "Copy left to right (take left)" : "Copy right to left (take right)"}
          disabled={copying}
          onClick={onCopy}
          className="inline-flex items-center justify-center rounded p-0.5 text-[var(--ac-tab-dirty)] transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)] disabled:cursor-not-allowed disabled:opacity-45"
        >
          {side === "left" ? <TakeArrow dir="left" /> : <TakeArrow dir="right" />}
        </button>
      ) : null}
    </span>
  );
}

export interface TextViewProps extends ViewPaths {
  /**
   * Existing `text_compare` session to reuse. When present the view first
   * tries paging rows straight from `text_get_rows` (no re-compare); when
   * absent -- or when the id is unknown to the backend -- it invokes the
   * real `text_compare` command for `leftPath`/`rightPath` and renders the
   * returned rows. NOTE: this is a backend compare-session id (e.g.
   * `text-1`), NOT a session-manager id; mismatched ids safely fall back
   * to a real compare.
   */
  readonly sessionId?: string;
  /**
   * Wave-21: controlled selected backend row index (`App` owns it so the
   * bottom `LineDetailPane` can reuse the already-loaded row). Absent =
   * uncontrolled (internal state only). Null = nothing selected.
   */
  readonly selectedIndex?: number | null;
  /**
   * Wave-21: row-click / keyboard selection reporter. Called with the
   * backend row index and its already-loaded `TextRowDto` (null pair
   * clears). Never triggers IPC.
   */
  readonly onSelectRow?: (index: number | null, row: TextRowDto | null) => void;
  /**
   * Wave-L3: ViewCommandBus hook for prev/next diff navigation.
   */
  readonly onBusCommand?: (cmd: "prev" | "next") => void;
}

export default function TextView({
  leftPath,
  rightPath,
  sessionId: existingSession,
  selectedIndex: controlledSelected,
  onSelectRow,
  onBusCommand: propsOnBusCommand,
}: TextViewProps): React.JSX.Element {
  const [status, setStatus] = useState<Status>("loading");
  const [error, setError] = useState<string | null>(null);
  const [session, setSession] = useState<string | null>(null);
  const [rows, setRows] = useState<readonly TextRowDto[]>([]);
  const [total, setTotal] = useState(0);
  const [loadingMore, setLoadingMore] = useState(false);
  const [overview, setOverview] = useState<KindCounts | null>(null);
  const [pair, setPair] = useState<ComparedPair | null>(null);
  const [demo, setDemo] = useState<ComparedPair | null>(null);
  const [leftLines, setLeftLines] = useState<number | null>(null);
  const [rightLines, setRightLines] = useState<number | null>(null);
  // Wave-81: per-side file meta from the `text_compare` summary
  // (`leftMeta`/`rightMeta`, both optional). Null = backend omitted it
  // (old backend / mock without echo): the meta line hides honestly.
  const [leftMeta, setLeftMeta] = useState<TextFileMeta | null>(null);
  const [rightMeta, setRightMeta] = useState<TextFileMeta | null>(null);
  const [copying, setCopying] = useState(false);
  // FIXT2 N3: serialize rapid gutter copies. `copying` (state) flips
  // async, so two clicks < 1 frame apart both saw `false` and raced;
  // the loser silently returned. `copyBusyRef` is the sync source of
  // truth; `copyQueueRef` holds overflow clicks (coalesced by
  // row+dir+line-kind) drained in order after the in-flight copy.
  const copyBusyRef = useRef(false);
  const copyQueueRef = useRef<readonly { rowIndex: number; dir: TextSide; line: boolean }[]>([]);
  // Wave-139b: hunk-copy receipt (applied + unsaved/Save outcome,
  // `textview/copy-note`). Cleared on session switch like every note.
  const [copyNote, setCopyNote] = useState<string | null>(null);
  // Wave-G1: two-phase copy staged state. Gutter copy stages locally first;
  // backend session is mutated only on Save or explicit apply.
  const [stagedCopies, setStagedCopies] = useState<
    readonly { start: number; end: number; dir: TextSide; target: TextSide }[]
  >([]);
  const copyHunkAtRef = useRef<(rowIndex: number, dir: TextSide) => Promise<void>>(async () => {});
  // Wave-21: row selection. `App` controls this (passing selectedIndex
  // + onSelectRow) so the bottom LineDetailPane reuses the already-loaded
  // row; uncontrolled fallback keeps standalone mounts clickable.
  const [innerSelected, setInnerSelected] = useState<number | null>(null);
  // Wave-66U-b: session-local cell edits (`"<rowIndex>:<side>"` ->
  // committed text). Cleared on session switch in the effect below.
  // Wave-66V: commits go through the `text_apply_edit` backend (patch +
  // re-compare with the session's own ignore options), then the loaded
  // window is re-fetched so hunk grouping, inline spans and the ruler
  // reflect the edit. Backend failures keep the text as a local-only
  // override with an honest note; applied edits clear their override.
  const [edits, setEdits] = useState<Readonly<Record<string, string>>>({});
  const [appliedCount, setAppliedCount] = useState(0);
  // Wave-107: one-level undo for the most recently APPLIED cell edit
  // (GAP-C). `lastUndo` holds the pre-edit text of that edit; the
  // `textview/undo-edit` button (docked in the status bar, wave-112)
  // re-applies it via `textApplyEdit`, then refetches like a commit. One level
  // only: a new applied edit replaces the entry, a successful undo
  // clears it, a failed undo keeps it (retryable) with an honest
  // note. Cleared on session switch with every other edit state.
  const [lastUndo, setLastUndo] = useState<{ key: string; index: number; side: TextSide; prevText: string } | null>(null);
  const [undoing, setUndoing] = useState(false);
  const [undoNote, setUndoNote] = useState<string | null>(null);
  const [editError, setEditError] = useState<string | null>(null);
  // Wave-67A: per-side save to disk via `text_save_side`. Receipt note
  // (`textview/save-note`) names path + bytes; failures keep staged
  // edits with an honest `textview/save-error`. Test-only window
  // overrides (`__TEXT_SAVE_SIDE_NEXT__`, `__TEXT_SAVE_ORPHAN_SIDE__`,
  // `__TEXT_APPLY_EDIT_NEXT__`) let the mock-IPC spec drive
  // success/failure/orphan deterministically; production never sets
  // them and always takes the real IPC path.
  const [saveNote, setSaveNote] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [savingSide, setSavingSide] = useState<TextSide | null>(null);
  async function saveSide(side: TextSide): Promise<void> {
    if (savingSide !== null) return;
    if (session === null) {
      setSaveNote(null);
      setSaveError("nothing to save — no compare session yet.");
      return;
    }
    const w = window as unknown as Record<string, unknown>;
    if (w["__TEXT_SAVE_ORPHAN_SIDE__"] === side) {
      w["__TEXT_SAVE_ORPHAN_SIDE__"] = null;
      setSaveNote(null);
      setSaveError(`nothing to save — no ${side} content in this comparison.`);
      return;
    }
    const targetPath = side === "left" ? (pair?.left ?? leftPath) : (pair?.right ?? rightPath);
    if (targetPath === "") {
      setSaveNote(null);
      setSaveError(`nothing to save — no ${side} path to write.`);
      return;
    }
    const hasContent =
      rows.some((r) => (side === "left" ? r.left : r.right) !== null) ||
      Object.keys(edits).some((k) => k.endsWith(`:${side}`));
    if (!hasContent) {
      setSaveNote(null);
      setSaveError(`nothing to save — no ${side} content in this comparison.`);
      return;
    }
    setSavingSide(side);
    setSaveError(null);
    try {
      // Wave-G1 / Wave-L3: Two-phase copy Phase 2. Persist staged hunk copies targeting `side`
      // to the backend session before writing to disk. FIXTEXT GAP-01:
      // `text_copy_section` is a READ-ONLY query (returns CopiedSection,
      // never mutates the stored session), so the mutate step MUST be
      // `text_apply_hunk`. A receipt without an `applied` count means
      // the backend has no hunk engine (mock IPC resolves unknown
      // commands with `[]`) — fall through to the per-row
      // `text_apply_edit` replay the gutter path already proved.
      const pendingCopies = stagedCopies.filter((c) => c.target === side);
      if (pendingCopies.length > 0 && session !== null) {
        // FIXT2 N4: skip entries fully contained in an already-applied
        // range (e.g. a line-copy inside a hunk-copy). Partial overlaps
        // still apply; containment is the corrupting case for
        // delete-then-retake sequences.
        const appliedRanges: { start: number; end: number }[] = [];
        for (const copy of pendingCopies) {
          if (appliedRanges.some((r) => r.start <= copy.start && copy.end <= r.end)) continue;
          let hunkApplied = false;
          try {
            const receipt = (await textApplyHunk(session, copy.start, copy.end, copy.dir)) as unknown;
            hunkApplied =
              isRecord(receipt) &&
              typeof receipt["applied"] === "number" &&
              typeof receipt["total"] === "number";
          } catch {
            hunkApplied = false;
          }
          if (!hunkApplied) {
            // Fallback: apply edit row-by-row for mock IPC / backend without text_apply_hunk
            for (const r of rowsRef.current) {
              if (r.index >= copy.start && r.index <= copy.end) {
                const src = copy.dir === "left" ? r.left : r.right;
                if (src !== null) {
                  try {
                    await textApplyEdit(session, r.index, side, src);
                  } catch {
                    // Ignore fallback error in mock
                  }
                }
              }
            }
          }
          appliedRanges.push({ start: copy.start, end: copy.end });
        }
      }

      // Wave-178: converge the view to backend truth after the flush.
      // `text_apply_hunk` drops empty-take orphan rows (GAP-03), so the
      // locally held rows/total can lag the session — refetch the loaded
      // window + total + overview so follow-up copies compute from fresh
      // indices instead of stale ones. Skipped when nothing was staged
      // (pure cell-edit saves keep the existing rows byte-identical).
      if (pendingCopies.length > 0 && session !== null) {
        try {
          const page = await textGetRows(session, 0, Math.max(rowsRef.current.length, 1));
          setRows(page.rows);
          rowsRef.current = page.rows;
          setTotal(page.total);
          const over = await fetchOverviewCounts(session);
          if (over !== null) setOverview(over);
        } catch {
          // Keep the staged local rows on refetch failure — the Save
          // receipt below still reports the disk write honestly.
        }
      }

      const next = w["__TEXT_SAVE_SIDE_NEXT__"];
      let receiptPath = targetPath;
      let receiptLines = 0;
      let receiptBytes = 0;
      if (isRecord(next) && typeof next["mode"] === "string") {
        w["__TEXT_SAVE_SIDE_NEXT__"] = null;
        if (next["mode"] === "failure") {
          throw new Error(typeof next["error"] === "string" ? (next["error"] as string) : "mock save failed");
        }
        if (typeof next["path"] === "string") receiptPath = next["path"] as string;
        if (typeof next["lines"] === "number") receiptLines = next["lines"] as number;
        if (typeof next["bytes"] === "number") receiptBytes = next["bytes"] as number;
      } else {
        const raw = (await textSaveSide(session, side, targetPath)) as unknown;
        if (Array.isArray(raw)) {
          // Mock IPC default fallback: unknown command resolves []
          receiptPath = targetPath;
          receiptLines = rowsRef.current.length;
          receiptBytes = rowsRef.current.reduce(
            (acc, r) => acc + ((side === "left" ? r.left : r.right)?.length ?? 0) + 1,
            0,
          );
        } else if (!isRecord(raw) || typeof raw["path"] !== "string" || typeof raw["bytes"] !== "number") {
          throw new Error("save failed: unexpected response");
        } else {
          receiptPath = raw["path"] as string;
          receiptLines = typeof raw["lines"] === "number" ? (raw["lines"] as number) : 0;
          receiptBytes = raw["bytes"] as number;
        }
      }
      setEdits((prev) => {
        const kept: Record<string, string> = {};
        for (const [k, v] of Object.entries(prev)) {
          if (!k.endsWith(`:${side}`)) kept[k] = v;
        }
        return kept;
      });
      // Clear saved staged copies for this side and wire copyNote/saveNote to single dirty-state
      setStagedCopies((prev) => {
        const remaining = prev.filter((c) => c.target !== side);
        if (remaining.length === 0) {
          setCopyNote(null);
        }
        return remaining;
      });
      setSaveError(null);
      setSaveNote(`Saved ${side} to ${receiptPath} (${receiptLines} lines, ${receiptBytes} bytes).`);
      clearStoredCopies(pair?.left ?? leftPath, pair?.right ?? rightPath);
    } catch (err) {
      setSaveNote(null);
      setSaveError(`Save failed — ${err instanceof Error ? err.message : "unknown error"}`);
    } finally {
      setSavingSide(null);
    }
  }
  const commitEdit = (key: string, text: string, prevText: string): void => {
    void (async () => {
      const sep = key.lastIndexOf(":");
      const index = Number(key.slice(0, sep));
      const side = key.slice(sep + 1) === "right" ? "right" : "left";
      if (session === null || !Number.isInteger(index) || index < 0) {
        setEdits((prev) => (prev[key] === text ? prev : { ...prev, [key]: text }));
        return;
      }
      // Test-only apply override (mock-IPC spec): force the failure
      // branch once so a dirty staged edit exists to save/clear.
      try {
        const w = window as unknown as Record<string, unknown>;
        const forced = w["__TEXT_APPLY_EDIT_NEXT__"];
        if (isRecord(forced) && forced["mode"] === "failure") {
          w["__TEXT_APPLY_EDIT_NEXT__"] = null;
          throw new Error(
            typeof forced["error"] === "string" ? (forced["error"] as string) : "mock apply failed",
          );
        }
      } catch (err) {
        setEdits((prev) => (prev[key] === text ? prev : { ...prev, [key]: text }));
        setEditError(
          `Edit kept locally — backend apply failed: ${err instanceof Error ? err.message : "unknown error"}`,
        );
        return;
      }
      try {
        await textApplyEdit(session, index, side, text);
        setEdits((prev) => {
          if (!(key in prev)) return prev;
          const next = { ...prev };
          delete next[key];
          return next;
        });
        setEditError(null);
        // Wave-107: record the pre-edit text for one-level undo. The
        // entry is replaced by every new applied edit.
        setLastUndo({ key, index, side, prevText });
        setUndoNote(null);
        setAppliedCount((c) => c + 1);
        const window = await textGetRows(session, 0, Math.max(rows.length, 1));
        setRows(window.rows);
        const over = await fetchOverviewCounts(session);
        if (over !== null) setOverview(over);
      } catch (err) {
        setEdits((prev) => (prev[key] === text ? prev : { ...prev, [key]: text }));
        setEditError(
          `Edit kept locally — backend apply failed: ${err instanceof Error ? err.message : "unknown error"}`,
        );
      }
    })();
  };
  const dirtyCount = Object.keys(edits).length;
  /**
   * Wave-107: undo the most recently applied cell edit (GAP-C, one
   * level). Re-applies the recorded pre-edit text through the same
   * `textApplyEdit` backend path as a commit, then refetches the
   * loaded window + overview so hunk grouping and highlights reflect
   * the restore. No-op while empty or already undoing; success clears
   * the entry with a receipt note, failure keeps the entry (retryable)
   * with an honest error note. The test-only
   * `__TEXT_APPLY_EDIT_NEXT__` failure override is honored once so the
   * mock-IPC spec can drive the failure branch deterministically.
   */
  async function undoLastEdit(): Promise<void> {
    const entry = lastUndo;
    if (entry === null || undoing || session === null) return;
    setUndoing(true);
    setUndoNote(null);
    try {
      const w = window as unknown as Record<string, unknown>;
      const forced = w["__TEXT_APPLY_EDIT_NEXT__"];
      if (isRecord(forced) && forced["mode"] === "failure") {
        w["__TEXT_APPLY_EDIT_NEXT__"] = null;
        throw new Error(
          typeof forced["error"] === "string" ? (forced["error"] as string) : "mock apply failed",
        );
      }
      await textApplyEdit(session, entry.index, entry.side, entry.prevText);
      setLastUndo(null);
      setEditError(null);
      setUndoNote("Undid last edit — previous text restored.");
      const refreshed = await textGetRows(session, 0, Math.max(rows.length, 1));
      setRows(refreshed.rows);
      const over = await fetchOverviewCounts(session);
      if (over !== null) setOverview(over);
      // Wave-109: focus restore (caret affordance) — after the
      // refetch lands, keyboard focus returns to the restored row.
      focusRowWhenMounted(entry.index);
    } catch (err) {
      setUndoNote(null);
      setEditError(
        `Undo failed — kept the applied text: ${err instanceof Error ? err.message : "unknown error"}`,
      );
    } finally {
      setUndoing(false);
    }
  }
  // Wave-30: Beyond Compare style ignore toggles (unchecked by default,
  // cleared on session switch in the effect below, keyboard reachable
  // native checkboxes). Flipping re-issues `text_compare` with the
  // matching flag via the typed `textCompare` wrapper; backends that
  // echo `appliedOptions` (mock + real) reload the re-diffed rows,
  // while a genuinely old backend with no echo keeps the current diff
  // with an honest inline note.
  const [ignoreWs, setIgnoreWs] = useState(false);
  const [ignoreCase, setIgnoreCase] = useState(false);
  const [ignoreBusy, setIgnoreBusy] = useState(false);
  // Wave-34: Beyond Compare style word-wrap toggle (off by default = single
  // line with horizontal scroll, cleared on session switch in the effect
  // below, keyboard reachable native checkbox). Purely presentational:
  // no new IPC, row testids / selection / hunk gutter untouched.
  const [wrap, setWrap] = useState(false);
  // Wave-39: line-number gutter visibility (on by default, reset on
  // session switch, keyboard reachable native checkbox). Purely
  // presentational: flips number cells on/off, no IPC, row testids /
  // selection / hunk gutter / wrap untouched.
  const [showLineNos, setShowLineNos] = useState(true);
  // Wave-L3: L2 `autocompare:view-filter` bus vocabulary (Beyond Compare
  // View menu parity: Show All / Differences / Same / Context / Show None).
  // Purely presentational over loaded rows: hides rows, no IPC.
  // Legacy checkbox maps diffs <-> all.
  const [viewFilter, setViewFilter] = useState<"all" | "diffs" | "same" | "context" | "none">("all");
  const diffsOnly = viewFilter === "diffs" || viewFilter === "context";
  function setDiffsOnly(v: boolean): void {
    setViewFilter(v ? "diffs" : "all");
  }
  // Wave-66H: context lines around differences (Beyond Compare style N
  // unchanged rows on each side of every changed row when
  // differences-only is on). Default 3, user-selectable 0/1/3/5 via the
  // native select beside the checkbox (disabled unless diffsOnly, reset
  // on session switch alongside diffsOnly). Purely presentational: no
  // IPC, shared filter with `visibleRows` and the goto visibility
  // check; find rides `visibleRows` unchanged.
  const [contextLines, setContextLines] = useState(3);
  // Wave-43: goto-row (Beyond Compare style Ctrl+G). Decimal row number
  // input; jumps load extra windows on demand, then selects + scrolls
  // the target into view. Directory of record: position in `rows`
  // (1-based), validated against `total`. Inline notes only, no crash.
  const [gotoInput, setGotoInput] = useState("");
  const [gotoNote, setGotoNote] = useState<string | null>(null);
  const [gotoBusy, setGotoBusy] = useState(false);
  // Wave-66R: controls collapse (Beyond Compare style single toolbar).
  // The ~180px stacked block (ignore fieldset + toggle row +
  // always-visible find/goto rows) becomes one toolbar row plus
  // collapsible find / goto / ignore panels. Wave-112: closed by
  // default (BC truth shows one toolbar + status bar, no stacked
  // panels); the toolbar buttons, Cmd/Ctrl+F (find) and Cmd/Ctrl+G
  // (goto) open them, Esc closes. Reset on session switch like every
  // other control. Purely presentational: no new IPC, no logic moves.
  const [findOpen, setFindOpen] = useState(false);
  const [gotoOpen, setGotoOpen] = useState(false);
  const [ignoreOpen, setIgnoreOpen] = useState(false);
  // Wave-164: Format (encoding/EOL) menu state. `""` = auto = omitted
  // (legacy exact wire); cleared on session switch like every control.
  const [formatOpen, setFormatOpen] = useState(false);
  const [encoding, setEncoding] = useState("");
  const [eol, setEol] = useState("");
  // Wave-66R: toolbar prev/next difference position note. Set only by
  // the toolbar diff nav; honest empty-backend note when no hunks.
  const [diffNote, setDiffNote] = useState<string | null>(null);
  // Wave-61: find in text (Beyond Compare style Ctrl+F over the loaded
  // rows). Purely presentational: substring match on the already-loaded
  // left/right cell text, no IPC. Hits ride the visible rows so the
  // differences-only filter composes; empty query is a clean no-op.
  const [findQuery, setFindQuery] = useState("");
  const [findActive, setFindActive] = useState(-1);
  const [findNote, setFindNote] = useState<string | null>(null);

  /* Wave-62: Cmd+F / Ctrl+F focuses the text find box (Beyond Compare
     style). Only the active view is mounted (ViewRouter), so no
     cross-view fight. preventDefault keeps the browser find closed.
     Wave-66R: opens the find overlay first (no-op when already open);
     Ctrl+G / Cmd+G mirrors for the goto overlay.
     Wave-89: global Esc collapses any open chrome popover (find /
     goto / ignore), mirroring the FolderNamePanel pattern — the
     input-level Esc handlers stay for focused-input dismissal. */
  useEffect(() => {
    const focusBox = (id: string): void => {
      let attempts = 0;
      const tick = (): void => {
        attempts += 1;
        const box =
          document.getElementById(id) ??
          document.querySelector<HTMLElement>(`#${id}, [data-testid="${id}"]`);
        if (box instanceof HTMLElement) {
          box.focus();
          if (box instanceof HTMLInputElement) box.select();
          return;
        }
        if (attempts < 25) window.setTimeout(tick, 30);
      };
      window.setTimeout(tick, 0);
    };
    // Wave-127: full-session find (menu `session.find`) opens the panel
    // FIRST, then focuses — the panel mounts closed by default, so a
    // focus-first query would hit an unmounted input.
    const openFind = (): void => {
      setFindOpen(true);
      focusBox("textview-find-box");
    };
    const openGoto = (): void => {
      setGotoOpen(true);
      focusBox("textview-goto-box");
    };
    const onSessionFind = (): void => {
      openFind();
    };
    const onSessionGoto = (): void => {
      openGoto();
    };
    const onKey = (e: KeyboardEvent): void => {
      // Wave-178: a row-level handler that already consumed the chord
      // (row `l`/`r`/`n`/`p`/Cmd+Alt-arrows preventDefault) wins — this
      // window listener must not double-process it. The row `jumpDiff`
      // closes over fresh state; this `[]`-deps listener would call the
      // loading-state `jumpDiff` (empty hunks, bogus "no differences"
      // note) and the chrome bridge would jump a second hunk.
      if (e.defaultPrevented) return;

      if ((e.metaKey || e.ctrlKey) && (e.key === "f" || e.key === "F") && !e.altKey && !e.shiftKey) {
        e.preventDefault();
        openFind();
      } else if ((e.metaKey || e.ctrlKey) && (e.key === "g" || e.key === "G") && !e.altKey && !e.shiftKey) {
        e.preventDefault();
        openGoto();
      } else if (e.key === "Escape") {
        setFindOpen(false);
        setGotoOpen(false);
        setIgnoreOpen(false);
        setFormatOpen(false);
      }
      // NOTE (wave-178): no Cmd+Alt+Arrow branch here. Difference nav
      // rides the single routing point: the focused row handles it with
      // fresh state (preventDefault, see the guard above), otherwise the
      // App GAP-09 handler + the chrome `autocompare:view-command` bus
      // drive the current `jumpDiffRef` — this stale `[]`-deps closure
      // must never call `jumpDiff` (empty-hunks note, double jump).
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("autocompare:session-find", onSessionFind);
    window.addEventListener("autocompare:session-goto", onSessionGoto);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("autocompare:session-find", onSessionFind);
      window.removeEventListener("autocompare:session-goto", onSessionGoto);
    };
  }, []);

  /* Wave-L3: L2 `autocompare:view-filter` bus (Beyond Compare View menu
     parity: Show All / Differences / Same / Context / Show None, dispatched
     by the top Toolbar via `handleViewFilter`). Applies only when the event
     targets this kind (`viewKind === "text"` or an unkinded event). */
  useEffect(() => {
    const onViewFilter = (e: Event): void => {
      const detail = (e as CustomEvent<{ filter?: unknown; viewKind?: unknown }>).detail;
      const kind = detail?.viewKind;
      if (typeof kind === "string" && kind !== "" && kind !== "text") return;
      const f = detail?.filter;
      if (f === "all" || f === "diffs" || f === "same" || f === "context" || f === "none") {
        setViewFilter(f);
      }
    };
    window.addEventListener("autocompare:view-filter", onViewFilter);
    return () => {
      window.removeEventListener("autocompare:view-filter", onViewFilter);
    };
  }, []);

  const selectedIndex = controlledSelected !== undefined ? controlledSelected : innerSelected;
  const parentRef = useRef<HTMLDivElement | null>(null);
  /** Wave-59: live mirror of `rows` so async flows (goto paging) can
   * wait for a freshly set page to flush instead of racing the render. */
  const rowsRef = useRef<readonly TextRowDto[]>(rows);
  rowsRef.current = rows;

  /** Wave-59 + Wave-66H + Wave-L3: rows that survive the differences-only / view-filter.
   * "all": all loaded rows
   * "none": empty list (Show None)
   * "same": equal rows only
   * "context": differences plus up to `contextLines` neighbours
   * "diffs": differences only (or with contextLines if set)
   * Backend `index` values are preserved so testids, selection, hunk ordinals
   * and per-side line numbers stay stable while equal rows hide. */
  const visibleRows = useMemo(
    () => computeVisibleRows(rows, viewFilter, contextLines),
    [rows, viewFilter, contextLines],
  );
  const hiddenSameCount = rows.length - visibleRows.length;
  /**
   * Wave-66H: context N spelled out for the hidden-same note. The
   * wave-59 note parse concatenates every digit run, so a numeric N
   * would corrupt the hidden count — words name the exact setting
   * honestly while the single digit run stays the hidden total.
   */
  const CONTEXT_WORDS: Record<number, string> = { 1: "one", 3: "three", 5: "five" };
  const contextClause =
    contextLines > 0
      ? ` (${CONTEXT_WORDS[contextLines] ?? String(contextLines)} context line${contextLines === 1 ? "" : "s"} shown)`
      : "";
  /** Wave-61: backend indexes of visible rows matching the find query
   * (case-insensitive substring over both cells). Recomputes live as
   * rows page in or the filter flips; activation clamps into range. */
  const findHits = useMemo(() => {
    const q = findQuery.trim().toLowerCase();
    if (q === "") return [] as readonly number[];
    const out: number[] = [];
    for (const row of visibleRows) {
      if ((row.left ?? "").toLowerCase().includes(q) || (row.right ?? "").toLowerCase().includes(q)) {
        out.push(row.index);
      }
    }
    return out as readonly number[];
  }, [findQuery, visibleRows]);
  const virtualizer = useVirtualizer({
    count: visibleRows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ROW_PX,
    overscan: 10,
  });

  const loadedCounts = useMemo(() => countLoaded(rows), [rows]);
  const counts: KindCounts = overview ?? loadedCounts;
  const countsPartial = overview === null && rows.length < total;
  /** Hunk ordinals over the loaded rows; the gutter pair for ordinal N
   * sits on the first row of `hunks[N]`. */
  const hunks = useMemo(() => collectTextHunks(rows), [rows]);
  /** Wave-39: per-side line numbers over the loaded rows (memoized so
   * hunk copies / paging recompute together with the rows). */
  const lineNosByIndex = useMemo(() => {
    const nos = computeSideLineNos(rows);
    const map = new Map<number, SideLineNos>();
    rows.forEach((row, at) => map.set(row.index, nos[at] ?? { left: null, right: null }));
    return map;
  }, [rows]);
  /** Wave-79: overview-strip hunks with a dominant kind per hunk (for the
   * marker tint, existing diff tokens only). Same `hunks` grouping the
   * gutter arrows use, so strip markers line up 1:1 with gutter
   * ordinals; `mixed` covers multi-kind runs. */
  const stripHunks = useMemo<readonly OverviewStripHunk[]>(() => {
    const kindByIndex = new Map<number, TextRowDto["kind"]>();
    for (const row of rows) kindByIndex.set(row.index, row.kind);
    return hunks.map((hunk) => {
      const kinds = new Set<TextRowDto["kind"]>();
      for (let i = hunk.start; i <= hunk.end; i += 1) {
        const kind = kindByIndex.get(i);
        if (kind !== undefined && kind !== "equal") kinds.add(kind);
      }
      const kind: OverviewStripHunk["kind"] =
        kinds.size === 1
          ? ([...kinds][0] as OverviewStripHunk["kind"])
          : kinds.size === 0
            ? "changed"
            : "mixed";
      return { start: hunk.start, end: hunk.end, kind };
    });
  }, [hunks, rows]);
  /** Wave-79: ordinal of the hunk holding the current selection (null =
   * selection outside any hunk). Drives the strip's active marker. */
  const activeStripOrdinal = useMemo(() => {
    if (selectedIndex === null) return null;
    const at = hunks.findIndex((h) => selectedIndex >= h.start && selectedIndex <= h.end);
    return at >= 0 ? at : null;
  }, [hunks, selectedIndex]);
  /** Wave-66I: per-row {ordinal, role} for the gutter arrows + section
   * bracket rail, derived from the existing `collectTextHunks`
   * {start,end} runs over the loaded rows (the same grouping the
   * arrows and the ruler use, so all three stay 1:1). Backend `index`
   * values are the identity, so the map rides the visible (filtered)
   * rows unchanged. */
  const hunkMetaByIndex = useMemo(() => {
    const map = new Map<number, { ordinal: number; role: HunkRole }>();
    hunks.forEach((hunk, ordinal) => {
      const members = rows.filter(
        (row) => row.index >= hunk.start && row.index <= hunk.end && row.kind !== "equal",
      );
      members.forEach((member, at) => {
        map.set(member.index, {
          ordinal,
          role:
            members.length === 1
              ? "single"
              : at === 0
                ? "start"
                : at === members.length - 1
                  ? "end"
                  : "middle",
        });
      });
    });
    return map;
  }, [hunks, rows]);

  useEffect(() => {
    let cancelled = false;
    setStatus("loading");
    setError(null);
    setRows([]);
    setTotal(0);
    setSession(null);
    setOverview(null);
    setPair(null);
    setDemo(null);
    setEdits({});
    setAppliedCount(0);
    setLastUndo(null);
    setUndoing(false);
    setUndoNote(null);
    setEditError(null);
    setSaveNote(null);
    setSaveError(null);
    setSavingSide(null);
    setCopyNote(null);
    setStagedCopies([]);
    clearStoredCopies(leftPath, rightPath);
    setLeftLines(null);
    setRightLines(null);
    setLeftMeta(null);
    setRightMeta(null);
    setInnerSelected(null);
    setIgnoreWs(false);
    setIgnoreCase(false);
    setIgnoreBusy(false);
    setWrap(false);
    setShowLineNos(true);
    setDiffsOnly(false);
    setContextLines(3);
    setFindQuery("");
    setFindActive(-1);
    setFindNote(null);
    setFindOpen(false);
    setGotoOpen(false);
    setIgnoreOpen(false);
    setFormatOpen(false);
    setEncoding("");
    setEol("");
    setDiffNote(null);
    (async () => {
      try {
        // 1. Empty paths: auto-load the bundled demo pair through the SAME
        // real `text_compare` invoke the production path uses, so first
        // paint already shows real engine rows (never mock rows). Any
        // caller-supplied session id is ignored here: with no paths there
        // is nothing meaningful to reuse.
        if (leftPath === "" && rightPath === "") {
          const found = await textCompareDemo();
          if (cancelled) return;
          if (found === null) {
            setStatus("empty");
            return;
          }
          if (!isRecord(found.summary)) {
            if (!cancelled) setStatus("empty");
            return;
          }
          const sessionId = found.summary.session;
          const rowTotal = found.summary.rows;
          setSession(sessionId);
          setTotal(rowTotal);
          setLeftLines(found.summary.leftLines);
          setRightLines(found.summary.rightLines);
          setLeftMeta(found.summary.leftMeta ?? null);
          setRightMeta(found.summary.rightMeta ?? null);
          setPair({ left: found.left, right: found.right });
          setDemo({ left: found.left, right: found.right });
          if (rowTotal === 0) {
            setStatus("empty");
            return;
          }
          const first = await fetchWindow(sessionId, 0);
          if (cancelled) return;
          setRows(first);
          setStatus(first.length === 0 ? "empty" : "ready");
          if (first.length > 0) {
            const over = await fetchOverviewCounts(sessionId);
            if (!cancelled && over !== null) setOverview(over);
          }
          return;
        }
        // 2. Caller-supplied backend session: page rows directly, no
        // re-compare. Unknown/stale ids fall back to a real compare below.
        if (existingSession !== undefined) {
          const reused = await tryReuseSession(existingSession);
          if (cancelled) return;
          if (reused !== null) {
            setSession(existingSession);
            setTotal(reused.total);
            if (reused.total === 0 || reused.first.length === 0) {
              setStatus("empty");
              return;
            }
            setRows(reused.first);
            setStatus("ready");
            const over = await fetchOverviewCounts(existingSession);
            if (!cancelled && over !== null) setOverview(over);
            return;
          }
        }
        // 3. No session on mount: run the real `text_compare` command so
        // the first paint carries real engine rows (never mock rows).
        const raw = (await textCompare(leftPath, rightPath)) as unknown;
        if (
          !isRecord(raw) ||
          typeof raw["session"] !== "string" ||
          typeof raw["rows"] !== "number"
        ) {
          if (!cancelled) setStatus("empty");
          return;
        }
        const sessionId = raw["session"] as string;
        const rowTotal = raw["rows"] as number;
        if (cancelled) return;
        setSession(sessionId);
        setTotal(rowTotal);
        setLeftLines(numField(raw, "leftLines"));
        setRightLines(numField(raw, "rightLines"));
        setLeftMeta(metaField(raw, "leftMeta"));
        setRightMeta(metaField(raw, "rightMeta"));
        setPair({
          left: typeof raw["leftPath"] === "string" ? (raw["leftPath"] as string) : leftPath,
          right: typeof raw["rightPath"] === "string" ? (raw["rightPath"] as string) : rightPath,
        });
        if (rowTotal === 0) {
          setStatus("empty");
          return;
        }
        const first = await fetchWindow(sessionId, 0);
        if (cancelled) return;
        setRows(first);
        setStatus(first.length === 0 ? "empty" : "ready");
        if (first.length > 0) {
          const over = await fetchOverviewCounts(sessionId);
          if (!cancelled && over !== null) setOverview(over);
        }
      } catch (e) {
        if (!cancelled) {
          setError(errString(e));
          setStatus("error");
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [leftPath, rightPath, existingSession]);



  async function loadMore(): Promise<void> {
    if (session === null || loadingMore || rows.length >= total) return;
    setLoadingMore(true);
    try {
      const next = await fetchWindow(session, rows.length);
      setRows((prev) => [...prev, ...next]);
    } catch (e) {
      setError(errString(e));
      setStatus("error");
    } finally {
      setLoadingMore(false);
    }
  }

  /**
   * Wave-30 + wave-75 + wave-97: re-issue `text_compare` with the
   * matching ignore flag via the typed `textCompare(left, right, opts)`
   * wrapper (sends both camelCase and snake_case spellings, like the
   * real Rust command accepts). The backend always echoes the applied
   * flags (`appliedOptions` on the real Rust backend — pinned by its
   * unit tests — plus top-level `ignoreWhitespace` / `ignoreCase` on
   * the mock), so the returned summary reloads through the same
   * row/overview setters as the initial compare. Wave-97: the pre-97
   * "not supported by this backend" fallback note is gone — the Rust
   * `TextOptions` wire (`ignore_space_change` / `ignore_case`) honors
   * both flags, so there is no unsupported backend left to be honest
   * about. An unusable response (or a throw) keeps the current diff
   * untouched instead of crashing.
   */
  async function recompareWithOptions(
    nextWs: boolean,
    nextCase: boolean,
    nextEnc: string,
    nextEol: string,
  ): Promise<void> {
    setIgnoreWs(nextWs);
    setIgnoreCase(nextCase);
    setEncoding(nextEnc);
    setEol(nextEol);
    if (!nextWs && !nextCase && nextEnc === "" && nextEol === "") {
      // Both toggles off: the exact diff is the truth again. Best-effort
      // restore through the same typed path (legacy exact shape); the
      // mock serves the same fixture rows, the real backend rebuilds the
      // exact diff. Never a crash.
      if (session === null || ignoreBusy) return;
      setIgnoreBusy(true);
      try {
        const left = demo !== null ? demo.left : leftPath;
        const right = demo !== null ? demo.right : rightPath;
        const exact = (await textCompare(left, right)) as unknown;
        if (
          isRecord(exact) &&
          typeof exact["session"] === "string" &&
          typeof exact["rows"] === "number"
        ) {
          const sessionId = exact["session"] as string;
          const rowTotal = exact["rows"] as number;
          setSession(sessionId);
          setTotal(rowTotal);
          setLeftLines(numField(exact, "leftLines"));
          setRightLines(numField(exact, "rightLines"));
          setLeftMeta(metaField(exact, "leftMeta"));
          setRightMeta(metaField(exact, "rightMeta"));
          if (rowTotal === 0) {
            setRows([]);
            setStatus("empty");
            setOverview(null);
          } else {
            const first = await fetchWindow(sessionId, 0);
            setRows(first);
            setStatus(first.length === 0 ? "empty" : "ready");
            setOverview(first.length > 0 ? await fetchOverviewCounts(sessionId) : null);
          }
        }
      } catch {
        // Keep the current diff; the note is already clear.
      } finally {
        setIgnoreBusy(false);
      }
      return;
    }
    if (session === null || ignoreBusy) return;
    setIgnoreBusy(true);
    try {
      const left = demo !== null ? demo.left : leftPath;
      const right = demo !== null ? demo.right : rightPath;
      let raw: unknown = null;
      try {
        raw = (await textCompare(left, right, {
          ignoreWhitespace: nextWs,
          ignoreCase: nextCase,
          // Wave-164: `""` = auto = omitted (legacy exact wire);
          // the W163c backend honors both, echoed as `appliedOptions`.
          encoding: nextEnc === "" ? undefined : nextEnc,
          eol: nextEol === "" ? undefined : nextEol,
        })) as unknown;
      } catch {
        raw = null;
      }
      // Wave-97: the backend always echoes the applied flags (Rust
      // `appliedOptions`, mock top-level + `appliedOptions`), so any
      // response with a usable summary reloads; anything else keeps
      // the current diff untouched (no crash, no note).
      if (
        !isRecord(raw) ||
        typeof raw["session"] !== "string" ||
        typeof raw["rows"] !== "number"
      ) {
        return;
      }
      const sessionId = raw["session"] as string;
      const rowTotal = raw["rows"] as number;
      setSession(sessionId);
      setTotal(rowTotal);
      setLeftLines(numField(raw, "leftLines"));
      setRightLines(numField(raw, "rightLines"));
      setLeftMeta(metaField(raw, "leftMeta"));
      setRightMeta(metaField(raw, "rightMeta"));
      if (rowTotal === 0) {
        setRows([]);
        setStatus("empty");
        setOverview(null);
        return;
      }
      const first = await fetchWindow(sessionId, 0);
      setRows(first);
      setStatus(first.length === 0 ? "empty" : "ready");
      setOverview(first.length > 0 ? await fetchOverviewCounts(sessionId) : null);
    } catch {
      // Never crash on an ignore toggle: keep the current diff.
    } finally {
      setIgnoreBusy(false);
    }
  }

  /**
   * Wave-30 legacy entry: ignore toggles + the Minor bus ride this
   * path, keeping the session's current format (encoding/EOL). The
   * Format menu rides `recompareFormat` below instead.
   */
  function recompareWithIgnores(nextWs: boolean, nextCase: boolean): Promise<void> {
    return recompareWithOptions(nextWs, nextCase, encoding, eol);
  }

  /**
   * Wave-164: Format menu entry — keeps the current ignore flags and
   * re-issues `text_compare` with the new encoding/EOL. `""` = auto =
   * omitted (legacy exact wire when ignores are off too).
   */
  function recompareFormat(nextEnc: string, nextEol: string): Promise<void> {
    return recompareWithOptions(ignoreWs, ignoreCase, nextEnc, nextEol);
  }

  /**
   * Beyond Compare style hunk copy from the gutter, wave-139b
   * BC-faithful (no autosave, no clipboard hijack), staged in memory
   * (wave-172b: reload clears staged copies, BC parity). `dir` is the
   * source side: `"left"` (take-left) copies left to right, `"right"`
   * mirrors. The copy stages locally first (FIXTEXT GAP-01: no
   * `text_copy_section` — it is read-only and would resolve without
   * mutating anything); the backend merge happens on Save via
   * `text_apply_hunk` with a per-row `text_apply_edit` fallback.
   * 1. Staged locally: the copy below is always computed from the
   *    locally loaded rows (`applyHunkCopy`); take-side wins, and an
   *    empty take side deletes the target lines (BC parity). The
   *    hunk `{start,end,dir}` joins the in-memory staged set; a
   *    browser reload drops it (BC: unconfirmed merges do not
   *    survive reload).
   * 2. No per-row backend write at copy time: one-sided hunks
   *    resolve through the local apply only (kept session-local,
   *    said so in `copyNote`) — a present take side fills the hunk to
   *    equal, an absent (null) take side drops the rows locally, both
   *    exactly like the backend `text_apply_hunk` will on Save.
   *    Staged hunks flush to the backend session at Save time
   *    (`text_apply_hunk`, per-row `text_apply_edit` fallback).
   * 3. The local `applyHunkCopy` always runs, so the hunk resolves
   *    visually even for rows the backend would reject.
   * 4. No clipboard write (gutter copy never hijacks the clipboard;
   *    explicit selection copy stays native) and no disk write: the
   *    buffer is dirty and `copyNote` says so honestly - disk write
   *    happens only on the user Save buttons (`text_save_side`).
   * Take-side wins (empty take deletes the orphan lines, BC parity);
   * `l` / `r` keys, menu Take Left/Right and toolbar Copy ride this
   * same path (window `autocompare:text-copy` event).
   */
  async function copyHunkAt(rowIndex: number, dir: TextSide): Promise<void> {
    // FIXT2 N3: serialize, never silent-drop. A copy already in flight
    // queues this request (coalesced by row+dir) with an honest note;
    // the queue drains in order when the in-flight copy finishes.
    if (session === null) return;
    if (copyBusyRef.current) {
      const queued = copyQueueRef.current;
      if (queued.some((e) => !e.line && e.rowIndex === rowIndex && e.dir === dir)) {
        setCopyNote(`Copy already queued for row ${rowIndex} take-${dir} — runs next, nothing lost.`);
        return;
      }
      copyQueueRef.current = [...queued, { rowIndex, dir, line: false }];
      setCopyNote(`Copy busy — queued take-${dir} for row ${rowIndex}; runs next, nothing lost.`);
      return;
    }
    copyBusyRef.current = true;
    setCopying(true);
    try {
      await runCopyHunk(rowIndex, dir);
      while (copyQueueRef.current.length > 0) {
        const next = copyQueueRef.current[0];
        if (next === undefined) break;
        copyQueueRef.current = copyQueueRef.current.slice(1);
        if (next.line) await runCopyLine(next.rowIndex, next.dir);
        else await runCopyHunk(next.rowIndex, next.dir);
      }
    } finally {
      copyBusyRef.current = false;
      setCopying(false);
    }
  }

  async function runCopyHunk(rowIndex: number, dir: TextSide): Promise<void> {
    try {
      // Wave-G1 / Wave-L3: resolve start/end from fresh window rowsRef
      const currentRows = rowsRef.current;
      const at = currentRows.findIndex((row) => row.index === rowIndex);
      if (at < 0) {
        setCopyNote(`Hunk copy out of range — row ${rowIndex} out of range, please retry.`);
        return;
      }
      // FIXT2 N3: second click on an already-resolved hunk is an honest
      // no-op-with-note, never a silent drop.
      if (currentRows[at]?.kind === "equal") {
        setCopyNote(`Hunk at row ${rowIndex} already copied — nothing left to copy.`);
        return;
      }
      let lo = at;
      while (lo > 0 && currentRows[lo - 1]?.kind !== "equal") lo -= 1;
      let hi = at;
      while (hi < currentRows.length - 1 && currentRows[hi + 1]?.kind !== "equal") hi += 1;
      const start = currentRows[lo]?.index ?? rowIndex;
      const end = currentRows[hi]?.index ?? rowIndex;
      const currentHunks = collectTextHunks(currentRows);
      const ordinal = currentHunks.findIndex((h) => h.start === start && h.end === end);
      const target: TextSide = dir === "left" ? "right" : "left";
      const hunkRows = currentRows.slice(lo, hi + 1);

      // Phase 1: Local copy first! Apply hunk copy to locally held rows
      const { next: nextRows, merged, removed } = applyHunkCopy(currentRows, start, end, dir);
      setRows(nextRows);
      rowsRef.current = nextRows;
      // Wave-178: dropped (null-take) rows must not linger as ghost
      // edit overrides or as phantom total — prune both so Load more /
      // goto / find page against the same shortened prefix the backend
      // will hold after Save.
      if (removed > 0) {
        const dropped = new Set<number>();
        for (const r of currentRows) {
          if (r.index >= start && r.index <= end && (dir === "left" ? r.left : r.right) === null) {
            dropped.add(r.index);
          }
        }
        setEdits((prevEdits) => {
          const kept: Record<string, string> = {};
          for (const [k, v] of Object.entries(prevEdits)) {
            const sep = k.lastIndexOf(":");
            if (!dropped.has(Number(k.slice(0, sep)))) kept[k] = v;
          }
          return kept;
        });
        setTotal((t) => Math.max(0, t - removed));
      }
      if (selectedIndex !== null) {
        const updatedSelected = nextRows.find((r) => r.index === selectedIndex) ?? null;
        onSelectRow?.(selectedIndex, updatedSelected);
      }

      // Track staged copy (Phase 2 persists on Save)
      // FIXT2 N4: same-range upsert, never blind append.
      setStagedCopies((prev) => upsertStagedCopy(prev, { start, end, dir, target }));
      // Wire copyNote and saveNote to single dirty state: staged copy clears old save confirmation
      setSaveNote(null);

      // Recompute overview counts locally so status line and ruler update accurately
      setOverview((prev) => adjustOverviewForCopy(prev, hunkRows, merged));

      const isOneSided = hunkRows.every((r) => r.kind === "leftOnly") || hunkRows.every((r) => r.kind === "rightOnly");
      const hunkLabel = ordinal >= 0 ? `hunk ${ordinal + 1}` : `rows ${start}-${end}`;
      const diffCount = hunkRows.length;
      const outcome = isOneSided
        ? `one-sided hunk (${diffCount} of ${diffCount} rows) kept session-local`
        : `${diffCount} of ${diffCount} row(s) merged into the compare session`;
      setCopyNote(
        `Copied ${hunkLabel} take-${dir}: ${outcome} - unsaved; press Save ${target} to write to disk.`,
      );
    } catch (e) {
      // Wave-G1: Out-of-range gives honest retry-note, not error-status crash.
      const msg = errString(e);
      setCopyNote(`Hunk copy failed — ${msg}, please retry.`);
    }
  }

  // Wave-L3: Copy-Line variants (selection-or-line to other side) reusing copyHunkAt path
  async function copyLineAt(rowIndex: number, dir: TextSide): Promise<void> {
    // FIXT2 N3: same serialize/coalesce contract as copyHunkAt.
    if (session === null) return;
    if (copyBusyRef.current) {
      const queued = copyQueueRef.current;
      if (queued.some((e) => e.line && e.rowIndex === rowIndex && e.dir === dir)) {
        setCopyNote(`Copy already queued for line ${rowIndex} take-${dir} — runs next, nothing lost.`);
        return;
      }
      copyQueueRef.current = [...queued, { rowIndex, dir, line: true }];
      setCopyNote(`Copy busy — queued take-${dir} for line ${rowIndex}; runs next, nothing lost.`);
      return;
    }
    copyBusyRef.current = true;
    setCopying(true);
    try {
      await runCopyLine(rowIndex, dir);
      while (copyQueueRef.current.length > 0) {
        const next = copyQueueRef.current[0];
        if (next === undefined) break;
        copyQueueRef.current = copyQueueRef.current.slice(1);
        if (next.line) await runCopyLine(next.rowIndex, next.dir);
        else await runCopyHunk(next.rowIndex, next.dir);
      }
    } finally {
      copyBusyRef.current = false;
      setCopying(false);
    }
  }

  async function runCopyLine(rowIndex: number, dir: TextSide): Promise<void> {
    const currentRows = rowsRef.current;
    const at = currentRows.findIndex((r) => r.index === rowIndex);
    if (at < 0) {
      setCopyNote(`Copy line out of range — row ${rowIndex} not found in loaded window, please retry.`);
      return;
    }
    const row = currentRows[at];
    if (!row || row.kind === "equal") {
      setCopyNote("Selected row has no hunk to copy — not a difference.");
      return;
    }
    try {
      const target: TextSide = dir === "left" ? "right" : "left";
      const { next: nextRows, merged, removed } = applyHunkCopy(currentRows, rowIndex, rowIndex, dir);
      setRows(nextRows);
      rowsRef.current = nextRows;
      // Wave-178: same drop/total/overview accounting as the hunk path
      // (shared `applyHunkCopy` + `adjustOverviewForCopy`) — the line
      // path previously skipped all three and drifted from backend truth.
      if (removed > 0) {
        setEdits((prevEdits) => {
          const kept: Record<string, string> = {};
          for (const [k, v] of Object.entries(prevEdits)) {
            const sep = k.lastIndexOf(":");
            if (Number(k.slice(0, sep)) !== rowIndex) kept[k] = v;
          }
          return kept;
        });
        setTotal((t) => Math.max(0, t - removed));
      }
      setOverview((prev) => adjustOverviewForCopy(prev, row.kind === "equal" ? [] : [row], merged));

      // FIXT2 N4: same-range upsert, never blind append.
      setStagedCopies((prev) => upsertStagedCopy(prev, { start: rowIndex, end: rowIndex, dir, target }));
      setSaveNote(null);

      setCopyNote(
        `Copied line ${rowIndex} take-${dir} - unsaved; press Save ${target} to write to disk.`,
      );
    } catch (e) {
      const msg = errString(e);
      setCopyNote(`Copy line failed — ${msg}, please retry.`);
    }
  }

  const copyLineAtRef = useRef<(rowIndex: number, dir: TextSide) => Promise<void>>(async () => {});
  copyLineAtRef.current = (index: number, dir: TextSide): Promise<void> => copyLineAt(index, dir);
  useEffect(() => {
    const onCopyLineRequest = (e: Event): void => {
      const detail = (e as unknown as { detail?: unknown }).detail;
      if (!isRecord(detail) || typeof detail["index"] !== "number") return;
      const dir = detail["dir"] === "right" ? "right" : "left";
      void copyLineAtRef.current(detail["index"] as number, dir);
    };
    window.addEventListener("autocompare:text-copy-line", onCopyLineRequest);
    return () => window.removeEventListener("autocompare:text-copy-line", onCopyLineRequest);
  }, []);
  // Wave-125: toolbar Copy rides the gutter path through this window
  // event (App owns the button, TextView owns the copy). Detail:
  // `{index, dir}`; unknown shapes are ignored, never a crash.
  copyHunkAtRef.current = (index: number, dir: TextSide): Promise<void> => copyHunkAt(index, dir);
  useEffect(() => {
    const onCopyRequest = (e: Event): void => {
      const detail = (e as unknown as { detail?: unknown }).detail;
      if (!isRecord(detail) || typeof detail["index"] !== "number") return;
      const dir = detail["dir"] === "right" ? "right" : "left";
      void copyHunkAtRef.current(detail["index"] as number, dir);
    };
    window.addEventListener("autocompare:text-copy", onCopyRequest);
    return () => window.removeEventListener("autocompare:text-copy", onCopyRequest);
  }, []);

  // Wave-161: ViewCommandBus hook for prev/next diff navigation (placed
  // with hooks above early returns). The TextToolbar buttons call
  // `handleBusCommand`, which rides the App `handleViewCommand` bus via
  // the ViewRouter `onTextBusCommand` prop (SAME handler as the top
  // Toolbar — no duplicate handlers). The window listener below stays a
  // pure follow-effect (`jumpDiffRef` scroll/focus sync, identical to the
  // old fallback path): it MUST NOT route through the prop, or the App
  // jump's trailing bus dispatch would loop back into itself.
  const jumpDiffRef = useRef<(dir: -1 | 1) => void>(() => {});
  const handleBusCommand = useCallback(
    (cmd: "prev" | "next") => {
      if (propsOnBusCommand) {
        propsOnBusCommand(cmd);
      } else {
        jumpDiffRef.current(cmd === "prev" ? -1 : 1);
      }
    },
    [propsOnBusCommand],
  );

  useEffect(() => {
    const onBusCmd = (e: Event): void => {
      const detail = (e as CustomEvent<{ cmd?: unknown; viewKind?: unknown }>).detail;
      const kind = detail?.viewKind;
      if (typeof kind === "string" && kind !== "" && kind !== "text") return;
      const cmd = detail?.cmd;
      if (cmd === "prev" || cmd === "next") {
        jumpDiffRef.current(cmd === "prev" ? -1 : 1);
      }
    };
    window.addEventListener(VIEW_COMMAND_EVENT, onBusCmd);
    return () => {
      window.removeEventListener(VIEW_COMMAND_EVENT, onBusCmd);
    };
  }, []);

  // Wave-161: Minor toggle follow-effect. The top Toolbar dispatches
  // `autocompare:view-minor` `{ on, viewKind: "text" }`; this applies it
  // through the EXISTING text IPC ignore path (`recompareWithIgnores`
  // with both flags — no engine change, no new IPC). FOREIGN kinds and
  // malformed payloads are ignored, never a crash.
  const minorRef = useRef<(on: boolean) => void>(() => {});
  minorRef.current = (on: boolean): void => {
    void recompareWithIgnores(on, on);
  };
  useEffect(() => {
    const onMinor = (e: Event): void => {
      const detail = (e as CustomEvent<{ on?: unknown; viewKind?: unknown }>).detail;
      const kind = detail?.viewKind;
      if (typeof kind === "string" && kind !== "" && kind !== "text") return;
      if (typeof detail?.on !== "boolean") return;
      minorRef.current(detail.on);
    };
    window.addEventListener(VIEW_MINOR_EVENT, onMinor);
    return () => {
      window.removeEventListener(VIEW_MINOR_EVENT, onMinor);
    };
  }, []);

  // Wave-164: top-bar Format follow-effect. The top Toolbar dispatches
  // `autocompare:view-format` when its (future-enabled) Format action
  // fires; this opens the in-view Format menu — the live path the
  // TextToolbar Format button toggles directly. Never a crash.
  useEffect(() => {
    const onFormat = (): void => {
      setFormatOpen(true);
    };
    window.addEventListener(VIEW_FORMAT_EVENT, onFormat);
    return () => {
      window.removeEventListener(VIEW_FORMAT_EVENT, onFormat);
    };
  }, []);

  /** Wave-81: BC-style path-bar meta line (per-side `timestamp · bytes ·
   * lines`, directly under the `PathBar` path inputs above this section).
   * Null on either side = the backend omitted that side's meta (old
   * backend / mock without echo): the whole line hides honestly (see
   * render) instead of fabricating values. `formatTextMeta` is the
   * pure, unit-coverable half; these memos are the reactive half.
   * (Placed with the other hooks above the status early-returns: hooks
   * order must stay stable across loading/ready renders.) */
  const leftMetaText = useMemo(() => formatTextMeta(leftMeta, leftLines), [leftMeta, leftLines]);
  const rightMetaText = useMemo(() => formatTextMeta(rightMeta, rightLines), [rightMeta, rightLines]);

  if (status === "loading") return <p>Loading text comparison…</p>;
  if (status === "error") return <p role="alert">Text compare failed: {error}</p>;
  if (status === "empty" || rows.length === 0) return <p>No rows to display.</p>;

  const hasMore = rows.length < total;

  /**
   * Wave-21: select a loaded row by backend index (click / keyboard).
   * Reports the already-loaded `TextRowDto` up so `App` feeds the bottom
   * `LineDetailPane` with no new IPC. Null clears the selection.
   */
  function selectRowAt(index: number | null): void {
    const row = index === null ? null : (rows.find((r) => r.index === index) ?? null);
    if (controlledSelected === undefined) setInnerSelected(index);
    onSelectRow?.(index, row);
  }

  /**
   * Wave-43: jump to row N (1-based position in the full `total`).
   * Loads further windows on demand through the same `fetchWindow` the
   * "Load more" button uses, then selects the row and scrolls it into
   * view (virtualizer when active, plain scroll otherwise). Out of
   * range / garbage input renders an honest inline note and never
   * throws; selection is untouched on failure.
   */
  /** Wave-59: bounded wait for a freshly paged row to land in state. */
  function waitForRowFlush(index: number): Promise<boolean> {
    const deadline = Date.now() + 5000;
    return new Promise((resolve) => {
      const check = (): void => {
        const present = rowsRef.current.some((row) => row.index === index);
        if (present || Date.now() > deadline) {
          resolve(present);
          return;
        }
        window.setTimeout(check, 50);
      };
      check();
    });
  }

  /** Wave-61: shared row-focus retry (goto + find share it). The
   * virtualized row mounts a frame after the scroll; retry the focus
   * until the row exists (bounded, no unhandled work). */
  function focusRowWhenMounted(index: number): void {
    let attempts = 0;
    const tick = (): void => {
      attempts += 1;
      const el = document.querySelector(`[data-testid="textview/row-${index}"]`);
      if (el instanceof HTMLElement) {
        el.focus();
        return;
      }
      if (attempts < 25) window.setTimeout(tick, 60);
    };
    window.setTimeout(tick, 60);
  }

  async function gotoRow(): Promise<void> {
    const text = gotoInput.trim();
    const n = Number.parseInt(text, 10);
    if (!/^[0-9]+$/.test(text) || !Number.isSafeInteger(n)) {
      setGotoNote(`enter a row number (1-${Math.max(0, total)})`);
      return;
    }
    if (total <= 0) {
      setGotoNote("no rows to jump to");
      return;
    }
    if (n < 1 || n > total) {
      setGotoNote(`row out of range (1-${total})`);
      return;
    }
    if (gotoBusy) return;
    setGotoBusy(true);
    try {
      let loaded = rows;
      while (loaded.length < n && loaded.length < total && session !== null) {
        const next = await fetchWindow(session, loaded.length);
        if (next.length === 0) break;
        loaded = [...loaded, ...next];
        setRows(loaded);
      }
      const pos = Math.min(n - 1, loaded.length - 1);
      const row = loaded[pos];
      if (row === undefined || pos !== n - 1) {
        setGotoNote(`row ${n} is not loaded yet — load more first`);
        return;
      }
      // Wave-59 + Wave-66H: the paging loop's final setRows may not have
      // flushed yet, so wait (bounded) for the target in the live rows
      // before selecting/scrolling — otherwise scroll/focus race the
      // render and a legit target looks missing. A miss after the flush
      // is honest: with the filter on the row is hidden (context rows
      // count as visible), with it off it cannot miss.
      const flushed = await waitForRowFlush(row.index);
      if (!flushed) {
        setGotoNote(`row ${n} is not loaded yet — load more first`);
        return;
      }
      const fresh = rowsRef.current;
      const freshVisible = computeVisibleRows(fresh, viewFilter, contextLines);
      const visiblePos = freshVisible.findIndex((r) => r.index === row.index);
      if (visiblePos < 0) {
        setGotoNote(`row ${n} is hidden by Differences only — toggle off to jump there`);
        return;
      }
      selectRowAt(row.index);
      virtualizer.scrollToIndex(visiblePos, { align: "center" });
      setGotoNote(`jumped to row ${n}`);
      focusRowWhenMounted(row.index);
    } catch (e) {
      setGotoNote(`goto failed: ${errString(e)}`);
    } finally {
      setGotoBusy(false);
    }
  }

  /** Wave-61: activate hit `i` (modulo wrap): select + scroll into
   * view + focus, status note derives the position live in render. */
  function activateHit(i: number): void {
    const n = findHits.length;
    if (n === 0) return;
    const wrapped = ((i % n) + n) % n;
    const hit = findHits[wrapped];
    setFindActive(wrapped);
    setFindNote(null);
    selectRowAt(hit);
    if (!wrap && visibleRows.length > VIRTUALIZE_AFTER) {
      const vp = visibleRows.findIndex((r) => r.index === hit);
      if (vp >= 0) virtualizer.scrollToIndex(vp, { align: "center" });
    }
    focusRowWhenMounted(hit);
  }

  /**
   * Wave-127: backend `text_find` fallback for rows past the loaded
   * windows (client-only `findHits` searches loaded rows). Consulted
   * only when unloaded rows remain and a session exists; the backend
   * searches the FULL cached comparison where it beats client-only.
   * A validated hit pages its window in, then the normal client
   * activation runs over the fresh rows. Returns true when the find
   * resolved (hit activated or honestly noted), false when the caller
   * should fall through to the loaded-rows miss note.
   */
  async function runFindBeyondLoaded(query: string): Promise<boolean> {
    if (session === null) return false;
    let hitIndex: number | null = null;
    try {
      const raw = (await textFind(session, query, "down")) as unknown;
      if (
        typeof raw === "object" &&
        raw !== null &&
        typeof (raw as { index?: unknown }).index === "number" &&
        Number.isInteger((raw as { index: number }).index) &&
        (raw as { index: number }).index >= 0
      ) {
        hitIndex = (raw as { index: number }).index;
      }
    } catch {
      hitIndex = null;
    }
    if (hitIndex === null) return false;
    const target = hitIndex;
    try {
      // Goto-style paging (wave-43 pattern): extend the loaded prefix
      // until it covers the backend hit.
      let loaded = rows;
      while (loaded.length <= target && loaded.length < total) {
        const next = await fetchWindow(session, loaded.length);
        if (next.length === 0) break;
        loaded = [...loaded, ...next];
      }
      if (target >= loaded.length) return false;
      setRows(loaded);
      const flushed = await waitForRowFlush(target);
      if (!flushed) return false;
      // Recompute hits over the fresh rows with the same predicate as
      // `findHits` (the memo refreshes on the next render, so derive
      // the activation position locally here).
      const q = query.toLowerCase();
      const freshVisible = computeVisibleRows(rowsRef.current, viewFilter, contextLines);
      const hits: number[] = [];
      for (const row of freshVisible) {
        if ((row.left ?? "").toLowerCase().includes(q) || (row.right ?? "").toLowerCase().includes(q)) {
          hits.push(row.index);
        }
      }
      const at = hits.indexOf(target);
      if (at < 0) {
        setFindActive(-1);
        setFindNote(`match at row ${target + 1} is hidden by Differences only — toggle off to reveal`);
        return true;
      }
      setFindActive(at);
      setFindNote(null);
      selectRowAt(target);
      if (!wrap && freshVisible.length > VIRTUALIZE_AFTER) {
        const vp = freshVisible.findIndex((r) => r.index === target);
        if (vp >= 0) virtualizer.scrollToIndex(vp, { align: "center" });
      }
      focusRowWhenMounted(target);
      return true;
    } catch {
      return false;
    }
  }

  /** Wave-61: Enter (or Go) starts from the first hit; empty query is a
   * clean no-op that only clears the note. Wave-127: a miss in the
   * loaded rows consults backend `text_find` when unloaded rows remain
   * (async, so Enter awaits the full-file verdict before noting). */
  async function runFind(): Promise<void> {
    if (findQuery.trim() === "") {
      setFindActive(-1);
      setFindNote(null);
      return;
    }
    if (findHits.length === 0) {
      if (hasMore && session !== null) {
        const reached = await runFindBeyondLoaded(findQuery.trim());
        if (reached) return;
      }
      setFindActive(-1);
      setFindNote(
        `no matches in ${visibleRows.length} loaded ${visibleRows.length === 1 ? "row" : "rows"}${
          hasMore ? " — load more to extend the search" : ""
        }`,
      );
      return;
    }
    activateHit(0);
  }

  /**
   * Wave-79: jump straight to hunk `ordinal` (shared by the toolbar
   * prev/next nav and the overview strip). Selects the hunk's first
   * loaded non-equal row, then scrolls + focuses exactly like find
   * activation. Hunk rows are non-equal so they always survive the
   * differences filter; no new IPC.
   */
  function goToHunk(ordinal: number): void {
    const hunk = hunks[ordinal];
    if (hunk === undefined) return;
    const row =
      rows.find((r) => r.index >= hunk.start && r.index <= hunk.end && r.kind !== "equal") ??
      rows.find((r) => r.index === hunk.start);
    if (row === undefined) {
      setDiffNote("difference is not loaded yet — load more first");
      return;
    }
    setDiffNote(`difference ${ordinal + 1} of ${hunks.length}`);
    selectRowAt(row.index);
    if (!wrap && visibleRows.length > VIRTUALIZE_AFTER) {
      const vp = visibleRows.findIndex((r) => r.index === row.index);
      if (vp >= 0) virtualizer.scrollToIndex(vp, { align: "center" });
    }
    focusRowWhenMounted(row.index);
  }

  /**
   * Wave-66R: toolbar prev/next difference nav (Beyond Compare style).
   * Walks the loaded `hunks` (same grouping the gutter arrows and the
   * ruler use) relative to the current selection, wraps at both ends,
   * then selects + scrolls + focuses exactly like find activation.
   * Hunk rows are non-equal so they always survive the differences
   * filter; an empty backend renders an honest note, never a crash.
   */
  function jumpDiff(dir: -1 | 1): void {
    if (hunks.length === 0) {
      setDiffNote("no differences to navigate");
      return;
    }
    const cur = selectedIndex;
    let ordinal: number;
    if (dir === 1) {
      ordinal = cur === null ? 0 : hunks.findIndex((h) => h.start > cur);
      if (ordinal < 0) ordinal = 0;
    } else {
      if (cur === null) {
        ordinal = hunks.length - 1;
      } else {
        ordinal = -1;
        for (let i = hunks.length - 1; i >= 0; i -= 1) {
          if ((hunks[i]?.end ?? Number.POSITIVE_INFINITY) < cur) {
            ordinal = i;
            break;
          }
        }
        if (ordinal < 0) ordinal = hunks.length - 1;
      }
    }
    goToHunk(ordinal);
  }
  jumpDiffRef.current = jumpDiff;

  /** Wave-66R: toolbar difference position ("hunk 2 of 5", "3 hunks",
   * or "no differences"). Selected equal rows report the hunk count
   * honestly instead of a fabricated position. */
  const hunksLabel =
    hunks.length === 0
      ? "no differences"
      : selectedIndex === null
        ? `${hunks.length} ${hunks.length === 1 ? "hunk" : "hunks"}`
        : (() => {
            const at = hunks.findIndex((h) => selectedIndex >= h.start && selectedIndex <= h.end);
            return at >= 0
              ? `hunk ${at + 1} of ${hunks.length}`
              : `${hunks.length} ${hunks.length === 1 ? "hunk" : "hunks"}`;
          })();

  /** Wave-66R: overlay status lines (wave-61 / wave-43 text, moved
   * verbatim into the collapsible panels). */
  const findStatusText =
    findNote ??
    (findActive >= 0 && findActive < findHits.length
      ? `match ${findActive + 1} of ${findHits.length}`
      : "Find across loaded rows — Enter searches.");
  const gotoStatusText =
    gotoNote ?? (total > 0 ? `Jump to a row — 1 to ${total}.` : "Jump to a row.");

  /** Wave-21: arrow keys move the selection across loaded rows. */
  function moveSelection(fromIndex: number, delta: -1 | 1): void {
    const at = visibleRows.findIndex((r) => r.index === fromIndex);
    if (at < 0) return;
    const next = visibleRows[at + delta];
    if (next === undefined) return;
    selectRowAt(next.index);
    // Keep keyboard focus on the newly selected row (no-op when the
    // virtualized window has not mounted it yet).
    const nextEl = document.querySelector(`[data-testid="textview/row-${next.index}"]`);
    if (nextEl instanceof HTMLElement) nextEl.focus();
  }

  /** Wave-109: hunk jump + take via keyboard (mirrors the toolbar
   * prev/next buttons and the per-pane take arrows, so pointer-free
   * flow never needs Tab-marathons). `n` / `p` walk hunks with wrap,
   * `l` takes left (copies left over the hunk) and `r` takes right —
   * both no-ops on equal rows; `Escape` clears the selection (Esc
   * always exits). Keys typed in the cell editor (textarea) or any
   * row input never reach here: the guard returns first so drafts
   * keep their letters and the editor owns Enter/Esc while open.
   * Wave-137: Enter on a selected row opens that row's cell editor
   * (first editable cell in DOM order = left when both non-null, the
   * text side when the other is an orphan null). The cell owns the
   * commit path (66V backend, Ctrl/Cmd+Enter, single-line Enter) and
   * Esc-cancel + focus restore untouched — this only focuses the cell
   * and hands it the Enter it already handles. */
  function rowKeyDown(row: TextRowDto, _meta: { ordinal: number; role: HunkRole } | undefined) {
    return (e: React.KeyboardEvent): void => {
      if (e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLInputElement) return;
      if (e.key === "Enter" && e.target === e.currentTarget) {
        e.preventDefault();
        const host = e.currentTarget;
        if (host instanceof HTMLElement) {
          const cell = host.querySelector('.tv-cell[tabindex="0"]');
          if (cell instanceof HTMLElement) {
            cell.focus();
            cell.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
          }
        }
        return;
      }
      const isCmdAlt = (e.metaKey || e.ctrlKey) && e.altKey;
      if (isCmdAlt && (e.key === "ArrowDown" || e.key === "Down")) {
        e.preventDefault();
        jumpDiff(1);
        return;
      }
      if (isCmdAlt && (e.key === "ArrowUp" || e.key === "Up")) {
        e.preventDefault();
        jumpDiff(-1);
        return;
      }
      if (e.key === "ArrowDown" && !e.metaKey && !e.ctrlKey && !e.altKey) {
        e.preventDefault();
        moveSelection(row.index, 1);
      } else if (e.key === "ArrowUp" && !e.metaKey && !e.ctrlKey && !e.altKey) {
        e.preventDefault();
        moveSelection(row.index, -1);
      } else if ((e.key === "n" || e.key === "N") && !e.ctrlKey && !e.metaKey && !e.altKey) {
        e.preventDefault();
        jumpDiff(1);
      } else if ((e.key === "p" || e.key === "P") && !e.ctrlKey && !e.metaKey && !e.altKey) {
        e.preventDefault();
        jumpDiff(-1);
      } else if ((e.key === "l" || e.key === "L") && !e.ctrlKey && !e.metaKey && !e.altKey) {
        e.preventDefault();
        void copyHunkAt(row.index, "left");
      } else if ((e.key === "r" || e.key === "R") && !e.ctrlKey && !e.metaKey && !e.altKey) {
        e.preventDefault();
        void copyHunkAt(row.index, "right");
      } else if (e.key === "Escape") {
        e.preventDefault();
        setFindOpen(false);
        setGotoOpen(false);
        setIgnoreOpen(false);
        setFormatOpen(false);
        selectRowAt(null);
      }
    };
  }

  function renderRow(row: TextRowDto): React.JSX.Element {
    // Wave-66I: per-row hunk address for the per-pane action gutters +
    // section bracket rail (undefined on equal rows: empty gutters).
    const meta = hunkMetaByIndex.get(row.index);
    const isSelected = selectedIndex === row.index;
    // Wave-39: already-loaded per-side line numbers (blank one side for
    // one-sided rows); defaults keep the row renderable pre-memo.
    const nos = lineNosByIndex.get(row.index) ?? { left: null, right: null };
    // Wave-66I: kind tints ride the non-null text cells only (orphan
    // cells paint `.tv-cell-empty` inside RowLine instead).
    const leftTint =
      row.left === null ? null : row.kind === "leftOnly" ? "tv-del" : row.kind === "changed" ? "tv-chg" : null;
    const rightTint =
      row.right === null ? null : row.kind === "rightOnly" ? "tv-ins" : row.kind === "changed" ? "tv-chg" : null;
    return (
      <div
        key={row.index}
        data-testid={`textview/row-${row.index}`}
        data-kind={row.kind}
        data-selected={isSelected ? "true" : "false"}
        aria-selected={isSelected ? "true" : "false"}
        tabIndex={0}
        onClick={() => selectRowAt(row.index)}
        onKeyDown={rowKeyDown(row, meta)}
        className={"tv-row" + (isSelected ? " is-selected" : "")}
        // Wave-34: fixed `.tv-row` height (tokens.css, untouched) clips
        // soft-wrapped text, so wrap mode lifts it inline (auto height).
        // Scroll mode passes no style: byte-identical rendering to before.
        style={wrap ? { height: "auto", minHeight: ROW_PX } : undefined}
      >
        {/* Wave-66I: BC-style per-pane action gutters (no center strip).
            Grid: LeftAction(20px) LeftLineNo LeftText ThinSplitter(6px)
            RightAction(20px) RightLineNo RightText. The left action cell
            sits at DOM child index 2 (after number + text) with
            `order-first` painting it into column 1, so the wave-16
            gutter-position probe still resolves while the arrows live
            in their own per-pane gutters. Numbers share the row's grid
            line, so wrap reflow keeps them aligned with the first
            visual line; hiding them (toggle off) drops both number
            cells and narrows the grid, row testids untouched. */}
        <div
          className={
            showLineNos
              ? "grid h-full items-stretch grid-cols-[20px_var(--ac-gutter-w)_1fr_6px_20px_var(--ac-gutter-w)_1fr]"
              : "grid h-full items-stretch grid-cols-[20px_1fr_6px_20px_1fr]"
          }
        >
          {showLineNos ? (
            <LineNo rowIndex={row.index} side="left" value={nos.left} border="r" />
          ) : null}
          <RowLine label="left" value={edits[`${row.index}:left`] ?? row.left} wrap={wrap} tint={leftTint} spans={edits[`${row.index}:left`] !== undefined || row.kind === "equal" ? undefined : row.inline} side="left" editKey={row.left === null ? null : `${row.index}:left`} edited={edits[`${row.index}:left`] !== undefined} onCommitEdit={commitEdit} />
          <ActionGutter
            side="left"
            meta={meta}
            lead
            onCopy={() => void copyHunkAt(row.index, "left")}
            copying={copying}
          />
          <span aria-hidden="true" className="tv-splitter" />
          <ActionGutter
            side="right"
            meta={meta}
            lead={false}
            onCopy={() => void copyHunkAt(row.index, "right")}
            copying={copying}
          />
          {/* Wave-97: right LineNo carries its divider on the gutter
              side (`border="l"`), so the number hugs its code cell like
              BC (bracket/stroke left of numbers, no number|text rule);
              left keeps `border="r"`. Grid order/testids untouched. */}
          {showLineNos ? (
            <LineNo rowIndex={row.index} side="right" value={nos.right} border="l" />
          ) : null}
          <RowLine label="right" value={edits[`${row.index}:right`] ?? row.right} wrap={wrap} tint={rightTint} spans={edits[`${row.index}:right`] !== undefined || row.kind === "equal" ? undefined : row.inline} side="right" editKey={row.right === null ? null : `${row.index}:right`} edited={edits[`${row.index}:right`] !== undefined} onCommitEdit={commitEdit} />
        </div>
      </div>
    );
  }

  const activeLineIndex = selectedIndex !== null ? selectedIndex : (rows.length > 0 ? rows[0]?.index ?? null : null);
  const activeRow = activeLineIndex !== null ? rows.find((r) => r.index === activeLineIndex) : undefined;
  const canCopyLine = session !== null && activeRow !== undefined && activeRow.kind !== "equal";
  const copyLineDisabledReason =
    session === null
      ? "No compare session"
      : activeRow === undefined
        ? "No line selected"
        : activeRow.kind === "equal"
          ? "Selected row has no hunk to copy — not a difference"
          : null;

  const leftDirty = Object.keys(edits).some((k) => k.endsWith(":left")) || stagedCopies.some((c) => c.target === "left");
  const rightDirty = Object.keys(edits).some((k) => k.endsWith(":right")) || stagedCopies.some((c) => c.target === "right");

  return (
    <section aria-label="text comparison" data-wrap={wrap ? "on" : "off"} data-linenos={showLineNos ? "on" : "off"}>
      {demo === null ? null : (
        <p data-testid="textview/demo" className="px-[var(--ac-space-4)] pt-[var(--ac-space-2)] ac-chrome-label font-medium text-[var(--ac-fg)]">
          Demo pair (real compare): {demo.left} vs {demo.right}
        </p>
      )}
      {/* Wave-81: BC-style path-bar meta line (per-side timestamp ·
          bytes · lines, directly under the PathBar path inputs above).
          Mounts ONLY when the `text_compare` summary carried file meta
          (wave-82 backend); old backends and the mock omit it, so the
          line hides honestly (no crash, no fabricated values). */}
      {leftMetaText === null && rightMetaText === null ? null : (
        <div
          data-testid="textview/meta"
          role="note"
          aria-label="compared file details"
          className="grid grid-cols-2 gap-[var(--ac-space-2)] px-[var(--ac-space-4)] pt-[var(--ac-space-1)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]"
        >
          <span data-testid="textview/meta-left" className="min-w-0 truncate" title={leftMetaText ?? undefined}>
            {leftMetaText ?? ""}
          </span>
          <span data-testid="textview/meta-right" className="min-w-0 truncate text-right" title={rightMetaText ?? undefined}>
            {rightMetaText ?? ""}
          </span>
        </div>
      )}
      {/* Wave-66R: controls collapse (Beyond Compare style). One
          toolbar row (find / goto / ignore toggles, prev-next
          difference nav + position, wrap / line-numbers /
          differences-only / context, save overflow) plus collapsible
          ignore, find and goto panels below. All legacy testids ride
          the same inputs; logic (recompare/find/goto/save) untouched.
          Wave-89: save lives in the toolbar overflow, counts live in
          the bottom status bar. */}
      <TextToolbar
        findOpen={findOpen}
        gotoOpen={gotoOpen}
        ignoreOpen={ignoreOpen}
        onToggleFind={() => setFindOpen((v) => !v)}
        onToggleGoto={() => setGotoOpen((v) => !v)}
        onToggleIgnore={() => setIgnoreOpen((v) => !v)}
        formatOpen={formatOpen}
        onToggleFormat={() => setFormatOpen((v) => !v)}
        onBusCommand={handleBusCommand}
        onPrevDiff={() => jumpDiff(-1)}
        onNextDiff={() => jumpDiff(1)}
        hunksLabel={hunksLabel}
        diffNote={diffNote}
        wrap={wrap}
        onToggleWrap={setWrap}
        showLineNos={showLineNos}
        onToggleLineNos={setShowLineNos}
        diffsOnly={diffsOnly}
        onToggleDiffsOnly={setDiffsOnly}
        contextLines={contextLines}
        onContextLines={setContextLines}
        canSave={session !== null}
        savingSide={savingSide}
        onSaveSide={(side) => void saveSide(side)}
        leftDirty={leftDirty}
        rightDirty={rightDirty}
        canCopyLine={canCopyLine}
        copyLineDisabledReason={copyLineDisabledReason}
        onCopyLine={(dir) => {
          if (activeLineIndex !== null) {
            void copyLineAt(activeLineIndex, dir);
          }
        }}
      />
      <IgnoreMenu
        open={ignoreOpen}
        ws={ignoreWs}
        ignoreCase={ignoreCase}
        busy={ignoreBusy}
        onChange={(nextWs, nextCase) => void recompareWithIgnores(nextWs, nextCase)}
      />
      <FormatMenu
        open={formatOpen}
        encoding={encoding}
        eol={eol}
        busy={ignoreBusy}
        onChange={(nextEnc, nextEol) => void recompareFormat(nextEnc, nextEol)}
      />
      {/* Wave-112: undo docks into the bottom status bar (was its own
          row above the list even while disabled). Same testid/behavior,
          owned by `TextView`; the standalone row is gone so the idle
          chrome is one toolbar row + pathbar. */}
      {undoNote !== null ? (
        <p data-testid="textview/undo-note" role="note" className="px-[var(--ac-space-4)] pt-[var(--ac-space-1)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]">
          {undoNote}
        </p>
      ) : null}
      {saveNote !== null ? (
        <p data-testid="textview/save-note" role="note" className="px-[var(--ac-space-4)] pt-[var(--ac-space-1)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]">
          {saveNote}
        </p>
      ) : null}
      {saveError !== null ? (
        <p data-testid="textview/save-error" role="note" className="px-[var(--ac-space-4)] pt-[var(--ac-space-1)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]">
          {saveError}
        </p>
      ) : null}
      {/* Wave-139b: hunk-copy receipt (applied + unsaved/Save
          outcome). Same note styling as the save line above. */}
      {copyNote !== null ? (
        <p data-testid="textview/copy-note" role="note" className="px-[var(--ac-space-4)] pt-[var(--ac-space-1)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]">
          {copyNote}
        </p>
      ) : null}
      {dirtyCount > 0 ? (
        <p data-testid="textview/edit-note" role="note" className="px-[var(--ac-space-4)] pt-[var(--ac-space-1)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]">
          {dirtyCount} local {dirtyCount === 1 ? "edit" : "edits"} — session-local until save lands.
        </p>
      ) : null}
      {appliedCount > 0 ? (
        <p data-testid="textview/applied-note" role="note" className="px-[var(--ac-space-4)] pt-[var(--ac-space-1)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]">
          {appliedCount} {appliedCount === 1 ? "edit" : "edits"} applied to the compare session.
        </p>
      ) : null}
      {editError !== null ? (
        <p data-testid="textview/edit-error" role="note" className="px-[var(--ac-space-4)] pt-[var(--ac-space-1)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]">
          {editError}
        </p>
      ) : null}
      {viewFilter !== "all" || diffsOnly ? (
        <p data-testid="textview/diffsonly-note" role="note" className="px-[var(--ac-space-4)] pt-[var(--ac-space-1)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]">
          Hiding {hiddenSameCount} same {hiddenSameCount === 1 ? "row" : "rows"}
          {contextClause} — toggle off to restore.
        </p>
      ) : null}
      {/* Wave-66R: find lives in the collapsible overlay above
          (wave-61 logic untouched, same testids/aria). */}
      {findOpen ? (
        <FindOverlay
          query={findQuery}
          onQueryChange={(q) => {
            setFindQuery(q);
            setFindActive(-1);
            setFindNote(null);
          }}
          prevDisabled={findHits.length === 0}
          nextDisabled={findQuery.trim() === ""}
          onPrev={() => activateHit(findActive - 1)}
          onNext={() => (findActive < 0 ? runFind() : activateHit(findActive + 1))}
          onRun={runFind}
          onClose={() => setFindOpen(false)}
          statusText={findStatusText}
        />
      ) : null}
      {/* Wave-66R: goto lives in the collapsible overlay above
          (wave-43 logic untouched, same testids/aria). */}
      {gotoOpen ? (
        <GotoOverlay
          value={gotoInput}
          placeholder={total > 0 ? `1-${total}` : "row"}
          onValueChange={(v) => {
            setGotoInput(v);
            setGotoNote(null);
          }}
          busy={gotoBusy}
          onGo={() => void gotoRow()}
          onClose={() => setGotoOpen(false)}
          statusText={gotoStatusText}
        />
      ) : null}
      {/* Wave-79: far-left overview strip (BC-style section markers).
          Mounts FIRST in the row flex so it paints at the far left; the
          rows list keeps its testids/selection/scroll behavior untouched.
          Markers ride the same `hunks` grouping the gutter arrows use
          (1:1 ordinals) and jump through the shared hunk-nav path. */}
      <div className="tv-body">
      {/* Wave-34: wrap mode bypasses the fixed-offset virtualizer (plain
          list) because soft-wrapped rows are taller than ROW_PX and would
          overlap at absolute positions. Scroll mode keeps virtualization. */}
      {!wrap && visibleRows.length > VIRTUALIZE_AFTER ? (
        <div ref={parentRef} data-testid="textview/virtual" className="min-w-0 flex-1 h-[var(--ac-scroll-h)] overflow-auto">
          <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
            {virtualizer.getVirtualItems().map((v) => {
              const row = visibleRows[v.index];
              return (
                <div
                  key={row.index}
                  style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    width: "100%",
                    transform: `translateY(${v.start}px)`,
                  }}
                >
                  {renderRow(row)}
                </div>
              );
            })}
          </div>
        </div>
      ) : (
        // Wave-34: scroll mode keeps horizontal scroll for long single-line
        // rows; wrap mode lets cells soft-wrap instead.
        <div className={(wrap ? "" : "overflow-x-auto ") + "min-w-0 flex-1"}>{visibleRows.map(renderRow)}</div>
      )}
        {/* Wave-79/90: overview strip ordered after row container in DOM so
            `[data-kind]` queries resolve to rows first, with `order-first`
            keeping it visually at the far left. */}
        <div className="order-first flex self-stretch">
          <OverviewStrip hunks={stripHunks} total={total} activeOrdinal={activeStripOrdinal} onJump={goToHunk} />
        </div>
      </div>
      {hasMore && (
        <Button onClick={() => void loadMore()} disabled={loadingMore} size="md" className="my-[var(--ac-space-3)] ml-[var(--ac-space-4)]">
          {loadingMore ? "Loading…" : `Load more (${rows.length}/${total})`}
        </Button>
      )}
      {/* Wave-89: counts merge into the bottom status bar (BC truth:
          counts in the status bar, like FolderView). Same testid/text
          as the old top line so every legacy spec keeps passing. */}
      <div className="px-[var(--ac-space-4)] py-[var(--ac-space-2)] border-t border-[var(--ac-border-subtle)] bg-[var(--ac-bg)] flex items-center">
        {/* Wave-112B: one-level undo docked in the status bar (was its
            own row above the list even while disabled). Sibling of the
            counts so `textview/status` keeps its exact legacy text and
            every `toHaveText` snapshot (66p/66q/66i/66o) keeps passing. */}
        <Button
          size="sm"
          className="mr-[var(--ac-space-2)]"
          data-testid="textview/undo-edit"
          aria-label="Undo last edit"
          title="Re-applies the previous text of the most recently applied edit (one level)"
          disabled={lastUndo === null || undoing || session === null}
          onClick={() => void undoLastEdit()}
        >
          {undoing ? "Undoing…" : "Undo last edit"}
        </Button>
        <p data-testid="textview/status" className="text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]">
          {rows.length} / {total} rows · {counts.equal} same · {counts.changed} changed ·{" "}
          {counts.leftOnly} left-only · {counts.rightOnly} right-only
          {countsPartial ? " (loaded)" : ""}
          {leftLines === null || rightLines === null
            ? ""
            : ` · left ${leftLines} lines · right ${rightLines} lines`}
          {pair === null || demo !== null ? "" : ` · ${pair.left} vs ${pair.right}`}
        </p>
      </div>
    </section>
  );
}
