import React, { useCallback, useEffect, useMemo, useState } from "react";
import { FileText, Folder, FolderOpen } from "lucide-react";
import { folderCompare, folderGetTree } from "../../ipc/folder";
import type { FolderEntryDto } from "../../ipc/folder";
import V2ErrorBanner from "../components/V2ErrorBanner";
import { isMissingFixtureError } from "../utils/demoGuard";
import { useSplitPane } from "../hooks/useSplitPane";
import type { FilterMode, V2StatusInfo, V2ViewCommands } from "../types";

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
  const [entries, setEntries] = useState<readonly FolderEntryDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filterMode, setFilterMode] = useState<FilterMode>("all");
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const { containerRef, splitPct, dragging, startDrag, resetSplit } = useSplitPane(50);

  useEffect(() => {
    setLeftDraft(leftPath);
    setRightDraft(rightPath);
  }, [leftPath, rightPath]);

  const runCompare = useCallback(async (l: string, r: string): Promise<void> => {
    setLoading(true);
    setError(null);
    const lEff = l || "tools/parity-harness/fixtures/folder/left";
    const rEff = r || "tools/parity-harness/fixtures/folder/right";
    try {
      const dto = await folderCompare(lEff, rEff);
      const page = await folderGetTree(dto.session, 0, 500, "all");
      setEntries(page.entries);
      const ts = new Date().toLocaleTimeString();
      setLogs((prev) => [
        ...prev.slice(-4),
        `${ts}  Loaded ${page.entries.length} entries (${lEff} <-> ${rEff})`,
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
  }, []);

  useEffect(() => {
    void runCompare(leftPath, rightPath);
  }, [leftPath, rightPath, runCompare]);

  const visibleEntries = useMemo(() => {
    if (filterMode === "all") return entries;
    if (filterMode === "same") return entries.filter((e) => e.status === "same");
    return entries.filter((e) => e.status !== "same");
  }, [entries, filterMode]);

  const diffCount = useMemo(
    () => entries.filter((e) => e.status !== "same").length,
    [entries],
  );

  useEffect(() => {
    onRegisterCommands({
      filterMode,
      setFilterMode,
      swapSides: () => onPathsChange(rightDraft, leftDraft),
      reload: () => void runCompare(leftDraft, rightDraft),
    });
  }, [onRegisterCommands, filterMode, onPathsChange, leftDraft, rightDraft, runCompare]);

  useEffect(() => {
    onStatusChange({
      left: `${visibleEntries.length} items shown · ${diffCount} differences`,
      center: filterMode === "all" ? "All items" : filterMode === "diffs" ? "Differences only" : "Same only",
      right: `${entries.length} total`,
    });
  }, [onStatusChange, visibleEntries.length, diffCount, filterMode, entries.length]);

  const handleBrowse = async (side: "left" | "right"): Promise<void> => {
    try {
      const mod = await import("@tauri-apps/plugin-dialog");
      const picked = await mod.open({ multiple: false, directory: true });
      if (typeof picked === "string" && picked) {
        const nextL = side === "left" ? picked : leftDraft;
        const nextR = side === "right" ? picked : rightDraft;
        setLeftDraft(nextL);
        setRightDraft(nextR);
        onPathsChange(nextL, nextR);
      }
    } catch {
      // Browser fallback
    }
  };

  return (
    <div data-testid="v2/folder-view" className="flex h-full min-h-0 flex-col overflow-hidden bg-[var(--v2-surface)]">
      {/* Dual-Pane Path Bars */}
      <form
        onSubmit={(e) => {
          e.preventDefault();
          onPathsChange(leftDraft.trim(), rightDraft.trim());
        }}
        style={{ gridTemplateColumns: `${splitPct}fr ${100 - splitPct}fr` }}
        className="grid shrink-0 border-b border-[var(--v2-border)] bg-[var(--v2-surface-subtle)]"
      >
        {(["left", "right"] as const).map((side) => {
          const val = side === "left" ? leftDraft : rightDraft;
          const setVal = side === "left" ? setLeftDraft : setRightDraft;
          return (
            <div
              key={side}
              className={`flex items-center gap-1.5 px-2.5 py-1.5 ${
                side === "left" ? "border-r border-[var(--v2-border)]" : ""
              }`}
            >
              <input
                type="text"
                value={val}
                onChange={(e) => setVal(e.target.value)}
                placeholder={side === "left" ? "Left folder path…" : "Right folder path…"}
                className="v2-input v2-mono min-w-0 flex-1 text-[11.5px]"
              />
              <button
                type="button"
                title={`Browse ${side} folder`}
                onClick={() => void handleBrowse(side)}
                className="v2-btn px-2"
              >
                <FolderOpen size={13} />
              </button>
            </div>
          );
        })}
      </form>

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

      <V2ErrorBanner message={error} onDismiss={() => setError(null)} />

      {/* Synchronized Dual-Pane Tree Rows */}
      <div ref={containerRef} className="min-h-0 flex-1 overflow-y-auto text-[12px]">
        {loading ? (
          <div className="flex h-full items-center justify-center text-[var(--v2-fg-muted)]">
            Comparing folders…
          </div>
        ) : visibleEntries.length === 0 ? (
          <div className="flex h-full items-center justify-center text-[var(--v2-fg-muted)]">
            No matching folder entries.
          </div>
        ) : (
          visibleEntries.map((entry) => {
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
          })
        )}
      </div>

      {/* Compact BC Bottom Activity Log */}
      <div className="v2-mono h-[54px] shrink-0 overflow-y-auto border-t border-[var(--v2-border)] bg-[var(--v2-surface-subtle)] px-3 py-1 text-[11px] leading-[16px] text-[var(--v2-fg-muted)]">
        {logs.length === 0 ? "Ready" : logs.map((line, i) => <div key={i}>{line}</div>)}
      </div>
    </div>
  );
}
