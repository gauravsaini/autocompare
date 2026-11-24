import { useEffect, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ArrowLeftRight, Check, ChevronLeft, ChevronRight, Clock, File, Folder, FolderUp, X } from "lucide-react";
import { folderCompare, folderGetTree } from "../../ipc/folder";
import type { FolderEntryDto, FolderStatusDto } from "../../ipc/folder";
import { arrayField, errString, isRecord } from "./types";
import type { ViewPaths } from "./types";
import { FolderNamePanel, FolderToolbar } from "./folder/controls";

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
  const [subDir, setSubDir] = useState<string>("");
  const prevLeftRef = useRef(leftPath);
  const prevRightRef = useRef(rightPath);

  if (prevLeftRef.current !== leftPath || prevRightRef.current !== rightPath) {
    prevLeftRef.current = leftPath;
    prevRightRef.current = rightPath;
    if (subDir !== "") {
      setSubDir("");
    }
  }

  const join = (base: string, rel: string): string =>
    rel ? `${base.replace(/\/+$/, "")}/${rel}` : base;

  const currentLeft = join(leftPath, subDir);
  const currentRight = join(rightPath, subDir);

  const [status, setStatus] = useState<Status>("loading");
  const [error, setError] = useState<string | null>(null);
  const [session, setSession] = useState<string | null>(null);
  const [entries, setEntries] = useState<readonly FolderEntryDto[]>([]);
  const [total, setTotal] = useState(0);
  const [collapsed, setCollapsed] = useState<readonly string[]>([]);
  const [rowFilter, setRowFilter] = useState<"all" | "diffs" | "same">("all");
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

  const nameNeedle = nameQuery.trim().toLowerCase();
  const filtered = entries.filter((e) => {
    if (rowFilter === "same" && e.status !== "same") return false;
    if (rowFilter === "diffs" && e.status === "same") return false;
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
        const first = await fetchWindow(sessionId, 0, subDir ? 500 : PAGE_SIZE);
        if (!cancelled) {
          const isMockSubdir = subDir !== "" && first.some((e) => e.relPath.startsWith(`${subDir}/`));
          const displayed = isMockSubdir ? reRootEntries(first, subDir) : first;
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
  }, [currentLeft, currentRight, subDir]);

  async function loadMore(): Promise<void> {
    if (session === null || loadingMore || entries.length >= total) return;
    setLoadingMore(true);
    try {
      const next = await fetchWindow(session, entries.length, subDir ? 500 : PAGE_SIZE);
      const isMockSubdir = subDir !== "" && next.some((e) => e.relPath.startsWith(`${subDir}/`));
      const displayedNext = isMockSubdir ? reRootEntries(next, subDir) : next;
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
    setSubDir(nextSubdir);
  }

  function goUp(): void {
    if (!subDir) return;
    const parts = subDir.split("/").filter(Boolean);
    parts.pop();
    setSubDir(parts.join("/"));
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
    const fullRel = subDir ? `${subDir}/${entry.relPath}` : entry.relPath;
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
    const compactIconBtn = isSelected ? undefined : { padding: 0 };
    const compactCopyBtn = isSelected ? undefined : { lineHeight: "14px" };
    // Wave-66K: BC orphan idiom. One-sided rows read the missing side as
    // filler (`.fv-cell-empty` hatch over the row) and expose `data-side`
    // plus a single-line `data-bracket` (no rail: single-line rows never
    // span). Two-sided rows mount neither attribute nor hatch class.
    const isOrphan = entry.status === "leftOnly" || entry.status === "rightOnly";
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
      <div data-testid={`folderview/node-${i}`} className={isSelected ? "fv-node is-selected font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)]" : "fv-node font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)]"} style={compactNode}>
        <span className="inline-flex items-center gap-[var(--ac-space-2)] px-[var(--ac-space-3)] py-[var(--ac-space-1)] ac-chrome-label font-normal" style={compactLabel}>
          {entry.isDir ? (
            <button
              type="button"
              data-testid={`folder-collapse-${i}`}
              style={compactIconBtn}
              onClick={(e) => {
                e.stopPropagation();
                toggle(entry.relPath);
              }}
              aria-label={isCollapsed ? "expand" : "collapse"}
              className="rounded p-0.5 text-[var(--ac-fg-muted)] hover:text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus-ring)]"
            >
              <Folder size={14} aria-hidden="true" />
            </button>
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
            <button
              type="button"
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
              className="fv-copy rounded border border-[var(--ac-border)] px-1 py-0 ac-meta text-[var(--ac-fg-muted)] hover:text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus-ring)]"
            >
              copy path
            </button>
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
            <input
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
              className="w-24 rounded-md border border-[var(--ac-border)] bg-[var(--ac-bg)] px-[var(--ac-space-3)] py-[var(--ac-space-1)] ac-mono text-[length:var(--ac-font-size-code)] font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
            />
            <button
              type="button"
              data-testid="folderview/goto-go"
              onClick={goToEntry}
              className="inline-flex items-center rounded-md border border-[var(--ac-border)] bg-[var(--ac-bg-raised)] px-[var(--ac-space-3)] py-[var(--ac-space-1)] ac-chrome-label font-medium text-[var(--ac-fg)] shadow-xs transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
            >
              Go
            </button>
            <button
              type="button"
              data-testid="folderview/goto-close"
              aria-label="Dismiss go to entry (Esc)"
              title="Dismiss go to entry (Esc)"
              onClick={() => setGotoOpen(false)}
              className="inline-flex items-center rounded-md border border-[var(--ac-border)] bg-[var(--ac-bg-raised)] px-[var(--ac-space-3)] py-[var(--ac-space-1)] ac-chrome-label font-medium text-[var(--ac-fg)] shadow-xs transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
            >
              <X size={12} aria-hidden="true" />
            </button>
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
      {subDir !== "" && (
        <div
          data-testid="folder-breadcrumb-bar"
          className="flex items-center gap-[var(--ac-space-2)] border-b border-[var(--ac-border-subtle)] bg-[var(--ac-bg)] px-[var(--ac-space-4)] py-[var(--ac-space-1)] ac-chrome-label font-normal"
        >
          <button
            type="button"
            data-testid="folder-up-level"
            onClick={goUp}
            aria-label="Up to parent folder"
            className="inline-flex items-center gap-1 rounded border border-[var(--ac-border)] bg-[var(--ac-bg-raised)] px-2 py-0.5 text-xs font-medium text-[var(--ac-fg)] shadow-xs transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus-ring)]"
          >
            <FolderUp size={13} aria-hidden="true" />
            <span>Up</span>
          </button>
          <nav
            data-testid="folder-breadcrumb"
            aria-label="Folder breadcrumbs"
            className="inline-flex items-center gap-1 text-xs text-[var(--ac-fg-muted)]"
          >
            <button
              type="button"
              data-testid="folder-breadcrumb-root"
              onClick={() => setSubDir("")}
              className="hover:text-[var(--ac-fg)] hover:underline focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus-ring)]"
            >
              root
            </button>
            {subDir.split("/").map((segment, idx, arr) => {
              const pathUpTo = arr.slice(0, idx + 1).join("/");
              const isLast = idx === arr.length - 1;
              return (
                <span key={pathUpTo} className="inline-flex items-center gap-1">
                  <span>/</span>
                  {isLast ? (
                    <span className="font-semibold text-[var(--ac-fg)]">{segment}</span>
                  ) : (
                    <button
                      type="button"
                      onClick={() => setSubDir(pathUpTo)}
                      className="hover:text-[var(--ac-fg)] hover:underline focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus-ring)]"
                    >
                      {segment}
                    </button>
                  )}
                </span>
              );
            })}
          </nav>
        </div>
      )}
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
        <button type="button" data-testid="folderview/load-more" onClick={() => void loadMore()} disabled={loadingMore} className="my-[var(--ac-space-3)] ml-[var(--ac-space-4)] inline-flex items-center rounded-md border border-[var(--ac-border)] bg-[var(--ac-bg-raised)] px-[var(--ac-space-3)] py-[var(--ac-space-2)] ac-chrome-label font-medium text-[var(--ac-fg)] shadow-xs transition hover:bg-[var(--ac-toolbar-btn-hover-bg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)] disabled:cursor-not-allowed disabled:opacity-45">
          {loadingMore ? "Loading…" : `Load more (${entries.length}/${total})`}
        </button>
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
