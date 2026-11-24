import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import type { SessionDocument } from "./session";

/**
 * Typed Tauri `invoke` wrapper with a fixture-mock adapter.
 *
 * - Production: forwards to `@tauri-apps/api` `invoke`.
 * - `VITE_MOCK_IPC=1` (Playwright / `npm run dev` without the shell):
 *   serves in-memory fixtures so the Home view boots with zero IPC surface
 *   (Wave 0 registers no commands yet).
 *
 * Wave 1 replaces the fixture branch command-by-command as `commands/*`
 * lands in `src-tauri`.
 */

function isMockIpc(): boolean {
  if (import.meta.env.VITE_MOCK_IPC === "1") return true;
  if (typeof window !== "undefined") {
    const w = window as unknown as Record<string, unknown>;
    if (w["__TAURI_INTERNALS__"] === undefined && w["__TAURI__"] === undefined) {
      return true;
    }
  }
  return false;
}

// ---------------------------------------------------------------------------
// Wave 4A: real engine-shaped fixtures for the mock branch ONLY.
//
// `dumped.json` is committed real output of
// `src-tauri/examples/dump_fixtures.rs` (real `text_compare` /
// `folder_compare` / `table_compare` engine calls over built-in inputs).
// It is lazy-loaded here so production bundles never execute this path,
// and the mock honors `(offset, limit)` paging exactly like the Rust
// handlers (out-of-range start yields an empty window, never an error).
// Unknown commands keep the pre-4A empty-fixture behavior (`[]`).
// ---------------------------------------------------------------------------

interface DumpedTextRow {
  readonly index: number;
  readonly kind: string;
  readonly left: string | null;
  readonly right: string | null;
  readonly inline?: readonly unknown[];
}

interface DumpedFolderSide {
  readonly size: number;
  readonly modifiedMs: number | null;
}

interface DumpedFolderNode {
  readonly relPath: string;
  readonly isDir: boolean;
  readonly left: DumpedFolderSide | null;
  readonly right: DumpedFolderSide | null;
  readonly status: string;
}

interface DumpedTableCells {
  readonly header: readonly string[];
  readonly headerR: readonly string[];
  readonly rowsL: ReadonlyArray<readonly string[]>;
  readonly rowsR: ReadonlyArray<readonly string[]>;
  readonly totalRowsL: number;
  readonly totalRowsR: number;
  readonly totalCols: number;
  readonly leftCols: number;
  readonly rightCols: number;
  readonly equal: boolean;
}

interface DumpedFixture {
  readonly textRows: readonly DumpedTextRow[];
  readonly folderNodes: readonly DumpedFolderNode[];
  readonly tableCells: DumpedTableCells;
}

let dumpedCache: Promise<DumpedFixture | null> | null = null;

function loadDumped(): Promise<DumpedFixture | null> {
  if (dumpedCache === null) {
    dumpedCache = import("../../e2e/helpers/dumped.json").then(
      (mod) => (mod as unknown as { default: DumpedFixture }).default,
      () => null,
    );
  }
  return dumpedCache;
}

function numArg(args: Record<string, unknown>, names: readonly string[], fallback: number): number {
  for (const name of names) {
    const value = args[name];
    if (typeof value === "number" && Number.isFinite(value) && value >= 0) {
      return Math.floor(value);
    }
  }
  return fallback;
}

function strArg(args: Record<string, unknown>, names: readonly string[], fallback: string): string {
  for (const name of names) {
    const value = args[name];
    if (typeof value === "string") return value;
  }
  return fallback;
}

const MOCK_SESSIONS_KEY = "autocompare:mockSessionStore:v1";
const MOCK_SESSION_SEQ_KEY = "autocompare:mockSessionSeq:v1";

function mockSessionStoreRead(): Record<string, SessionDocument> {
  try {
    if (typeof window === "undefined" || !window.localStorage) return {};
    const raw = window.localStorage.getItem(MOCK_SESSIONS_KEY);
    if (!raw) return {};
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {};
    return parsed as Record<string, SessionDocument>;
  } catch {
    return {};
  }
}

