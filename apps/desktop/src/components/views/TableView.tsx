import { useEffect, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ChevronDown, ChevronUp } from "lucide-react";
import { tableCompare, tableFind, tableGetPage } from "../../ipc/table";
import type { TableHit } from "../../ipc/table";
import { TableToolbar } from "./table/controls";
import { errString, isRecord } from "./types";
import type { ViewPaths } from "./types";

/**
 * Table (CSV) comparison grid (Wave 2B).
 *
 * IPC: `table_compare` (summary + session; `byKey` selects key-aligned
 * vs positional mode) then paged `table_get_page` windows of `ROW_PAGE`
 * rows x `COL_PAGE` columns. `table_get_page` takes no `byKey` param —
 * alignment flows via the session created by `table_compare`, so the
 * toggle passes `byKey` through the compare call and pages the resulting
 * session. "Load more" appends the next row window; rows virtualize past
 * `VIRTUALIZE_AFTER`. Cells that differ between sides get the `tc-diff`
 * class; missing cells render empty.
 *
 * Wave-66M: key-column picker (Beyond Compare style user-designated
 * keys, single-column only). One small `key` toggle per header cell
 * (`table-key-<c>`, `aria-pressed`) requests that column explicitly
 * (`key_columns: [c]`, mirrored to `window.__TABLE_KEYCOLS_LAST__`)
 * and enables key mode; clicking it again clears to auto (`[]` =
 * legacy path). Effective mode still comes from the backend echo, so
 * the wave-24 fallback note covers an explicit request the backend
 * ignores. Sort/hide/find/export/scroll/paging behavior untouched.
 *
 * Wave 24: by-key toggle (Beyond Compare style keyed row alignment).
 * Checkbox `table-bykey-toggle` flips between positional (`byKey: false`)
 * and key-aligned (`byKey: true`) comparison and reloads the session, so
 * row counts + diff classes update. When the backend honors the flag the
 * summary echoes `byKey: true` with non-empty `keyColumns`, and those
 * header columns get `tc-key` highlight (`data-key="true"`). When the
 * backend ignores it (e.g. the `VITE_MOCK_IPC=1` fixture always returns
 * `byKey: false, keyColumns: []`), the view still sends the explicit
 * `by_key: true` param, shows an honest inline fallback note
 * (`table-bykey-note`), highlights nothing, and never crashes. Find
 * (`table_find` + client-side scan) works in both modes against the
 * current session. Last requested mode is mirrored to
 * `window.__TABLE_BYKEY_LAST__` as a headless-proof hook.
 *
 * Wave 32: column visibility (Beyond Compare style show/hide).
 * A `Columns` menu (`table-cols-toggle` -> `table-cols-menu`) lists the
 * already-loaded header with one checkbox per column
 *
 * Wave-44 column sort (Beyond Compare style header click): header buttons
 * (`table-sort-<c>`) cycle unsorted -> ascending -> descending for the
 * LOADED rows only (new pages join the sort when loaded). Numeric-aware,
 * stable, left-preferred value resolution, physical row numbers kept.
 * Find hits stay valid (physical cell testids); the export follows the
 * displayed order and names it. Pure client-side, no new IPC.
 * (`table-col-visibility-<index>`); unchecking hides that column from the
 * grid without refetching (data stays loaded, `tc-diff` classes follow
 * the visible cells). The counts line (`table-cols-count`) reads
 * `N of M cols (K hidden)` while anything is hidden. Find skips hidden
 * columns client-side (IPC hits filtered + local scan skips hidden
 * cells) and shows an honest inline note (`table-find-hidden-note`).
 * `Show all` (`table-cols-show-all`) restores every column. Hidden state
 * is component state only (no new IPC) and clears on session switch
 * (effect reset). Toggle + checkboxes + show-all are native controls,
 * so the whole flow is keyboard reachable.
 *
 *  * Wave 38: export CSV (Beyond Compare style save-report). An `Export CSV`
 * button (`table-export`, native control so it stays keyboard reachable)
 * serializes the currently-visible grid — loaded rows in session order
 * (which already reflects the wave-24 by-key / positional alignment),
 * only `visibleCols` (hidden columns excluded, no new IPC, no filesystem
 * writes) — to a `text/csv` Blob downloaded as `table-export.csv` via a
 * temporary anchor + `URL.createObjectURL`. The inline receipt
 * (`table-export-receipt`, `role="status"`) reports row/col counts and
 * notes the scope honestly: find-active exports contain only hit rows
 * (`find "q": H hits to R rows`), otherwise all loaded rows, plus the
 * alignment (`by-key order` vs `positional order`) and hidden-column
 * exclusions. Empty grid / all-columns-hidden / find-with-no-matches
 * refuse inline (receipt note, no download, never a crash). Receipt
 * clears on session (re)load, find runs, and visibility changes so it
 * never describes a stale grid. Last successful export counts are also
 * mirrored to `window.__TABLE_EXPORT_LAST__` as a headless-proof hook.
 *
 * Wave-68: table chrome collapse (Beyond Compare style, mirrors
 * wave-67B folder + wave-66R text). The ~6 stacked chrome rows above
 * the grid collapse into one `TableToolbar` row (by-key toggle +
 * status, Columns toggle + Show all + visible count, Export +
 * receipt, Find toggle + prev/next difference nav) with collapsible
 * in-flow panels: the find panel (closed by default since wave-113A,
 * toolbar toggle / Ctrl+F opens + focuses, Esc closes) and the cols menu (closed by default — the
 * wave-32 contract asserts it). Diff counts + load time live in the
 * bottom status bar (`tableview/status`). All legacy testids ride the
 * same controls. The `table_compare` summary carries the total diff
 * count (wave-69 `diffRows`), so the bar shows the FULL-table total
 * while prev/next nav still walks the LOADED rows.
 *
 * Wave-86: per-side file meta (`leftMeta`/`rightMeta`
 * `{ bytes, mtimeUnix }`, both optional) off the `table_compare`
 * summary via the pure exported `formatTableMeta`. Wave-87 renders it
 * in the two-pane header below (`tableview/panes` with
 * `tableview/meta-left`/`tableview/meta-right` inside a
 * `tableview/meta` contents-wrapper) so no new stacked chrome row is
 * added. Absent meta (old backend, mock without echo) renders
 * nothing extra — honest-hidden fallback, no placeholder.
 *
 * Wave-87: two-pane header (Beyond Compare style, mirrors the
 * wave-66I/81 text pane grid). One compact `grid-cols-[1fr_6px_1fr]`
 * row with a thin `tv-splitter` center rail sits directly under the
 * single toolbar: left pane (`tableview/pane-left`) shows its own
 * path line (`tableview/path-left`) over its meta line, right pane
 * mirrors (`tableview/pane-right`, `tableview/path-right`). The data
 * grid below stays the single merged comparison table (all legacy
 * cell/row/hit/diff testids byte-identical) and the bottom status
 * bar keeps the FULL-table `diffRows` total.
 *
 * Wave-91: per-side data grids (Beyond Compare style, mirrors the
 * wave-87 two-pane shell). The single merged comparison table is
 * replaced by one `tableview/grids` row (`grid-cols-[1fr_6px_1fr]`
 * with a thin `tv-splitter` center rail): the left grid
 * (`tableview/grid-left`) renders the LEFT column values with its
 * own per-side column headers, the right grid
 * (`tableview/grid-right`) renders the RIGHT values with mirrored
 * headers. Changed cells on BOTH sides carry the red diff wash
 * (`.tc-diff` repainted onto the existing `--ac-diff-del-*` theme
 * tokens, so light + dark follow the scheme with no new palette).
 * Legacy testids stay on the left grid only (`tableview/row-R`,
 * `tableview/cell-R-C`, `table-sort-C`, `table-key-C`,
 * `table-hit-N`, `data-hit`); the right grid mounts `-right`
 * suffixed siblings (`tableview/row-right-R`,
 * `tableview/cell-right-R-C`, `table-sort-right-C`,
 * `table-key-right-C`, `table-hit-right-N`, `data-hit-right`) so
 * pre-91 specs keep passing unmodified. Find-hit mapping is
 * unchanged on the left (any side's hit at (r, c) lights the cell,
 * exactly the old merged rule); the right grid lights only
 * right-side hits. Sort/key state is shared: either side's header
 * buttons drive the same session order.
 */

