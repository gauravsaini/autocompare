import type { HexPage, HexSummary } from "./hex";
import type { FolderMergeApplyReport, TextMergeApplyReport, TextMergePlan, TreeMergePlan, TreeMergeStep } from "./merge";
import type { SessionDocument } from "./session";

/**
 * Fixture-mock IPC adapter for browser / Playwright / `VITE_MOCK_IPC=1` runs.
 *
 * Lazy-imported by `client.ts` only when running outside Tauri or with
 * `VITE_MOCK_IPC=1`, so production Tauri bundles keep this out of the main
 * entry chunk.
 */

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

// Wave-165: mock file-mask support (mock branch ONLY). Mirrors the engine
// grammar (`parse_file_masks` in `crates/compare/src/folder.rs`):
// semicolon list, `*`/`?` wildcards, `-` prefix = exclude, matched
// against the file name and the `/`-separated relPath. Directories always
// pass so masked trees stay walkable. Masks apply at compare time, before
// the status filter, exactly like the Rust handlers.
let mockFolderMasks: string | null = null;

function mockGlobToRegExp(glob: string): RegExp {
  let out = "^";
  for (const ch of glob) {
    if (ch === "*") out += ".*";
    else if (ch === "?") out += ".";
    else out += ch.replace(/[.*+?^${}()|[\]\\]/, "\\$&");
  }
  return new RegExp(`${out}$`);
}

interface MockMasks {
  readonly includes: readonly RegExp[];
  readonly excludes: readonly RegExp[];
}

function parseMockMasks(raw: unknown): MockMasks {
  const includes: RegExp[] = [];
  const excludes: RegExp[] = [];
  if (typeof raw !== "string") return { includes, excludes };
  for (const item of raw.split(";")) {
    const trimmed = item.trim();
    if (trimmed === "" || trimmed === "-") continue;
    if (trimmed.startsWith("-")) {
      const pattern = trimmed.slice(1).trim();
      if (pattern !== "") excludes.push(mockGlobToRegExp(pattern));
    } else {
      includes.push(mockGlobToRegExp(trimmed));
    }
  }
  return { includes, excludes };
}

function mockMasksAllow(parsed: MockMasks, relPath: string, isDir: boolean): boolean {
  if (isDir) return true;
  if (parsed.includes.length === 0 && parsed.excludes.length === 0) return true;
  const fileName = relPath.split("/").pop() ?? relPath;
  if (parsed.excludes.some((re) => re.test(fileName) || re.test(relPath))) return false;
  if (parsed.includes.length === 0) return true;
  return parsed.includes.some((re) => re.test(fileName) || re.test(relPath));
}
// Wave-66V mock edit overlay: `text_apply_edit` patches served rows so
// edit + re-compare wiring is provable without the Rust backend. Cleared
// on reload (module state, never localStorage).
const mockTextEdits = new Map<number, { left?: string | null; right?: string | null }>();
const mockSuppressedLines = new Set<number>();

// ---------------------------------------------------------------------------
// Wave-176: Real file diffing support for mock text_compare.
//
// When `text_compare` is given real readable paths, it reads their contents
// (via Playwright binding __MOCK_READ_FILE__, window.__MOCK_FILES__,
// localStorage file store, Node fs in node env, or fetch), splits lines
// per Rust engine semantics (strip \r, trailing \n does not create phantom
// empty line), runs difflib SequenceMatcher to produce identical row & hunk
// counts as python difflib ground truth, and serves those rows via
// `text_get_rows`, `text_find`, and `text_apply_edit`.
//
// When paths are fake or missing (or demo fixtures), it falls back 100% to
// `dumped.textRows` preserving existing fixture behavior for all existing tests.
// ---------------------------------------------------------------------------

interface MockFileReadResult {
  readonly content: string;
  readonly bytes?: number;
  readonly mtimeUnix?: number;
}

interface MockTextSession {
  readonly session: string;
  readonly rows: readonly DumpedTextRow[];
  readonly leftLines: number;
  readonly rightLines: number;
  readonly opCount: number;
  readonly leftPath: string;
  readonly rightPath: string;
  readonly leftMeta?: { readonly bytes?: number; readonly mtimeUnix?: number };
  readonly rightMeta?: { readonly bytes?: number; readonly mtimeUnix?: number };
}

const mockTextSessions = new Map<string, MockTextSession>();
let activeMockTextSessionId = "mock-text-1";

// Wave-176b: the fixture session mirrors the real backend contract —
// `text_get_rows` only serves sessions created by a prior `text_compare`.
// Unknown ids reject so `tryReuseSession` falls through to a real compare
// with the caller's (possibly new) paths instead of pinning stale fixture
// rows. The fixture entry is (re)registered on every fixture compare and
// lazily seeded for direct `mock-text-1` row reads (demo boot path).
function registerFixtureTextSession(
  rows: readonly DumpedTextRow[],
  leftPath: string,
  rightPath: string,
): MockTextSession {
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
  const sess: MockTextSession = {
    session: "mock-text-1",
    rows,
    leftLines,
    rightLines,
    opCount,
    leftPath,
    rightPath,
  };
  mockTextSessions.set("mock-text-1", sess);
  return sess;
}

function splitMockLines(text: string): string[] {
  if (text.length === 0) return [];
  const lines = text.split("\n").map((line) => (line.endsWith("\r") ? line.slice(0, -1) : line));
  if (text.endsWith("\n")) lines.pop();
  return lines;
}

