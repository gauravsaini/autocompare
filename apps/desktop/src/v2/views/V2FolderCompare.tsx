import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ArrowLeft, ArrowRight, FileText, Folder, Trash2 } from "lucide-react";
import {
  folderCompare,
  folderCopyEntry,
  folderDeleteEntry,
  folderGetTree,
} from "../../ipc/folder";
import type { FolderEntryDto } from "../../ipc/folder";
import V2ErrorBanner from "../components/V2ErrorBanner";
import V2ReloadPromptBanner from "../components/V2ReloadPromptBanner";
import V2DualPathBar from "../components/V2DualPathBar";
import V2FolderRulesModal from "../components/folder/V2FolderRulesModal";
import type { FolderRulesTab, FolderSessionRules } from "../components/folder/V2FolderRulesModal";
import { isMissingFixtureError } from "../utils/demoGuard";
import { useSplitPane } from "../hooks/useSplitPane";
import { useVirtualRows } from "../hooks/useVirtualRows";
import { useExternalChangeWatch } from "../hooks/useExternalChangeWatch";
import type { FilterMode, V2StatusInfo, V2ViewCommands } from "../types";

const FOLDER_ROW_HEIGHT = 22;

export interface V2FolderCompareProps {
  readonly leftPath: string;
  readonly rightPath: string;
  readonly onPathsChange: (left: string, right: string) => void;
  readonly onOpenTextPair: (left: string, right: string, title: string) => void;
  readonly onRegisterCommands: (cmds: V2ViewCommands) => void;
  readonly onStatusChange: (status: V2StatusInfo) => void;
}

