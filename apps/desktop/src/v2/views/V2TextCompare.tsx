import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { FolderOpen, Save, Search, X } from "lucide-react";
import {
  textApplyEdit,
  textApplyHunk,
  textCompare,
  textCompareDemo,
  textGetRows,
  textSaveSide,
} from "../../ipc/text";
import type { InlineSpanDto, TextCompareDto, TextRowDto, TextSide } from "../../ipc/text";
import V2Modal from "../components/V2Modal";
import { useSplitPane } from "../hooks/useSplitPane";
import type { FilterMode, V2StatusInfo, V2ViewCommands } from "../types";

export interface V2TextCompareProps {
  readonly leftPath: string;
  readonly rightPath: string;
  readonly onPathsChange: (left: string, right: string) => void;
  readonly onDirtyChange: (dirty: boolean) => void;
  readonly onRegisterCommands: (cmds: V2ViewCommands) => void;
  readonly onStatusChange: (status: V2StatusInfo) => void;
}

type HunkRole = "single" | "start" | "middle" | "end";

interface TextHunk {
  readonly ordinal: number;
  readonly start: number;
  readonly end: number;
}

function collectHunks(rows: readonly TextRowDto[]): TextHunk[] {
  const out: TextHunk[] = [];
  let start = -1;
  let end = -1;
  for (const r of rows) {
    if (r.kind === "equal") {
      if (start >= 0) {
        out.push({ ordinal: out.length, start, end });
        start = -1;
        end = -1;
      }
    } else {
      if (start < 0) start = r.index;
      end = r.index;
    }
  }
  if (start >= 0) out.push({ ordinal: out.length, start, end });
  return out;
}