function mockSequenceMatcher(
  a: readonly string[],
  b: readonly string[],
): Array<[string, number, number, number, number]> {
  const b2j = new Map<string, number[]>();
  for (let j = 0; j < b.length; j += 1) {
    const elt = b[j];
    let list = b2j.get(elt);
    if (list === undefined) {
      list = [];
      b2j.set(elt, list);
    }
    list.push(j);
  }

  function findLongestMatch(alo: number, ahi: number, blo: number, bhi: number): [number, number, number] {
    let besti = alo;
    let bestj = blo;
    let bestsize = 0;
    let j2len = new Map<number, number>();
    for (let i = alo; i < ahi; i += 1) {
      const newj2len = new Map<number, number>();
      const list = b2j.get(a[i]);
      if (list !== undefined) {
        for (const j of list) {
          if (j < blo) continue;
          if (j >= bhi) break;
          const k = (j2len.get(j - 1) ?? 0) + 1;
          newj2len.set(j, k);
          if (k > bestsize) {
            besti = i - k + 1;
            bestj = j - k + 1;
            bestsize = k;
          }
        }
      }
      j2len = newj2len;
    }
    while (besti > alo && bestj > blo && a[besti - 1] === b[bestj - 1]) {
      besti -= 1;
      bestj -= 1;
      bestsize += 1;
    }
    while (besti + bestsize < ahi && bestj + bestsize < bhi && a[besti + bestsize] === b[bestj + bestsize]) {
      bestsize += 1;
    }
    return [besti, bestj, bestsize];
  }

  const queue: Array<[number, number, number, number]> = [[0, a.length, 0, b.length]];
  const matchingBlocks: Array<[number, number, number]> = [];
  while (queue.length > 0) {
    const [alo, ahi, blo, bhi] = queue.pop()!;
    const [i, j, k] = findLongestMatch(alo, ahi, blo, bhi);
    if (k > 0) {
      matchingBlocks.push([i, j, k]);
      if (alo < i && blo < j) queue.push([alo, i, blo, j]);
      if (i + k < ahi && j + k < bhi) queue.push([i + k, ahi, j + k, bhi]);
    }
  }
  matchingBlocks.sort((x, y) => (x[0] !== y[0] ? x[0] - y[0] : x[1] - y[1]));

  let i1 = 0;
  let j1 = 0;
  let k1 = 0;
  const nonAdjacent: Array<[number, number, number]> = [];
  for (const [i2, j2, k2] of matchingBlocks) {
    if (i1 + k1 === i2 && j1 + k1 === j2) {
      k1 += k2;
    } else {
      if (k1 > 0) nonAdjacent.push([i1, j1, k1]);
      i1 = i2;
      j1 = j2;
      k1 = k2;
    }
  }
  if (k1 > 0) nonAdjacent.push([i1, j1, k1]);
  nonAdjacent.push([a.length, b.length, 0]);

  const opcodes: Array<[string, number, number, number, number]> = [];
  let i = 0;
  let j = 0;
  for (const [ai, bj, size] of nonAdjacent) {
    let tag = "";
    if (i < ai && j < bj) tag = "replace";
    else if (i < ai) tag = "delete";
    else if (j < bj) tag = "insert";
    if (tag) opcodes.push([tag, i, ai, j, bj]);
    i = ai + size;
    j = bj + size;
    if (size > 0) opcodes.push(["equal", ai, i, bj, j]);
  }
  return opcodes;
}

function computeMockInlineSpans(
  left: string,
  right: string,
): Array<{ aStart: number; aEnd: number; bStart: number; bEnd: number }> {
  const ops = mockSequenceMatcher(left.split(""), right.split(""));
  const spans: Array<{ aStart: number; aEnd: number; bStart: number; bEnd: number }> = [];
  for (const [tag, i1, i2, j1, j2] of ops) {
    if (tag !== "equal") {
      spans.push({ aStart: i1, aEnd: i2, bStart: j1, bEnd: j2 });
    }
  }
  return spans;
}