const ROW_PAGE = 100;
const COL_PAGE = 20;
const VIRTUALIZE_AFTER = 500;
/** Wave-91: unified 22px density (matches --ac-row-h, the source of truth). Exported so e2e can pin the unification. */
export const ROW_PX = 22;

type Status = "loading" | "ready" | "empty" | "error";

/**
 * Wave-86: per-side file stats echoed by `table_compare` when the
 * backend stats the inputs. Exact wire spelling locked with the
 * wave-85 backend: `leftMeta`/`rightMeta` = `{ bytes, mtimeUnix }`
 * (`mtimeUnix` whole seconds, null when unknown). Old backends and
 * the mock omit the object entirely — the view hides the meta line
 * honestly instead of fabricating values.
 */
interface TableFileMeta {
  readonly bytes: number;
  readonly mtimeUnix: number | null;
}

/**
 * Wave-86: compact per-side meta text (`4.1 KB, 2026-11-20` style).
 * Pure formatter so the render path is unit-coverable without IPC:
 * returns null when there is no file meta to show (old backend / mock
 * without echo), so the caller renders nothing extra instead of
 * placeholder text. `mtimeUnix` null/unknown renders the size alone.
 */
export function formatTableMeta(meta: TableFileMeta | null | undefined): string | null {
  if (meta == null) return null;
  const bytes: unknown = (meta as { bytes?: unknown }).bytes;
  if (typeof bytes !== "number" || !Number.isFinite(bytes) || bytes < 0) return null;
  const size =
    bytes < 1024
      ? `${bytes} B`
      : bytes < 1024 * 1024
        ? `${(bytes / 1024).toFixed(1)} KB`
        : bytes < 1024 * 1024 * 1024
          ? `${(bytes / 1024 / 1024).toFixed(1)} MB`
          : `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`;
  const mtimeUnix: unknown = (meta as { mtimeUnix?: unknown }).mtimeUnix;
  if (typeof mtimeUnix === "number" && Number.isFinite(mtimeUnix)) {
    const day = new Date(mtimeUnix * 1000).toISOString().slice(0, 10);
    return `${size}, ${day}`;
  }
  return size;
}

/**
 * Wave-86: read one side's file meta from a `table_compare` summary.
 * Resolves null unless the payload carries a meta-shaped object with
 * a known numeric `bytes` (`mtimeUnix` optional, null when unknown) —
 * old backends and the mock omit it, which hides the meta line.
 */
function tableMetaField(raw: Record<string, unknown>, key: string): TableFileMeta | null {
  const value = raw[key];
  if (!isRecord(value)) return null;
  const bytes = value["bytes"];
  if (typeof bytes !== "number" || !Number.isFinite(bytes) || (bytes as number) < 0)
    return null;
  const mtimeUnix = value["mtimeUnix"];
  return {
    bytes: bytes as number,
    mtimeUnix:
      typeof mtimeUnix === "number" && Number.isFinite(mtimeUnix)
        ? (mtimeUnix as number)
        : null,
  };
}

interface GridData {
  readonly header: readonly string[];
  readonly rowsL: ReadonlyArray<readonly string[]>;
  readonly rowsR: ReadonlyArray<readonly string[]>;
  readonly totalRows: number;
  readonly totalCols: number;
}

function toStringArray(v: unknown): readonly string[] {
  return Array.isArray(v) && v.every((x) => typeof x === "string")
    ? (v as readonly string[])
    : [];
}

function toRowMatrix(v: unknown): ReadonlyArray<readonly string[]> {
  if (!Array.isArray(v)) return [];
  return v.filter(
    (r): r is readonly string[] => Array.isArray(r) && r.every((x) => typeof x === "string"),
  );
}

