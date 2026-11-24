import { useEffect, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
  ArrowLeft,
  ArrowLeftRight,
  ArrowRight,
  Check,
  ChevronLeft,
  ChevronRight,
  Clock,
  File,
  Folder,
  FolderUp,
  X,
} from "lucide-react";
import { folderCompare, folderGetTree } from "../../ipc/folder";
import type { FolderEntryDto, FolderStatusDto } from "../../ipc/folder";
import { pickOpenDialog } from "../session/useSession";
import { arrayField, errString, isRecord } from "./types";
import type { ViewPaths } from "./types";
import { FolderNamePanel, FolderToolbar } from "./folder/controls";
import { Button, Input, ToolbarButton } from "../ui";

/**
 * Folder comparison tree view (Wave 2B).
 *
 * IPC: `folder_compare` (summary + session) then paged `folder_get_tree`
 * windows of `PAGE_SIZE` entries with the `"all"` filter. Directories can
 * be collapsed client-side (descendants hidden by path prefix); collapse
 * state is local only and never sent back to the backend.
 *
 * Wave 35 adds a Beyond Compare style "Changes only" toggle: one button
 * hides every `same` row, shows an honest changes count, and composes
 * with the existing all/diffs/same filter plus per-folder collapse.
 * Pure client-side over the already-loaded entries; no new IPC.
 */

const PAGE_SIZE = 50;
const VIRTUALIZE_AFTER = 500;
/** Wave-96: BC 17px folder density (observe-93 measured folder ~17 logical px). Exported so e2e pins the BC pitch. */
export const ROW_PX = 17;

type Status = "loading" | "ready" | "empty" | "error";

function StatusIcon({ status }: { status: FolderStatusDto }): React.JSX.Element {
  switch (status) {
    case "same":
      return <Check size={14} aria-label="same" />;
    case "different":
      return <ArrowLeftRight size={14} aria-label="different" />;
    case "leftOnly":
      // Wave-66K: BC orphan idiom (66I chevron). The present side carries
      // the directional cue: content lives on the left, so a rightward
      // 12px golden-amber chevron points outward toward the missing side.
      return <ChevronRight size={12} aria-label="left only" className="text-[var(--ac-tab-dirty)]" />;
    case "rightOnly":
      // Wave-66K: mirrored — content lives on the right, leftward chevron.
      return <ChevronLeft size={12} aria-label="right only" className="text-[var(--ac-tab-dirty)]" />;
    case "newerLeft":
    case "newerRight":
      return <Clock size={14} aria-label={status} />;
  }
}

async function fetchWindow(
  session: string,
  start: number,
  limit: number = PAGE_SIZE,
): Promise<readonly FolderEntryDto[]> {
  const raw = (await folderGetTree(session, start, limit, "all")) as unknown;
  if (!isRecord(raw)) return [];
  return arrayField<FolderEntryDto>(raw, "entries");
}

function reRootEntries(
  rawEntries: readonly FolderEntryDto[],
  subDir: string,
): readonly FolderEntryDto[] {
  if (!subDir) return rawEntries;
  const prefix = `${subDir}/`;
  const matching = rawEntries.filter((e) => e.relPath.startsWith(prefix));
  if (matching.length === 0) return rawEntries;
  return matching.map((e) => ({
    ...e,
    relPath: e.relPath.slice(prefix.length),
  }));
}

interface FolderNavState {
  readonly leftBase: string;
  readonly rightBase: string;
  readonly baseSubDir: string;
  readonly subDir: string;
}