function formatBytes(n?: number): string {
  if (n === undefined) return "";
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(2)} MB`;
}

function formatMtime(unix?: number): string {
  if (!unix) return "";
  try {
    return new Date(unix * 1000).toLocaleString(undefined, {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return "";
  }
}

function TakeArrow({ dir }: { dir: TextSide }): React.JSX.Element {
  return (
    <svg
      width="12"
      height="12"
      viewBox="0 0 12 12"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {dir === "left" ? (
        <>
          <path d="M1.5 2.5 5.5 6 1.5 9.5" />
          <path d="M6.5 2.5 10.5 6 6.5 9.5" />
        </>
      ) : (
        <>
          <path d="M10.5 2.5 6.5 6 10.5 9.5" />
          <path d="M5.5 2.5 1.5 6 5.5 9.5" />
        </>
      )}
    </svg>
  );
}

function renderHighlightedLine(
  text: string,
  spans: readonly InlineSpanDto[],
  side: TextSide,
  inlineBgVar: string,
): React.ReactNode {
  if (!spans || spans.length === 0) return text;
  const chars = Array.from(text);
  const ranges = spans
    .map((s) => ({
      start: Math.max(0, Math.min(chars.length, side === "left" ? s.aStart : s.bStart)),
      end: Math.max(0, Math.min(chars.length, side === "left" ? s.aEnd : s.bEnd)),
    }))
    .filter((r) => r.end > r.start)
    .sort((a, b) => a.start - b.start);

  if (ranges.length === 0) return text;

  const nodes: React.ReactNode[] = [];
  let cursor = 0;
  ranges.forEach((r, idx) => {
    if (r.start > cursor) {
      nodes.push(<span key={`t-${idx}`}>{chars.slice(cursor, r.start).join("")}</span>);
    }
    const segStart = Math.max(cursor, r.start);
    if (r.end > segStart) {
      nodes.push(
        <mark
          key={`h-${idx}`}
          style={{ backgroundColor: inlineBgVar, color: "inherit", borderRadius: 2 }}
        >
          {chars.slice(segStart, r.end).join("")}
        </mark>,
      );
      cursor = r.end;
    }
  });
  if (cursor < chars.length) {
    nodes.push(<span key="tail">{chars.slice(cursor).join("")}</span>);
  }
  return nodes;
}

export default function V2TextCompare({
  leftPath,
  rightPath,
  onPathsChange,
  onDirtyChange,
  onRegisterCommands,
  onStatusChange,
}: V2TextCompareProps): React.JSX.Element {
  const [leftDraft, setLeftDraft] = useState(leftPath);
  const [rightDraft, setRightDraft] = useState(rightPath);
  const [summary, setSummary] = useState<TextCompareDto | null>(null);
  const [rows, setRows] = useState<readonly TextRowDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  // BC view options
  const [filterMode, setFilterMode] = useState<FilterMode>("all");
  const [contextEnabled, setContextEnabled] = useState(true);
  const [minorEnabled, setMinorEnabled] = useState(false);
  const [ignoreWs, setIgnoreWs] = useState(false);
  const [ignoreCase, setIgnoreCase] = useState(false);
  const [ignoreBlank, setIgnoreBlank] = useState(false);
  const [algorithm, setAlgorithm] = useState<"myers" | "patience">("myers");
  const [encoding, setEncoding] = useState("UTF-8");
  const [eol, setEol] = useState("UNIX");

  // Selection, editing, modals, find
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);
  const [editing, setEditing] = useState<{ index: number; side: TextSide; text: string } | null>(
    null,
  );
  const [rulesOpen, setRulesOpen] = useState(false);
  const [formatOpen, setFormatOpen] = useState(false);
  const [findOpen, setFindOpen] = useState(false);
  const [findQuery, setFindQuery] = useState("");
  const [findIdx, setFindIdx] = useState(0);

  const scrollRef = useRef<HTMLDivElement | null>(null);
  const findInputRef = useRef<HTMLInputElement | null>(null);
  const { containerRef, splitPct, dragging, startDrag, resetSplit } = useSplitPane(50);

  useEffect(() => {
    setLeftDraft(leftPath);
    setRightDraft(rightPath);
  }, [leftPath, rightPath]);

  const runCompare = useCallback(
    async (l: string, r: string): Promise<void> => {
      setLoading(true);
      setError(null);
      setNotice(null);
      try {
        let dto: TextCompareDto;
        if (!l && !r) {
          const demo = await textCompareDemo();
          if (!demo) {
            setSummary(null);
            setRows([]);
            setLoading(false);
            return;
          }
          dto = demo.summary;
          setLeftDraft(demo.left);
          setRightDraft(demo.right);
        } else {
          dto = await textCompare(l, r, {
            ignoreWhitespace: ignoreWs,
            ignoreCase,
            ignoreBlankLines: ignoreBlank,
            minorUnimportant: minorEnabled,
            algorithm,
            encoding,
            eol,
          });
        }
        setSummary(dto);
        const page = await textGetRows(dto.session, 0, 400);
        setRows(page.rows);
        if (dto.encoding) setEncoding(dto.encoding);
        if (dto.eol) setEol(dto.eol);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setLoading(false);
      }
    },
    [ignoreWs, ignoreCase, ignoreBlank, minorEnabled, algorithm, encoding, eol],
  );

  useEffect(() => {
    void runCompare(leftPath, rightPath);
  }, [leftPath, rightPath, runCompare]);

  // Global Cmd+F
  useEffect(() => {
    const onKey = (e: KeyboardEvent): void => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "f") {
        e.preventDefault();
        setFindOpen(true);
        setTimeout(() => findInputRef.current?.focus(), 20);
      } else if (e.key === "Escape" && findOpen) {
        setFindOpen(false);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [findOpen]);

  const hunks = useMemo(() => collectHunks(rows), [rows]);

  const hunkMetaByIndex = useMemo(() => {
    const map = new Map<number, { ordinal: number; role: HunkRole; hunk: TextHunk }>();
    for (const h of hunks) {
      const members = rows.filter(
        (r) => r.index >= h.start && r.index <= h.end && r.kind !== "equal",
      );
      members.forEach((m, i) => {
        const role: HunkRole =
          members.length === 1
            ? "single"
            : i === 0
              ? "start"
              : i === members.length - 1
                ? "end"
                : "middle";
        map.set(m.index, { ordinal: h.ordinal, role, hunk: h });
      });
    }
    return map;
  }, [hunks, rows]);

  const lineNosByIndex = useMemo(() => {
    let l = 0;
    let r = 0;
    const map = new Map<number, { left: number | null; right: number | null }>();
    for (const row of rows) {
      map.set(row.index, {
        left: row.left !== null ? ++l : null,
        right: row.right !== null ? ++r : null,
      });
    }
    return map;
  }, [rows]);

  const visibleRows = useMemo(() => {
    if (filterMode === "all") return rows;
    if (filterMode === "same") return rows.filter((r) => r.kind === "equal");
    // filterMode === "diffs"
    if (!contextEnabled) return rows.filter((r) => r.kind !== "equal");
    const keep = new Set<number>();
    rows.forEach((r, idx) => {
      if (r.kind === "equal") return;
      for (let d = -3; d <= 3; d += 1) {
        const nb = rows[idx + d];
        if (nb) keep.add(nb.index);
      }
    });
    return rows.filter((r) => keep.has(r.index));
  }, [rows, filterMode, contextEnabled]);

  const findHits = useMemo(() => {
    const q = findQuery.trim().toLowerCase();
    if (!q) return [] as number[];
    return visibleRows
      .filter(
        (r) =>
          (r.left ?? "").toLowerCase().includes(q) || (r.right ?? "").toLowerCase().includes(q),
      )
      .map((r) => r.index);
  }, [findQuery, visibleRows]);

  const scrollToRow = useCallback((rowIndex: number): void => {
    setSelectedIndex(rowIndex);
    const el = scrollRef.current?.querySelector(`[data-row-index="${rowIndex}"]`);
    if (el instanceof HTMLElement) {
      el.scrollIntoView({ block: "center", behavior: "smooth" });
    }
  }, []);

  const stepSection = useCallback(
    (delta: 1 | -1): void => {
      if (hunks.length === 0) return;
      if (selectedIndex === null) {
        scrollToRow(delta === 1 ? hunks[0].start : hunks[hunks.length - 1].start);
        return;
      }
      if (delta === 1) {
        const next = hunks.find((h) => h.start > selectedIndex) ?? hunks[0];
        scrollToRow(next.start);
      } else {
        const prev = [...hunks].reverse().find((h) => h.end < selectedIndex) ?? hunks[hunks.length - 1];
        scrollToRow(prev.start);
      }
    },
    [hunks, selectedIndex, scrollToRow],
  );

  const handleCopyHunk = useCallback(
    async (hunk: TextHunk, dir: TextSide): Promise<void> => {
      if (!summary) return;
      try {
        await textApplyHunk(summary.session, hunk.start, hunk.end, dir);
        const page = await textGetRows(summary.session, 0, Math.max(400, rows.length));
        setRows(page.rows);
        onDirtyChange(true);
        setNotice(`Copied section ${hunk.ordinal + 1} (${dir === "left" ? "L → R" : "R → L"})`);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    },
    [summary, rows.length, onDirtyChange],
  );

  const handleCommitEdit = async (): Promise<void> => {
    if (!editing || !summary) return;
    try {
      await textApplyEdit(summary.session, editing.index, editing.side, editing.text);
      const page = await textGetRows(summary.session, 0, Math.max(400, rows.length));
      setRows(page.rows);
      onDirtyChange(true);
      setEditing(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const handleSaveSide = async (side: TextSide): Promise<void> => {
    if (!summary) return;
    const targetPath = side === "left" ? leftDraft : rightDraft;
    if (!targetPath) {
      setNotice(`No file path set for ${side} side.`);
      return;
    }
    try {
      const saved = await textSaveSide(summary.session, side, targetPath);
      onDirtyChange(false);
      setNotice(`Saved ${side} (${saved.lines} lines, ${formatBytes(saved.bytes)})`);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const handleBrowse = async (side: TextSide): Promise<void> => {
    try {
      const mod = await import("@tauri-apps/plugin-dialog");
      const picked = await mod.open({ multiple: false, directory: false });
      if (typeof picked === "string" && picked) {
        const nextL = side === "left" ? picked : leftDraft;
        const nextR = side === "right" ? picked : rightDraft;
        setLeftDraft(nextL);
        setRightDraft(nextR);
        onPathsChange(nextL, nextR);
      }
    } catch {
      setNotice("Native file picker requires the Tauri desktop app — type a path and press Enter.");
    }
  };

  const selectedRow = useMemo(
    () => (selectedIndex === null ? null : (rows.find((r) => r.index === selectedIndex) ?? null)),
    [rows, selectedIndex],
  );

  const selectedHunk = useMemo(() => {
    if (selectedIndex === null) return null;
    return hunkMetaByIndex.get(selectedIndex)?.hunk ?? null;
  }, [selectedIndex, hunkMetaByIndex]);

  // Register unified top toolbar commands
  useEffect(() => {
    onRegisterCommands({
      filterMode,
      setFilterMode,
      contextEnabled,
      toggleContext: () => setContextEnabled((v) => !v),
      minorEnabled,
      toggleMinor: () => setMinorEnabled((v) => !v),
      openRules: () => setRulesOpen(true),
      openFormat: () => setFormatOpen(true),
      openFind: () => {
        setFindOpen(true);
        setTimeout(() => findInputRef.current?.focus(), 20);
      },
      canCopy: selectedHunk !== null,
      copySelected: () => {
        if (selectedHunk) void handleCopyHunk(selectedHunk, "left");
      },
      hasSections: hunks.length > 0,
      prevSection: () => stepSection(-1),
      nextSection: () => stepSection(1),
      swapSides: () => {
        onPathsChange(rightDraft, leftDraft);
      },
      reload: () => void runCompare(leftDraft, rightDraft),
    });
  }, [
    onRegisterCommands,
    filterMode,
    contextEnabled,
    minorEnabled,
    selectedHunk,
    handleCopyHunk,
    hunks.length,
    stepSection,
    onPathsChange,
    leftDraft,
    rightDraft,
    runCompare,
  ]);

  // Publish clean 3-zone status bar
  useEffect(() => {
    onStatusChange({
      left:
        hunks.length === 0
          ? "Files are identical"
          : `${hunks.length} difference ${hunks.length === 1 ? "section" : "sections"}`,
      center: notice ?? `${algorithm === "patience" ? "Patience" : "Myers"} · ${encoding} · ${eol}`,
      right: summary
        ? `${summary.leftLines} / ${summary.rightLines} lines`
        : `${rows.length} rows`,
    });
  }, [onStatusChange, hunks.length, notice, algorithm, encoding, eol, summary, rows.length]);

  const submitPaths = (e: React.FormEvent): void => {
    e.preventDefault();
    onPathsChange(leftDraft.trim(), rightDraft.trim());
  };

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden bg-[var(--v2-surface)]">
      {/* Per-pane 50/50 Path + Metadata Header (Beyond Compare alignment) */}
      <form
        onSubmit={submitPaths}
        style={{ gridTemplateColumns: `${splitPct}fr ${100 - splitPct}fr` }}
        className="grid shrink-0 border-b border-[var(--v2-border)] bg-[var(--v2-surface-subtle)]"
      >
        {(["left", "right"] as const).map((side) => {
          const val = side === "left" ? leftDraft : rightDraft;
          const setVal = side === "left" ? setLeftDraft : setRightDraft;
          const meta = side === "left" ? summary?.leftMeta : summary?.rightMeta;
          const lineCount = side === "left" ? summary?.leftLines : summary?.rightLines;
          return (
            <div
              key={side}
              className={`flex flex-col gap-1 px-2.5 py-1.5 ${
                side === "left" ? "border-r border-[var(--v2-border)]" : ""
              }`}
            >
              <div className="flex items-center gap-1.5">
                <input
                  type="text"
                  value={val}
                  onChange={(e) => setVal(e.target.value)}
                  placeholder={side === "left" ? "Left file path…" : "Right file path…"}
                  className="v2-input v2-mono min-w-0 flex-1 text-[11.5px]"
                />
                <button
                  type="button"
                  title={`Browse ${side} file`}
                  onClick={() => void handleBrowse(side)}
                  className="v2-btn px-2"
                >
                  <FolderOpen size={13} />
                </button>
                <button
                  type="button"
                  title={`Save ${side} file`}
                  onClick={() => void handleSaveSide(side)}
                  className="v2-btn px-2"
                >
                  <Save size={13} />
                </button>
              </div>
              <div className="flex items-center gap-3 text-[11px] text-[var(--v2-fg-muted)]">
                {meta?.mtimeUnix ? <span>{formatMtime(meta.mtimeUnix)}</span> : null}
                {meta?.bytes !== undefined ? <span>{formatBytes(meta.bytes)}</span> : null}
                {lineCount !== undefined ? <span>{lineCount} lines</span> : null}
                <span>{encoding}</span>
                <span>{eol}</span>
              </div>
            </div>
          );
        })}
      </form>

      {/* Docked Cmd+F Find Bar */}
      {findOpen ? (
        <div className="flex shrink-0 items-center gap-2 border-b border-[var(--v2-border)] bg-[var(--v2-chrome)] px-3 py-1.5">
          <Search size={13} className="text-[var(--v2-fg-muted)]" />
          <input
            ref={findInputRef}
            type="text"
            value={findQuery}
            onChange={(e) => {
              setFindQuery(e.target.value);
              setFindIdx(0);
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" && findHits.length > 0) {
                const next = (findIdx + (e.shiftKey ? -1 : 1) + findHits.length) % findHits.length;
                setFindIdx(next);
                scrollToRow(findHits[next]);
              }
            }}
            placeholder="Find in diff…"
            className="v2-input v2-mono w-64"
          />
          <button
            type="button"
            disabled={findHits.length === 0}
            onClick={() => {
              const prev = (findIdx - 1 + findHits.length) % findHits.length;
              setFindIdx(prev);
              scrollToRow(findHits[prev]);
            }}
            className="v2-btn"
          >
            Prev
          </button>
          <button
            type="button"
            disabled={findHits.length === 0}
            onClick={() => {
              const next = (findIdx + 1) % findHits.length;
              setFindIdx(next);
              scrollToRow(findHits[next]);
            }}
            className="v2-btn"
          >
            Next
          </button>
          <span className="text-[11px] text-[var(--v2-fg-muted)]">
            {findQuery.trim()
              ? findHits.length === 0
                ? "No matches"
                : `${findIdx + 1} of ${findHits.length}`
              : "Press Enter to jump"}
          </span>
          <button
            type="button"
            aria-label="Close find"
            onClick={() => setFindOpen(false)}
            className="ml-auto inline-flex h-5 w-5 items-center justify-center rounded text-[var(--v2-fg-muted)] hover:bg-[var(--v2-btn-hover)]"
          >
            <X size={13} />
          </button>
        </div>
      ) : null}

      {/* Inline Error Notice */}
      {error ? (
        <div className="flex shrink-0 items-center justify-between border-b border-red-300 bg-red-50 px-3 py-1.5 text-[12px] text-red-700 dark:border-red-900 dark:bg-red-950/60 dark:text-red-300">
          <span>{error}</span>
          <button type="button" onClick={() => setError(null)} className="text-[11px] underline">
            Dismiss
          </button>
        </div>
      ) : null}

      {/* Main Workspace: Far-Left Overview Minimap + 7-Column BC Diff Grid */}
      <div className="flex min-h-0 flex-1 overflow-hidden">
        {/* Far-Left Overview Minimap (BC signature) */}
        <div
          aria-label="Difference overview map"
          className="relative w-[14px] shrink-0 cursor-pointer border-r border-[var(--v2-border)] bg-[var(--v2-surface-subtle)]"
          onClick={(e) => {
            if (rows.length === 0) return;
            const rect = e.currentTarget.getBoundingClientRect();
            const ratio = Math.max(0, Math.min(1, (e.clientY - rect.top) / rect.height));
            const idx = Math.min(rows.length - 1, Math.floor(ratio * rows.length));
            scrollToRow(rows[idx].index);
          }}
        >
          {rows.length > 0 &&
            hunks.map((h) => {
              const firstRow = rows.find((r) => r.index === h.start);
              const topPct = (h.start / rows.length) * 100;
              const heightPct = Math.max(1.2, ((h.end - h.start + 1) / rows.length) * 100);
              const color =
                firstRow?.kind === "leftOnly"
                  ? "var(--v2-ruler-del)"
                  : firstRow?.kind === "rightOnly"
                    ? "var(--v2-ruler-ins)"
                    : "var(--v2-ruler-chg)";
              return (
                <div
                  key={h.ordinal}
                  style={{
                    position: "absolute",
                    left: 2,
                    right: 2,
                    top: `${topPct}%`,
                    height: `${heightPct}%`,
                    backgroundColor: color,
                    borderRadius: 1,
                  }}
                />
              );
            })}
        </div>

        {/* Scrollable Synchronized Diff Panes */}
        <div
          ref={(el) => {
            scrollRef.current = el;
            (containerRef as React.MutableRefObject<HTMLDivElement | null>).current = el;
          }}
          className="min-h-0 flex-1 overflow-auto bg-[var(--v2-surface)]"
        >
          {loading ? (
            <div className="flex h-full items-center justify-center text-[12px] text-[var(--v2-fg-muted)]">
              Comparing files…
            </div>
          ) : visibleRows.length === 0 ? (
            <div className="flex h-full flex-col items-center justify-center gap-2 text-center">
              <p className="text-[13px] font-medium text-[var(--v2-fg-secondary)]">
                {rows.length === 0 ? "No files loaded" : "No rows match the current filter"}
              </p>
              {rows.length > 0 ? (
                <button type="button" onClick={() => setFilterMode("all")} className="v2-btn">
                  Show All Rows
                </button>
              ) : null}
            </div>
          ) : (
            <div className="v2-mono min-w-full text-[12px] leading-[21px]">
              {visibleRows.map((row) => {
                const meta = hunkMetaByIndex.get(row.index);
                const nos = lineNosByIndex.get(row.index) ?? { left: null, right: null };
                const isSelected = selectedIndex === row.index;
                const showArrow = meta && (meta.role === "single" || meta.role === "start");
                const railClass =
                  !meta || meta.role === "single"
                    ? ""
                    : meta.role === "start"
                      ? "v2-rail-start"
                      : meta.role === "middle"
                        ? "v2-rail-middle"
                        : "v2-rail-end";

                const leftBg = isSelected
                  ? "var(--v2-sel-bg)"
                  : row.left === null
                    ? undefined
                    : row.kind === "leftOnly"
                      ? "var(--v2-del-bg)"
                      : row.kind === "changed"
                        ? "var(--v2-chg-bg)"
                        : undefined;
                const leftFg = isSelected
                  ? "var(--v2-sel-fg)"
                  : row.kind === "leftOnly"
                    ? "var(--v2-del-fg)"
                    : row.kind === "changed"
                      ? "var(--v2-chg-fg)"
                      : "var(--v2-fg)";

                const rightBg = isSelected
                  ? "var(--v2-sel-bg)"
                  : row.right === null
                    ? undefined
                    : row.kind === "rightOnly"
                      ? "var(--v2-ins-bg)"
                      : row.kind === "changed"
                        ? "var(--v2-chg-bg)"
                        : undefined;
                const rightFg = isSelected
                  ? "var(--v2-sel-fg)"
                  : row.kind === "rightOnly"
                    ? "var(--v2-ins-fg)"
                    : row.kind === "changed"
                      ? "var(--v2-chg-fg)"
                      : "var(--v2-fg)";

                return (
                  <div
                    key={row.index}
                    data-row-index={row.index}
                    onClick={() => setSelectedIndex(row.index)}
                    style={{
                      gridTemplateColumns: `22px 42px minmax(0, ${splitPct}fr) 6px 22px 42px minmax(0, ${100 - splitPct}fr)`,
                    }}
                    className="grid h-[21px] items-stretch"
                  >
                    {/* Left Action Gutter */}
                    <span
                      className={`relative flex items-center justify-center bg-[var(--v2-surface-subtle)] ${railClass}`}
                    >
                      {showArrow && meta ? (
                        <button
                          type="button"
                          title="Copy section Left → Right"
                          onClick={(e) => {
                            e.stopPropagation();
                            void handleCopyHunk(meta.hunk, "left");
                          }}
                          className="relative z-10 inline-flex h-[15px] w-[15px] items-center justify-center rounded text-[var(--v2-amber)] hover:bg-[var(--v2-btn-hover)]"
                        >
                          <TakeArrow dir="left" />
                        </button>
                      ) : null}
                    </span>

                    {/* Left Line Number */}
                    <span className="select-none border-r border-[var(--v2-border-subtle)] bg-[var(--v2-surface-subtle)] pr-2 text-right text-[11px] text-[var(--v2-fg-muted)]">
                      {nos.left ?? ""}
                    </span>

                    {/* Left Code Cell */}
                    <div
                      onDoubleClick={() => {
                        if (row.left !== null) {
                          setEditing({ index: row.index, side: "left", text: row.left });
                        }
                      }}
                      style={{ backgroundColor: leftBg, color: leftFg }}
                      className={`truncate whitespace-pre px-2 ${
                        row.left === null && !isSelected ? "v2-hatch" : ""
                      }`}
                    >
                      {editing?.index === row.index && editing.side === "left" ? (
                        <input
                          autoFocus
                          value={editing.text}
                          onChange={(e) => setEditing({ ...editing, text: e.target.value })}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") void handleCommitEdit();
                            if (e.key === "Escape") setEditing(null);
                          }}
                          onBlur={() => setEditing(null)}
                          className="h-[19px] w-full bg-[var(--v2-surface)] px-1 text-[var(--v2-fg)] outline outline-1 outline-[var(--v2-accent)]"
                        />
                      ) : row.left !== null ? (
                        renderHighlightedLine(row.left, row.inline, "left", "var(--v2-chg-inline)")
                      ) : (
                        ""
                      )}
                    </div>

                    {/* Draggable Center Splitter (double-click resets to 50/50) */}
                    <span
                      role="separator"
                      aria-orientation="vertical"
                      title="Drag to resize panes (double-click to reset 50/50)"
                      onMouseDown={startDrag}
                      onDoubleClick={resetSplit}
                      className="group relative cursor-col-resize bg-[var(--v2-surface-subtle)]"
                    >
                      <span
                        className={`absolute inset-y-0 left-1/2 -translate-x-1/2 transition-colors ${
                          dragging
                            ? "w-[2px] bg-[var(--v2-accent)]"
                            : "w-px bg-[var(--v2-splitter)] group-hover:w-[2px] group-hover:bg-[var(--v2-accent)]"
                        }`}
                      />
                    </span>

                    {/* Right Action Gutter */}
                    <span
                      className={`relative flex items-center justify-center bg-[var(--v2-surface-subtle)] ${railClass}`}
                    >
                      {showArrow && meta ? (
                        <button
                          type="button"
                          title="Copy section Right → Left"
                          onClick={(e) => {
                            e.stopPropagation();
                            void handleCopyHunk(meta.hunk, "right");
                          }}
                          className="relative z-10 inline-flex h-[15px] w-[15px] items-center justify-center rounded text-[var(--v2-amber)] hover:bg-[var(--v2-btn-hover)]"
                        >
                          <TakeArrow dir="right" />
                        </button>
                      ) : null}
                    </span>

                    {/* Right Line Number (symmetric border-r between number & code!) */}
                    <span className="select-none border-r border-[var(--v2-border-subtle)] bg-[var(--v2-surface-subtle)] pr-2 text-right text-[11px] text-[var(--v2-fg-muted)]">
                      {nos.right ?? ""}
                    </span>

                    {/* Right Code Cell */}
                    <div
                      onDoubleClick={() => {
                        if (row.right !== null) {
                          setEditing({ index: row.index, side: "right", text: row.right });
                        }
                      }}
                      style={{ backgroundColor: rightBg, color: rightFg }}
                      className={`truncate whitespace-pre px-2 ${
                        row.right === null && !isSelected ? "v2-hatch" : ""
                      }`}
                    >
                      {editing?.index === row.index && editing.side === "right" ? (
                        <input
                          autoFocus
                          value={editing.text}
                          onChange={(e) => setEditing({ ...editing, text: e.target.value })}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") void handleCommitEdit();
                            if (e.key === "Escape") setEditing(null);
                          }}
                          onBlur={() => setEditing(null)}
                          className="h-[19px] w-full bg-[var(--v2-surface)] px-1 text-[var(--v2-fg)] outline outline-1 outline-[var(--v2-accent)]"
                        />
                      ) : row.right !== null ? (
                        renderHighlightedLine(row.right, row.inline, "right", "var(--v2-chg-inline)")
                      ) : (
                        ""
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>

      {/* Bottom Stacked Line Detail Pane (Beyond Compare Signature) */}
      <div className="v2-mono shrink-0 border-t border-[var(--v2-border)] bg-[var(--v2-surface-subtle)] text-[11.5px]">
        <div className="grid grid-cols-[26px_1fr] border-b border-[var(--v2-border-subtle)] leading-[21px]">
          <span className="flex items-center justify-center border-r border-[var(--v2-border-subtle)] text-[10px] font-semibold text-[var(--v2-fg-muted)]">
            L
          </span>
          <div
            className={`truncate whitespace-pre px-2 ${
              selectedRow && selectedRow.left === null ? "v2-hatch" : ""
            }`}
          >
            {selectedRow
              ? selectedRow.left !== null
                ? renderHighlightedLine(
                    selectedRow.left,
                    selectedRow.inline,
                    "left",
                    "var(--v2-chg-inline)",
                  )
                : ""
              : "Click any line to inspect character-level differences"}
          </div>
        </div>
        <div className="grid grid-cols-[26px_1fr] leading-[21px]">
          <span className="flex items-center justify-center border-r border-[var(--v2-border-subtle)] text-[10px] font-semibold text-[var(--v2-fg-muted)]">
            R
          </span>
          <div
            className={`truncate whitespace-pre px-2 ${
              selectedRow && selectedRow.right === null ? "v2-hatch" : ""
            }`}
          >
            {selectedRow && selectedRow.right !== null
              ? renderHighlightedLine(
                  selectedRow.right,
                  selectedRow.inline,
                  "right",
                  "var(--v2-chg-inline)",
                )
              : ""}
          </div>
        </div>
      </div>

      {/* Centered Rules Modal */}
      <V2Modal
        open={rulesOpen}
        title="Comparison Rules"
        subtitle="Applies directly to the Rust diff engine"
        widthClass="max-w-md"
        onClose={() => setRulesOpen(false)}
        footer={
          <>
            <button type="button" onClick={() => setRulesOpen(false)} className="v2-btn">
              Close
            </button>
            <button
              type="button"
              onClick={() => {
                setRulesOpen(false);
                void runCompare(leftDraft, rightDraft);
              }}
              className="v2-btn v2-btn-primary"
            >
              Apply & Recompare
            </button>
          </>
        }
      >
        <div className="space-y-3 text-[12px]">
          <label className="flex cursor-pointer items-center gap-2">
            <input
              type="checkbox"
              checked={ignoreWs}
              onChange={(e) => setIgnoreWs(e.target.checked)}
              className="accent-[var(--v2-accent)]"
            />
            <span>Ignore whitespace changes</span>
          </label>
          <label className="flex cursor-pointer items-center gap-2">
            <input
              type="checkbox"
              checked={ignoreCase}
              onChange={(e) => setIgnoreCase(e.target.checked)}
              className="accent-[var(--v2-accent)]"
            />
            <span>Ignore character case</span>
          </label>
          <label className="flex cursor-pointer items-center gap-2">
            <input
              type="checkbox"
              checked={ignoreBlank}
              onChange={(e) => setIgnoreBlank(e.target.checked)}
              className="accent-[var(--v2-accent)]"
            />
            <span>Ignore blank lines</span>
          </label>
          <div className="pt-1">
            <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
              Diff Algorithm
            </span>
            <select
              value={algorithm}
              onChange={(e) => setAlgorithm(e.target.value as "myers" | "patience")}
              className="v2-input w-full"
            >
              <option value="myers">Myers (standard)</option>
              <option value="patience">Patience (better block alignment)</option>
            </select>
          </div>
        </div>
      </V2Modal>

      {/* Centered Format Modal */}
      <V2Modal
        open={formatOpen}
        title="File Format & Encoding"
        widthClass="max-w-sm"
        onClose={() => setFormatOpen(false)}
        footer={
          <button
            type="button"
            onClick={() => {
              setFormatOpen(false);
              void runCompare(leftDraft, rightDraft);
            }}
            className="v2-btn v2-btn-primary"
          >
            Apply
          </button>
        }
      >
        <div className="space-y-3 text-[12px]">
          <div>
            <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
              Encoding
            </span>
            <select
              value={encoding}
              onChange={(e) => setEncoding(e.target.value)}
              className="v2-input w-full"
            >
              <option value="UTF-8">Unicode (UTF-8)</option>
              <option value="UTF-16">Unicode (UTF-16)</option>
              <option value="ASCII">ASCII</option>
            </select>
          </div>
          <div>
            <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
              Line Endings (EOL)
            </span>
            <select value={eol} onChange={(e) => setEol(e.target.value)} className="v2-input w-full">
              <option value="UNIX">UNIX (LF)</option>
              <option value="DOS">Windows (CRLF)</option>
              <option value="MIX">Mixed</option>
            </select>
          </div>
        </div>
      </V2Modal>
    </div>
  );
}