function buildMockDiffRows(
  leftLines: readonly string[],
  rightLines: readonly string[],
  opts?: {
    ignoreWhitespace?: boolean;
    ignoreCase?: boolean;
    eol?: string;
    alignmentMode?: string;
    neverAlignDifferences?: boolean;
  },
): { rows: DumpedTextRow[]; opCount: number } {
  const normalize = (line: string): string => {
    let s = line;
    if (opts?.eol) s = s.replace(/\r/g, "");
    if (opts?.ignoreWhitespace) s = s.trim().split(/\s+/).join(" ");
    if (opts?.ignoreCase) s = s.toLowerCase();
    return s;
  };
  const keysA = leftLines.map(normalize);
  const keysB = rightLines.map(normalize);

  if (opts?.alignmentMode === "unaligned") {
    const rows: DumpedTextRow[] = [];
    const maxLen = Math.max(leftLines.length, rightLines.length);
    for (let i = 0; i < maxLen; i += 1) {
      const l = i < leftLines.length ? leftLines[i] : null;
      const r = i < rightLines.length ? rightLines[i] : null;
      if (l !== null && r !== null) {
        const equal = keysA[i] === keysB[i];
        rows.push({
          index: rows.length,
          kind: equal ? "equal" : "changed",
          left: l,
          right: r,
          inline: equal ? [] : computeMockInlineSpans(l, r),
        });
      } else if (l !== null) {
        rows.push({ index: rows.length, kind: "leftOnly", left: l, right: null, inline: [] });
      } else if (r !== null) {
        rows.push({ index: rows.length, kind: "rightOnly", left: null, right: r, inline: [] });
      }
    }
    return { rows, opCount: rows.length };
  }

  const opcodes = mockSequenceMatcher(keysA, keysB);

  const rows: DumpedTextRow[] = [];
  for (const [tag, i1, i2, j1, j2] of opcodes) {
    const len1 = i2 - i1;
    const len2 = j2 - j1;
    if (tag === "equal") {
      for (let k = 0; k < len1; k += 1) {
        const l = leftLines[i1 + k];
        const r = rightLines[j1 + k];
        const kind = l === r ? "equal" : "changed";
        const inline = l === r ? [] : computeMockInlineSpans(l, r);
        rows.push({ index: rows.length, kind, left: l, right: r, inline });
      }
    } else if (tag === "delete") {
      for (let k = 0; k < len1; k += 1) {
        rows.push({ index: rows.length, kind: "leftOnly", left: leftLines[i1 + k], right: null, inline: [] });
      }
    } else if (tag === "insert") {
      for (let k = 0; k < len2; k += 1) {
        rows.push({ index: rows.length, kind: "rightOnly", left: null, right: rightLines[j1 + k], inline: [] });
      }
    } else if (tag === "replace") {
      if (opts?.neverAlignDifferences) {
        for (let k = 0; k < len1; k += 1) {
          rows.push({ index: rows.length, kind: "leftOnly", left: leftLines[i1 + k], right: null, inline: [] });
        }
        for (let k = 0; k < len2; k += 1) {
          rows.push({ index: rows.length, kind: "rightOnly", left: null, right: rightLines[j1 + k], inline: [] });
        }
      } else {
        const paired = Math.min(len1, len2);
        for (let k = 0; k < paired; k += 1) {
          const l = leftLines[i1 + k];
          const r = rightLines[j1 + k];
          rows.push({ index: rows.length, kind: "changed", left: l, right: r, inline: computeMockInlineSpans(l, r) });
        }
        for (let k = paired; k < len1; k += 1) {
          rows.push({ index: rows.length, kind: "leftOnly", left: leftLines[i1 + k], right: null, inline: [] });
        }
        for (let k = paired; k < len2; k += 1) {
          rows.push({ index: rows.length, kind: "rightOnly", left: null, right: rightLines[j1 + k], inline: [] });
        }
      }
    }
  }
  return { rows, opCount: opcodes.length };
}

async function readMockFile(path: string): Promise<MockFileReadResult | null> {
  if (!path || typeof path !== "string" || path.trim() === "") return null;
  // Always preserve fixture behavior for demo fixture candidate paths
  if (path.includes("fixtures/t_left.txt") || path.includes("fixtures/t_right.txt")) {
    return null;
  }

  // 1. Playwright exposed function or window mock hooks
  if (typeof window !== "undefined") {
    const w = window as unknown as Record<string, unknown>;
    const readerFn = w["__MOCK_READ_FILE__"] ?? w["__MOCK_FILE_READER__"];
    if (typeof readerFn === "function") {
      try {
        const res = await (readerFn as (p: string) => Promise<unknown> | unknown)(path);
        if (typeof res === "string") {
          return { content: res, bytes: res.length, mtimeUnix: Math.floor(Date.now() / 1000) };
        }
        if (res && typeof res === "object") {
          const r = res as Record<string, unknown>;
          if (typeof r["content"] === "string") {
            return {
              content: r["content"] as string,
              bytes: typeof r["bytes"] === "number" ? (r["bytes"] as number) : (r["content"] as string).length,
              mtimeUnix: typeof r["mtimeUnix"] === "number" ? (r["mtimeUnix"] as number) : Math.floor(Date.now() / 1000),
            };
          }
        }
      } catch {}
    }

    if (w["__MOCK_FILES__"] && typeof w["__MOCK_FILES__"] === "object") {
      const dict = w["__MOCK_FILES__"] as Record<string, unknown>;
      const val = dict[path];
      if (typeof val === "string") {
        return { content: val, bytes: val.length, mtimeUnix: Math.floor(Date.now() / 1000) };
      }
      if (val && typeof val === "object" && typeof (val as Record<string, unknown>)["content"] === "string") {
        const v = val as Record<string, unknown>;
        return {
          content: v["content"] as string,
          bytes: typeof v["bytes"] === "number" ? (v["bytes"] as number) : (v["content"] as string).length,
          mtimeUnix: typeof v["mtimeUnix"] === "number" ? (v["mtimeUnix"] as number) : Math.floor(Date.now() / 1000),
        };
      }
    }

    try {
      if (window.localStorage) {
        const raw = window.localStorage.getItem("autocompare:mockFileStore:v1");
        if (raw) {
          const dict = JSON.parse(raw) as Record<string, unknown>;
          const val = dict[path];
          if (typeof val === "string") {
            return { content: val, bytes: val.length, mtimeUnix: Math.floor(Date.now() / 1000) };
          }
        }
      }
    } catch {}
  }

  // 2. Node.js environment (e.g. testing in Node or test harness)
  try {
    if (typeof process !== "undefined" && process.versions?.node) {
      const fsSpec = ["node", "fs"].join(":");
      const fsMod = await import(/* @vite-ignore */ fsSpec);
      if (fsMod && fsMod.existsSync && fsMod.existsSync(path)) {
        const stat = fsMod.statSync(path);
        const content = fsMod.readFileSync(path, "utf8");
        return { content, bytes: stat.size, mtimeUnix: Math.floor(stat.mtimeMs / 1000) };
      }
    }
  } catch {}

  // NOTE (Wave-176b): no `fetch("/@fs/...")` fallback here. Under vite dev
  // the SPA fallback serves index.html (or another substitute document)
  // for missing files, so fake paths would read as identical content
  // instead of falling back to the dumped fixture. Unreadable => null.
  return null;
}