function mockSessionStoreWrite(next: Record<string, SessionDocument>): void {
  try {
    if (typeof window === "undefined" || !window.localStorage) return;
    window.localStorage.setItem(MOCK_SESSIONS_KEY, JSON.stringify(next));
  } catch {
    // Storage unavailable: sessions stay memory-only, like a backend
    // whose disk write failed. Never throws into the UI.
  }
}

function mockSessionStoreMint(): string {
  let seq = 1;
  try {
    if (typeof window !== "undefined" && window.localStorage) {
      seq = Number(window.localStorage.getItem(MOCK_SESSION_SEQ_KEY) ?? "1") || 1;
      window.localStorage.setItem(MOCK_SESSION_SEQ_KEY, String(seq + 1));
    }
  } catch {
    seq = 1;
  }
  return `sess-mock-${Date.now()}-${seq}`;
}

// Wave-66V mock edit overlay: `text_apply_edit` patches served rows so
// edit + re-compare wiring is provable without the Rust backend. Cleared
// on reload (module state, never localStorage).
const mockTextEdits = new Map<number, { left?: string | null; right?: string | null }>();

export function __mockTextEditResetForTests(): void {
  mockTextEdits.clear();
}

async function mockInvoke<T>(cmd: string, _args?: Record<string, unknown>): Promise<T> {
  // Wave-65: localStorage-backed mock session store (mock branch ONLY).
  // The real backend persists under the system temp dir; the mock mirrors
  // that here so boot-restore across page.reload() is provable in WebKit.
  // Additive only: no existing mock case is touched, production never runs
  // this path.
  const store = mockSessionStoreRead();

  const args: Record<string, unknown> = _args ?? {};
  const dumped = await loadDumped();
  if (dumped === null) {
    // Fixture file missing: preserve the pre-4A empty-fixture behavior so
    // every view degrades to its "empty" state instead of crashing.
    return Promise.resolve([] as unknown as T);
  }
  switch (cmd) {
    case "session_list": {
      return Object.entries(store).map(([id, doc]) => ({
        id,
        name: typeof doc.name === "string" && doc.name !== "" ? doc.name : id,
        tabs: Array.isArray(doc.tabs) ? doc.tabs.length : 0,
        locked: doc.locked === true,
      })) as unknown as T;
    }
    case "session_save": {
      const descriptor = args["descriptor"];
      if (typeof descriptor !== "object" || descriptor === null || Array.isArray(descriptor)) {
        return Promise.resolve([] as unknown as T);
      }
      const id = mockSessionStoreMint();
      store[id] = descriptor as SessionDocument;
      mockSessionStoreWrite(store);
      return id as unknown as T;
    }
    case "session_load": {
      const id = strArg(args, ["id"], "");
      const doc = store[id];
      if (!doc) throw new Error(`unknown session: ${id}`);
      return doc as unknown as T;
    }
    case "session_delete": {
      const id = strArg(args, ["id"], "");
      if (store[id] !== undefined) {
        delete store[id];
        mockSessionStoreWrite(store);
      }
      return true as unknown as T;
    }
    case "text_compare": {
      const rows = dumped.textRows;
      let leftLines = 0;
      let rightLines = 0;
      let opCount = 0;
      let prevKind = "";
      for (const row of rows) {
        if (row.left !== null) leftLines += 1;
        if (row.right !== null) rightLines += 1;
        if (row.kind !== prevKind) {
          opCount += 1;
          prevKind = row.kind;
        }
      }
      // Wave-75: echo appliedOptions for EITHER spelling (camelCase
      // from the TextView probe path, snake_case from the typed
      // wrapper path), mirroring the real Rust backend which accepts
      // both and always echoes `appliedOptions`. Legacy `{left,
      // right}` (no flags) stays byte-identical with no echo. The
      // mock has no whitespace/case engine: rows are fixture rows, so
      // the echo proves the flags round-tripped and the view hides
      // the honest fallback note; real collapsing is covered by the
      // Rust unit tests in `commands/text.rs`.
      const hasIgnoreFlags =
        typeof args["ignore_whitespace"] === "boolean" ||
        typeof args["ignore_case"] === "boolean" ||
        typeof args["ignoreWhitespace"] === "boolean" ||
        typeof args["ignoreCase"] === "boolean";
      const base = {
        session: "mock-text-1",
        leftPath: strArg(args, ["left"], ""),
        rightPath: strArg(args, ["right"], ""),
        leftLines,
        rightLines,
        rows: rows.length,
        opCount,
        exact: true,
        truncated: false,
      } as unknown as Record<string, unknown>;
      if (hasIgnoreFlags) {
        const ws = args["ignore_whitespace"] === true || args["ignoreWhitespace"] === true;
        const cs = args["ignore_case"] === true || args["ignoreCase"] === true;
        base["appliedOptions"] = { ignoreWhitespace: ws, ignoreCase: cs };
        base["ignoreWhitespace"] = ws;
        base["ignoreCase"] = cs;
      }
      return base as unknown as T;
    }
    case "text_get_rows": {
      const total = dumped.textRows.length;
      const start = Math.min(numArg(args, ["start"], 0), total);
      const limit = Math.min(numArg(args, ["limit"], 200), 500);
      const rows = dumped.textRows.slice(start, start + limit).map((row) => {
        const patch = mockTextEdits.get(row.index);
        if (patch === undefined) return row;
        const left = patch.left !== undefined ? patch.left : row.left;
        const right = patch.right !== undefined ? patch.right : row.right;
        // Mock-only kind recompute: a patched row whose sides are both
        // present and identical becomes equal (spans cleared); orphans
        // keep their kind (null edits are rejected below anyway).
        const kind =
          left !== null && right !== null && left === right
            ? "equal"
            : row.kind;
        return {
          ...row,
          left,
          right,
          kind,
          inline: left !== null && right !== null && left === right ? [] : row.inline,
        };
      });
      return { session: "mock-text-1", start, total, rows } as unknown as T;
    }
    case "text_find": {
      // Wave-127: full-fixture search mirroring backend `text_find`
      // (`find_in_rows` in `commands/text.rs`): case-sensitive
      // substring over ALL cached rows, first hit wins (`down` scans
      // top-first left-then-right, `up` scans bottom-first
      // right-then-left). Lets the TextView prove its beyond-loaded-
      // rows fallback under `VITE_MOCK_IPC=1`. Empty query resolves
      // null (the view never sends one; the real backend errors).
      const query = strArg(args, ["query"], "");
      const direction = strArg(args, ["direction"], "down");
      if (query === "") return null as unknown as T;
      const ordered =
        direction === "up" ? [...dumped.textRows].reverse() : dumped.textRows;
      for (const row of ordered) {
        const sides: ReadonlyArray<{ readonly side: string; readonly line: string | null }> =
          direction === "up"
            ? [
                { side: "right", line: row.right },
                { side: "left", line: row.left },
              ]
            : [
                { side: "left", line: row.left },
                { side: "right", line: row.right },
              ];
        for (const { side, line } of sides) {
          if (line !== null && line.includes(query)) {
            return { index: row.index, side, column: line.indexOf(query), line } as unknown as T;
          }
        }
      }
      return null as unknown as T;
    }
    case "text_apply_edit": {
      const index = numArg(args, ["index"], -1);
      const side = strArg(args, ["side"], "");
      const text = strArg(args, ["text"], "");
      if (!Number.isInteger(index) || index < 0 || index >= dumped.textRows.length) {
        throw new Error(`row index ${index} out of range`);
      }
      if (side !== "left" && side !== "right") {
        throw new Error(`unknown side '${side}'`);
      }
      const row = dumped.textRows[index];
      if (row === undefined || row.index !== index) {
        throw new Error(`row index ${index} out of range`);
      }
      const cell = side === "left" ? row.left : row.right;
      if (cell === null || cell === undefined) {
        throw new Error(`row ${index} has no ${side} content to edit`);
      }
      if (typeof args["text"] !== "string") {
        throw new Error("text must be a string");
      }
      const prev = mockTextEdits.get(index) ?? {};
      mockTextEdits.set(index, { ...prev, [side]: text });
      const left = side === "left" ? text : (prev.left !== undefined ? prev.left : row.left);
      const right = side === "right" ? text : (prev.right !== undefined ? prev.right : row.right);
      const kind =
        left !== null && right !== null && left === right ? "equal" : row.kind;
      return {
        session: "mock-text-1",
        index,
        side,
        kind,
        rows: dumped.textRows.length,
      } as unknown as T;
    }
    case "folder_compare": {
      return {
        session: "mock-folder-1",
        leftRoot: strArg(args, ["left"], ""),
        rightRoot: strArg(args, ["right"], ""),
        total: dumped.folderNodes.length,
        truncated: false,
      } as unknown as T;
    }
    case "folder_get_tree": {
      const filter = strArg(args, ["filter"], "all");
      const entries = dumped.folderNodes.filter((entry) => {
        if (filter === "all") return true;
        if (filter === "changed") return entry.status !== "same";
        return entry.status === filter;
      });
      const total = entries.length;
      const start = Math.min(numArg(args, ["start"], 0), total);
      const limit = Math.min(numArg(args, ["limit"], 50), 500);
      return {
        session: "mock-folder-1",
        start,
        total,
        filter,
        entries: entries.slice(start, start + limit),
      } as unknown as T;
    }
    case "folder_cancel": {
      return {
        session: strArg(args, ["session"], "mock-folder-1"),
        cancelled: true,
      } as unknown as T;
    }
    case "table_compare": {
      const cells = dumped.tableCells;
      // Wave-69: total diff rows over the FULL fixture tables
      // (positional: zipped full-row mismatches + length delta),
      // mirroring the Rust `diff_rows` so the status bar total is
      // truthful under `VITE_MOCK_IPC=1`.
      const common = Math.min(cells.rowsL.length, cells.rowsR.length);
      let mismatches = 0;
      for (let i = 0; i < common; i += 1) {
        const a = cells.rowsL[i];
        const b = cells.rowsR[i];
        if (a.length !== b.length || a.some((v, c) => v !== b[c])) mismatches += 1;
      }
      const diffRows =
        mismatches + Math.abs(cells.rowsL.length - cells.rowsR.length);
      return {
        sessionId: "mock-table-1",
        leftRows: cells.totalRowsL,
        rightRows: cells.totalRowsR,
        leftCols: cells.leftCols,
        rightCols: cells.rightCols,
        equal: cells.equal,
        byKey: false,
        diffRows,
        keyColumns: [],
      } as unknown as T;
    }
    case "table_get_page": {
      const cells = dumped.tableCells;
      const startRow = Math.min(numArg(args, ["start_row", "startRow"], 0), cells.totalRowsL);
      const limit = Math.min(numArg(args, ["limit"], 100), 1000);
      const startCol = numArg(args, ["start_col", "startCol"], 0);
      const colLimit = Math.min(numArg(args, ["col_limit", "colLimit"], 20), 128);
      const sliceCols = (row: readonly string[]): readonly string[] =>
        row.slice(startCol, startCol + colLimit);
      const rowsL = cells.rowsL.slice(startRow, startRow + limit).map(sliceCols);
      const rowsR = cells.rowsR.slice(startRow, startRow + limit).map(sliceCols);
      const rowsRemain =
        startRow + limit < cells.totalRowsL || startRow + limit < cells.totalRowsR;
      return {
        sessionId: "mock-table-1",
        startRow,
        limit,
        startCol,
        colLimit,
        headerL: cells.header.slice(startCol, startCol + colLimit),
        headerR: cells.headerR.slice(startCol, startCol + colLimit),
        rowsL,
        rowsR,
        totalRowsL: cells.totalRowsL,
        totalRowsR: cells.totalRowsR,
        totalCols: cells.totalCols,
        truncated: rowsRemain || startCol + colLimit < cells.totalCols,
      } as unknown as T;
    }
    case "table_find": {
      // Wave-66G: numeric-aware find — substring win, else exact f64 equality (mirrors the exact-equality half of cells_equal_with_tolerance with no tolerance: find carries none).
      const query = strArg(args, ["query"], "");
      if (query === "") return [] as unknown as T;
      const matches = (cell: string): boolean => {
        if (cell.includes(query)) return true;
        const q = query.trim();
        const c = cell.trim();
        if (q === "" || c === "") return false;
        if (/^[+-]?0[xXbBoO]/.test(q) || /^[+-]?0[xXbBoO]/.test(c)) return false;
        const qn = Number(q);
        const cn = Number(c);
        return Number.isFinite(qn) && Number.isFinite(cn) && qn === cn;
      };
      // Mock-only: searches the loaded window (first 100 fixture rows, the
      // ROW_PAGE TableView pages) so IPC hits match the fallback localScan;
      // IPC knows nothing about visibility (the view filters hidden cols).
      const cells = dumped.tableCells;
      const hits: { side: string; row: number; col: number; value: string }[] = [];
      const sides: ReadonlyArray<{ side: string; rows: ReadonlyArray<readonly string[]> }> = [
        { side: "left", rows: cells.rowsL.slice(0, 100) },
        { side: "right", rows: cells.rowsR.slice(0, 100) },
      ];
      for (const { side, rows } of sides) {
        for (let r = 0; r < rows.length; r += 1) {
          const row = rows[r];
          for (let c = 0; c < row.length; c += 1) {
            if (matches(row[c])) {
              if (hits.length >= 100) return hits as unknown as T;
              hits.push({ side, row: r, col: c, value: row[c] });
            }
          }
        }
      }
      return hits as unknown as T;
    }
    case "sync_plan": {
      // Mock-only deterministic plan from the bundled engine folder
      // fixtures (engine default direction: left wins). Real semantics
      // belong to the Rust engine; this branch only lets the SyncView
      // spec prove the full plan -> apply loop without touching disk.
      const steps = dumped.folderNodes
        .filter((node) => node.status !== "same")
        .map((node) => ({
          rel_path: node.relPath,
          is_dir: node.isDir,
          action: node.left === null ? "CopyToLeft" : "CopyToRight",
        }));
      return { steps, skipped: [], truncated: false } as unknown as T;
    }
    case "sync_apply": {
      const steps = dumped.folderNodes
        .filter((node) => node.status !== "same")
        .map((node) => ({
          rel_path: node.relPath,
          is_dir: node.isDir,
          action: node.left === null ? "CopyToLeft" : "CopyToRight",
        }));
      const copies = steps.length;
      const dry = args["dry_run"] === true || args["dryRun"] === true;
      return {
        dry_run: dry,
        copied: dry ? 0 : copies,
        deleted: 0,
        planned_copies: dry ? copies : 0,
        planned_deletes: 0,
        failed: [],
      } as unknown as T;
    }
    case "picture_compare": {
      // Mock-only fixed summary (proves MediaView wiring end to end;
      // real metrics come from the Rust engine).
      return {
        receipt: "mock-picture-1",
        identical: false,
        format: "png",
        widthL: 64,
        heightL: 64,
        widthR: 64,
        heightR: 64,
        sizeL: 4096,
        sizeR: 4100,
        score: 0.997,
        differingBytes: 12,
        totalBytes: 4100,
        samples: [
          { offset: 101, left: 12, right: 13 },
          { offset: 202, left: 200, right: 201 },
        ],
        truncated: false,
      } as unknown as T;
    }
    case "audio_compare": {
      return {
        receipt: "mock-audio-1",
        identical: false,
        configMismatch: false,
        sampleRateL: 44100,
        sampleRateR: 44100,
        channelsL: 2,
        channelsR: 2,
        durationL: 1.0,
        durationR: 1.0,
        tolerance: 0,
        windowsTotal: 100,
        windowsDiffering: 3,
        firstDiffAt: 0.12,
        score: 0.97,
        diffWindows: [
          { window: 12, atSeconds: 0.12 },
          { window: 45, atSeconds: 0.45 },
          { window: 78, atSeconds: 0.78 },
        ],
        truncated: false,
      } as unknown as T;
    }
    default:
      return Promise.resolve([] as unknown as T);
  }
}

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isMockIpc()) {
    return mockInvoke<T>(cmd, args);
  }
  return tauriInvoke<T>(cmd, args);
}
