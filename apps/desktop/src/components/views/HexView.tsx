import { useEffect, useRef, useState } from "react";
import { hexCompare, hexFind, hexGetPage } from "../../ipc/hex";
import { HexFindPanel, HexGotoPanel, HexToolbar } from "./hex/controls";
import { errString, isRecord } from "./types";
import type { ViewPaths } from "./types";

/**
 * Hex (binary) comparison view (Wave 2B).
 *
 * IPC: `hex_compare` (summary + session) then one bounded `hex_get_page`
 * window per page (`PAGE_LEN` bytes, 16 bytes per grid row). The server
 * clamps windows to 4096 bytes; pages here stay well under that.
 * Page 0 covers `[0, PAGE_LEN)`; Prev/Next step by whole pages.
 *
 * Wave-42 goto-offset (Beyond Compare style Ctrl+G): a decimal or
 * `0x`-hex input jumps the pager to the offset's page and selects the
 * byte, with honest inline notes for garbage or out-of-range input.
 * Pure client-side over the already-known `total`; no new IPC.
 *
 * Wave 19 find: a hex-string control (raw hex, spaces tolerated) searches
 * forward via `hex_find(session, needleHex, fromOffset)`. A fresh query
 * starts at the current page end; repeating Find next continues from the
 * active match end (same normalized query), and a miss wraps once from 0.
 * The pager jumps to the match page and the matched bytes highlight.
 * Bad input (empty / odd digits / non-hex) renders an inline note and
 * never touches IPC, so empty/invalid input cannot crash.
 *
 * Mock-IPC fallback (merge-depth pattern): `VITE_MOCK_IPC=1` serves no hex
 * fixtures (the mock adapter resolves `[]`), so when `hex_compare`
 * resolves a mock fixture instead of a real `HexSummary` record the view
 * pages + finds over the embedded byte-identical copies of
 * `tools/parity-harness/fixtures/h_left.bin` ("hello world") and
 * `h_right.bin` ("hallo world") purely locally -- no IPC, zero crashes.
 *
 * Wave 36 export selection (Beyond Compare style): Copy-hex + Copy-ASCII
 * buttons copy the selected byte (left preferred, right fallback) or --
 * with no selection -- the current page summary string. Results go through
 * `navigator.clipboard.writeText` with an inline receipt; when the
 * clipboard is unavailable (headless) the receipt carries the value inline
 * so it stays manually copyable. Ctrl/Cmd-C on a focused byte copies hex.
 *
 * Wave-101 hex parity (Beyond Compare bar): a per-side pane strip above
 * the grid (left/right file + byte count, from the compare summary), a
 * right-pane offset gutter doubling as the pane divider, per-char ASCII
 * `hx-diff` tint on both sides, and per-side counts + client-measured
 * load time in the bottom status. All legacy testids/logic untouched.
 *
 * HEX2 BC parity batch (owns hex view + hex controls + hex IPC only):
 * L2 `autocompare:view-filter` bus (all/diffs/same/context, hex-kind or
 * unkinded events only), diff-only prev/next navigation by scanning the
 * loaded page then bounded `hex_get_page` windows (existing IPC only,
 * no new command), bytes-per-line layout (8/16/32, default 16) and a
 * sync-scroll follow toggle — all toolbar-row chrome, no new stacked
 * rows. Every legacy `hex-*` / `hexview/*` testid stays byte-identical.
 */

const PAGE_LEN = 256;
/** Default grid width; HEX2 makes it a toolbar layout control (8/16/32). */
const DEFAULT_BYTES_PER_ROW = 16;

/** Byte-identical copy of `tools/parity-harness/fixtures/h_left.bin`. */
const EMBEDDED_LEFT: readonly number[] = [
  0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
];
/** Byte-identical copy of `tools/parity-harness/fixtures/h_right.bin`. */
const EMBEDDED_RIGHT: readonly number[] = [
  0x68, 0x61, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
];
/** Sentinel session id for the mock-only local fallback (never a real id). */
const LOCAL_SESSION = "local-mock-hex";

/** Client-side parse of the find box (mirrors `parse_needle_hex`). */
function parseFindQuery(raw: string): { readonly bytes: readonly number[] } | { readonly error: string } {
  const compact = raw.replace(/\s+/g, "");
  if (compact.length === 0) return { error: "Enter hex bytes to search (0-9, a-f)." };
  if (compact.length % 2 !== 0) return { error: "Invalid hex: even number of digits." };
  if (!/^[0-9a-fA-F]+$/.test(compact)) return { error: "Invalid hex: use 0-9 and a-f only." };
  const bytes: number[] = [];
  for (let i = 0; i < compact.length; i += 2) {
    bytes.push(parseInt(compact.slice(i, i + 2), 16));
  }
  return { bytes };
}

/** Local mirror of `hex_find`: first needle offset in `data` at/after `from`. */
function localFind(
  data: readonly number[],
  needle: readonly number[],
  from: number,
): number | null {
  if (needle.length === 0) return null;
  const start = Math.max(0, Math.floor(from));
  outer: for (let i = start; i + needle.length <= data.length; i += 1) {
    for (let j = 0; j < needle.length; j += 1) {
      if (data[i + j] !== needle[j]) continue outer;
    }
    return i;
  }
  return null;
}

type Status = "loading" | "ready" | "empty" | "error";

interface PageData {
  readonly left: readonly number[];
  readonly right: readonly number[];
}

function hex2(b: number): string {
  return b.toString(16).padStart(2, "0");
}

function ascii(b: number): string {
  return b >= 32 && b <= 126 ? String.fromCharCode(b) : ".";
}

/** Wave-101: file name tail for the per-side pane strip (BC file bars). */
function baseName(p: string): string {
  const s = p.replace(/\\/g, "/");
  const i = s.lastIndexOf("/");
  return i >= 0 ? s.slice(i + 1) : s;
}