export function __mockTextEditResetForTests(): void {
  mockTextEdits.clear();
  mockSuppressedLines.clear();
  mockTextSessions.clear();
  activeMockTextSessionId = "mock-text-1";
}

export async function mockInvoke<T>(cmd: string, _args?: Record<string, unknown>): Promise<T> {
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
    case "report_export": {
      const outPath = strArg(args, ["out_path", "outPath"], "autocompare-report.html");
      return (outPath.startsWith("/") ? outPath : `/tmp/${outPath}`) as unknown as T;
    }
    case "text_compare": {
      const leftPath = strArg(args, ["left"], "");
      const rightPath = strArg(args, ["right"], "");
      const [leftRes, rightRes] = await Promise.all([
        readMockFile(leftPath),
        readMockFile(rightPath),
      ]);

      const rawOpts = (args["opts"] && typeof args["opts"] === "object" ? args["opts"] : undefined) as
        | Record<string, unknown>
        | undefined;

      const ws =
        args["ignore_whitespace"] === true ||
        args["ignoreWhitespace"] === true ||
        rawOpts?.["ignore_whitespace"] === true ||
        rawOpts?.["ignoreWhitespace"] === true;
      const cs =
        args["ignore_case"] === true ||
        args["ignoreCase"] === true ||
        rawOpts?.["ignore_case"] === true ||
        rawOpts?.["ignoreCase"] === true;
      const minor =
        args["minor_unimportant"] === true ||
        args["minorUnimportant"] === true ||
        rawOpts?.["minor_unimportant"] === true ||
        rawOpts?.["minorUnimportant"] === true;
      const enc =
        typeof args["encoding"] === "string" && args["encoding"] !== ""
          ? (args["encoding"] as string)
          : typeof rawOpts?.["encoding"] === "string" && rawOpts["encoding"] !== ""
            ? (rawOpts["encoding"] as string)
            : undefined;
      const eol =
        typeof args["eol"] === "string" && args["eol"] !== ""
          ? (args["eol"] as string)
          : typeof rawOpts?.["eol"] === "string" && rawOpts["eol"] !== ""
            ? (rawOpts["eol"] as string)
            : undefined;
      const suppRaw =
        args["suppressedLines"] ??
        args["suppressed_lines"] ??
        rawOpts?.["suppressedLines"] ??
        rawOpts?.["suppressed_lines"];
      const supp = Array.isArray(suppRaw) ? (suppRaw as number[]) : undefined;

      const hasIgnoreFlags =
        typeof args["ignore_whitespace"] === "boolean" ||
        typeof args["ignore_case"] === "boolean" ||
        typeof args["ignoreWhitespace"] === "boolean" ||
        typeof args["ignoreCase"] === "boolean" ||
        typeof args["minor_unimportant"] === "boolean" ||
        typeof args["minorUnimportant"] === "boolean" ||
        (typeof args["encoding"] === "string" && args["encoding"] !== "") ||
        (typeof args["eol"] === "string" && args["eol"] !== "") ||
        Array.isArray(args["suppressed_lines"]) ||
        Array.isArray(args["suppressedLines"]) ||
        (rawOpts !== undefined &&
          (ws || cs || minor || enc !== undefined || eol !== undefined || (supp !== undefined && supp.length > 0)));

      mockSuppressedLines.clear();
      if (supp !== undefined) {
        for (const idx of supp) mockSuppressedLines.add(idx);
      }

      let rows: readonly DumpedTextRow[];
      let leftLines = 0;
      let rightLines = 0;
      let opCount = 0;
      let sessionId = "mock-text-1";
      let leftMeta: { readonly bytes?: number; readonly mtimeUnix?: number } | undefined;
      let rightMeta: { readonly bytes?: number; readonly mtimeUnix?: number } | undefined;

      if (leftRes !== null && rightRes !== null) {
        // Real files provided & readable -> compute real diff rows
        const lLines = splitMockLines(leftRes.content);
        const rLines = splitMockLines(rightRes.content);
        leftLines = lLines.length;
        rightLines = rLines.length;
        leftMeta = { bytes: leftRes.bytes, mtimeUnix: leftRes.mtimeUnix };
        rightMeta = { bytes: rightRes.bytes, mtimeUnix: rightRes.mtimeUnix };

        const alignMode = strArg(args, ["alignmentMode", "alignment_mode"], "standard");
        const neverAlign =
          args["neverAlignDifferences"] === true || args["never_align_differences"] === true;
        const diffResult = buildMockDiffRows(lLines, rLines, {
          ignoreWhitespace: ws,
          ignoreCase: cs,
          eol,
          alignmentMode: alignMode,
          neverAlignDifferences: neverAlign,
        });
        rows = diffResult.rows;
        opCount = diffResult.opCount;
        sessionId = `mock-text-real-${Date.now()}`;
        mockTextSessions.set(sessionId, {
          session: sessionId,
          rows,
          leftLines,
          rightLines,
          opCount,
          leftPath,
          rightPath,
          leftMeta,
          rightMeta,
        });
        activeMockTextSessionId = sessionId;
      } else {
        // Fallback to fixture rows (registered, so later row reads for
        // this session resolve the same rows the summary reports).
        const fixture = registerFixtureTextSession(dumped.textRows, leftPath, rightPath);
        rows = fixture.rows;
        leftLines = fixture.leftLines;
        rightLines = fixture.rightLines;
        opCount = fixture.opCount;
        activeMockTextSessionId = "mock-text-1";
      }

      const base = {
        session: sessionId,
        leftPath,
        rightPath,
        leftLines,
        rightLines,
        rows: rows.length,
        opCount,
        exact: true,
        truncated: false,
      } as unknown as Record<string, unknown>;

      if (leftMeta !== undefined) base["leftMeta"] = leftMeta;
      if (rightMeta !== undefined) base["rightMeta"] = rightMeta;

      if (hasIgnoreFlags) {
        const applied: Record<string, unknown> = {
          ignoreWhitespace: ws,
          ignoreCase: cs,
          minorUnimportant: minor,
        };
        if (enc !== undefined) applied.encoding = enc;
        if (eol !== undefined) applied.eol = eol;
        if (supp !== undefined) applied.suppressedLines = supp;

        base["appliedOptions"] = applied;
        base["ignoreWhitespace"] = ws;
        base["ignoreCase"] = cs;
        base["minorUnimportant"] = minor;
        if (enc !== undefined) base["encoding"] = enc;
        if (eol !== undefined) base["eol"] = eol;
        if (supp !== undefined) base["suppressedLines"] = supp;
      }
      return base as unknown as T;
    }
    case "text_get_rows": {
      const sessId = strArg(args, ["session"], activeMockTextSessionId);
      const sess =
        mockTextSessions.get(sessId) ??
        (sessId === "mock-text-1"
          ? registerFixtureTextSession(dumped.textRows, "", "")
          : undefined);
      if (sess === undefined) {
        // Real-backend parity: unknown/stale ids reject so the view runs
        // a fresh `text_compare` instead of rendering stale rows.
        throw new Error(`unknown text session '${sessId}'`);
      }
      const sourceRows = sess.rows;
      const total = sourceRows.length;
      const start = Math.min(numArg(args, ["start"], 0), total);
      const limit = Math.min(numArg(args, ["limit"], 200), 500);
      const rows = sourceRows.slice(start, start + limit).map((row) => {
        const patch = mockTextEdits.get(row.index);
        const left = patch !== undefined && patch.left !== undefined ? patch.left : row.left;
        const right = patch !== undefined && patch.right !== undefined ? patch.right : row.right;
        // Mock-only kind recompute: a patched row whose sides are both
        // present and identical becomes equal (spans cleared); orphans
        // keep their kind (null edits are rejected below anyway).
        const kind =
          left !== null && right !== null && left === right
            ? "equal"
            : row.kind;
        const suppressed = mockSuppressedLines.has(row.index);
        return {
          ...row,
          left,
          right,
          kind,
          inline: left !== null && right !== null && left === right ? [] : (row.inline ?? []),
          suppressed,
        };
      });
      return { session: sessId, start, total, rows } as unknown as T;
    }
    case "text_find": {
      // Wave-127: full-fixture search mirroring backend `text_find`
      // (`find_in_rows` in `commands/text.rs`): case-sensitive
      // substring over ALL cached rows, first hit wins (`down` scans
      // top-first left-then-right, `up` scans bottom-first
      // right-then-left). Lets the TextView prove its beyond-loaded-
      // rows fallback under `VITE_MOCK_IPC=1`. Empty query resolves
      // null (the view never sends one; the real backend errors).
      const sessId = strArg(args, ["session"], activeMockTextSessionId);
      const sess = mockTextSessions.get(sessId);
      const sourceRows = sess !== undefined ? sess.rows : dumped.textRows;
      const query = strArg(args, ["query"], "");
      const direction = strArg(args, ["direction"], "down");
      if (query === "") return null as unknown as T;
      const ordered =
        direction === "up" ? [...sourceRows].reverse() : sourceRows;
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
      const sessId = strArg(args, ["session"], activeMockTextSessionId);
      const sess = mockTextSessions.get(sessId);
      const sourceRows = sess !== undefined ? sess.rows : dumped.textRows;
      const index = numArg(args, ["index"], -1);
      const side = strArg(args, ["side"], "");
      const text = strArg(args, ["text"], "");
      if (!Number.isInteger(index) || index < 0 || index >= sourceRows.length) {
        throw new Error(`row index ${index} out of range`);
      }
      if (side !== "left" && side !== "right") {
        throw new Error(`unknown side '${side}'`);
      }
      const row = sourceRows[index];
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
        session: sessId,
        index,
        side,
        kind,
        rows: sourceRows.length,
      } as unknown as T;
    }
    case "text_apply_hunk": {
      // Wave-178: mock parity with Rust `text_apply_hunk` (copy-persist
      // path only). Take-side wins with no fallback: a present take side
      // fills both cells and flips the row to equal (identity preserved);
      // an absent take side (`null`) drops the row (FIXTEXT GAP-03).
      // Unknown sessions reject so the view falls back to the per-row
      // `text_apply_edit` replay instead of rendering stale rows.
      const sessId = strArg(args, ["session"], activeMockTextSessionId);
      const start = numArg(args, ["start"], -1);
      const end = numArg(args, ["end"], -1);
      const dirStr = strArg(args, ["dir"], "");
      if (dirStr !== "left" && dirStr !== "right") {
        throw new Error(`unknown side '${dirStr}'`);
      }
      if (!Number.isInteger(start) || !Number.isInteger(end) || start < 0 || end < 0 || start > end) {
        throw new Error(`invalid hunk range: start ${start} > end ${end}`);
      }
      let sess = mockTextSessions.get(sessId);
      if (sess === undefined) {
        if (sessId === "mock-text-1") {
          sess = registerFixtureTextSession(dumped.textRows, "", "");
        } else {
          throw new Error(`unknown text session '${sessId}'`);
        }
      }
      const maxIdx = sess.rows.reduce((m, r) => (r.index > m ? r.index : m), 0);
      if (sess.rows.length === 0) {
        throw new Error("cannot apply hunk to empty session");
      }
      if (start > maxIdx) {
        throw new Error(`row index ${start} out of range`);
      }
      const clampedEnd = Math.min(end, maxIdx);
      const nextRows: DumpedTextRow[] = [];
      let applied = 0;
      let total = 0;
      for (const row of sess.rows) {
        if (row.index < start || row.index > clampedEnd) {
          nextRows.push(row);
          continue;
        }
        total += 1;
        const take = dirStr === "left" ? row.left : row.right;
        const other = dirStr === "left" ? row.right : row.left;
        if (take !== null && take !== undefined && other !== null && other !== undefined) {
          applied += 1;
        }
        if (take === null || take === undefined) continue;
        nextRows.push({ index: row.index, kind: "equal", left: take, right: take, inline: [] });
      }
      let leftLines = 0;
      let rightLines = 0;
      let opCount = 0;
      let prevKind = "";
      for (const row of nextRows) {
        if (row.left !== null) leftLines += 1;
        if (row.right !== null) rightLines += 1;
        if (row.kind !== prevKind) {
          opCount += 1;
          prevKind = row.kind;
        }
      }
      mockTextSessions.set(sessId, {
        ...sess,
        rows: nextRows,
        leftLines,
        rightLines,
        opCount,
      });
      return {
        session: sessId,
        start,
        end: start + total - 1,
        dir: dirStr,
        applied,
        total,
        rows: nextRows.length,
      } as unknown as T;
    }
    case "text_restore_rows": {
      const sessId = strArg(args, ["session"], activeMockTextSessionId);
      const rawRows = Array.isArray(args["rows"]) ? (args["rows"] as DumpedTextRow[]) : [];
      const prev = mockTextSessions.get(sessId);
      let leftLines = 0;
      let rightLines = 0;
      let opCount = 0;
      let prevKind = "";
      for (const r of rawRows) {
        if (r.left !== null) leftLines += 1;
        if (r.right !== null) rightLines += 1;
        if (r.kind !== prevKind) {
          opCount += 1;
          prevKind = r.kind;
        }
      }
      mockTextEdits.clear();
      mockTextSessions.set(sessId, {
        session: sessId,
        rows: rawRows,
        leftLines,
        rightLines,
        opCount,
        leftPath: prev?.leftPath ?? "",
        rightPath: prev?.rightPath ?? "",
        leftMeta: prev?.leftMeta,
        rightMeta: prev?.rightMeta,
      });
      return rawRows.length as unknown as T;
    }
    case "stat_paths": {
      const paths = Array.isArray(args["paths"]) ? (args["paths"] as string[]) : [];
      const results = await Promise.all(
        paths.map(async (p) => {
          const res = await readMockFile(p);
          if (!res) return null;
          return { bytes: res.bytes, mtimeUnix: res.mtimeUnix };
        }),
      );
      return results as unknown as T;
    }
    case "text_update_side": {
      const sessId = strArg(args, ["session"], activeMockTextSessionId);
      const side = strArg(args, ["side"], "left");
      const newLines = Array.isArray(args["lines"]) ? (args["lines"] as string[]) : [];
      const prev =
        mockTextSessions.get(sessId) ??
        registerFixtureTextSession(dumped.textRows, "", "");
      const prevLeft: string[] = [];
      const prevRight: string[] = [];
      for (const r of prev.rows) {
        const patch = mockTextEdits.get(r.index);
        const lv = patch?.left !== undefined ? patch.left : r.left;
        const rv = patch?.right !== undefined ? patch.right : r.right;
        if (lv !== null) prevLeft.push(lv);
        if (rv !== null) prevRight.push(rv);
      }
      const leftLines = side === "left" ? newLines : prevLeft;
      const rightLines = side === "right" ? newLines : prevRight;
      const diffResult = buildMockDiffRows(leftLines, rightLines);
      mockTextEdits.clear();
      mockTextSessions.set(sessId, {
        ...prev,
        rows: diffResult.rows,
        leftLines: leftLines.length,
        rightLines: rightLines.length,
        opCount: diffResult.opCount,
      });
      return diffResult.rows.length as unknown as T;
    }
    case "folder_compare": {
      const masks = typeof args["masks"] === "string" ? (args["masks"] as string) : null;
      mockFolderMasks = masks;
      const parsed = parseMockMasks(masks);
      const nodes = dumped.folderNodes.filter((node) =>
        mockMasksAllow(parsed, node.relPath, node.isDir),
      );
      return {
        session: "mock-folder-1",
        leftRoot: strArg(args, ["left"], ""),
        rightRoot: strArg(args, ["right"], ""),
        total: nodes.length,
        truncated: false,
      } as unknown as T;
    }
    case "folder_get_tree": {
      const filter = strArg(args, ["filter"], "all");
      const parsed = parseMockMasks(mockFolderMasks);
      const entries = dumped.folderNodes
        .filter((node) => mockMasksAllow(parsed, node.relPath, node.isDir))
        .filter((entry) => {
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
    case "folder_copy_entry": {
      return {
        session: strArg(args, ["session"], "mock-folder-1"),
        relPath: strArg(args, ["rel_path", "relPath"], ""),
        action: strArg(args, ["dir"], "leftToRight"),
        bytesAffected: 128,
      } as unknown as T;
    }
    case "folder_delete_entry": {
      return {
        session: strArg(args, ["session"], "mock-folder-1"),
        relPath: strArg(args, ["rel_path", "relPath"], ""),
        action: `delete:${strArg(args, ["side"], "left")}`,
        bytesAffected: 128,
      } as unknown as T;
    }
    case "table_apply_edit": {
      return {
        sessionId: strArg(args, ["session", "sessionId"], "mock-table-1"),
        side: strArg(args, ["side"], "left"),
        row: numArg(args, ["row"], 0),
        col: numArg(args, ["col"], 0),
        value: strArg(args, ["value"], ""),
      } as unknown as T;
    }
    case "table_save_side": {
      return {
        sessionId: strArg(args, ["session", "sessionId"], "mock-table-1"),
        side: strArg(args, ["side"], "left"),
        path: strArg(args, ["path"], ""),
        rowsWritten: dumped.tableCells.totalRowsL,
        bytesWritten: 256,
      } as unknown as T;
    }
    case "table_compare": {
      const a = (args && typeof args === "object" ? args : {}) as Record<string, unknown>;
      const cells = ("_mockCells" in a && a["_mockCells"])
        ? (a["_mockCells"] as DumpedTableCells)
        : dumped.tableCells;
      // Wave-69: total diff rows over the FULL fixture tables
      // (positional: zipped full-row mismatches + length delta),
      // mirroring the Rust `diff_rows` so the status bar total is
      // truthful under `VITE_MOCK_IPC=1`.
      // Wave-164 + Wave-168: honor the W163c ignore flags (`ignoreCase` /
      // `ignore_case`, `ignoreSpace` / `ignore_space`, either
      // spelling). Mirrors `norm_cell` in
      // `src-tauri/src/commands/table.rs`: whitespace runs fold when
      // `ignore_space`, case folds when `ignore_case`. On the bundled
      // fixture this is a faithful no-op (no case/space-only diffs),
      // but unknown flags must never break the mock path — the e2e
      // gate drives this branch with flags set.
      const ignoreCs = a["ignoreCase"] === true || a["ignore_case"] === true;
      const ignoreWs = a["ignoreSpace"] === true || a["ignore_space"] === true;
      const normCell = (cell: string): string => {
        let s = ignoreWs ? cell.replace(/\s+/g, " ").trim() : cell;
        if (ignoreCs) s = s.toLowerCase();
        return s;
      };
      const common = Math.min(cells.rowsL.length, cells.rowsR.length);
      let mismatches = 0;
      for (let i = 0; i < common; i += 1) {
        const rowA = cells.rowsL[i];
        const rowB = cells.rowsR[i];
        if (rowA.length !== rowB.length || rowA.some((v, c) => normCell(v) !== normCell(rowB[c]))) mismatches += 1;
      }
      const diffRows =
        mismatches + Math.abs(cells.rowsL.length - cells.rowsR.length);
      return {
        sessionId: "mock-table-1",
        leftRows: cells.totalRowsL,
        rightRows: cells.totalRowsR,
        leftCols: cells.leftCols,
        rightCols: cells.rightCols,
        equal: diffRows === 0 && cells.rowsL.length === cells.rowsR.length,
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
      const rawOpts = (args["opts"] && typeof args["opts"] === "object" ? args["opts"] : args) as
        | Record<string, unknown>
        | undefined;
      const direction = typeof rawOpts?.["direction"] === "string" ? rawOpts["direction"] : "LeftToRight";
      const isRightToLeft = direction === "RightToLeft";

      const steps = dumped.folderNodes
        .filter((node) => node.status !== "same")
        .map((node) => ({
          rel_path: node.relPath,
          is_dir: node.isDir,
          action: isRightToLeft
            ? (node.right === null ? "DeleteLeft" : "CopyToLeft")
            : (node.left === null ? "CopyToLeft" : "CopyToRight"),
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
      const tol = typeof args["tolerance"] === "number" ? args["tolerance"] : 0;
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
        tolerance: tol,
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
    case "hex_compare": {
      const res: HexSummary = {
        sessionId: "mock-hex-1",
        leftSize: 64,
        rightSize: 64,
        equal: false,
        firstDiff: 8,
      };
      return res as unknown as T;
    }
    case "hex_get_page": {
      const sessionId = strArg(args, ["session", "sessionId"], "mock-hex-1");
      const offset = numArg(args, ["offset"], 0);
      const reqLen = numArg(args, ["len"], 64);
      const len = Math.min(reqLen, 4096);
      const fullLeft = Array.from({ length: 64 }, (_, i) => i);
      const fullRight = Array.from({ length: 64 }, (_, i) => (i === 8 ? 0xff : i));
      const left = offset >= 64 ? [] : fullLeft.slice(offset, offset + len);
      const right = offset >= 64 ? [] : fullRight.slice(offset, offset + len);
      const res: HexPage = {
        sessionId,
        offset,
        len,
        left,
        right,
        total: 64,
        truncated: false,
      };
      return res as unknown as T;
    }
    case "hex_find": {
      const fromOffset = numArg(args, ["from_offset", "fromOffset"], 0);
      const needleHex = strArg(args, ["needle_hex", "needleHex", "needle"], "")
        .replace(/\s+/g, "")
        .toLowerCase();
      if (!needleHex) return null as unknown as T;
      const fullLeft = Array.from({ length: 64 }, (_, i) => i);
      const bytes: number[] = [];
      for (let i = 0; i + 1 < needleHex.length; i += 2) {
        const val = parseInt(needleHex.slice(i, i + 2), 16);
        if (Number.isNaN(val)) return null as unknown as T;
        bytes.push(val);
      }
      if (bytes.length === 0) return null as unknown as T;
      outer: for (let i = Math.max(0, fromOffset); i + bytes.length <= fullLeft.length; i += 1) {
        for (let j = 0; j < bytes.length; j += 1) {
          if (fullLeft[i + j] !== bytes[j]) continue outer;
        }
        const res: number | null = i;
        return res as unknown as T;
      }
      if (fromOffset <= 8 && (needleHex === "08" || needleHex === "ff" || bytes.includes(8) || bytes.includes(0xff))) {
        const res: number | null = 8;
        return res as unknown as T;
      }
      const res: number | null = null;
      return res as unknown as T;
    }
    case "hex_apply_edit": {
      const sessionId = strArg(args, ["session", "sessionId"], "mock-hex-1");
      const side = strArg(args, ["side"], "left");
      const offset = numArg(args, ["offset"], 0);
      const value = numArg(args, ["value"], 0) & 0xff;
      return {
        sessionId,
        side,
        offset,
        value,
        pendingEdits: 1,
      } as unknown as T;
    }
    case "hex_save_side": {
      const sessionId = strArg(args, ["session", "sessionId"], "mock-hex-1");
      const side = strArg(args, ["side"], "left");
      const path = strArg(args, ["path"], "");
      return {
        sessionId,
        side,
        path,
        bytesWritten: 64,
        editsApplied: 1,
      } as unknown as T;
    }
    case "merge_text_plan": {
      const res: TextMergePlan = {
        conflicts: [
          {
            id: 1,
            base: ["base line"],
            left: ["left line"],
            right: ["right line"],
          },
        ],
        conflictCount: 1,
        cleanBlockCount: 2,
        hasConflicts: true,
      };
      return res as unknown as T;
    }
    case "merge_text_apply": {
      const res: TextMergeApplyReport = {
        outPath: strArg(args, ["outPath", "out_path"], "/tmp/merged.txt"),
        writtenLines: 3,
        conflictCount: 1,
        resolvedCount: 1,
        unresolvedCount: 0,
        hasConflictMarkers: false,
      };
      return res as unknown as T;
    }
    case "merge_folder_plan": {
      const steps: TreeMergeStep[] = dumped.folderNodes
        .filter((node) => node.status !== "same")
        .map((node, idx) => ({
          rel_path: node.relPath,
          action: node.isDir
            ? "MakeDir"
            : node.left === null
              ? "TakeRight"
              : node.right === null
                ? "TakeLeft"
                : idx % 3 === 0
                  ? "Conflict"
                  : "MergedText",
          merged: null,
        }));
      const conflicts = steps.filter((s) => s.action === "Conflict").map((s) => s.rel_path);
      const res: TreeMergePlan = { steps, conflicts, skipped: [] };
      return res as unknown as T;
    }
    case "merge_folder_apply": {
      const outPath = strArg(args, ["out_path", "outPath"], "/tmp/merged-folder");
      const rawPlan = args["plan"] as TreeMergePlan | undefined;
      const steps = Array.isArray(rawPlan?.steps) ? rawPlan.steps : [];
      const conflicts = Array.isArray(rawPlan?.conflicts) ? rawPlan.conflicts : [];
      const dirsCreated = steps.filter((s) => s.action === "MakeDir").length;
      const written = steps.filter((s) => s.action !== "MakeDir" && s.action !== "Conflict").length;
      const res: FolderMergeApplyReport = {
        outPath,
        written,
        dirsCreated,
        conflictCount: conflicts.length,
        conflictPaths: conflicts,
        failed: [],
      };
      return res as unknown as T;
    }
    default:
      return Promise.resolve([] as unknown as T);
  }
}