export default function FolderView({
  leftPath,
  rightPath,
  onOpenPaths,
}: ViewPaths & {
  /**
   * Wave-46 drilldown (optional): file-row double-click opens the pair
   * as a text session. Wired by `App` through `ViewRouter`; absent when
   * the view is mounted directly.
   */
  readonly onOpenPaths?: (kind: "text", left: string, right: string) => void;
}): React.JSX.Element {
  const [baseLeft, setBaseLeft] = useState<string>(leftPath);
  const [baseRight, setBaseRight] = useState<string>(rightPath);
  const [baseSubDir, setBaseSubDir] = useState<string>("");
  const [subDir, setSubDir] = useState<string>("");

  const [history, setHistory] = useState<readonly FolderNavState[]>([
    { leftBase: leftPath, rightBase: rightPath, baseSubDir: "", subDir: "" },
  ]);
  const [historyIndex, setHistoryIndex] = useState<number>(0);

  const prevLeftRef = useRef(leftPath);
  const prevRightRef = useRef(rightPath);

  if (prevLeftRef.current !== leftPath || prevRightRef.current !== rightPath) {
    prevLeftRef.current = leftPath;
    prevRightRef.current = rightPath;
    setBaseLeft(leftPath);
    setBaseRight(rightPath);
    setBaseSubDir("");
    setSubDir("");
    setHistory([{ leftBase: leftPath, rightBase: rightPath, baseSubDir: "", subDir: "" }]);
    setHistoryIndex(0);
  }

  const join = (base: string, rel: string): string =>
    rel ? `${base.replace(/\/+$/, "")}/${rel}` : base;

  const fullSubDir = baseSubDir ? (subDir ? `${baseSubDir}/${subDir}` : baseSubDir) : subDir;
  const currentLeft = join(baseLeft, subDir);
  const currentRight = join(baseRight, subDir);
  const isBasePinned = baseSubDir !== "" || baseLeft !== leftPath || baseRight !== rightPath;
  const isAtRoot = subDir === "" && !isBasePinned;
  const canGoUpBoth = !isAtRoot;
  const canGoBack = historyIndex > 0;
  const canGoForward = historyIndex < history.length - 1;

  function navigateTo(
    nextLeft: string,
    nextRight: string,
    nextBaseSubDir: string,
    nextSubDir: string,
  ): void {
    const nextState: FolderNavState = {
      leftBase: nextLeft,
      rightBase: nextRight,
      baseSubDir: nextBaseSubDir,
      subDir: nextSubDir,
    };
    const current = history[historyIndex];
    if (
      !current ||
      current.leftBase !== nextLeft ||
      current.rightBase !== nextRight ||
      current.baseSubDir !== nextBaseSubDir ||
      current.subDir !== nextSubDir
    ) {
      const nextHistory = [...history.slice(0, historyIndex + 1), nextState];
      setHistory(nextHistory);
      setHistoryIndex(nextHistory.length - 1);
    }
    setBaseLeft(nextLeft);
    setBaseRight(nextRight);
    setBaseSubDir(nextBaseSubDir);
    setSubDir(nextSubDir);
    setOpenNote(null);
  }

  function goBack(): void {
    if (historyIndex <= 0) return;
    const prevIdx = historyIndex - 1;
    const target = history[prevIdx];
    if (!target) return;
    setHistoryIndex(prevIdx);
    setBaseLeft(target.leftBase);
    setBaseRight(target.rightBase);
    setBaseSubDir(target.baseSubDir);
    setSubDir(target.subDir);
    setOpenNote(null);
  }

  function goForward(): void {
    if (historyIndex >= history.length - 1) return;
    const nextIdx = historyIndex + 1;
    const target = history[nextIdx];
    if (!target) return;
    setHistoryIndex(nextIdx);
    setBaseLeft(target.leftBase);
    setBaseRight(target.rightBase);
    setBaseSubDir(target.baseSubDir);
    setSubDir(target.subDir);
    setOpenNote(null);
  }

  function parentDir(p: string): string {
    const clean = p.replace(/\/+$/, "");
    const idx = clean.lastIndexOf("/");
    if (idx <= 0) return "/";
    return clean.slice(0, idx);
  }

  function goUpBoth(): void {
    if (subDir !== "") {
      const parts = subDir.split("/").filter(Boolean);
      parts.pop();
      navigateTo(baseLeft, baseRight, baseSubDir, parts.join("/"));
    } else if (baseSubDir !== "") {
      const parts = baseSubDir.split("/").filter(Boolean);
      parts.pop();
      const nextBaseSubDir = parts.join("/");
      const nextLeft = nextBaseSubDir ? join(leftPath, nextBaseSubDir) : leftPath;
      const nextRight = nextBaseSubDir ? join(rightPath, nextBaseSubDir) : rightPath;
      navigateTo(nextLeft, nextRight, nextBaseSubDir, "");
    }
  }

  function goUpLeft(): void {
    if (subDir !== "") {
      const nextLeft = parentDir(currentLeft);
      navigateTo(nextLeft, baseRight, baseSubDir, subDir);
    } else {
      const nextLeft = parentDir(baseLeft);
      navigateTo(nextLeft, baseRight, baseSubDir, "");
    }
  }

  function goUpRight(): void {
    if (subDir !== "") {
      const nextRight = parentDir(currentRight);
      navigateTo(baseLeft, nextRight, baseSubDir, subDir);
    } else {
      const nextRight = parentDir(baseRight);
      navigateTo(baseLeft, nextRight, baseSubDir, "");
    }
  }

  function setAsBase(): void {
    if (subDir === "") {
      setOpenNote("already at base folder — select a subdirectory first");
      return;
    }
    const nextLeft = currentLeft;
    const nextRight = currentRight;
    const nextBaseSubDir = fullSubDir;
    navigateTo(nextLeft, nextRight, nextBaseSubDir, "");
  }

  // Wave-L3: Browse L/R folder picker (BC Browse parity). Native
  // directory dialog via the existing session dialog helper
  // (`pickOpenDialog`, `@tauri-apps/plugin-dialog` with the
  // `__MOCK_DIALOG_OPEN__` stub hook); a picked folder re-roots that
  // side (subDir/baseSubDir clear so the new root compares clean) and
  // refetches through the existing effect. Cancelled/unavailable
  // dialogs render an honest inline note, never a throw. In mock /
  // plain-browser runs there is no native dialog, so both buttons park
  // disabled with the contract reason (a stubbed
  // `__MOCK_DIALOG_OPEN__` re-enables them for probes).
  const canBrowse =
    typeof window !== "undefined" &&
    (typeof (window as unknown as Record<string, unknown>).__MOCK_DIALOG_OPEN__ === "function" ||
      (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ !== undefined);
  const browseReason = canBrowse ? null : "folder picker needs the Tauri shell (mock has no native dialog)";

  async function browseSide(side: "left" | "right"): Promise<void> {
    setOpenNote(null);
    const picked = await pickOpenDialog({
      directory: true,
      multiple: false,
      defaultPath: side === "left" ? baseLeft : baseRight,
    });
    const dir = Array.isArray(picked) ? picked[0] : picked;
    if (typeof dir !== "string" || dir === "") {
      setOpenNote(
        `folder picker ${side === "left" ? "left" : "right"} unavailable here — no folder was chosen`,
      );
      return;
    }
    if (side === "left") navigateTo(dir, baseRight, "", "");
    else navigateTo(baseLeft, dir, "", "");
  }

  const navRef = useRef({
    canGoBack,
    canGoForward,
    canGoUpBoth,
    goBack,
    goForward,
    goUpBoth,
  });
  navRef.current = {
    canGoBack,
    canGoForward,
    canGoUpBoth,
    goBack,
    goForward,
    goUpBoth,
  };

  const [status, setStatus] = useState<Status>("loading");
  const [error, setError] = useState<string | null>(null);
  const [session, setSession] = useState<string | null>(null);
  const [entries, setEntries] = useState<readonly FolderEntryDto[]>([]);
  const [total, setTotal] = useState(0);
  const [collapsed, setCollapsed] = useState<readonly string[]>([]);
  // Wave-L3: L2 `autocompare:view-filter` bus vocabulary
  // (all/diffs/same/context, BC View menu parity, TableView twin).
  // `context` keeps each differing row plus one neighbor on either side
  // (tree order); the toolbar SegmentedControl keeps its four legacy
  // options (all/diffs/same/changes-only) with none pressed under
  // context. Pure client-side over the loaded entries, no new IPC.
  const [rowFilter, setRowFilter] = useState<"all" | "diffs" | "same" | "context">("all");
  // Wave-35 changes-only (Beyond Compare style show-differences): one
  // toggle hiding every `same` row. Composes with rowFilter + collapse.
  const [changesOnly, setChangesOnly] = useState(false);
  // Wave-63: name filter (Beyond Compare style quick-find over the tree).
  // Substring match on the already-loaded relPath, case-insensitive.
  // Composes with rowFilter + changesOnly + collapse. Purely
  // presentational, no new IPC; reset on session switch.
  const [nameQuery, setNameQuery] = useState("");
  const [nameOpen, setNameOpen] = useState(false);
  // Wave-98: goto entry (Beyond Compare style Ctrl+G, wave-89 text
  // pattern ported to folder). Decimal 1-based position in the
  // currently visible list; jumps focus (+selects file rows) with no
  // new IPC. Closed by default (no legacy goto specs; BC shows no
  // open panels); Ctrl+G opens + focuses, Esc closes globally as
  // well as from the input. Reset on session switch like the rest.
  const [gotoInput, setGotoInput] = useState("");
  const [gotoNote, setGotoNote] = useState<string | null>(null);
  const [gotoOpen, setGotoOpen] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  // Wave-28 row actions (Beyond Compare style): file rows are selectable
  // with an inline detail affordance rendered from the already-loaded
  // entry (no new IPC), plus a copy-path button with an inline receipt.
  // Folder rows stay collapse-only. All clipboard failures render inline
  // and never throw.
  // Wave-41 keyboard navigation (Beyond Compare style): a focused file
  // row answers ArrowUp/ArrowDown (previous/next file, clamped at the
  // ends) and Home/End (first/last file). Moving selects the target
  // file (detail follows) and moves DOM focus to its row. Directory
  // rows are skipped for selection; filtering/collapse compose because
  // navigation walks the same `visible` order the list renders.
  // Pure client-side, no new IPC.
  const [selected, setSelected] = useState<string | null>(null);
  const [copied, setCopied] = useState<string | null>(null);
  const [copyNote, setCopyNote] = useState<string | null>(null);
  // Wave-46 drilldown receipt: one-sided rows cannot open (honest note,
  // selection untouched, never a throw).
  const [openNote, setOpenNote] = useState<string | null>(null);
  const parentRef = useRef<HTMLDivElement | null>(null);

  /* Wave-62 pattern in folder: Cmd+F / Ctrl+F focuses the name filter.
     Wave-67B: opens the collapsible name panel first (no-op when
     already open), then focuses — mirrors the text find overlay.
     Wave-98 (wave-89 port): global Esc collapses any open chrome
     popover (name / goto) without clearing the query (mirrors text —
     the input-level Esc keeps the legacy clear+close); Ctrl+G /
     Cmd+G opens the goto popover + focuses it. */
  useEffect(() => {
    // Wave-127: full-session find (menu `session.find`) opens the name
    // panel FIRST, then focuses — same open-then-focus as Cmd+F below.
    // The panel commits after `setNameOpen`, so focus retries bounded
    // until the box mounts (no focus-first query on unmounted input).
    const openNameFilter = (): void => {
      setNameOpen(true);
      let attempts = 0;
      const tick = (): void => {
        attempts += 1;
        const box = document.getElementById("folder-name-box");
        if (box instanceof HTMLElement) {
          box.focus();
          return;
        }
        if (attempts < 20) window.setTimeout(tick, 50);
      };
      window.setTimeout(tick, 0);
    };
    const onSessionFind = (): void => {
      openNameFilter();
    };
    const onKey = (e: KeyboardEvent): void => {
      if ((e.metaKey || e.ctrlKey) && (e.key === "f" || e.key === "F")) {
        e.preventDefault();
        openNameFilter();
      } else if ((e.metaKey || e.ctrlKey) && (e.key === "g" || e.key === "G")) {
        e.preventDefault();
        setGotoOpen(true);
        window.setTimeout(() => {
          const box = document.getElementById("folder-goto-box");
          if (box instanceof HTMLElement) box.focus();
        }, 0);
      } else if (e.altKey && e.key === "ArrowLeft") {
        if (navRef.current.canGoBack) {
          e.preventDefault();
          navRef.current.goBack();
        }
      } else if (e.altKey && e.key === "ArrowRight") {
        if (navRef.current.canGoForward) {
          e.preventDefault();
          navRef.current.goForward();
        }
      } else if (e.altKey && e.key === "ArrowUp") {
        if (navRef.current.canGoUpBoth) {
          e.preventDefault();
          navRef.current.goUpBoth();
        }
      } else if (e.key === "Escape") {
        setNameOpen(false);
        setGotoOpen(false);
      }
    };
    // Wave-127: full-session find (menu `session.find`) rides the
    // `openNameFilter` above — open FIRST, then focus.
    window.addEventListener("keydown", onKey);
    window.addEventListener("autocompare:session-find", onSessionFind);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("autocompare:session-find", onSessionFind);
    };
  }, []);

  /* Wave-L3: L2 `autocompare:view-filter` bus (BC View menu parity:
     Show All / Differences / Same / Context, dispatched by the top
     Toolbar via `handleViewFilter`). Applies only when the event
     targets this kind (`viewKind === "folder"` or an unkinded event);
     other kinds are ignored so text/hex filters never leak across
     views. A bus selection defines the view, so it clears
     `changesOnly` (the toggle follows, staying in sync). TableView
     twin; same contract. */
  useEffect(() => {
    const onViewFilter = (e: Event): void => {
      const detail = (e as CustomEvent<{ filter?: unknown; viewKind?: unknown }>).detail;
      const kind = detail?.viewKind;
      if (typeof kind === "string" && kind !== "" && kind !== "folder") return;
      const f = detail?.filter;
      if (f === "all" || f === "diffs" || f === "same" || f === "context") {
        setRowFilter(f);
        setChangesOnly(false);
      }
    };
    window.addEventListener("autocompare:view-filter", onViewFilter);
    return () => {
      window.removeEventListener("autocompare:view-filter", onViewFilter);
    };
  }, []);

  const nameNeedle = nameQuery.trim().toLowerCase();
  const filtered = entries.filter((e, idx, arr) => {
    if (rowFilter === "same" && e.status !== "same") return false;
    if (rowFilter === "diffs" && e.status === "same") return false;
    // Wave-L3 context (TableView twin): differing rows plus one
    // same-status neighbor on either side; the rest hide.
    if (rowFilter === "context" && e.status === "same") {
      const prev = arr[idx - 1];
      const next = arr[idx + 1];
      const nearDiff =
        (prev !== undefined && prev.status !== "same") ||
        (next !== undefined && next.status !== "same");
      if (!nearDiff) return false;
    }
    if (changesOnly && e.status === "same") return false;
    if (nameNeedle !== "" && !e.relPath.toLowerCase().includes(nameNeedle)) return false;
    return true;
  });
  const visible = filtered.filter(
    (e) => !collapsed.some((d) => e.relPath !== d && e.relPath.startsWith(`${d}/`)),
  );
  const sameCount = entries.filter((e) => e.status === "same").length;
  const diffsCount = entries.length - sameCount;

  const virtualizer = useVirtualizer({
    count: visible.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ROW_PX,
    overscan: 10,
  });

  useEffect(() => {
    let cancelled = false;
    // Wave-130: no paths = pick-two-paths empty (App.hasPaths gate parity:
    // both sides required). No folder_compare fires, no error.
    if (leftPath === "" || rightPath === "") {
      setStatus("empty");
      return;
    }
    setStatus("loading");
    setError(null);
    setEntries([]);
    setTotal(0);
    setSession(null);
    setCollapsed([]);
    setRowFilter("all");
    setChangesOnly(false);
    setNameQuery("");
    setNameOpen(false);
    setGotoInput("");
    setGotoNote(null);
    setGotoOpen(false);
    setSelected(null);
    setCopied(null);
    setCopyNote(null);
    setOpenNote(null);
    (async () => {
      try {
        const raw = (await folderCompare(currentLeft, currentRight)) as unknown;
        if (
          !isRecord(raw) ||
          typeof raw["session"] !== "string" ||
          typeof raw["total"] !== "number"
        ) {
          if (!cancelled) setStatus("empty");
          return;
        }
        const sessionId = raw["session"] as string;
        const entryTotal = raw["total"] as number;
        if (cancelled) return;
        setSession(sessionId);
        if (entryTotal === 0) {
          setTotal(0);
          setStatus("empty");
          return;
        }
        const first = await fetchWindow(sessionId, 0, fullSubDir ? 500 : PAGE_SIZE);
        if (!cancelled) {
          const isMockSubdir = fullSubDir !== "" && first.some((e) => e.relPath.startsWith(`${fullSubDir}/`));
          const displayed = isMockSubdir ? reRootEntries(first, fullSubDir) : first;
          const displayedTotal = isMockSubdir ? displayed.length : entryTotal;
          setTotal(displayedTotal);
          setEntries(displayed);
          setStatus(displayed.length === 0 ? "empty" : "ready");
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
  }, [currentLeft, currentRight, fullSubDir]);

  async function loadMore(): Promise<void> {
    if (session === null || loadingMore || entries.length >= total) return;
    setLoadingMore(true);
    try {
      const next = await fetchWindow(session, entries.length, fullSubDir ? 500 : PAGE_SIZE);
      const isMockSubdir = fullSubDir !== "" && next.some((e) => e.relPath.startsWith(`${fullSubDir}/`));
      const displayedNext = isMockSubdir ? reRootEntries(next, fullSubDir) : next;
      setEntries((prev) => [...prev, ...displayedNext]);
    } catch (e) {
      setError(errString(e));
      setStatus("error");
    } finally {
      setLoadingMore(false);
    }
  }

  function toggle(dir: string): void {
    setCollapsed((prev) =>
      prev.includes(dir) ? prev.filter((d) => d !== dir) : [...prev, dir],
    );
  }

  // Wave-130: blank paths render the pick-two-paths empty (no
  // error-fire); a compared-but-empty tree keeps the legacy line.
  if (leftPath === "" || rightPath === "") return <p data-testid="folderview/empty">Pick two folders to compare — needs two paths.</p>;
  if (status === "loading") return <p>Loading folder comparison…</p>;
  if (status === "error") return <p role="alert">Folder compare failed: {error}</p>;
  if (status === "empty" || entries.length === 0) return <p>No entries to display.</p>;
  const hasMore = entries.length < total;

  function activate(entry: FolderEntryDto): void {
    if (entry.isDir) return;
    setSelected(entry.relPath);
  }

  /** Wave-78: directory-row activation descends into two-sided subdirectories. */
  function descend(entry: FolderEntryDto): void {
    if (!entry.isDir) return;
    if (entry.left === null || entry.right === null) {
      const missing = entry.left === null ? "left" : "right";
      setOpenNote(`one-sided row — no ${missing} folder to descend (${entry.relPath})`);
      return;
    }
    setOpenNote(null);
    const nextSubdir = subDir ? `${subDir}/${entry.relPath}` : entry.relPath;
    navigateTo(baseLeft, baseRight, baseSubDir, nextSubdir);
  }

  /** Wave-46: double-click drills a two-sided file into a text session. */
  function openPair(entry: FolderEntryDto): void {
    if (entry.isDir) return;
    setSelected(entry.relPath);
    if (entry.left === null || entry.right === null) {
      const missing = entry.left === null ? "left" : "right";
      setOpenNote(`one-sided row — no ${missing} file to open (${entry.relPath})`);
      return;
    }
    if (onOpenPaths === undefined) {
      setOpenNote(`drilldown unavailable here (${entry.relPath})`);
      return;
    }
    setOpenNote(null);
    const fullRel = fullSubDir ? `${fullSubDir}/${entry.relPath}` : entry.relPath;
    onOpenPaths("text", join(leftPath, fullRel), join(rightPath, fullRel));
  }

  const visibleFiles = visible.filter((e) => !e.isDir);

  function focusRow(relPath: string): void {
    const idx = visible.findIndex((e) => e.relPath === relPath);
    if (idx < 0) return;
    const el = document.querySelector(`[data-testid="folder-row-${idx}"]`);
    if (el instanceof HTMLElement) el.focus();
  }

  function stepSelection(current: string, dir: 1 | -1): void {
    const order = visibleFiles.map((e) => e.relPath);
    if (order.length === 0) return;
    const at = order.indexOf(current);
    const next =
      at < 0
        ? dir === 1
          ? order[0]
          : order[order.length - 1]
        : order[Math.min(order.length - 1, Math.max(0, at + dir))];
    if (!next || next === current) return;
    setSelected(next);
    // Rows persist across selection (only data-selected flips), so the
    // target row is already mounted: focus it synchronously to keep
    // selection and focus in the same frame for keyboard users.
    focusRow(next);
  }

  function jumpSelection(edge: "first" | "last"): void {
    const order = visibleFiles.map((e) => e.relPath);
    const next = edge === "first" ? order[0] : order[order.length - 1];
    if (!next) return;
    setSelected(next);
    focusRow(next);
  }

  /** Wave-98: jump to the Nth visible entry (1-based, wave-89 text
      goto ported to folder). Purely presentational over the
      already-loaded `visible` order: file rows select (detail
      follows), every row takes DOM focus; out-of-range input renders
      an honest inline note, never a throw. */
  function goToEntry(): void {
    const n = Number.parseInt(gotoInput.trim(), 10);
    if (!Number.isFinite(n) || n < 1 || n > visible.length) {
      setGotoNote(`entry '${gotoInput.trim()}' out of range — enter 1-${visible.length}`);
      return;
    }
    const target = visible[n - 1];
    if (!target) {
      setGotoNote(`entry ${n} is not rendered — load more rows first`);
      return;
    }
    setGotoNote(null);
    if (!target.isDir) setSelected(target.relPath);
    window.setTimeout(() => {
      const el = document.querySelector(`[data-testid="folder-row-${n - 1}"]`);
      if (el instanceof HTMLElement) {
        el.scrollIntoView({ block: "nearest" });
        el.focus({ preventScroll: true });
      } else {
        setGotoNote(`entry ${n} is outside the rendered window`);
      }
    }, 0);
  }

  async function copyPath(entry: FolderEntryDto): Promise<void> {
    try {
      const nav = navigator as Navigator & {
        clipboard?: { writeText(text: string): Promise<void> };
      };
      if (!nav.clipboard || typeof nav.clipboard.writeText !== "function") {
        setCopied(null);
        setCopyNote(`clipboard unavailable in this browser (${entry.relPath})`);
        return;
      }
      await nav.clipboard.writeText(entry.relPath);
      setCopied(entry.relPath);
      setCopyNote(null);
    } catch {
      setCopied(null);
      setCopyNote(`copy failed in this browser (${entry.relPath})`);
    }
  }

  function formatSize(size: number | null): string {
    if (size === null || !Number.isFinite(size)) return "size unknown";
    if (size < 1024) return `${size} B`;
    const kb = size / 1024;
    if (kb < 1024) return `${kb.toFixed(1)} KB`;
    const mb = kb / 1024;
    if (mb < 1024) return `${mb.toFixed(1)} MB`;
    return `${(mb / 1024).toFixed(1)} GB`;
  }

  function formatDay(modifiedMs: number | null): string {
    if (modifiedMs === null) return "modified unknown";
    const d = new Date(modifiedMs);
    if (Number.isNaN(d.getTime())) return "modified unknown";
    return d.toISOString().slice(0, 10);
  }

  function sideMeta(side: FolderEntryDto["left"]): string {
    if (side === null) return "size unknown / modified unknown";
    return `${formatSize(side.size)}, ${formatDay(side.modifiedMs)}`;
  }

  function describeSide(
    side: FolderEntryDto["left"],
    label: string,
  ): string {
    if (side === null) return `${label}: —`;
    const when =
      side.modifiedMs === null
        ? "modified: unknown"
        : `modified: ${new Date(side.modifiedMs).toISOString()}`;
    return `${label}: ${side.size} bytes, ${when}`;
  }

  function renderNode(entry: FolderEntryDto, i: number): React.JSX.Element {
    const isCollapsed = entry.isDir && collapsed.includes(entry.relPath);
    const isSelected = !entry.isDir && selected === entry.relPath;
    const isCopied = copied === entry.relPath;
    // Wave-96: BC 17px folder density (observe-93 measured ~17 logical px).
    // Unselected rows pin to ROW_PX so the measured pitch matches BC;
    // a selected row grows to reveal its inline detail (wave-28 UX kept).
    const compact = isSelected ? undefined : { height: ROW_PX, overflow: "hidden" as const };
    const compactNode = isSelected
      ? undefined
      : {
          // minHeight beats the .fv-node stylesheet min-height (22px
          // token, off limits); height alone would lose to it.
          height: ROW_PX,
          minHeight: ROW_PX,
          boxSizing: "border-box" as const,
          lineHeight: "16px",
          overflow: "hidden" as const,
        };
    const compactLabel = isSelected
      ? undefined
      : { paddingTop: 0, paddingBottom: 0, lineHeight: "16px" };
    // Wave-96: 14px icons + bordered buttons must fit the 16px content
    // box (17px row minus the 1px .fv-node divider). Zero the collapse
    // padding (14px icon) and tighten the copy-button line (14px + 2px
    // border = 16px) so nothing bleeds into the next row.
    const compactIconBtn = isSelected
      ? undefined
      : { padding: 0, height: 14, minHeight: 14, maxHeight: 14, lineHeight: "14px", borderWidth: 0 };
    const compactCopyBtn = isSelected
      ? undefined
      : { lineHeight: "14px", height: 16, minHeight: 16, maxHeight: 16, paddingTop: 0, paddingBottom: 0 };
    // Wave-66K: BC orphan idiom. One-sided rows read the missing side as
    // filler (`.fv-cell-empty` hatch over the row) and expose `data-side`
    // plus a single-line `data-bracket` (no rail: single-line rows never
    // span). Two-sided rows mount neither attribute nor hatch class.
    const isOrphan = entry.status === "leftOnly" || entry.status === "rightOnly";
    // Agent F rightOnly parity: rendered-geometry probe showed every row
    // (incl. rightOnly) painting content on the left half while BC parks
    // orphan content on its OWN side (hatch = missing side only). The row
    // div keeps the 66K/115 hatch (50% bg anchored to the missing side);
    // the content node takes the own-side half (leftOnly -> left half,
    // rightOnly -> right half, two-sided -> full width, unchanged). Inline
    // style only (tokens.css untouched); testids/classes/heights identical.
    const sidePlace =
      entry.status === "rightOnly"
        ? { width: "50%", marginLeft: "auto", boxSizing: "border-box" as const }
        : entry.status === "leftOnly"
          ? { width: "50%", marginRight: "auto", boxSizing: "border-box" as const }
          : undefined;
    return (
      <div
        key={entry.relPath}
        data-testid={`folder-row-${i}`}
        data-status={entry.status}
        data-path={entry.relPath}
        data-selected={isSelected ? "true" : "false"}
        data-side={isOrphan ? (entry.status === "leftOnly" ? "left-only" : "right-only") : undefined}
        data-bracket={isOrphan ? "single" : undefined}
        // Wave-115: `fv-row` hook for the hover/select copy affordance
        // + bounded orphan hatch (66K `fv-cell-empty`/`fv-rail` kept).
        className={isOrphan ? "fv-row fv-cell-empty fv-rail" : "fv-row"}
        style={compact}
        {...(entry.isDir
          ? {
              tabIndex: 0,
              role: "button",
              "aria-label": `descend ${entry.relPath}`,
              onDoubleClick: () => descend(entry),
              onKeyDown: (e: React.KeyboardEvent) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  descend(entry);
                } else if (e.key === "ArrowDown") {
                  e.preventDefault();
                  stepSelection(entry.relPath, 1);
                } else if (e.key === "ArrowUp") {
                  e.preventDefault();
                  stepSelection(entry.relPath, -1);
                } else if (e.key === "Home") {
                  e.preventDefault();
                  jumpSelection("first");
                } else if (e.key === "End") {
                  e.preventDefault();
                  jumpSelection("last");
                }
              },
            }
          : {
              tabIndex: 0,
              role: "button",
              "aria-label": `open ${entry.relPath}`,
              "aria-expanded": isSelected,
              onClick: () => activate(entry),
              onDoubleClick: () => openPair(entry),
              onKeyDown: (e: React.KeyboardEvent) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  activate(entry);
                } else if (e.key === "ArrowDown") {
                  e.preventDefault();
                  stepSelection(entry.relPath, 1);
                } else if (e.key === "ArrowUp") {
                  e.preventDefault();
                  stepSelection(entry.relPath, -1);
                } else if (e.key === "Home") {
                  e.preventDefault();
                  jumpSelection("first");
                } else if (e.key === "End") {
                  e.preventDefault();
                  jumpSelection("last");
                }
              },
            })}
      >
      <div data-testid={`folderview/node-${i}`} className={isSelected ? "fv-node is-selected font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)]" : "fv-node font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)]"} style={sidePlace === undefined && compactNode === undefined ? undefined : { ...sidePlace, ...compactNode }}>
        <span className="inline-flex items-center gap-[var(--ac-space-2)] px-[var(--ac-space-3)] py-[var(--ac-space-1)] ac-chrome-label font-normal" style={compactLabel}>
          {entry.isDir ? (
            <Button
              variant="ghost"
              size="sm"
              data-testid={`folder-collapse-${i}`}
              style={compactIconBtn}
              onClick={(e) => {
                e.stopPropagation();
                toggle(entry.relPath);
              }}
              aria-label={isCollapsed ? "expand" : "collapse"}
              className="!h-[14px] !min-h-[14px] !max-h-[14px] !p-0 !border-0 text-[var(--ac-fg-muted)] hover:text-[var(--ac-fg)]"
            >
              <Folder size={14} aria-hidden="true" />
            </Button>
          ) : (
            <File size={14} aria-hidden="true" />
          )}
          <StatusIcon status={entry.status} />
          <span className={`fv-status fv-status-${entry.status}`}>{entry.relPath}</span>
          <span className="text-[var(--ac-fg-muted)]">[{entry.status}]</span>
          <span
            data-testid={`folder-meta-${i}`}
            title={`${describeSide(entry.left, "left")} | ${describeSide(entry.right, "right")}`}
            className="truncate text-[var(--ac-fg-muted)] ac-meta"
          >
            {sideMeta(entry.left)} · {sideMeta(entry.right)}
          </span>
          {!entry.isDir && (
            <Button
              variant="ghost"
              size="sm"
              data-testid={`folder-copy-${i}`}
              aria-label={`copy path ${entry.relPath}`}
              style={compactCopyBtn}
              onClick={(e) => {
                e.stopPropagation();
                void copyPath(entry);
              }}
              onKeyDown={(e) => e.stopPropagation()}
              // Wave-115: `fv-copy` hides the per-row button until
              // hover/focus/select (BC affordance); testid kept.
              className="fv-copy !h-[16px] !min-h-[16px] !max-h-[16px] !border-[var(--ac-border)] !px-1 !py-0 ac-meta text-[var(--ac-fg-muted)] hover:text-[var(--ac-fg)]"
            >
              copy path
            </Button>
          )}
          {!entry.isDir && isCopied && (
            <span data-testid={`folder-copied-${i}`} role="status">
              copied
            </span>
          )}
        </span>
        {!entry.isDir && isSelected && (
          <div
            data-testid={`folder-detail-${i}`}
            className="mx-[var(--ac-space-3)] mb-[var(--ac-space-1)] rounded border border-[var(--ac-border-subtle)] px-[var(--ac-space-2)] py-[var(--ac-space-1)] ac-chrome-label font-normal text-[var(--ac-fg-muted)]"
          >
            <span>{entry.relPath}</span> <span>[{entry.status}]</span>{" "}
            <span>{describeSide(entry.left, "left")}</span> <span>{describeSide(entry.right, "right")}</span>
          </div>
        )}
      </div>
      </div>
    );
  }

  return (
    <section aria-label="folder comparison">
      {/* Wave-67B: chrome collapse (Beyond Compare style). One toolbar row
          (all/diffs/same + changes-only + name-filter toggle) plus a
          collapsible in-flow name panel (closed by default, Ctrl+F/Esc).
          Wave-113B: default-closed to match BC one-row chrome.
          Wave-98 (wave-89 port): a second in-flow goto popover (closed
          by default — no legacy goto specs — Ctrl+G/Esc) plus diffs /
          same counts merged into the bottom status line. Totals +
          counts live in the bottom status line. All legacy testids
          ride the same inputs; filter logic byte-identical. */}
      <FolderToolbar
        rowFilter={rowFilter}
        onRowFilter={setRowFilter}
        changesOnly={changesOnly}
        onToggleChangesOnly={() => setChangesOnly((v) => !v)}
        loadedCount={entries.length}
        diffsCount={diffsCount}
        sameCount={sameCount}
        nameFilterOpen={nameOpen}
        onToggleNameFilter={() => setNameOpen((v) => !v)}
      />
      <FolderNamePanel
        open={nameOpen}
        query={nameQuery}
        onQueryChange={setNameQuery}
        onClose={() => {
          setNameQuery("");
          setNameOpen(false);
        }}
      />
      {gotoOpen && (
        <section data-testid="folderview/goto-panel" className="tv-controls-panel">
          <div className="flex flex-wrap items-center gap-[var(--ac-space-2)]" role="search" aria-label="go to entry by number">
            <label htmlFor="folder-goto-box" className="text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]">
              Go to entry
            </label>
            <Input
              id="folder-goto-box"
              data-testid="folder-goto-input"
              inputMode="numeric"
              autoComplete="off"
              spellCheck={false}
              placeholder={`1-${visible.length}`}
              title="Go to entry by number (Ctrl+G)"
              value={gotoInput}
              onChange={(event) => setGotoInput(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") goToEntry();
                else if (event.key === "Escape") setGotoOpen(false);
              }}
              className="w-24 ac-mono text-[length:var(--ac-font-size-code)]"
            />
            <Button
              variant="secondary"
              data-testid="folderview/goto-go"
              onClick={goToEntry}
            >
              Go
            </Button>
            <ToolbarButton
              data-testid="folderview/goto-close"
              label="Dismiss go to entry (Esc)"
              title="Dismiss go to entry (Esc)"
              onClick={() => setGotoOpen(false)}
            >
              <X size={12} aria-hidden="true" />
            </ToolbarButton>
          </div>
          {gotoNote !== null && (
            <p data-testid="folder-goto-note" role="status" className="pt-[var(--ac-space-1)] ac-chrome-label font-normal text-[var(--ac-fg-muted)]">
              {gotoNote}
            </p>
          )}
        </section>
      )}
      {copyNote !== null && (
        <p data-testid="folder-copy-note" role="status" className="px-[var(--ac-space-4)] py-[var(--ac-space-1)] ac-chrome-label font-normal text-[var(--ac-fg-muted)]">
          {copyNote}
        </p>
      )}
      {openNote !== null && (
        <p data-testid="folder-open-note" role="status" className="px-[var(--ac-space-4)] py-[var(--ac-space-1)] ac-chrome-label font-normal text-[var(--ac-fg-muted)]">
          {openNote}
        </p>
      )}
      {/* Wave-FNAV: folder navigation bar with Back, Forward, Up variants, Set-as-Base, and Breadcrumbs */}
      <div
        data-testid="folder-breadcrumb-bar"
        className="flex items-center gap-[var(--ac-space-2)] border-b border-[var(--ac-border-subtle)] bg-[var(--ac-bg)] px-[var(--ac-space-4)] py-[var(--ac-space-1)] ac-chrome-label font-normal"
      >
        <Button
          variant="secondary"
          size="sm"
          data-testid="folder-back"
          onClick={goBack}
          disabled={!canGoBack}
          title={canGoBack ? "Back in history (Alt+Left)" : "No previous folder in history"}
          data-disabled-reason={!canGoBack ? "At start of history" : undefined}
          aria-label="Back in folder history"
          className="!h-6 !px-2 text-xs"
        >
          <ArrowLeft size={12} aria-hidden="true" />
          <span>Back</span>
        </Button>
        <Button
          variant="secondary"
          size="sm"
          data-testid="folder-forward"
          onClick={goForward}
          disabled={!canGoForward}
          title={canGoForward ? "Forward in history (Alt+Right)" : "No next folder in history"}
          data-disabled-reason={!canGoForward ? "At end of history" : undefined}
          aria-label="Forward in folder history"
          className="!h-6 !px-2 text-xs"
        >
          <ArrowRight size={12} aria-hidden="true" />
          <span>Forward</span>
        </Button>
        {/* Wave-L3: Browse L/R sit in this same bar (no new stacked
            rows, chrome budget holds): native directory picker per side,
            disabled with the contract reason where no shell exists. */}
        <Button
          variant="ghost"
          size="sm"
          data-testid="folder-browse-left"
          onClick={() => void browseSide("left")}
          disabled={!canBrowse}
          title={canBrowse ? "Browse for the left folder" : browseReason ?? undefined}
          data-disabled-reason={!canBrowse ? browseReason ?? undefined : undefined}
          aria-label="Browse for the left folder"
          className="!h-6 !px-1.5 text-xs text-[var(--ac-fg-muted)] hover:text-[var(--ac-fg)]"
        >
          Browse L
        </Button>
        <Button
          variant="ghost"
          size="sm"
          data-testid="folder-browse-right"
          onClick={() => void browseSide("right")}
          disabled={!canBrowse}
          title={canBrowse ? "Browse for the right folder" : browseReason ?? undefined}
          data-disabled-reason={!canBrowse ? browseReason ?? undefined : undefined}
          aria-label="Browse for the right folder"
          className="!h-6 !px-1.5 text-xs text-[var(--ac-fg-muted)] hover:text-[var(--ac-fg)]"
        >
          Browse R
        </Button>
        <Button
          variant="ghost"
          size="sm"
          data-testid="folder-up-both"
          onClick={goUpBoth}
          disabled={!canGoUpBoth}
          title={isAtRoot ? "At root folder — cannot go up further" : "Up one level (both sides)"}
          data-disabled-reason={isAtRoot ? "At root folder — cannot go up further" : undefined}
          aria-label="Up one level (both sides)"
          className="!h-6 !px-1.5 text-xs text-[var(--ac-fg-muted)] hover:text-[var(--ac-fg)]"
        >
          Up Both
        </Button>
        <Button
          variant="ghost"
          size="sm"
          data-testid="folder-up-left"
          onClick={goUpLeft}
          title="Up one level (left side)"
          aria-label="Up one level (left side)"
          className="!h-6 !px-1.5 text-xs text-[var(--ac-fg-muted)] hover:text-[var(--ac-fg)]"
        >
          Up L
        </Button>
        <Button
          variant="ghost"
          size="sm"
          data-testid="folder-up-right"
          onClick={goUpRight}
          title="Up one level (right side)"
          aria-label="Up one level (right side)"
          className="!h-6 !px-1.5 text-xs text-[var(--ac-fg-muted)] hover:text-[var(--ac-fg)]"
        >
          Up R
        </Button>
        <Button
          variant="secondary"
          size="sm"
          data-testid="folder-set-base"
          onClick={setAsBase}
          disabled={subDir === ""}
          title={subDir === "" ? "Already at base folder" : "Set current subdirectory as base folder"}
          data-disabled-reason={subDir === "" ? "Already at base folder" : undefined}
          aria-label="Set as base folder"
          className="!h-6 !px-2 text-xs"
        >
          <span>Set as Base</span>
        </Button>

        {(subDir !== "" || isBasePinned) && (
          <>
            <Button
              variant="secondary"
              size="sm"
              data-testid="folder-up-level"
              onClick={goUpBoth}
              disabled={!canGoUpBoth}
              title={isAtRoot ? "At root folder — cannot go up further" : "Up to parent folder"}
              data-disabled-reason={isAtRoot ? "At root folder — cannot go up further" : undefined}
              aria-label="Up to parent folder"
              className="!h-6 !px-2 text-xs"
            >
              <FolderUp size={12} aria-hidden="true" />
              <span>Up</span>
            </Button>
            <nav
              data-testid="folder-breadcrumb"
              aria-label="Folder breadcrumbs"
              className="inline-flex items-center gap-1 text-xs text-[var(--ac-fg-muted)] ml-[var(--ac-space-2)]"
            >
              <Button
                variant="ghost"
                size="sm"
                data-testid="folder-breadcrumb-root"
                onClick={() => navigateTo(leftPath, rightPath, "", "")}
                className="!h-auto !px-1 !py-0.5 text-xs text-[var(--ac-fg-muted)] hover:text-[var(--ac-fg)] hover:underline"
              >
                root
              </Button>
              {fullSubDir.split("/").filter(Boolean).map((segment, idx, arr) => {
                const pathUpTo = arr.slice(0, idx + 1).join("/");
                const isLast = idx === arr.length - 1;
                return (
                  <span key={pathUpTo} className="inline-flex items-center gap-1">
                    <span>/</span>
                    {isLast ? (
                      <span className="font-semibold text-[var(--ac-fg)]">
                        {segment}
                        {baseSubDir && pathUpTo === baseSubDir ? " [base]" : ""}
                      </span>
                    ) : (
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => {
                          if (baseSubDir && pathUpTo === baseSubDir) {
                            navigateTo(baseLeft, baseRight, baseSubDir, "");
                          } else if (baseSubDir && pathUpTo.startsWith(`${baseSubDir}/`)) {
                            const relToPinned = pathUpTo.slice(baseSubDir.length + 1);
                            navigateTo(baseLeft, baseRight, baseSubDir, relToPinned);
                          } else {
                            navigateTo(leftPath, rightPath, "", pathUpTo);
                          }
                        }}
                        className="!h-auto !px-1 !py-0.5 text-xs text-[var(--ac-fg-muted)] hover:text-[var(--ac-fg)] hover:underline"
                      >
                        {segment}
                      </Button>
                    )}
                  </span>
                );
              })}
            </nav>
          </>
        )}
      </div>
      {visible.length > VIRTUALIZE_AFTER ? (
        <div ref={parentRef} data-testid="folderview/scroller" className="h-[var(--ac-scroll-h)] overflow-auto">
          <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
            {virtualizer.getVirtualItems().map((v) => {
              const entry = visible[v.index];
              if (!entry) return null;
              return (
                <div
                  key={entry.relPath}
                  style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    width: "100%",
                    transform: `translateY(${v.start}px)`,
                  }}
                >
                  {renderNode(entry, v.index)}
                </div>
              );
            })}
          </div>
        </div>
      ) : (
        <div data-testid="folderview/list">{visible.map(renderNode)}</div>
      )}
      {hasMore && (
        <Button
          variant="secondary"
          size="md"
          data-testid="folderview/load-more"
          onClick={() => void loadMore()}
          disabled={loadingMore}
          className="my-[var(--ac-space-3)] ml-[var(--ac-space-4)]"
        >
          {loadingMore ? "Loading…" : `Load more (${entries.length}/${total})`}
        </Button>
      )}
      {/* Wave-67B: totals + counts live in the bottom status line (BC
          truth: counts in the status bar). Legacy testids ride the same
          strings so wave-28/35/63 specs keep passing unmodified.
          Wave-98 (wave-89 port): diffs / same counts always visible
          in a sibling `folderview/counts` span, so the status bar owns
          every count while `folderview/totals` stays byte-identical
          (exact-match pins in wave-66K specs). */}
      <div data-testid="folderview/status" className="px-[var(--ac-space-4)] py-[var(--ac-space-2)] ac-chrome-label font-normal text-[var(--ac-fg-muted)] border-t border-[var(--ac-border-subtle)] bg-[var(--ac-bg)]">
        <span data-testid="folderview/totals">
          {entries.length} / {total} entries
        </span>
        <span data-testid="folderview/counts">
          {" · "}{diffsCount} diffs · {sameCount} same
        </span>
        {changesOnly && (
          <span data-testid="folder-changes-count" role="status">
            {" · "}Showing {visible.length} changes — {sameCount} same hidden
          </span>
        )}
        {nameNeedle !== "" && (
          <span data-testid="folder-name-note" role="status">
            {" · "}{visible.length} of {entries.length} loaded {entries.length === 1 ? "entry" : "entries"} match &ldquo;{nameQuery.trim()}&rdquo; — clear to restore.
          </span>
        )}
      </div>
    </section>
  );
}