export default function HexView({ leftPath, rightPath }: ViewPaths): React.JSX.Element {
  const [status, setStatus] = useState<Status>("loading");
  const [error, setError] = useState<string | null>(null);
  const [session, setSession] = useState<string | null>(null);
  const [total, setTotal] = useState(0);
  // Wave-101: per-side sizes for the pane strip + status (BC prints
  // "28 bytes / 30 bytes" per pane; `total` stays the paging max).
  const [leftSize, setLeftSize] = useState(0);
  const [rightSize, setRightSize] = useState(0);
  // Wave-101: honest client-measured compare latency for the status
  // bar (BC shows "Load time: 0.04 seconds"; no backend change).
  const [loadSecs, setLoadSecs] = useState<number | null>(null);
  const [equal, setEqual] = useState(false);
  const [pageIdx, setPageIdx] = useState(0);
  const [page, setPage] = useState<PageData | null>(null);
  const [pageLoading, setPageLoading] = useState(false);
  const [localMode, setLocalMode] = useState(false);
  const [query, setQuery] = useState("");

  /* Wave-62 Cmd+F / Ctrl+F focuses the hex find box (Beyond Compare
     style). Only the active view is mounted (ViewRouter), so no
     cross-view fight. preventDefault keeps the browser find closed.
     Wave-71: the find/goto panels collapse, so the shortcuts open their
     panel first (no-op when already open) then focus -- mirrors the
     merge wave-70 / table wave-68 pattern. Ctrl+G opens the goto box. */
  useEffect(() => {
    const focusBox = (id: string): void => {
      window.setTimeout(() => {
        const box = document.getElementById(id);
        if (box instanceof HTMLElement) box.focus();
      }, 0);
    };
    const onKey = (e: KeyboardEvent): void => {
      if ((e.metaKey || e.ctrlKey) && (e.key === "f" || e.key === "F")) {
        e.preventDefault();
        setFindOpen(true);
        focusBox("hex-find-box");
      } else if ((e.metaKey || e.ctrlKey) && (e.key === "g" || e.key === "G")) {
        e.preventDefault();
        setGotoOpen(true);
        focusBox("hex-goto-box");
        // Wave-142: global Esc collapses both panels (text/folder parity;
        // input-level Esc already closes the focused panel).
      } else if (e.key === "Escape") {
        setFindOpen(false);
        setGotoOpen(false);
      }
    };
    // Wave-127: full-session find (menu `session.find`) opens the panel
    // FIRST, then focuses — same open-then-focus as Cmd+F above. The
    // panel commits after `setFindOpen`, so focus retries bounded until
    // the box mounts (no focus-first query on an unmounted input).
    const onSessionFind = (): void => {
      setFindOpen(true);
      let attempts = 0;
      const tick = (): void => {
        attempts += 1;
        const box = document.getElementById("hex-find-box");
        if (box instanceof HTMLElement) {
          box.focus();
          return;
        }
        if (attempts < 20) window.setTimeout(tick, 50);
      };
      window.setTimeout(tick, 0);
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("autocompare:session-find", onSessionFind);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("autocompare:session-find", onSessionFind);
    };
  }, []);
  // Wave-60: differences-only display filter (HexView twin of the
  // text-59 toggle). HEX2 widens it to the L2 `autocompare:view-filter`
  // bus vocabulary (`all` / `diffs` / `same` / `context`, Beyond Compare
  // View menu parity): `diffs` is the legacy checkbox state (off by
  // default, reset on session switch, keyboard reachable), `same` shows
  // only matching rows, `context` keeps each differing row plus one
  // neighbor on either side. Purely presentational: hides rows with no
  // differing bytes, no IPC. The legacy checkbox maps diffs<->all.
  const [viewFilter, setViewFilter] = useState<"all" | "diffs" | "same" | "context">("all");
  const diffsOnly = viewFilter === "diffs";
  function setDiffsOnly(v: boolean): void {
    setViewFilter(v ? "diffs" : "all");
  }
  // HEX2: bytes-per-line layout (BC hex columns) + sync-scroll follow
  // (both panes track the selection together). Cleared on session
  // switch alongside the filter, toolbar-row controls only.
  const [bytesPerRow, setBytesPerRow] = useState(DEFAULT_BYTES_PER_ROW);
  const [syncScroll, setSyncScroll] = useState(true);
  // HEX2: diff-only navigation receipt (toolbar `hex-diff-status`) +
  // busy guard. No new stacked rows; the status line keeps its totals.
  const [diffNote, setDiffNote] = useState<string | null>(null);
  const [diffNavBusy, setDiffNavBusy] = useState(false);
  // Wave-71: collapsible in-flow find + goto panels (open by default so
  // the wave-19/42 find/goto boxes stay visible on load; toolbar toggle
  // + Ctrl+F/Ctrl+G/Esc collapse them). Mirrors merge wave-70.
  const [findOpen, setFindOpen] = useState(true);
  const [gotoOpen, setGotoOpen] = useState(true);
  // Wave-170: top-bar Next/Prev ride the `autocompare:view-command`
  // bus (Beyond Compare unified-toolbar parity). Hex owns the
  // follow-through: bus next/prev walk the hex diff cursor via the
  // same `stepDiff` path as the toolbar Prev/Next diff buttons, so no
  // App.tsx change is needed on this side. Applies only when the
  // event targets this kind (`viewKind === "hex"` or an unkinded
  // event); other kinds are ignored so text/merge nav never leaks
  // across views. Placed with the other bus effects (above the early
  // returns) so hook order stays stable; `stepDiffRef` always points
  // at the latest ready-render `stepDiff` (null until the first page
  // is ready, so an early bus event is an honest no-op, never a
  // crash). When no differences remain `stepDiff` parks honestly on
  // `hex-diff-status` ("no further/earlier differences").
  const stepDiffRef = useRef<((dir: 1 | -1) => Promise<void>) | null>(null);
  useEffect(() => {
    const onBusCmd = (e: Event): void => {
      const detail = (e as CustomEvent<{ cmd?: unknown; viewKind?: unknown }>)
        .detail;
      if (!detail) return;
      const kind = detail.viewKind;
      if (typeof kind === "string" && kind !== "" && kind !== "hex") return;
      const cmd = detail.cmd;
      if (cmd === "prev") void stepDiffRef.current?.(-1);
      else if (cmd === "next") void stepDiffRef.current?.(1);
    };
    window.addEventListener("autocompare:view-command", onBusCmd);
    return () => {
      window.removeEventListener("autocompare:view-command", onBusCmd);
    };
  }, []);
  /* HEX2: L2 `autocompare:view-filter` bus (Beyond Compare View menu
     parity: Show All / Differences / Same / Context, dispatched by the
     top Toolbar via `handleViewFilter`). Applies only when the event
     targets this kind (`viewKind === "hex"` or an unkinded event);
     other kinds are ignored so folder/text filters never leak across
     views. TableView twin; same contract. */
  useEffect(() => {
    const onViewFilter = (e: Event): void => {
      const detail = (e as CustomEvent<{ filter?: unknown; viewKind?: unknown }>).detail;
      const kind = detail?.viewKind;
      if (typeof kind === "string" && kind !== "" && kind !== "hex") return;
      const f = detail?.filter;
      if (f === "all" || f === "diffs" || f === "same" || f === "context") setViewFilter(f);
    };
    window.addEventListener("autocompare:view-filter", onViewFilter);
    return () => {
      window.removeEventListener("autocompare:view-filter", onViewFilter);
    };
  }, []);
  const [finding, setFinding] = useState(false);
  const [findNote, setFindNote] = useState<string | null>(null);
  const [gotoInput, setGotoInput] = useState("");
  const [gotoNote, setGotoNote] = useState<string | null>(null);
  const [match, setMatch] = useState<{
    readonly offset: number;
    readonly len: number;
    readonly query: string;
  } | null>(null);
  const [selectedOffset, setSelectedOffset] = useState<number | null>(null);
  const [copyNote, setCopyNote] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setStatus("loading");
    setError(null);
    setPage(null);
    setPageIdx(0);
    setLocalMode(false);
    setLeftSize(0);
    setRightSize(0);
    setLoadSecs(null);
    setQuery("");
    setFinding(false);
    setFindNote(null);
    setMatch(null);
    setSelectedOffset(null);
    setCopyNote(null);
    setDiffsOnly(false);
    setBytesPerRow(DEFAULT_BYTES_PER_ROW);
    setSyncScroll(true);
    setDiffNote(null);
    (async () => {
      try {
        const t0 = typeof performance !== "undefined" ? performance.now() : 0;
        const raw = (await hexCompare(leftPath, rightPath)) as unknown;
        const secs =
          typeof performance !== "undefined" ? (performance.now() - t0) / 1000 : 0;
        if (Array.isArray(raw)) {
          // Mock-IPC fallback: no hex fixtures under `VITE_MOCK_IPC=1`
          // (merge-depth pattern) -- serve the embedded h_* copies.
          if (cancelled) return;
          setLocalMode(true);
          setSession(LOCAL_SESSION);
          setTotal(Math.max(EMBEDDED_LEFT.length, EMBEDDED_RIGHT.length));
          setLeftSize(EMBEDDED_LEFT.length);
          setRightSize(EMBEDDED_RIGHT.length);
          setLoadSecs(secs);
          setEqual(false);
          setStatus("ready");
          return;
        }
        if (
          !isRecord(raw) ||
          typeof raw["sessionId"] !== "string" ||
          typeof raw["leftSize"] !== "number" ||
          typeof raw["rightSize"] !== "number"
        ) {
          if (!cancelled) setStatus("empty");
          return;
        }
        if (cancelled) return;
        setSession(raw["sessionId"] as string);
        setTotal(
          Math.max(raw["leftSize"] as number, raw["rightSize"] as number),
        );
        setLeftSize(raw["leftSize"] as number);
        setRightSize(raw["rightSize"] as number);
        setLoadSecs(secs);
        setEqual(raw["equal"] === true);
        setStatus("ready");
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
  }, [leftPath, rightPath]);

  useEffect(() => {
    if (session === null || status !== "ready") return;
    if (localMode) {
      const start = pageIdx * PAGE_LEN;
      setPage({
        left: EMBEDDED_LEFT.slice(start, start + PAGE_LEN),
        right: EMBEDDED_RIGHT.slice(start, start + PAGE_LEN),
      });
      setPageLoading(false);
      return;
    }
    let cancelled = false;
    setPageLoading(true);
    (async () => {
      try {
        const raw = (await hexGetPage(session, pageIdx * PAGE_LEN, PAGE_LEN)) as unknown;
        if (!cancelled) {
          if (!isRecord(raw) || !Array.isArray(raw["left"]) || !Array.isArray(raw["right"])) {
            setPage({ left: [], right: [] });
          } else {
            setPage({
              left: raw["left"] as readonly number[],
              right: raw["right"] as readonly number[],
            });
          }
        }
      } catch (e) {
        if (!cancelled) {
          setError(errString(e));
          setStatus("error");
        }
      } finally {
        if (!cancelled) setPageLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [session, pageIdx, status, localMode]);

  // Wave 23 selected-byte detail: keep the selection while its offset stays
  // on the visible page; clear it (without crashing) once paging or a
  // find-jump moves it off-screen.
  const baseForEffect = pageIdx * PAGE_LEN;
  useEffect(() => {
    if (selectedOffset === null) return;
    if (selectedOffset < baseForEffect || selectedOffset >= baseForEffect + PAGE_LEN) {
      setSelectedOffset(null);
    }
  }, [baseForEffect, selectedOffset]);
  // HEX2: sync-scroll follow (BC synced-panes parity). Both panes share
  // one grid, so "sync" means the landed/selected byte scrolls into view
  // on both sides together; off leaves the scroll position untouched.
  useEffect(() => {
    if (!syncScroll || selectedOffset === null) return;
    window.setTimeout(() => {
      const el = document.querySelector(`[data-testid="hexview/byte-${selectedOffset}"]`);
      if (el instanceof HTMLElement) el.scrollIntoView({ block: "nearest" });
    }, 0);
  }, [syncScroll, selectedOffset, pageIdx, page]);

  /** Parse a goto offset: `0x`-prefixed hex or plain decimal. */
  function parseGotoOffset(text: string): number | null {
    const t = text.trim().toLowerCase();
    if (t.startsWith("0x")) {
      if (!/^0x[0-9a-f]+$/.test(t)) return null;
      const n = parseInt(t.slice(2), 16);
      return Number.isSafeInteger(n) ? n : null;
    }
    if (!/^[0-9]+$/.test(t)) return null;
    const n = parseInt(t, 10);
    return Number.isSafeInteger(n) ? n : null;
  }

  /** Jump the pager to the offset's page and select the byte. */
  function gotoOffset(): void {
    if (total <= 0) {
      setGotoNote("no bytes to jump to");
      return;
    }
    const off = parseGotoOffset(gotoInput);
    if (off === null) {
      setGotoNote("enter a byte offset (decimal or 0x-hex)");
      return;
    }
    if (off < 0 || off >= total) {
      setGotoNote(`offset out of range (0-${total - 1})`);
      return;
    }
    setPageIdx(Math.floor(off / PAGE_LEN));
    setSelectedOffset(off);
    // Wave-60: the target page loads async, so only a same-page target
    // can be checked against the current rows; anything else stays on
    // the plain jump note and the filter keeps applying after paging.
    // HEX2: the check is filter-aware (`same`/`context` hide rows too).
    const samePage = Math.floor(off / PAGE_LEN) === pageIdx;
    const rowInPage = Math.floor((off - Math.floor(off / PAGE_LEN) * PAGE_LEN) / bytesPerRow);
    const hiddenTarget = samePage && page !== null && !rowShown(rowInPage);
    setGotoNote(
      hiddenTarget
        ? `offset ${off} sits in a hidden same row — toggle Differences only off to view`
        : `jumped to offset ${off} (0x${off.toString(16)})`,
    );
  }

  /** Forward find: fresh query starts at the current page end, a repeated
   * Find next with the same query continues from the active match end, and
   * a miss wraps once from offset 0. Jumps the pager to the match page. */
  async function findNext(): Promise<void> {
    if (finding || session === null) return;
    const parsed = parseFindQuery(query);
    if ("error" in parsed) {
      setFindNote(parsed.error);
      return;
    }
    const needle = parsed.bytes;
    const compact = query.replace(/\s+/g, "").toLowerCase();
    const continuing =
      match !== null && match.query === compact && match.len === needle.length;
    const from = continuing ? match.offset + match.len : (pageIdx + 1) * PAGE_LEN;
    setFinding(true);
    try {
      const searchAt = async (off: number): Promise<number | null> => {
        if (localMode) return localFind(EMBEDDED_LEFT, needle, off);
        const raw = (await hexFind(session, query, off)) as unknown;
        return typeof raw === "number" && Number.isInteger(raw) && raw >= 0 ? raw : null;
      };
      let found = await searchAt(from);
      let wrapped = false;
      if (found === null && from > 0) {
        found = await searchAt(0);
        wrapped = found !== null;
      }
      if (found === null) {
        setMatch(null);
        setFindNote("no match");
        return;
      }
      setMatch({ offset: found, len: needle.length, query: compact });
      setPageIdx(Math.floor(found / PAGE_LEN));
      setFindNote(wrapped ? `match at offset ${found} (wrapped)` : `match at offset ${found}`);
    } catch (e) {
      setFindNote(`find failed: ${errString(e)}`);
    } finally {
      setFinding(false);
    }
  }

  if (status === "loading") return <p>Loading hex comparison…</p>;
  if (status === "error") return <p role="alert">Hex compare failed: {error}</p>;
  if (status === "empty" || session === null) return <p>No bytes to display.</p>;

  const pageCount = Math.max(1, Math.ceil(total / PAGE_LEN));
  const base = pageIdx * PAGE_LEN;
  const rows: number[] = [];
  const rowCount = Math.ceil(PAGE_LEN / bytesPerRow);
  for (let r = 0; r < rowCount; r += 1) rows.push(r);
  /** Wave-60: a hex row survives the `diffs` filter when any byte differs. */
  function rowHasDiff(r: number): boolean {
    if (page === null) return false;
    for (let c = 0; c < bytesPerRow; c += 1) {
      const i = r * bytesPerRow + c;
      const lb = page.left[i];
      const rb = page.right[i];
      if (lb !== undefined && rb !== undefined && lb !== rb) return true;
    }
    return false;
  }
  /** HEX2: filter-aware row visibility for goto/diff-nav honest notes. */
  function rowShown(r: number): boolean {
    if (viewFilter === "all") return true;
    if (page === null) return true;
    if (viewFilter === "diffs") return rowHasDiff(r);
    if (viewFilter === "same") return !rowHasDiff(r);
    return rowHasDiff(r) || rowHasDiff(r - 1) || rowHasDiff(r + 1);
  }
  const visibleRows =
    viewFilter === "all" || page === null
      ? rows
      : viewFilter === "diffs"
        ? rows.filter(rowHasDiff)
        : viewFilter === "same"
          ? rows.filter((r) => !rowHasDiff(r))
          : rows.filter((r) => rowHasDiff(r) || rowHasDiff(r - 1) || rowHasDiff(r + 1));
  const hiddenSameCount = rows.length - visibleRows.length;
  /** HEX2: one bounded page window via the existing hex IPC (no new command). */
  async function fetchWindow(offset: number): Promise<{ readonly left: readonly number[]; readonly right: readonly number[] } | null> {
    if (localMode) {
      return {
        left: EMBEDDED_LEFT.slice(offset, offset + PAGE_LEN),
        right: EMBEDDED_RIGHT.slice(offset, offset + PAGE_LEN),
      };
    }
    if (session === null) return null;
    try {
      const raw = (await hexGetPage(session, offset, PAGE_LEN)) as unknown;
      if (!isRecord(raw) || !Array.isArray(raw["left"]) || !Array.isArray(raw["right"])) return null;
      return {
        left: raw["left"] as readonly number[],
        right: raw["right"] as readonly number[],
      };
    } catch {
      return null;
    }
  }
  /** HEX2: land the pager + selection on a differing offset, honestly. */
  function landOnDiff(off: number): void {
    const targetPage = Math.floor(off / PAGE_LEN);
    setPageIdx(targetPage);
    setSelectedOffset(off);
    const samePage = targetPage === pageIdx;
    const rowInPage = Math.floor((off - targetPage * PAGE_LEN) / bytesPerRow);
    const hidden = samePage && page !== null && !rowShown(rowInPage);
    setDiffNote(
      hidden
        ? `difference at offset ${off} sits in a hidden row — toggle the filter to All to view`
        : `difference at offset ${off} (0x${off.toString(16)})`,
    );
  }
  /** HEX2: diff-only prev/next navigation (BC next/previous difference).
   * Scans the loaded page first (zero IPC), then walks neighboring pages
   * through the existing `hex_get_page` window (bounded: 64 windows max,
   * so a huge identical tail resolves honestly instead of hanging). The
   * differing-byte rule mirrors `rowHasDiff` (both sides present and
   * unequal); the landed row stays subject to the active view filter. */
  async function stepDiff(dir: 1 | -1): Promise<void> {
    if (diffNavBusy || session === null || total <= 0) return;
    const origin = selectedOffset ?? (dir === 1 ? -1 : total);
    setDiffNavBusy(true);
    try {
      const isDiffAt = (
        left: readonly number[],
        right: readonly number[],
        rel: number,
      ): boolean => {
        const lb = left[rel];
        const rb = right[rel];
        return lb !== undefined && rb !== undefined && lb !== rb;
      };
      if (page !== null) {
        const rel = origin - base;
        if (dir === 1) {
          for (let i = Math.max(rel + 1, 0); i < PAGE_LEN; i += 1) {
            if (isDiffAt(page.left, page.right, i)) {
              landOnDiff(base + i);
              return;
            }
          }
        } else {
          for (let i = Math.min(rel - 1, PAGE_LEN - 1); i >= 0; i -= 1) {
            if (isDiffAt(page.left, page.right, i)) {
              landOnDiff(base + i);
              return;
            }
          }
        }
      }
      const originPage = Math.floor(Math.max(origin, 0) / PAGE_LEN);
      let visited = 0;
      for (let step = 1; step < pageCount && visited < 64; step += 1) {
        const p = dir === 1 ? originPage + step : originPage - step;
        if (p < 0 || p >= pageCount) break;
        const win = await fetchWindow(p * PAGE_LEN);
        visited += 1;
        if (win === null) continue;
        if (dir === 1) {
          for (let i = 0; i < PAGE_LEN; i += 1) {
            const off = p * PAGE_LEN + i;
            if (off >= total) break;
            if (isDiffAt(win.left, win.right, i)) {
              landOnDiff(off);
              return;
            }
          }
        } else {
          const top = Math.min(PAGE_LEN - 1, total - 1 - p * PAGE_LEN);
          for (let i = top; i >= 0; i -= 1) {
            if (isDiffAt(win.left, win.right, i)) {
              landOnDiff(p * PAGE_LEN + i);
              return;
            }
          }
        }
      }
      setDiffNote(dir === 1 ? "no further differences" : "no earlier differences");
    } finally {
      setDiffNavBusy(false);
    }
  }
  // Wave-170: refresh the bus-command ref (see the `stepDiffRef`
  // effect above the early returns). Assignment lives here — the
  // first ready render — so the bus always calls the latest closure.
  stepDiffRef.current = stepDiff;

  // Wave 23 footer detail: resolve the selected absolute offset to its
  // visible page-relative bytes (local embedded copies or the IPC page).
  const selVisible =
    selectedOffset !== null &&
    selectedOffset >= base &&
    selectedOffset < base + PAGE_LEN &&
    page !== null;
  const selRel = selectedOffset !== null ? selectedOffset - base : -1;
  const selLeft: number | undefined =
    selVisible && selectedOffset !== null
      ? localMode
        ? EMBEDDED_LEFT[selectedOffset]
        : (page?.left[selRel] as number | undefined)
      : undefined;
  const selRight: number | undefined =
    selVisible && selectedOffset !== null
      ? localMode
        ? EMBEDDED_RIGHT[selectedOffset]
        : (page?.right[selRel] as number | undefined)
      : undefined;
  const selBothMissing = selVisible && selLeft === undefined && selRight === undefined;
  const selSame =
    selVisible && !selBothMissing && selLeft !== undefined
      ? selRight !== undefined && selLeft === selRight
      : selVisible && selBothMissing;

  function moveSelection(dir: number): void {
    if (total <= 0) return;
    if (selectedOffset === null) {
      const initial = Math.min(base, Math.max(0, total - 1));
      setSelectedOffset(initial);
      const el = document.querySelector(`[data-testid="hexview/byte-${initial}"]`);
      if (el instanceof HTMLElement) el.focus();
      return;
    }
    const next = selectedOffset + dir;
    if (next < 0 || next >= total) return;
    setPageIdx(Math.floor(next / PAGE_LEN));
    setSelectedOffset(next);
    const el = document.querySelector(`[data-testid="hexview/byte-${next}"]`);
    if (el instanceof HTMLElement) el.focus();
  }

  // Wave 36 copy selection: resolve an absolute offset to its byte value
  // (left preferred, right fallback -- mirrors the footer readout).
  function byteAt(offset: number): number | undefined {
    if (localMode) return EMBEDDED_LEFT[offset] ?? EMBEDDED_RIGHT[offset];
    if (page === null) return undefined;
    const rel = offset - pageIdx * PAGE_LEN;
    return page.left[rel] ?? page.right[rel];
  }

  async function copyValue(value: string, kind: string): Promise<void> {
    try {
      const clip =
        typeof navigator !== "undefined" ? navigator.clipboard : undefined;
      if (clip && typeof clip.writeText === "function") {
        await clip.writeText(value);
        setCopyNote(`copied ${kind}: ${value}`);
      } else {
        setCopyNote(`clipboard unavailable — ${kind}: ${value}`);
      }
    } catch {
      setCopyNote(`clipboard blocked — ${kind}: ${value}`);
    }
  }

  /** Copy the selected byte as lowercase hex, else the page summary. */
  function copyHex(): void {
    if (selectedOffset !== null) {
      const b = byteAt(selectedOffset);
      if (b === undefined) {
        setCopyNote("nothing to copy at selection");
        return;
      }
      void copyValue(hex2(b), "hex");
      return;
    }
    void copyValue(pageSummary(), "summary");
  }

  /** Copy the selected byte as its printable ASCII, else the page summary. */
  function copyAscii(): void {
    if (selectedOffset !== null) {
      const b = byteAt(selectedOffset);
      if (b === undefined) {
        setCopyNote("nothing to copy at selection");
        return;
      }
      void copyValue(ascii(b), "ascii");
      return;
    }
    void copyValue(pageSummary(), "summary");
  }

  function pageSummary(): string {
    const pages = Math.max(1, Math.ceil(total / PAGE_LEN));
    return `${equal ? "files identical" : "files differ"} · ${total} bytes · page ${pageIdx + 1}/${pages}`;
  }

  return (
    <section
      aria-label="hex comparison"
      tabIndex={0}
      data-testid={`hexview/page-${pageIdx}`}
      onKeyDown={(event) => {
        const target = event.target as HTMLElement | null;
        if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA")) return;
        if ((event.ctrlKey || event.metaKey) && (event.key === "c" || event.key === "C")) {
          if (selectedOffset === null) return;
          event.preventDefault();
          copyHex();
          return;
        }
        if (event.key !== "ArrowLeft" && event.key !== "ArrowRight" && event.key !== "ArrowUp" && event.key !== "ArrowDown") return;
        event.preventDefault();
        const delta = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : event.key === "ArrowDown" ? 16 : -16;
        moveSelection(delta);
      }}
    >
      {/* Wave-71: chrome collapse (Beyond Compare style). One toolbar row
          (pager + differences-only + copy + find/goto toggles) plus
          collapsible in-flow find/goto panels (open by default, Ctrl+F /
          Ctrl+G / Esc). Counts live in the bottom status line. All legacy
          testids ride the same controls; page/find/goto/copy logic
          byte-identical. */}
      <HexToolbar
        pageLabel={`page ${pageIdx + 1}/${pageCount}`}
        onPrevPage={() => setPageIdx((p) => Math.max(0, p - 1))}
        onNextPage={() => setPageIdx((p) => Math.min(pageCount - 1, p + 1))}
        prevDisabled={pageIdx === 0}
        nextDisabled={pageIdx >= pageCount - 1}
        diffsOnly={diffsOnly}
        onToggleDiffsOnly={setDiffsOnly}
        onCopyHex={copyHex}
        onCopyAscii={copyAscii}
        findOpen={findOpen}
        onToggleFind={() => setFindOpen((v) => !v)}
        gotoOpen={gotoOpen}
        onToggleGoto={() => setGotoOpen((v) => !v)}
        onPrevDiff={() => void stepDiff(-1)}
        onNextDiff={() => void stepDiff(1)}
        diffNavBusy={diffNavBusy}
        diffStatusText={diffNote ?? "Prev/Next diff walks differing bytes."}
        bytesPerRow={bytesPerRow}
        onBytesPerRow={(v) => {
          if (v === 8 || v === 16 || v === 32) setBytesPerRow(v);
        }}
        syncScroll={syncScroll}
        onToggleSyncScroll={setSyncScroll}
      />
      {findOpen && (
        <HexFindPanel
          query={query}
          onQueryChange={(q) => {
            setQuery(q);
            setMatch(null);
          }}
          finding={finding}
          onFindNext={() => void findNext()}
          onClose={() => setFindOpen(false)}
          statusText={findNote ?? "Type hex bytes (spaces ok) — Enter jumps to the next match."}
        />
      )}
      {gotoOpen && (
        <HexGotoPanel
          value={gotoInput}
          onValueChange={(v) => {
            setGotoInput(v);
            setGotoNote(null);
          }}
          onGo={gotoOffset}
          onClose={() => setGotoOpen(false)}
          statusText={gotoNote ?? `Jump to a byte offset — 0 to ${Math.max(0, total - 1)}.`}
        />
      )}
      {/* Wave-101: per-side pane strip (Beyond Compare file bars).
          One compact in-flow row directly above the grid naming each
          side's file + byte count (BC prints per-pane "28 bytes" /
          "30 bytes" headers). Purely presentational: paths come from
          the existing view props, sizes from the compare summary. */}
      <div
        data-testid="hexview/panes"
        className="flex flex-wrap items-stretch border-b border-[var(--ac-border-subtle)] bg-[var(--ac-bg)]"
      >
        <div
          data-testid="hexview/left-meta"
          title={leftPath}
          className="min-w-0 flex-1 truncate px-[var(--ac-space-4)] py-[var(--ac-space-1)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]"
        >
          <span className="font-medium text-[var(--ac-fg)]">Left</span>
          {" · "}
          <span className="ac-mono">{baseName(leftPath) || leftPath}</span>
          {" · "}
          {leftSize} bytes
        </div>
        <div
          data-testid="hexview/right-meta"
          title={rightPath}
          className="min-w-0 flex-1 truncate border-l border-[var(--ac-border-subtle)] px-[var(--ac-space-4)] py-[var(--ac-space-1)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]"
        >
          <span className="font-medium text-[var(--ac-fg)]">Right</span>
          {" · "}
          <span className="ac-mono">{baseName(rightPath) || rightPath}</span>
          {" · "}
          {rightSize} bytes
        </div>
      </div>
      {total === 0 ? (
        <p className="ac-meta px-[var(--ac-space-4)] py-[var(--ac-space-2)] text-[var(--ac-fg-muted)]">Both files are empty.</p>
      ) : pageLoading || page === null ? (
        <p className="ac-meta px-[var(--ac-space-4)] py-[var(--ac-space-2)] text-[var(--ac-fg-muted)]">Loading page {pageIdx + 1}…</p>
      ) : (
        <div className="ac-mono text-[length:var(--ac-font-size-code)] font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] p-[var(--ac-space-3)]">
          {visibleRows.map((r) => {
            const cells: React.JSX.Element[] = [];
            const rcells: React.JSX.Element[] = [];
            // Wave-101: per-char ASCII paint so differing positions
            // carry `hx-diff` on BOTH sides (BC paints LEF/RIGHT red,
            // not just the hex bytes). Containers stay exactly two
            // `.hx-ascii` spans per row; textContent unchanged.
            const text: { readonly ch: string; readonly diff: boolean }[] = [];
            const textR: { readonly ch: string; readonly diff: boolean }[] = [];
            // Wave-66L: a row is "live" when either side holds a byte in
            // it; a missing byte in a live row paints the 66I orphan
            // hatch, while fully-empty rows keep the blank muted filler.
            let live = false;
            for (let c = 0; c < bytesPerRow; c += 1) {
              const i = r * bytesPerRow + c;
              if (page.left[i] !== undefined || page.right[i] !== undefined) {
                live = true;
                break;
              }
            }
            for (let c = 0; c < bytesPerRow; c += 1) {
              const i = r * bytesPerRow + c;
              const lb = page.left[i];
              const rb = page.right[i];
              const diff = lb !== undefined && rb !== undefined && lb !== rb;
              const g = base + i;
              const hit =
                match !== null && g >= match.offset && g < match.offset + match.len;
              const isSel = selectedOffset === g;
              const selCls = isSel
                ? " hx-selected ring-2 ring-[var(--ac-focus-ring)] rounded-xs"
                : "";
              if (lb === undefined) {
                cells.push(
                  live ? (
                    <span key={c} className="hx-byte hx-cell-empty px-[var(--ac-space-1)]">
                      {"  "}
                    </span>
                  ) : (
                    <span key={c} className="hx-byte px-[var(--ac-space-1)] text-[var(--ac-fg-muted)]">
                      {"  "}
                    </span>
                  ),
                );
                text.push({ ch: " ", diff: false });
              } else {
                const tone = diff
                  ? "hx-byte hx-diff px-[var(--ac-space-1)] cursor-pointer"
                  : "hx-byte px-[var(--ac-space-1)] cursor-pointer";
                const hl = hit
                  ? " hx-find-match bg-[var(--ac-selection-bg)] text-[var(--ac-selection-fg)] rounded-xs"
                  : "";
                cells.push(
                  <span
                    key={c}
                    role="button"
                    tabIndex={0}
                    data-testid={`hexview/byte-${g}`}
                    data-selected={isSel ? "true" : undefined}
                    aria-selected={isSel}
                    onClick={() => setSelectedOffset(g)}
                    onKeyDown={(event) => {
                      if ((event.ctrlKey || event.metaKey) && (event.key === "c" || event.key === "C")) {
                        event.preventDefault();
                        event.stopPropagation();
                        setSelectedOffset(g);
                        const b = byteAt(g);
                        if (b === undefined) {
                          setCopyNote("nothing to copy at selection");
                        } else {
                          void copyValue(hex2(b), "hex");
                        }
                        return;
                      }
                      if (event.key === "Enter" || event.key === " ") {
                        event.preventDefault();
                        setSelectedOffset(g);
                        return;
                      }
                      if (event.key === "ArrowLeft" || event.key === "ArrowRight" || event.key === "ArrowUp" || event.key === "ArrowDown") {
                        event.preventDefault();
                        const delta = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : event.key === "ArrowDown" ? 16 : -16;
                        moveSelection(delta);
                        return;
                      }
                    }}
                    className={`${tone}${hl}${selCls}`}
                  >
                    {hex2(lb)}
                  </span>,
                );
                text.push({ ch: ascii(lb), diff });
              }
              // Wave-66L right pane: same grid, same affordances, shared
              // selection offset; find-match stays left-only (exact-count
              // specs), diff + selected rings compose on both sides.
              if (rb === undefined) {
                rcells.push(
                  live ? (
                    <span key={c} className="hx-byte hx-cell-empty px-[var(--ac-space-1)]">
                      {"  "}
                    </span>
                  ) : (
                    <span key={c} className="hx-byte px-[var(--ac-space-1)] text-[var(--ac-fg-muted)]">
                      {"  "}
                    </span>
                  ),
                );
                textR.push({ ch: " ", diff: false });
              } else {
                const rtone = diff
                  ? "hx-byte hx-diff px-[var(--ac-space-1)] cursor-pointer"
                  : "hx-byte px-[var(--ac-space-1)] cursor-pointer";
                rcells.push(
                  <span
                    key={c}
                    role="button"
                    tabIndex={0}
                    data-testid={`hexview/byte-R-${g}`}
                    data-selected={isSel ? "true" : undefined}
                    aria-selected={isSel}
                    onClick={() => setSelectedOffset(g)}
                    onKeyDown={(event) => {
                      if ((event.ctrlKey || event.metaKey) && (event.key === "c" || event.key === "C")) {
                        event.preventDefault();
                        event.stopPropagation();
                        setSelectedOffset(g);
                        const b = byteAt(g);
                        if (b === undefined) {
                          setCopyNote("nothing to copy at selection");
                        } else {
                          void copyValue(hex2(b), "hex");
                        }
                        return;
                      }
                      if (event.key === "Enter" || event.key === " ") {
                        event.preventDefault();
                        setSelectedOffset(g);
                        return;
                      }
                      if (event.key === "ArrowLeft" || event.key === "ArrowRight" || event.key === "ArrowUp" || event.key === "ArrowDown") {
                        event.preventDefault();
                        const delta = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : event.key === "ArrowDown" ? 16 : -16;
                        moveSelection(delta);
                        return;
                      }
                    }}
                    className={`${rtone}${selCls}`}
                  >
                    {hex2(rb)}
                  </span>,
                );
                textR.push({ ch: ascii(rb), diff });
              }
            }
            // Wave-160: exact 22px density pin (table parity, the
            // `--ac-row-h` contract). `py-0` keeps the 12px*1.55 =
            // 18.6px line box inside the token `.hx-row` 22px
            // min-height; the old `py-0.5` measured ~22.6px and
            // drifted off the unified contract.
            return (
              <div key={r} className="hx-row flex items-center gap-[var(--ac-space-2)] whitespace-pre px-[var(--ac-space-2)] py-0 hover:bg-[var(--ac-row-hover-bg)] rounded-xs">
                <span className="hx-offset w-[var(--ac-gutter-w-hex)] shrink-0 tabular-nums text-[var(--ac-fg-muted)] select-none">
                  {(base + r * bytesPerRow).toString(16).padStart(8, "0")}
                </span>
                <span className="hx-hex">{cells}</span>
                <span className="hx-ascii text-[var(--ac-fg-muted)] border-l border-[var(--ac-border-subtle)] pl-[var(--ac-space-2)]">{text.map((c, i) => (
                  <span key={i} className={c.diff ? "hx-diff" : undefined}>{c.ch}</span>
                ))}</span>
                {/* Wave-101: right-pane offset gutter (BC prints offsets
                    in both panes) doubling as the pane divider. Same
                    value as the left gutter; aria-hidden (redundant). */}
                <span
                  aria-hidden="true"
                  className="hx-offset w-[var(--ac-gutter-w-hex)] shrink-0 tabular-nums text-[var(--ac-fg-muted)] select-none border-l border-[var(--ac-border-subtle)] pl-[var(--ac-space-2)]"
                >
                  {(base + r * bytesPerRow).toString(16).padStart(8, "0")}
                </span>
                <span className="hx-hex">{rcells}</span>
                <span className="hx-ascii text-[var(--ac-fg-muted)] border-l border-[var(--ac-border-subtle)] pl-[var(--ac-space-2)]">{textR.map((c, i) => (
                  <span key={i} className={c.diff ? "hx-diff" : undefined}>{c.ch}</span>
                ))}</span>
              </div>
            );
          })}
        </div>
      )}
      <div
        data-testid="hex-detail"
        aria-live="polite"
        className="flex flex-wrap items-center gap-[var(--ac-space-3)] border-t border-[var(--ac-border-subtle)] px-[var(--ac-space-4)] py-[var(--ac-space-2)] ac-mono text-[length:var(--ac-font-size-code)] font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg)]"
      >
        {selVisible && selectedOffset !== null ? (
          <>
            <span data-testid="hex-detail-offset">
              0x{selectedOffset.toString(16).padStart(8, "0")} ({selectedOffset})
            </span>
            <span data-testid="hex-detail-left">
              left {selLeft !== undefined ? `${hex2(selLeft)} '${ascii(selLeft)}'` : "--"}
            </span>
            <span data-testid="hex-detail-right">
              right {selRight !== undefined ? `${hex2(selRight)} '${ascii(selRight)}'` : "--"}
            </span>
            <span data-testid="hex-detail-flag">{selSame ? "same" : "diff"}</span>
          </>
        ) : (
          <span className="text-[var(--ac-fg-muted)]">Click a byte to inspect offset + both-side values.</span>
        )}
      </div>
      {viewFilter !== "all" ? (
        <p data-testid="hexview/diffsonly-note" role="note" className="px-[var(--ac-space-4)] pt-[var(--ac-space-1)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]">
          {viewFilter === "diffs"
            ? `Hiding ${hiddenSameCount} ${hiddenSameCount === 1 ? "row" : "rows"} without differences — toggle off to restore.`
            : viewFilter === "same"
              ? `Hiding ${hiddenSameCount} differing ${hiddenSameCount === 1 ? "row" : "rows"} — toggle off to restore.`
              : `Showing differences with one neighbor row — ${visibleRows.length} of ${rows.length} rows.`}
        </p>
      ) : null}
      <p
        data-testid="hex-copy-status"
        aria-live="polite"
        className="px-[var(--ac-space-4)] pt-[var(--ac-space-1)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]"
      >
        {copyNote ?? "Select a byte, then copy its hex or ASCII — Ctrl+C copies hex."}
      </p>
      {/* Wave-71 bottom status bar (Beyond Compare style). Honest scope:
          reuses the existing `total` + `equal` state only -- "same" when
          the backend summary says equal, else "different", plus the total
          byte count. NO backend change: no per-diff count exists in the
          hex summary, so none is claimed here. */}
      <div
        data-testid="hexview/status"
        role="status"
        className="mt-[var(--ac-space-2)] flex flex-wrap items-center gap-[var(--ac-space-2)] border-t border-[var(--ac-border-subtle)] bg-[var(--ac-bg)] px-[var(--ac-space-4)] py-[var(--ac-space-2)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]"
      >
        <span data-testid="hexview/status-diff">
          Binary differences | {equal ? "same" : "different"}
        </span>
        <span data-testid="hexview/status-bytes">{total} bytes · {equal ? "files identical" : "files differ"} · page {pageIdx + 1}/{pageCount}</span>
        {/* Wave-101: per-side counts + client-measured load time (BC
            per-pane "28 bytes / 30 bytes" + "Load time: 0.04 seconds").
            Legacy `status-diff` / `status-bytes` wording untouched. */}
        <span data-testid="hexview/status-sizes">left {leftSize} bytes · right {rightSize} bytes</span>
        <span data-testid="hexview/status-load">
          {loadSecs === null ? "Load time: —" : `Load time: ${loadSecs.toFixed(2)} seconds`}
        </span>
      </div>
    </section>
  );
}