function formatSize(bytes?: number | null): string {
  if (bytes === undefined || bytes === null) return "";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatDate(ms?: number | null): string {
  if (!ms) return "";
  try {
    return new Date(ms).toLocaleDateString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  } catch {
    return "";
  }
}

function statusColor(status: FolderEntryDto["status"]): string {
  switch (status) {
    case "different":
      return "var(--v2-del-fg)";
    case "leftOnly":
      return "#7c3aed";
    case "rightOnly":
      return "#059669";
    case "newerLeft":
    case "newerRight":
      return "var(--v2-accent)";
    default:
      return "var(--v2-fg)";
  }
}

export default function V2FolderCompare({
  leftPath,
  rightPath,
  onPathsChange,
  onOpenTextPair,
  onRegisterCommands,
  onStatusChange,
}: V2FolderCompareProps): React.JSX.Element {
  const [leftDraft, setLeftDraft] = useState(leftPath);
  const [rightDraft, setRightDraft] = useState(rightPath);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [entries, setEntries] = useState<readonly FolderEntryDto[]>([]);
  const [totalEntries, setTotalEntries] = useState(0);
  const [loadingMore, setLoadingMore] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filterMode, setFilterMode] = useState<FilterMode>("all");
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [contextMenu, setContextMenu] = useState<{
    readonly x: number;
    readonly y: number;
    readonly entry: FolderEntryDto;
  } | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [folderRules, setFolderRules] = useState<FolderSessionRules>({
    contentCompare: false,
    timestampToleranceSec: 2,
    masks: "",
  });
  const [rulesTab, setRulesTab] = useState<FolderRulesTab | null>(null);
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const { containerRef, splitPct, dragging, startDrag, resetSplit } = useSplitPane(50);

  useEffect(() => {
    setLeftDraft(leftPath);
    setRightDraft(rightPath);
  }, [leftPath, rightPath]);

  useEffect(() => {
    const closeMenu = (): void => setContextMenu(null);
    window.addEventListener("click", closeMenu);
    return () => window.removeEventListener("click", closeMenu);
  }, []);

  const runCompare = useCallback(
    async (l: string, r: string): Promise<void> => {
      setLoading(true);
      setError(null);
      const lEff = l || "tools/parity-harness/fixtures/folder/left";
      const rEff = r || "tools/parity-harness/fixtures/folder/right";
      try {
        const masks = folderRules.masks.trim() || null;
        const dto = await folderCompare(
          lEff,
          rEff,
          masks,
          folderRules.contentCompare,
          !folderRules.contentCompare,
        );
        setSessionId(dto.session);
        const page = await folderGetTree(dto.session, 0, 500, "all");
        setEntries(page.entries);
        setTotalEntries(page.total);
        const ts = new Date().toLocaleTimeString();
        setLogs((prev) => [
          ...prev.slice(-4),
          `${ts}  Loaded ${page.entries.length}/${page.total} entries (${folderRules.contentCompare ? "binary" : "quick"}${masks ? `, mask: ${masks}` : ""})`,
        ]);
      } catch (e) {
        if (!l && !r && isMissingFixtureError(e)) {
          setEntries([]);
        } else {
          setError(e instanceof Error ? e.message : String(e));
        }
      } finally {
        setLoading(false);
      }
    },
    [folderRules.masks, folderRules.contentCompare],
  );

  useEffect(() => {
    void runCompare(leftPath, rightPath);
  }, [leftPath, rightPath, runCompare]);

  const watchedPaths = useMemo(
    () => [leftDraft.trim(), rightDraft.trim()].filter((p) => p.length > 0),
    [leftDraft, rightDraft],
  );
  const { changedPaths, dismiss: dismissReload, resetBaseline } = useExternalChangeWatch({
    paths: watchedPaths,
    enabled: !loading && watchedPaths.length > 0,
    onAutoReload: () => void runCompare(leftDraft, rightDraft),
  });

  const loadMoreEntries = useCallback(async (): Promise<void> => {
    if (!sessionId || loadingMore || entries.length >= totalEntries) return;
    setLoadingMore(true);
    try {
      const page = await folderGetTree(sessionId, entries.length, 500, "all");
      if (page.entries.length > 0) {
        setEntries((prev) => [...prev, ...page.entries]);
        setTotalEntries(page.total);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoadingMore(false);
    }
  }, [sessionId, loadingMore, entries.length, totalEntries]);

  const effectiveEntries = useMemo(() => {
    if (folderRules.contentCompare || folderRules.timestampToleranceSec <= 0) return entries;
    const tolMs = folderRules.timestampToleranceSec * 1000;
    return entries.map((e) => {
      if (
        (e.status === "newerLeft" || e.status === "newerRight") &&
        e.left !== null &&
        e.right !== null &&
        e.left.size === e.right.size &&
        e.left.modifiedMs !== null &&
        e.right.modifiedMs !== null &&
        Math.abs(e.left.modifiedMs - e.right.modifiedMs) <= tolMs
      ) {
        return { ...e, status: "same" as const };
      }
      return e;
    });
  }, [entries, folderRules.contentCompare, folderRules.timestampToleranceSec]);

  const visibleEntries = useMemo(() => {
    if (filterMode === "all") return effectiveEntries;
    if (filterMode === "same") return effectiveEntries.filter((e) => e.status === "same");
    return effectiveEntries.filter((e) => e.status !== "same");
  }, [effectiveEntries, filterMode]);

  const { startIndex, endIndex, totalHeight, offsetTop } = useVirtualRows({
    count: visibleEntries.length,
    rowHeight: FOLDER_ROW_HEIGHT,
    overscan: 16,
    scrollRef,
  });

  const windowedEntries = useMemo(
    () => visibleEntries.slice(startIndex, endIndex),
    [visibleEntries, startIndex, endIndex],
  );

  const diffCount = useMemo(
    () => effectiveEntries.filter((e) => e.status !== "same").length,
    [effectiveEntries],
  );

  const selectedEntry = useMemo(
    () => (selectedPath ? effectiveEntries.find((e) => e.relPath === selectedPath) ?? null : null),
    [effectiveEntries, selectedPath],
  );

  const handleCopyEntry = useCallback(
    async (entry: FolderEntryDto, dir: "leftToRight" | "rightToLeft"): Promise<void> => {
      if (!sessionId) return;
      setContextMenu(null);
      try {
        const res = await folderCopyEntry(sessionId, entry.relPath, dir);
        const ts = new Date().toLocaleTimeString();
        setLogs((prev) => [
          ...prev.slice(-6),
          `${ts}  Copied ${entry.relPath} (${dir === "leftToRight" ? "L → R" : "R → L"}, ${formatSize(res.bytesAffected)})`,
        ]);
        await runCompare(leftDraft, rightDraft);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    },
    [sessionId, runCompare, leftDraft, rightDraft],
  );

  const handleDeleteEntry = useCallback(
    async (entry: FolderEntryDto, side: "left" | "right"): Promise<void> => {
      if (!sessionId) return;
      setContextMenu(null);
      if (!window.confirm(`Delete "${entry.relPath}" on the ${side} side?`)) return;
      try {
        await folderDeleteEntry(sessionId, entry.relPath, side);
        const ts = new Date().toLocaleTimeString();
        setLogs((prev) => [
          ...prev.slice(-6),
          `${ts}  Deleted ${entry.relPath} (${side})`,
        ]);
        await runCompare(leftDraft, rightDraft);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    },
    [sessionId, runCompare, leftDraft, rightDraft],
  );

  useEffect(() => {
    onRegisterCommands({
      filterMode,
      setFilterMode,
      openRules: () => setRulesTab("comparison"),
      openFormat: () => setRulesTab("filters"),
      swapSides: () => onPathsChange(rightDraft, leftDraft),
      reload: () => void runCompare(leftDraft, rightDraft),
    });
  }, [onRegisterCommands, filterMode, onPathsChange, leftDraft, rightDraft, runCompare]);

  useEffect(() => {
    const modeLabel = folderRules.contentCompare ? "Binary compare" : `Quick (±${folderRules.timestampToleranceSec}s)`;
    const maskLabel = folderRules.masks.trim() ? ` · Filter: ${folderRules.masks.trim()}` : "";
    onStatusChange({
      left: `${visibleEntries.length} items shown · ${diffCount} differences`,
      center: `${modeLabel}${maskLabel}`,
      right: totalEntries > entries.length ? `${entries.length} / ${totalEntries} loaded` : `${entries.length} total`,
    });
  }, [onStatusChange, visibleEntries.length, diffCount, folderRules, entries.length, totalEntries]);

  return (
    <div data-testid="v2/folder-view" className="flex h-full min-h-0 flex-col overflow-hidden bg-[var(--v2-surface)]">
      <V2DualPathBar
        leftValue={leftDraft}
        rightValue={rightDraft}
        onLeftChange={setLeftDraft}
        onRightChange={setRightDraft}
        onSubmit={onPathsChange}
        leftPlaceholder="Left folder path…"
        rightPlaceholder="Right folder path…"
        directory
        splitPct={splitPct}
      />

      {/* Dual-Pane Column Headers (True BC 2-Side Folder Tree!) */}
      <div
        style={{ gridTemplateColumns: `minmax(0, ${splitPct}fr) 76px 96px 6px minmax(0, ${100 - splitPct}fr) 76px 96px` }}
        className="grid shrink-0 border-b border-[var(--v2-border)] bg-[var(--v2-chrome)] text-[11px] font-semibold text-[var(--v2-fg-secondary)]"
      >
        <div className="px-3 py-1">Name</div>
        <div className="px-2 py-1 text-right">Size</div>
        <div className="px-2 py-1 text-right">Modified</div>
        <div
          role="separator"
          aria-orientation="vertical"
          title="Drag to resize panes (double-click to reset 50/50)"
          onMouseDown={startDrag}
          onDoubleClick={resetSplit}
          className="cursor-col-resize bg-[var(--v2-splitter)]/40 hover:bg-[var(--v2-accent)]/60"
        />
        <div className="px-3 py-1">Name</div>
        <div className="px-2 py-1 text-right">Size</div>
        <div className="px-2 py-1 text-right">Modified</div>
      </div>

      {selectedEntry ? (
        <div className="flex shrink-0 items-center justify-between gap-2 border-b border-[var(--v2-border)] bg-[var(--v2-surface-subtle)] px-3 py-1 text-[11.5px]">
          <span className="v2-mono truncate text-[var(--v2-fg-secondary)]">{selectedEntry.relPath}</span>
          <div className="flex items-center gap-1.5">
            {selectedEntry.left !== null ? (
              <button
                type="button"
                onClick={() => void handleCopyEntry(selectedEntry, "leftToRight")}
                className="v2-btn h-[21px] px-2 text-[11px]"
              >
                Copy to Right <ArrowRight size={11} />
              </button>
            ) : null}
            {selectedEntry.right !== null ? (
              <button
                type="button"
                onClick={() => void handleCopyEntry(selectedEntry, "rightToLeft")}
                className="v2-btn h-[21px] px-2 text-[11px]"
              >
                <ArrowLeft size={11} /> Copy to Left
              </button>
            ) : null}
            {selectedEntry.left !== null ? (
              <button
                type="button"
                onClick={() => void handleDeleteEntry(selectedEntry, "left")}
                className="v2-btn h-[21px] px-2 text-[11px] text-[var(--v2-del-fg)]"
              >
                <Trash2 size={11} /> Delete L
              </button>
            ) : null}
            {selectedEntry.right !== null ? (
              <button
                type="button"
                onClick={() => void handleDeleteEntry(selectedEntry, "right")}
                className="v2-btn h-[21px] px-2 text-[11px] text-[var(--v2-del-fg)]"
              >
                <Trash2 size={11} /> Delete R
              </button>
            ) : null}
          </div>
        </div>
      ) : null}

      <V2ReloadPromptBanner
        changedPaths={changedPaths}
        onReload={() => {
          void resetBaseline();
          void runCompare(leftDraft, rightDraft);
        }}
        onDismiss={dismissReload}
      />
      <V2ErrorBanner message={error} onDismiss={() => setError(null)} />

      {/* Synchronized Dual-Pane Virtualized Tree Rows */}
      <div
        ref={(el) => {
          scrollRef.current = el;
          (containerRef as React.MutableRefObject<HTMLDivElement | null>).current = el;
        }}
        className="min-h-0 flex-1 overflow-y-auto text-[12px]"
      >
        {loading ? (
          <div className="flex h-full items-center justify-center text-[var(--v2-fg-muted)]">
            Comparing folders…
          </div>
        ) : visibleEntries.length === 0 ? (
          <div className="flex h-full items-center justify-center text-[var(--v2-fg-muted)]">
            No matching folder entries.
          </div>
        ) : (
          <div className="relative" style={{ height: totalHeight }}>
            <div style={{ transform: `translateY(${offsetTop}px)` }}>
          {windowedEntries.map((entry) => {
            const hasLeft = entry.left !== null || entry.status !== "rightOnly";
            const hasRight = entry.right !== null || entry.status !== "leftOnly";
            const isSelected = selectedPath === entry.relPath;
            const color = isSelected ? "var(--v2-sel-fg)" : statusColor(entry.status);
            const depth = entry.relPath.split("/").length - 1;
            const name = entry.relPath.split("/").pop() ?? entry.relPath;
            const Icon = entry.isDir ? Folder : FileText;

            return (
              <div
                key={entry.relPath}
                onClick={() => setSelectedPath(entry.relPath)}
                onContextMenu={(e) => {
                  e.preventDefault();
                  setSelectedPath(entry.relPath);
                  setContextMenu({ x: e.clientX, y: e.clientY, entry });
                }}
                onDoubleClick={() => {
                  if (!entry.isDir && leftDraft && rightDraft) {
                    onOpenTextPair(
                      `${leftDraft.replace(/\/$/, "")}/${entry.relPath}`,
                      `${rightDraft.replace(/\/$/, "")}/${entry.relPath}`,
                      name,
                    );
                  }
                }}
                style={{
                  backgroundColor: isSelected ? "var(--v2-sel-bg)" : undefined,
                  color,
                  gridTemplateColumns: `minmax(0, ${splitPct}fr) 76px 96px 6px minmax(0, ${100 - splitPct}fr) 76px 96px`,
                }}
                className="grid h-[22px] cursor-default items-center hover:bg-[var(--v2-btn-hover)]/50"
              >
                {/* Left Side */}
                {hasLeft ? (
                  <>
                    <div
                      className="flex min-w-0 items-center gap-1.5 truncate pr-2"
                      style={{ paddingLeft: 12 + depth * 14 }}
                    >
                      <Icon size={13} className="shrink-0 opacity-80" />
                      <span className="truncate">{name}</span>
                    </div>
                    <div className="v2-mono px-2 text-right text-[11px] opacity-80">
                      {entry.isDir ? "" : formatSize(entry.left?.size)}
                    </div>
                    <div className="v2-mono px-2 text-right text-[11px] opacity-75">
                      {formatDate(entry.left?.modifiedMs)}
                    </div>
                  </>
                ) : (
                  <div className="v2-hatch col-span-3 h-full" />
                )}

                {/* Draggable Center Splitter */}
                <span
                  role="separator"
                  aria-orientation="vertical"
                  title="Drag to resize panes (double-click to reset 50/50)"
                  onMouseDown={startDrag}
                  onDoubleClick={resetSplit}
                  className="group relative h-full cursor-col-resize bg-[var(--v2-surface-subtle)]"
                >
                  <span
                    className={`absolute inset-y-0 left-1/2 -translate-x-1/2 transition-colors ${
                      dragging
                        ? "w-[2px] bg-[var(--v2-accent)]"
                        : "w-px bg-[var(--v2-splitter)] group-hover:w-[2px] group-hover:bg-[var(--v2-accent)]"
                    }`}
                  />
                </span>

                {/* Right Side */}
                {hasRight ? (
                  <>
                    <div
                      className="flex min-w-0 items-center gap-1.5 truncate pr-2"
                      style={{ paddingLeft: 12 + depth * 14 }}
                    >
                      <Icon size={13} className="shrink-0 opacity-80" />
                      <span className="truncate">{name}</span>
                    </div>
                    <div className="v2-mono px-2 text-right text-[11px] opacity-80">
                      {entry.isDir ? "" : formatSize(entry.right?.size)}
                    </div>
                    <div className="v2-mono px-2 text-right text-[11px] opacity-75">
                      {formatDate(entry.right?.modifiedMs)}
                    </div>
                  </>
                ) : (
                  <div className="v2-hatch col-span-3 h-full" />
                )}
              </div>
            );
          })}
            </div>
          </div>
        )}
      </div>

      {entries.length < totalEntries ? (
        <div className="flex shrink-0 items-center justify-between border-t border-[var(--v2-border)] bg-[var(--v2-chrome)] px-3 py-1 text-[11px]">
          <span className="text-[var(--v2-fg-muted)]">
            Loaded {entries.length.toLocaleString()} of {totalEntries.toLocaleString()} entries
          </span>
          <button
            type="button"
            disabled={loadingMore}
            onClick={() => void loadMoreEntries()}
            className="v2-btn h-[21px] px-2 text-[11px]"
          >
            {loadingMore ? "Loading…" : "Load Next 500"}
          </button>
        </div>
      ) : null}

      {/* Compact BC Bottom Activity Log */}
      <div className="v2-mono h-[54px] shrink-0 overflow-y-auto border-t border-[var(--v2-border)] bg-[var(--v2-surface-subtle)] px-3 py-1 text-[11px] leading-[16px] text-[var(--v2-fg-muted)]">
        {logs.length === 0 ? "Ready" : logs.map((line, i) => <div key={i}>{line}</div>)}
      </div>

      {contextMenu ? (
        <div
          style={{ left: contextMenu.x, top: contextMenu.y }}
          className="fixed z-50 min-w-[190px] rounded-[7px] border border-[var(--v2-border)] bg-[var(--v2-chrome-elevated)] py-1 text-[12px] shadow-lg"
          onClick={(e) => e.stopPropagation()}
        >
          {contextMenu.entry.left !== null ? (
            <button
              type="button"
              onClick={() => void handleCopyEntry(contextMenu.entry, "leftToRight")}
              className="flex w-full items-center justify-between px-3 py-1.5 text-left hover:bg-[var(--v2-accent-soft)]"
            >
              <span>Copy to Right →</span>
            </button>
          ) : null}
          {contextMenu.entry.right !== null ? (
            <button
              type="button"
              onClick={() => void handleCopyEntry(contextMenu.entry, "rightToLeft")}
              className="flex w-full items-center justify-between px-3 py-1.5 text-left hover:bg-[var(--v2-accent-soft)]"
            >
              <span>← Copy to Left</span>
            </button>
          ) : null}
          {!contextMenu.entry.isDir && leftDraft && rightDraft ? (
            <button
              type="button"
              onClick={() => {
                const e = contextMenu.entry;
                const nm = e.relPath.split("/").pop() ?? e.relPath;
                setContextMenu(null);
                onOpenTextPair(
                  `${leftDraft.replace(/\/$/, "")}/${e.relPath}`,
                  `${rightDraft.replace(/\/$/, "")}/${e.relPath}`,
                  nm,
                );
              }}
              className="flex w-full items-center justify-between px-3 py-1.5 text-left hover:bg-[var(--v2-accent-soft)]"
            >
              <span>Open in Text Compare</span>
            </button>
          ) : null}
          {contextMenu.entry.left !== null ? (
            <button
              type="button"
              onClick={() => void handleDeleteEntry(contextMenu.entry, "left")}
              className="mt-0.5 flex w-full items-center justify-between border-t border-[var(--v2-border-subtle)] px-3 py-1.5 text-left text-[var(--v2-del-fg)] hover:bg-[var(--v2-accent-soft)]"
            >
              <span>Delete Left</span>
            </button>
          ) : null}
          {contextMenu.entry.right !== null ? (
            <button
              type="button"
              onClick={() => void handleDeleteEntry(contextMenu.entry, "right")}
              className="flex w-full items-center justify-between px-3 py-1.5 text-left text-[var(--v2-del-fg)] hover:bg-[var(--v2-accent-soft)]"
            >
              <span>Delete Right</span>
            </button>
          ) : null}
        </div>
      ) : null}

      <V2FolderRulesModal
        open={rulesTab !== null}
        initialTab={rulesTab ?? "comparison"}
        rules={folderRules}
        onClose={() => setRulesTab(null)}
        onApply={(next) => {
          setFolderRules(next);
          setRulesTab(null);
        }}
      />
    </div>
  );
}