export default function TableView({ leftPath, rightPath }: ViewPaths): React.JSX.Element {
  const [status, setStatus] = useState<Status>("loading");
  const [error, setError] = useState<string | null>(null);
  const [session, setSession] = useState<string | null>(null);
  const [grid, setGrid] = useState<GridData | null>(null);
  const [loadingMore, setLoadingMore] = useState(false);
  const [findInput, setFindInput] = useState("");
  const [findQuery, setFindQuery] = useState("");
  // Wave-68: collapsible in-flow find panel (closed by default since
  // wave-113A BC parity — the toolbar Find toggle + Cmd/Ctrl+F open it;
  // Esc collapses it). The cols menu stays closed by default — the
  // wave-32 contract asserts it.
  const [findOpen, setFindOpen] = useState(false);
  // Wave-68: prev/next difference nav cursor over the loaded diff rows.
  const [diffPos, setDiffPos] = useState(0);
  // Wave-68: compare wall-clock for the bottom status bar (BC truth
  // shows "Load time" next to the diff counts).
  const [loadSecs, setLoadSecs] = useState<number | null>(null);
  // Wave-69: total difference rows over the FULL tables from the
  // `table_compare` summary (`diffRows`); null until the summary
  // arrives or when the backend predates the field.
  const [totalDiffRows, setTotalDiffRows] = useState<number | null>(null);
  // Wave-86: per-side file meta from the `table_compare` summary
  // (`leftMeta`/`rightMeta`, both optional). Null = backend omitted it
  // (old backend / mock without echo): the meta line hides honestly.
  const [leftMeta, setLeftMeta] = useState<TableFileMeta | null>(null);
  const [rightMeta, setRightMeta] = useState<TableFileMeta | null>(null);

  /* Wave-62: Cmd+F / Ctrl+F focuses the table find box (Beyond Compare
     style). Only the active view is mounted (ViewRouter), so no
     cross-view fight. preventDefault keeps the browser find closed.
     Wave-68: opens the collapsible find panel first (no-op when
     already open), then focuses — mirrors the text/folder pattern. */
  useEffect(() => {
    // Wave-127: full-session find (menu `session.find`) opens the panel
    // FIRST, then focuses — the panel mounts closed by default, so a
    // focus-first query would hit an unmounted input. The panel commits
    // after `setFindOpen`, so focus retries bounded until the box mounts.
    const openFind = (): void => {
      setFindOpen(true);
      let attempts = 0;
      const tick = (): void => {
        attempts += 1;
        const box = document.getElementById("table-find-input");
        if (box instanceof HTMLElement) {
          box.focus();
          return;
        }
        if (attempts < 20) window.setTimeout(tick, 50);
      };
      window.setTimeout(tick, 0);
    };
    const onSessionFind = (): void => {
      openFind();
    };
    const onKey = (e: KeyboardEvent): void => {
      if ((e.metaKey || e.ctrlKey) && (e.key === "f" || e.key === "F")) {
        e.preventDefault();
        openFind();
        // Wave-142: global Esc closes the find panel (text/folder parity;
        // input-level Esc already closes it when focused).
      } else if (e.key === "Escape") {
        setFindOpen(false);
      }
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("autocompare:session-find", onSessionFind);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("autocompare:session-find", onSessionFind);
    };
  }, []);
  const [hits, setHits] = useState<readonly TableHit[]>([]);
  const [hitPos, setHitPos] = useState(0);
  const [finding, setFinding] = useState(false);
  // Wave 24: keyed row alignment mode. `false` = positional (default,
  // preserves Wave 2B behavior); `true` = request key-aligned comparison
  // via `table_compare(left, right, true)`.
  const [byKey, setByKey] = useState(false);
  // Wave-66M: user-designated key column (Beyond Compare parity,
  // single-column only to match the backend). `null` = auto (legacy
  // path: `key_columns: []`); a column index requests it explicitly and
  // implies key mode. Clicking the keyed column clears back to `null`.
  const [pickedKey, setPickedKey] = useState<number | null>(null);
  // Echo of the backend summary: effective mode + key columns. Empty
  // `keyColumns` with `byKey` requested means positional fallback.
  const [effectiveByKey, setEffectiveByKey] = useState(false);
  const [keyColumns, setKeyColumns] = useState<readonly number[]>([]);
  // Wave 32: hidden column indexes (component state only, no IPC).
  // Cleared on every session (re)load in the effect below, so switching
  // sessions never leaks visibility state across different headers.
  const [hiddenCols, setHiddenCols] = useState<ReadonlySet<number>>(new Set());
  // Wave 32: column menu open state. Button + checkboxes are native
  // controls, so the flow stays keyboard reachable (Tab / Space / Enter,
  // Escape closes the menu).
  const [colsOpen, setColsOpen] = useState(false);
  // Wave 38: last export receipt (counts + honest scope note). Cleared on
  // session (re)load, find runs, and visibility changes so it never
  // describes a stale grid.
  const [exportNote, setExportNote] = useState<string | null>(null);
  // Wave-44: column sort over loaded rows. `null` = session order.
  // Cleared on session (re)load; header buttons cycle
  // unsorted -> asc -> desc. Display-only: data stays loaded.
  const [sortCol, setSortCol] = useState<number | null>(null);
  const [sortDir, setSortDir] = useState<1 | -1>(1);
  const parentRef = useRef<HTMLDivElement | null>(null);

  const rowCount = grid?.rowsL.length ?? 0;
  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ROW_PX,
    overscan: 10,
  });

  useEffect(() => {
    let cancelled = false;
    setStatus("loading");
    setError(null);
    setGrid(null);
    setSession(null);
    setHits([]);
    setHitPos(0);
    setLoadSecs(null);
    setTotalDiffRows(null);
    setLeftMeta(null);
    setRightMeta(null);
    setEffectiveByKey(false);
    setKeyColumns([]);
    // Wave 32: a new session means a possibly different header — drop any
    // hidden-column state (and close the menu) on every (re)load.
    setHiddenCols(new Set());
    setColsOpen(false);
    // Wave 38: a new session invalidates any previous export receipt.
    setExportNote(null);
    // Wave-44: a new session means a possibly different header — drop sort.
    setSortCol(null);
    setSortDir(1);
    // Headless-proof hook: record the exact `byKey` the view passes to IPC.
    try {
      (window as unknown as Record<string, unknown>)["__TABLE_BYKEY_LAST__"] = byKey;
      // Wave-66M: same for the explicit key-column request (`[]` = auto,
      // so the mock-IPC request path stays provable without IPC changes).
      (window as unknown as Record<string, unknown>)["__TABLE_KEYCOLS_LAST__"] =
        pickedKey === null ? [] : [pickedKey];
    } catch {
      /* ignore */
    }
    (async () => {
      const t0 = performance.now();
      try {
        const raw = (await tableCompare(
          leftPath,
          rightPath,
          byKey,
          pickedKey === null ? [] : [pickedKey],
        )) as unknown;
        if (!isRecord(raw) || typeof raw["sessionId"] !== "string") {
          if (!cancelled) setStatus("empty");
          return;
        }
        const sessionId = raw["sessionId"] as string;
        if (cancelled) return;
        setSession(sessionId);
        // Wave 24: honor the backend echo when present; tolerate backends
        // that ignore the flag (missing fields => positional fallback).
        if (typeof raw["byKey"] === "boolean") setEffectiveByKey(raw["byKey"] as boolean);
        else setEffectiveByKey(byKey);
        if (Array.isArray(raw["keyColumns"])) {
          setKeyColumns(
            (raw["keyColumns"] as unknown[]).filter(
              (n): n is number => typeof n === "number" && Number.isInteger(n) && n >= 0,
            ),
          );
        } else {
          setKeyColumns([]);
        }
        // Wave-69: total diff rows for the status bar; absent on old
        // backends / fixtures that predate the field (null => the
        // status bar falls back to the loaded scope).
        if (
          typeof raw["diffRows"] === "number" &&
          Number.isInteger(raw["diffRows"]) &&
          (raw["diffRows"] as number) >= 0
        ) {
          setTotalDiffRows(raw["diffRows"] as number);
        } else {
          setTotalDiffRows(null);
        }
        // Wave-86: per-side file meta (honest-hidden when omitted).
        setLeftMeta(tableMetaField(raw, "leftMeta"));
        setRightMeta(tableMetaField(raw, "rightMeta"));
        const page = (await tableGetPage(sessionId, 0, ROW_PAGE, 0, COL_PAGE)) as unknown;
        if (cancelled) return;
        if (!isRecord(page)) {
          setStatus("empty");
          return;
        }
        const headerL = toStringArray(page["headerL"]);
        const headerR = toStringArray(page["headerR"]);
        const rowsL = toRowMatrix(page["rowsL"]);
        const rowsR = toRowMatrix(page["rowsR"]);
        const totalRows = Math.max(
          typeof page["totalRowsL"] === "number" ? (page["totalRowsL"] as number) : 0,
          typeof page["totalRowsR"] === "number" ? (page["totalRowsR"] as number) : 0,
        );
        const totalCols = typeof page["totalCols"] === "number" ? (page["totalCols"] as number) : 0;
        if (totalRows === 0) {
          setStatus("empty");
          return;
        }
        setGrid({
          header: headerL.length > 0 ? headerL : headerR,
          rowsL,
          rowsR,
          totalRows,
          totalCols,
        });
        setStatus("ready");
        setLoadSecs((performance.now() - t0) / 1000);
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
  }, [leftPath, rightPath, byKey, pickedKey]);

  async function loadMore(): Promise<void> {
    if (session === null || grid === null || loadingMore || grid.rowsL.length >= grid.totalRows)
      return;
    setLoadingMore(true);
    try {
      const page = (await tableGetPage(
        session,
        grid.rowsL.length,
        ROW_PAGE,
        0,
        COL_PAGE,
      )) as unknown;
      if (isRecord(page)) {
        const moreL = toRowMatrix(page["rowsL"]);
        const moreR = toRowMatrix(page["rowsR"]);
        setGrid({ ...grid, rowsL: [...grid.rowsL, ...moreL], rowsR: [...grid.rowsR, ...moreR] });
      }
    } catch (e) {
      setError(errString(e));
      setStatus("error");
    } finally {
      setLoadingMore(false);
    }
  }

  /**
   * Wave 20: find control. Queries `table_find` (case-sensitive
   * substring or exact numeric equality, left table first, row-major,
   * capped at 100 hits — same
   * contract as `commands::table::table_find`, where an empty query is
   * an error). Empty queries are a local no-op: no IPC, hits cleared,
   * never a crash.
   *
   * Mock-IPC honesty: the `VITE_MOCK_IPC=1` adapter serves a `table_find`
   * fixture with identical semantics over the loaded rows, so when IPC
   * yields zero hits the view falls back to a client-side scan of the
   * LOADED rows with identical semantics (case-sensitive `includes` or
   * exact numeric equality, left then right, cap 100). With a real backend
   * the IPC hits win; the fallback only fires when IPC reports nothing.
   */
  // Wave-66G: numeric-aware find — substring win, else exact f64 equality (mirrors the exact-equality half of cells_equal_with_tolerance with no tolerance: find carries none).
  function findCellMatches(cell: string, query: string): boolean {
    if (cell.includes(query)) return true;
    const q = query.trim();
    const c = cell.trim();
    if (q === "" || c === "") return false;
    if (/^[+-]?0[xXbBoO]/.test(q) || /^[+-]?0[xXbBoO]/.test(c)) return false;
    const qn = Number(q);
    const cn = Number(c);
    return Number.isFinite(qn) && Number.isFinite(cn) && qn === cn;
  }

  function localScan(query: string): TableHit[] {
    if (grid === null) return [];
    const out: TableHit[] = [];
    const sides: ReadonlyArray<{ readonly side: "left" | "right"; readonly rows: ReadonlyArray<readonly string[]> }> = [
      { side: "left", rows: grid.rowsL },
      { side: "right", rows: grid.rowsR },
    ];
    for (const { side, rows } of sides) {
      for (let r = 0; r < rows.length; r += 1) {
        const row = rows[r];
        for (let c = 0; c < row.length; c += 1) {
          // Wave 32: find skips hidden columns (data stays loaded, just
          // not searched); the inline note keeps this honest.
          if (hiddenCols.has(c)) continue;
          if (findCellMatches(row[c], query)) {
            if (out.length >= 100) return out;
            out.push({ side, row: r, col: c, value: row[c] });
          }
        }
      }
    }
    return out;
  }

  function isTableHitArray(v: unknown): v is readonly TableHit[] {
    return (
      Array.isArray(v) &&
      v.every(
        (h): h is TableHit =>
          isRecord(h) &&
          (h["side"] === "left" || h["side"] === "right") &&
          typeof h["row"] === "number" &&
          typeof h["col"] === "number" &&
          typeof h["value"] === "string",
      )
    );
  }

  async function runFind(raw: string): Promise<void> {
    const query = raw;
    setFindQuery(query);
    // Wave 38: a new find changes the export scope — drop any stale receipt.
    setExportNote(null);
    if (query === "" || session === null) {
      setHits([]);
      setHitPos(0);
      return;
    }
    setFinding(true);
    try {
      let list: readonly TableHit[] = [];
      try {
        const res = (await tableFind(session, query)) as unknown;
        if (isTableHitArray(res)) list = res;
      } catch {
        list = [];
      }
      if (list.length === 0) list = localScan(query);
      // Wave 32: IPC knows nothing about visibility (no new IPC params),
      // so hits landing on hidden columns are filtered client-side.
      if (hiddenCols.size > 0) list = list.filter((h) => !hiddenCols.has(h.col));
      setHits(list.slice(0, 100));
      setHitPos(0);
    } finally {
      setFinding(false);
    }
  }

  function nextHit(): void {
    if (hits.length === 0) return;
    setHitPos((p) => (p + 1) % hits.length);
  }

  // Jump the current hit cell into view whenever the cursor moves.
  useEffect(() => {
    if (hits.length === 0) return;
    document
      .querySelector(`[data-testid="table-hit-${hitPos}"]`)
      ?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [hitPos, hits]);

  const findStatusText =
    findQuery === ""
      ? "Find: no query"
      : finding
        ? "Searching…"
        : hits.length === 0
          ? "No matches"
          : `Hit ${hitPos + 1} of ${hits.length}`;

  if (status === "loading") return <p>Loading table comparison…</p>;
  if (status === "error") return <p role="alert">Table compare failed: {error}</p>;
  if (status === "empty" || grid === null) return <p>No cells to display.</p>;

  const colCount = Math.max(
    grid.header.length,
    grid.totalCols,
    ...grid.rowsL.map((r) => r.length),
    ...grid.rowsR.map((r) => r.length),
  );
  const cols: number[] = [];
  for (let c = 0; c < colCount; c += 1) cols.push(c);
  // Wave 32: visible columns = all loaded columns minus hidden ones. The
  // grid (head + cells + diff classes) renders only these; the data stays
  // loaded underneath so show-all restores without refetching.
  const visibleCols = cols.filter((c) => !hiddenCols.has(c));
  const hiddenCount = colCount - visibleCols.length;
  const loadedRows = Math.max(grid.rowsL.length, grid.rowsR.length);
  const hasMore = grid.rowsL.length < grid.totalRows;

  /**
   * Wave-68: loaded-scope difference rows for prev/next nav. Same
   * cell rule the grid renders (`lv` vs `rv` over the VISIBLE columns,
   * so hiding a column narrows the nav exactly like the `.tc-diff`
   * cells). Wave-69: the status bar total comes from the summary
   * (`totalDiffRows`), not from this loaded scope.
   */
  function isDiffRow(r: number): boolean {
    for (const c of visibleCols) {
      const lv = grid?.rowsL[r]?.[c];
      const rv = grid?.rowsR[r]?.[c];
      if (lv !== undefined && rv !== undefined && lv !== rv) return true;
    }
    return false;
  }

  function toggleCol(c: number): void {
    setHiddenCols((prev) => {
      const next = new Set(prev);
      if (next.has(c)) next.delete(c);
      else next.add(c);
      return next;
    });
    // Wave 38: visibility changes the export scope — drop any stale receipt.
    setExportNote(null);
  }

  function showAllCols(): void {
    setHiddenCols(new Set());
    // Wave 38: visibility changes the export scope — drop any stale receipt.
    setExportNote(null);
  }

  /**
   * Wave-91: per-side diff rule. A cell pair diffs when both sides
   * carry a value and they disagree (missing side = empty, never a
   * diff) — byte-identical to the pre-91 merged rule, applied
   * per-side so both grids tint the same changed coordinates.
   */
  function isDiffCell(r: number, c: number): boolean {
    const lv = grid?.rowsL[r]?.[c];
    const rv = grid?.rowsR[r]?.[c];
    return lv !== undefined && rv !== undefined && lv !== rv;
  }

  /**
   * Wave-91: hit indexes landing on a rendered cell. Legacy (left
   * grid) keeps the merged rule — left AND right hits share one
   * cell — so pre-91 find specs hold; the right grid takes only
   * right-side hits (`side === "right"`).
   */
  function hitsAt(r: number, c: number, side: "left" | "right"): number[] {
    const out: number[] = [];
    for (let i = 0; i < hits.length; i += 1) {
      const h = hits[i];
      if (h.row !== r || h.col !== c) continue;
      if (side === "left" || h.side === "right") out.push(i);
    }
    return out;
  }

  function hitStyle(isCurrent: boolean, isHit: boolean): React.CSSProperties | undefined {
    if (isCurrent)
      return { outline: "2px solid var(--ac-focus-ring, #f59e0b)", outlineOffset: "-2px" };
    if (isHit)
      return {
        backgroundColor: "color-mix(in srgb, var(--ac-focus-ring, #f59e0b) 25%, transparent)",
      };
    return undefined;
  }

  // Wave-146: 22px density pin (BC compact rows). Cells carry zero
  // vertical padding so the row content (12px * 1.55 = 18.6px) fits
  // inside the token `.tc-row` 22px height; `py-[space-1]` pushed rows
  // to 28px and fought the ROW_PX virtualizer estimate.
  const CELL_BASE =
    "tc-cell px-[var(--ac-space-2)] py-0 ac-mono font-normal tabular-nums";
  const TABLE_CLASS =
    "w-full border-collapse ac-mono text-[length:var(--ac-font-size-code)] font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)]";

  function renderLeftCells(r: number): React.JSX.Element[] {
    return visibleCols.map((c) => {
      const lv = grid?.rowsL[r]?.[c];
      const value = lv ?? "";
      const diff = isDiffCell(r, c);
      const cellHits = hitsAt(r, c, "left");
      const isCurrent = cellHits.includes(hitPos) && hits.length > 0;
      const isHit = cellHits.length > 0;
      const rv = grid?.rowsR[r]?.[c];
      return (
        <td
          key={c}
          data-testid={isCurrent ? `table-hit-${hitPos}` : `tableview/cell-${r}-${c}`}
          data-hit={isHit ? "true" : undefined}
          data-diff={diff ? "true" : undefined}
          className={diff ? `tc-cell tc-diff ${CELL_BASE}` : CELL_BASE}
          style={hitStyle(isCurrent, isHit)}
          title={rv !== undefined && rv !== lv ? `right: ${rv}` : undefined}
        >
          {value}
        </td>
      );
    });
  }

  function renderRightCells(r: number): React.JSX.Element[] {
    return visibleCols.map((c) => {
      const rv = grid?.rowsR[r]?.[c];
      const value = rv ?? "";
      const diff = isDiffCell(r, c);
      const cellHits = hitsAt(r, c, "right");
      const isCurrent =
        hits.length > 0 && hits[hitPos]?.side === "right" && cellHits.includes(hitPos);
      const isHit = cellHits.length > 0;
      const lv = grid?.rowsL[r]?.[c];
      return (
        <td
          key={c}
          data-testid={
            isCurrent ? `table-hit-right-${hitPos}` : `tableview/cell-right-${r}-${c}`
          }
          data-hit-right={isHit ? "true" : undefined}
          data-diff={diff ? "true" : undefined}
          className={diff ? `tc-cell tc-diff ${CELL_BASE}` : CELL_BASE}
          style={hitStyle(isCurrent, isHit)}
          title={lv !== undefined && lv !== rv ? `left: ${lv}` : undefined}
        >
          {value}
        </td>
      );
    });
  }

  // Wave-146: row-number gutter rides the same 22px density (py-0,
  // content 18.6px line box inside the 22px `.tc-row` height).
  const ROW_NUM_CLASS =
    "px-[var(--ac-space-2)] py-0 text-right ac-mono tabular-nums text-[length:var(--ac-font-size-code)] font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg-muted)]";

  function renderLeftRow(r: number, style?: React.CSSProperties): React.JSX.Element {
    return (
      <tr
        key={r}
        data-testid={`tableview/row-${r}`}
        className="tc-row"
        style={style}
      >
        <th className={ROW_NUM_CLASS}>{r + 1}</th>
        {renderLeftCells(r)}
      </tr>
    );
  }

  function renderRightRow(r: number, style?: React.CSSProperties): React.JSX.Element {
    return (
      <tr
        key={r}
        data-testid={`tableview/row-right-${r}`}
        className="tc-row"
        style={style}
      >
        <th className={ROW_NUM_CLASS}>{r + 1}</th>
        {renderRightCells(r)}
      </tr>
    );
  }

  const rowIdx: number[] = [];
  for (let r = 0; r < loadedRows; r += 1) rowIdx.push(r);
  /**
   * Wave-68: loaded-scope difference rows for prev/next nav
   * (`isDiffRow` above mirrors the grid's `.tc-diff` rule over the
   * visible columns). Wave-69: the status bar shows the FULL-table
   * total from the summary (`totalDiffRows`, falling back to this
   * loaded scope when the backend predates the field).
   */
  const diffRows: readonly number[] = rowIdx.filter(isDiffRow);
  const sameCount = loadedRows - diffRows.length;
  // Wave-69: FULL-table totals from the summary for the status bar
  // (null on old backends => fall back to the loaded scope above).
  const shownDiffs = totalDiffRows ?? diffRows.length;
  const shownSame =
    totalDiffRows === null || grid === null
      ? sameCount
      : Math.max(grid.totalRows - totalDiffRows, 0);
  const safeDiffPos = diffRows.length === 0 ? -1 : diffPos % diffRows.length;

  function scrollToDiffRow(r: number): void {
    document
      .querySelector(`[data-testid="tableview/row-${r}"]`)
      ?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }

  function stepDiff(dir: 1 | -1): void {
    if (diffRows.length === 0 || safeDiffPos < 0) return;
    const next = (safeDiffPos + dir + diffRows.length) % diffRows.length;
    setDiffPos(next);
    scrollToDiffRow(diffRows[next]);
  }
  // Wave 24: honest fallback — the toggle always sends an explicit
  // `by_key` param, but a backend without key support echoes
  // `byKey: false` / empty `keyColumns`. Never a crash: just note it.
  const showFallbackNote = byKey && !effectiveByKey;
  // Wave-86: compact per-side meta text. Null on either side = the
  // backend omitted that side's meta (old backend / mock without
  // echo): the whole line hides honestly (see render) instead of
  // fabricating values. `formatTableMeta` is the pure, unit-coverable
  // half; these consts are the reactive half.
  const leftMetaText = formatTableMeta(leftMeta);
  const rightMetaText = formatTableMeta(rightMeta);
  const isKeyCol = (c: number): boolean => effectiveByKey && keyColumns.includes(c);

  /**
   * Wave-44: display order of the loaded rows. Session order when no
   * sort is active; otherwise a stable, numeric-aware sort by the
   * column's display value (same left-preferred resolution the grid
   * and export render). Row numbers shown stay physical (r + 1).
   */
  const orderedRows: readonly number[] = (() => {
    if (sortCol === null || grid === null) return rowIdx;
    const col = sortCol;
    const dir = sortDir;
    const val = (r: number): string =>
      grid?.rowsL[r]?.[col] ?? grid?.rowsR[r]?.[col] ?? "";
    const num = (v: string): number | null => {
      if (v.trim() === "") return null;
      const n = Number(v);
      return Number.isFinite(n) ? n : null;
    };
    return [...rowIdx].sort((a, b) => {
      const va = val(a);
      const vb = val(b);
      const na = num(va);
      const nb = num(vb);
      let cmp: number;
      if (na !== null && nb !== null) cmp = na - nb;
      else cmp = va.localeCompare(vb);
      if (cmp !== 0) return dir * cmp;
      return a - b;
    });
  })();

  /**
   * Wave-66M: user-designated key column (single-column only, Beyond
   * Compare parity). Picking an unkeyed column requests it explicitly
   * (`key_columns: [c]`) and enables key mode; clicking the keyed column
   * clears back to auto (`[]` = legacy path). Pure state change -- the
   * session reloads through the compare effect.
   */
  function toggleKeyColumn(c: number): void {
    if (pickedKey === c) {
      setPickedKey(null);
    } else {
      setPickedKey(c);
      setByKey(true);
    }
  }

  /** Header click cycles unsorted -> ascending -> descending -> unsorted. */
  function cycleSort(c: number): void {
    if (sortCol !== c) {
      setSortCol(c);
      setSortDir(1);
    } else if (sortDir === 1) {
      setSortDir(-1);
    } else {
      setSortCol(null);
      setSortDir(1);
    }
    // The displayed (and exported) order changed — drop stale receipt.
    setExportNote(null);
  }

  function sortNoteText(): string | null {
    if (sortCol === null || grid === null) return null;
    const name = grid.header[sortCol] ?? `col ${sortCol + 1}`;
    const more = hasMore ? " (loaded rows — load more to extend)" : "";
    return `Sorted by ${name} ${sortDir === 1 ? "ascending" : "descending"}${more}.`;
  }

  /**
   * Wave 38: RFC-4180 style field quoting — quote only when the value
   * contains a comma, quote, CR, or LF; embedded quotes double up.
   */
  function csvField(v: string): string {
    return /[",\r\n]/.test(v) ? `"${v.replace(/"/g, '""')}"` : v;
  }

  /**
   * Wave 38: export the currently-visible grid (Beyond Compare style
   * save-report). Serializes loaded rows in session order (which already
   * reflects the wave-24 by-key / positional alignment) restricted to
   * `visibleCols`, so hidden columns are excluded without any new IPC.
   * A find with hits narrows the export to the distinct hit rows and the
   * receipt names that scope; empty grid / all-columns-hidden /
   * find-with-no-matches refuse inline (receipt note, no download, never
   * a crash). The CSV goes to a `table-export.csv` Blob download — no
   * filesystem writes, no new IPC. Native button handler, keyboard
   * reachable via Tab / Enter / Space.
   */
  function exportCsv(): void {
    try {
      if (grid === null || loadedRows === 0) {
        setExportNote("Nothing to export — grid is empty.");
        return;
      }
      if (visibleCols.length === 0) {
        setExportNote("Nothing to export — all columns hidden.");
        return;
      }
      if (findQuery !== "" && hits.length === 0) {
        setExportNote(`No matches for "${findQuery}" — nothing exported.`);
        return;
      }
      // Scope: distinct hit rows (ascending) when a find is active,
      // otherwise every loaded row. Bounds-checked — never a crash.
      let scopeRows: readonly number[];
      if (findQuery !== "" && hits.length > 0) {
        const uniq = new Set<number>();
        for (const h of hits) {
          if (h.row >= 0 && h.row < loadedRows) uniq.add(h.row);
        }
        scopeRows = [...uniq].sort((a, b) => a - b);
        if (scopeRows.length === 0) {
          setExportNote(`No matches for "${findQuery}" — nothing exported.`);
          return;
        }
      } else {
        // Wave-44: export the displayed order (sorted when a sort is on).
        scopeRows = [...orderedRows];
      }
      // Same visible values the grid renders: left preferred, right
      // fallback, missing cells empty.
      const lines: string[] = [
        visibleCols.map((c) => csvField(grid.header[c] ?? `col ${c + 1}`)).join(","),
      ];
      for (const r of scopeRows) {
        lines.push(
          visibleCols
            .map((c) => csvField(grid.rowsL[r]?.[c] ?? grid.rowsR[r]?.[c] ?? ""))
            .join(","),
        );
      }
      const blob = new Blob([lines.join("\r\n") + "\r\n"], {
        type: "text/csv;charset=utf-8",
      });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = "table-export.csv";
      document.body.appendChild(anchor);
      anchor.click();
      anchor.remove();
      window.setTimeout(() => URL.revokeObjectURL(url), 1000);
      const order =
        sortCol !== null && grid !== null
          ? `sorted by ${grid.header[sortCol] ?? `col ${sortCol + 1}`} ${sortDir === 1 ? "ascending" : "descending"}`
          : byKey && effectiveByKey
            ? "by-key order"
            : "positional order";
      const scope =
        findQuery !== "" && hits.length > 0
          ? `find "${findQuery}": ${hits.length} hits to ${scopeRows.length} rows (${order})`
          : `all ${scopeRows.length} loaded rows (${order})`;
      const hiddenSuffix =
        hiddenCount > 0
          ? `, ${hiddenCount} hidden column${hiddenCount === 1 ? "" : "s"} excluded`
          : "";
      setExportNote(
        `Exported ${scopeRows.length} rows × ${visibleCols.length} cols to table-export.csv — ${scope}${hiddenSuffix}.`,
      );
      try {
        (window as unknown as Record<string, unknown>)["__TABLE_EXPORT_LAST__"] = {
          rows: scopeRows.length,
          cols: visibleCols.length,
        };
      } catch {
        /* ignore */
      }
    } catch {
      // Never crash the view: report inline, download nothing.
      setExportNote("Export failed — nothing exported.");
    }
  }

  /**
   * Wave-91: per-side column headers. The left header keeps every
   * legacy testid (`table-sort-C`, `table-key-C`,
   * `tableview/key-col-C`); the right header mirrors the same state
   * through `-right` suffixed siblings driving the same shared
   * sort/key handlers.
   */
  function renderHeadCell(c: number, side: "left" | "right"): React.JSX.Element {
    const key = isKeyCol(c);
    const active = sortCol === c;
    const name = grid?.header[c] ?? `col ${c + 1}`;
    const right = side === "right";
    const sortId = right ? `table-sort-right-${c}` : `table-sort-${c}`;
    const keyId = right ? `table-key-right-${c}` : `table-key-${c}`;
    return (
      <th
        key={c}
        data-key={key ? "true" : undefined}
        data-testid={key ? (right ? `tableview/key-col-right-${c}` : `tableview/key-col-${c}`) : undefined}
        aria-sort={active ? (sortDir === 1 ? "ascending" : "descending") : "none"}
        className={key ? "tc-head tc-key px-[var(--ac-space-2)] py-[var(--ac-space-1)]" : "tc-head px-[var(--ac-space-2)] py-[var(--ac-space-1)]"}
        title={key ? "key column" : undefined}
      >
        <button
          type="button"
          data-testid={sortId}
          onClick={() => cycleSort(c)}
          aria-label={`sort by ${name}${right ? " (right)" : ""}${active ? (sortDir === 1 ? " (ascending, activate for descending)" : " (descending, activate to clear)") : ""}`}
          className="rounded-sm font-inherit focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
        >
          {name}
          {active ? (
            sortDir === 1 ? (
              <ChevronUp size={12} aria-hidden="true" className="inline" />
            ) : (
              <ChevronDown size={12} aria-hidden="true" className="inline" />
            )
          ) : null}
        </button>{" "}
        <button
          type="button"
          data-testid={keyId}
          aria-pressed={pickedKey === c}
          aria-label={pickedKey === c ? `remove key column ${name}` : `use ${name} as key column`}
          title={pickedKey === c ? "key column (activate to clear)" : "use as key column"}
          onClick={() => toggleKeyColumn(c)}
          className="tb-key-btn rounded-sm font-inherit focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
        >
          key
        </button>
      </th>
    );
  }

  function renderLeftHeadCell(c: number): React.JSX.Element {
    return renderHeadCell(c, "left");
  }

  function renderRightHeadCell(c: number): React.JSX.Element {
    return renderHeadCell(c, "right");
  }

  return (
    <section aria-label="table comparison">
      {/* Wave-68: single-row chrome (Beyond Compare style). One toolbar
          row owns the by-key toggle, Columns toggle, Export, Find
          toggle and difference nav; the find panel + cols menu collapse
          in-flow below it. All legacy testids ride the same controls. */}
      <TableToolbar
        byKey={byKey}
        onToggleByKey={(next) => {
          setByKey(next);
          // Wave-66M: leaving key mode drops the explicit designation
          // too, so an unchecked toggle is always pure positional.
          if (!next) setPickedKey(null);
        }}
        colsOpen={colsOpen}
        onToggleCols={() => setColsOpen((o) => !o)}
        showAllDisabled={hiddenCount === 0}
        onShowAll={showAllCols}
        hiddenCount={hiddenCount}
        onExport={exportCsv}
        findOpen={findOpen}
        onToggleFind={() => setFindOpen((v) => !v)}
        diffCount={diffRows.length}
        diffPos={safeDiffPos}
        onPrevDiff={() => stepDiff(-1)}
        onNextDiff={() => stepDiff(1)}
      />
      {/* Wave-87: BC-style two-pane header (mirrors the text pane
          grid). One compact `1fr / 6px / 1fr` row with a thin center
          splitter: each pane owns its path line over its meta line.
          Paths always render; meta lines mount ONLY when the
          `table_compare` summary carried file meta (wave-85 backend) —
          old backends and the mock omit it, so nothing extra renders
          (no crash, no fabricated values). The `tableview/meta`
          wrapper is `display: contents`, so it adds no box: its three
          children join the pane grid as the meta row. */}
      <div
        data-testid="tableview/panes"
        className="grid grid-cols-[1fr_6px_1fr] items-stretch gap-x-[var(--ac-space-2)] border-b border-[var(--ac-border-subtle)] bg-[var(--ac-bg)] px-[var(--ac-space-4)] py-[var(--ac-space-1)]"
      >
        <div data-testid="tableview/pane-left" className="min-w-0">
          <p
            data-testid="tableview/path-left"
            title={leftPath}
            className="ac-chrome-label truncate font-medium text-[var(--ac-fg)]"
          >
            {leftPath}
          </p>
        </div>
        <span aria-hidden="true" className="tv-splitter" />
        <div data-testid="tableview/pane-right" className="min-w-0 text-right">
          <p
            data-testid="tableview/path-right"
            title={rightPath}
            className="ac-chrome-label truncate font-medium text-[var(--ac-fg)]"
          >
            {rightPath}
          </p>
        </div>
        {leftMetaText === null && rightMetaText === null ? null : (
          <div data-testid="tableview/meta" className="contents">
            <p
              data-testid="tableview/meta-left"
              title={leftMetaText ?? undefined}
              className="ac-chrome-label min-w-0 truncate font-normal text-[var(--ac-fg-muted)]"
            >
              {leftMetaText ?? ""}
            </p>
            <span aria-hidden="true" className="tv-splitter" />
            <p
              data-testid="tableview/meta-right"
              title={rightMetaText ?? undefined}
              className="ac-chrome-label min-w-0 truncate text-right font-normal text-[var(--ac-fg-muted)]"
            >
              {rightMetaText ?? ""}
            </p>
          </div>
        )}
      </div>
      {exportNote !== null && (
        <p data-testid="table-export-receipt" role="status" className="px-[var(--ac-space-4)] py-[var(--ac-space-1)] ac-mono text-[length:var(--ac-font-size-code)] font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg-muted)] border-b border-[var(--ac-border-subtle)] bg-[var(--ac-bg)]">
          {exportNote}
        </p>
      )}
      {showFallbackNote && (
        <p data-testid="table-bykey-note" className="px-[var(--ac-space-4)] py-[var(--ac-space-2)] ac-chrome-label font-normal text-[var(--ac-fg-muted)] border-b border-[var(--ac-border-subtle)] bg-[var(--ac-bg)]">
          By-key requested — backend returned positional alignment (no key columns); showing positional rows.
        </p>
      )}
      {sortNoteText() === null ? null : (
        <p data-testid="table-sort-status" role="status" className="px-[var(--ac-space-4)] py-[var(--ac-space-1)] ac-chrome-label font-normal text-[var(--ac-fg-muted)] border-b border-[var(--ac-border-subtle)] bg-[var(--ac-bg)]">
          {sortNoteText()}
        </p>
      )}
      <p data-testid="table-cols-count" className="px-[var(--ac-space-4)] py-[var(--ac-space-1)] ac-chrome-label font-normal text-[var(--ac-fg-muted)] border-b border-[var(--ac-border-subtle)] bg-[var(--ac-bg)]">
        {loadedRows} / {grid.totalRows} rows ·{" "}
        {hiddenCount === 0
          ? `${colCount} cols`
          : `${visibleCols.length} of ${colCount} cols (${hiddenCount} hidden)`}
      </p>
      {colsOpen && (
        <div
          id="table-cols-menu"
          data-testid="table-cols-menu"
          role="group"
          aria-label="column visibility"
          onKeyDown={(e) => {
            if (e.key === "Escape") setColsOpen(false);
          }}
          className="flex flex-wrap items-center gap-[var(--ac-space-3)] px-[var(--ac-space-4)] py-[var(--ac-space-2)] border-b border-[var(--ac-border-subtle)] bg-[var(--ac-bg)]"
        >
          {cols.map((c) => {
            const name = grid?.header[c] ?? `col ${c + 1}`;
            const cid = `table-col-visibility-${c}`;
            return (
              <span key={c} className="inline-flex items-center gap-[var(--ac-space-1)]">
                <input
                  id={cid}
                  data-testid={cid}
                  type="checkbox"
                  checked={!hiddenCols.has(c)}
                  onChange={() => toggleCol(c)}
                />
                <label htmlFor={cid} className="ac-chrome-label font-medium text-[var(--ac-fg-muted)]">
                  {name}
                </label>
              </span>
            );
          })}
        </div>
      )}
      {findOpen && (
      <div role="search" aria-label="find in table" className="flex flex-wrap items-center gap-[var(--ac-space-2)] px-[var(--ac-space-4)] py-[var(--ac-space-2)] border-b border-[var(--ac-border-subtle)] bg-[var(--ac-bg)]">
        <label htmlFor="table-find-input" className="ac-chrome-label font-medium text-[var(--ac-fg-muted)]">
          Find
        </label>
        <input
          id="table-find-input"
          data-testid="table-find-input"
          type="search"
          value={findInput}
          onChange={(e) => setFindInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") void runFind(findInput);
            else if (e.key === "Escape") setFindOpen(false);
          }}
          placeholder="Find in table…"
          className="rounded-md border border-[var(--ac-border)] bg-[var(--ac-bg-raised)] px-[var(--ac-space-2)] py-[var(--ac-space-1)] ac-mono text-[length:var(--ac-font-size-code)] font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg)]"
        />
        <button
          type="button"
          data-testid="table-find-search"
          onClick={() => void runFind(findInput)}
          disabled={finding}
          className="inline-flex items-center rounded-md border border-[var(--ac-border)] bg-[var(--ac-bg-raised)] px-[var(--ac-space-3)] py-[var(--ac-space-2)] ac-chrome-label font-medium text-[var(--ac-fg)] shadow-xs transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)] disabled:cursor-not-allowed disabled:opacity-45"
        >
          {finding ? "Searching…" : "Find"}
        </button>
        <button
          type="button"
          data-testid="table-find-next"
          onClick={nextHit}
          disabled={hits.length === 0}
          className="inline-flex items-center rounded-md border border-[var(--ac-border)] bg-[var(--ac-bg-raised)] px-[var(--ac-space-3)] py-[var(--ac-space-2)] ac-chrome-label font-medium text-[var(--ac-fg)] shadow-xs transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)] disabled:cursor-not-allowed disabled:opacity-45"
        >
          Next
        </button>
        <span data-testid="table-find-status" role="status" className="ac-mono text-[length:var(--ac-font-size-code)] font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg-muted)]">
          {findStatusText}
        </span>
      </div>
      )}
      {hiddenCount > 0 && findQuery !== "" && findOpen && (
        <p data-testid="table-find-hidden-note" className="px-[var(--ac-space-4)] py-[var(--ac-space-2)] ac-chrome-label font-normal text-[var(--ac-fg-muted)] border-b border-[var(--ac-border-subtle)] bg-[var(--ac-bg)]">
          Find skips {hiddenCount} hidden column{hiddenCount === 1 ? "" : "s"} — unhide or Show all to include {hiddenCount === 1 ? "it" : "them"}.
        </p>
      )}
      {/* Wave-91: BC-table red diff cells in both schemes via the
          existing del tokens (no new palette; tokens.css untouched).
          Later in the cascade than the linked stylesheet, so this
          wins at equal specificity and the dark `@media` token swap
          still applies. */}
      <style data-testid="tableview/diff-style">{`.tc-diff{background-color:var(--ac-diff-del-bg);color:var(--ac-diff-del-fg);}`}</style>
      {loadedRows > VIRTUALIZE_AFTER ? (
        <div ref={parentRef} data-testid="tableview/grids-scroller" className="h-[var(--ac-scroll-h)] overflow-auto">
          <div
            data-testid="tableview/grids"
            className="grid grid-cols-[1fr_6px_1fr] items-start gap-x-[var(--ac-space-2)]"
          >
            <div data-testid="tableview/grid-left" className="min-w-0">
              <table className={TABLE_CLASS}>
                <thead className="sticky top-0">
                  <tr>
                    <th aria-label="row number" />
                    {visibleCols.map((c) => renderLeftHeadCell(c))}
                  </tr>
                </thead>
                <tbody style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
                  {virtualizer.getVirtualItems().map((v) => {
                    // Wave-44: virtual position -> physical row through the
                    // display order; the row-number cell stays physical.
                    const r = orderedRows[v.index] ?? v.index;
                    return renderLeftRow(r, {
                      position: "absolute",
                      top: 0,
                      left: 0,
                      width: "100%",
                      transform: `translateY(${v.start}px)`,
                    });
                  })}
                </tbody>
              </table>
            </div>
            <span aria-hidden="true" data-testid="tableview/grid-splitter" className="tv-splitter" />
            <div data-testid="tableview/grid-right" className="min-w-0">
              <table className={TABLE_CLASS}>
                <thead className="sticky top-0">
                  <tr>
                    <th aria-label="row number" />
                    {visibleCols.map((c) => renderRightHeadCell(c))}
                  </tr>
                </thead>
                <tbody style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
                  {virtualizer.getVirtualItems().map((v) => {
                    const r = orderedRows[v.index] ?? v.index;
                    return renderRightRow(r, {
                      position: "absolute",
                      top: 0,
                      left: 0,
                      width: "100%",
                      transform: `translateY(${v.start}px)`,
                    });
                  })}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      ) : (
        <div
          data-testid="tableview/grids"
          className="grid grid-cols-[1fr_6px_1fr] items-start gap-x-[var(--ac-space-2)]"
        >
          <div data-testid="tableview/grid-left" className="min-w-0 overflow-x-auto">
            <table className={TABLE_CLASS}>
              <thead>
                <tr>
                  <th aria-label="row number" />
                  {visibleCols.map((c) => renderLeftHeadCell(c))}
                </tr>
              </thead>
              <tbody>{orderedRows.map((r) => renderLeftRow(r))}</tbody>
            </table>
          </div>
          <span aria-hidden="true" data-testid="tableview/grid-splitter" className="tv-splitter" />
          <div data-testid="tableview/grid-right" className="min-w-0 overflow-x-auto">
            <table className={TABLE_CLASS}>
              <thead>
                <tr>
                  <th aria-label="row number" />
                  {visibleCols.map((c) => renderRightHeadCell(c))}
                </tr>
              </thead>
              <tbody>{orderedRows.map((r) => renderRightRow(r))}</tbody>
            </table>
          </div>
        </div>
      )}
      {hasMore && (
        <button type="button" onClick={() => void loadMore()} disabled={loadingMore} className="my-[var(--ac-space-3)] ml-[var(--ac-space-4)] inline-flex items-center rounded-md border border-[var(--ac-border)] bg-[var(--ac-bg-raised)] px-[var(--ac-space-3)] py-[var(--ac-space-2)] ac-chrome-label font-medium text-[var(--ac-fg)] shadow-xs transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)] disabled:cursor-not-allowed disabled:opacity-45">
          {loadingMore ? "Loading…" : `Load more (${grid.rowsL.length}/${grid.totalRows})`}
        </button>
      )}
      {/* Wave-69: the status bar shows the FULL-table total diff count
          from the `table_compare` summary (`diffRows`); prev/next nav
          still walks the LOADED rows. */}
      <div data-testid="tableview/status" className="px-[var(--ac-space-4)] py-[var(--ac-space-2)] ac-chrome-label font-normal text-[var(--ac-fg-muted)] border-t border-[var(--ac-border-subtle)] bg-[var(--ac-bg)]">
        <span data-testid="tableview/diffs">
          {shownDiffs} difference row(s)
        </span>
        <span data-testid="tableview/same">
          {" · "}{shownSame} same
        </span>
        <span>
          {" · "}{loadedRows} loaded of {grid.totalRows} rows
        </span>
        <span>
          {" · "}Load time: {loadSecs === null ? "…" : `${loadSecs.toFixed(2)} seconds`}
        </span>
      </div>
    </section>
  );
}
