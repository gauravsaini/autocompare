import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  textApplyEdit,
  textApplyHunk,
  textCompare,
  textCompareDemo,
  textGetRows,
  textSaveSide,
} from "../../ipc/text";
import type { TextCompareDto, TextRowDto, TextSide } from "../../ipc/text";
import V2ErrorBanner from "../components/V2ErrorBanner";
import V2DualPathBar from "../components/V2DualPathBar";
import V2TextFindBar from "../components/text/V2TextFindBar";
import V2TextDiffRow, {
  renderHighlightedLine,
} from "../components/text/V2TextDiffRow";
import type { EditingCell, HunkMeta, HunkRole, TextHunk } from "../components/text/V2TextDiffRow";
import { V2TextFormatModal, V2TextRulesModal } from "../components/text/V2TextModals";
import { useSplitPane } from "../hooks/useSplitPane";
import { useVirtualRows } from "../hooks/useVirtualRows";
import type { FilterMode, V2StatusInfo, V2ViewCommands } from "../types";

const ROW_HEIGHT = 21;
const PAGE_LIMIT = 500;
const MAX_VIRTUAL_ROWS = 20000;

export interface V2TextCompareProps {
  readonly leftPath: string;
  readonly rightPath: string;
  readonly onPathsChange: (left: string, right: string) => void;
  readonly onDirtyChange: (dirty: boolean) => void;
  readonly onRegisterCommands: (cmds: V2ViewCommands) => void;
  readonly onStatusChange: (status: V2StatusInfo) => void;
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

async function fetchSessionRows(session: string, expectedTotal: number): Promise<TextRowDto[]> {
  const cap = Math.min(Math.max(expectedTotal, PAGE_LIMIT), MAX_VIRTUAL_ROWS);
  const all: TextRowDto[] = [];
  let start = 0;
  while (start < cap) {
    const page = await textGetRows(session, start, PAGE_LIMIT);
    if (page.rows.length === 0) break;
    all.push(...page.rows);
    if (all.length >= page.total || page.rows.length < PAGE_LIMIT) break;
    start += page.rows.length;
  }
  return all;
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

  // Comparison options
  const [filterMode, setFilterMode] = useState<FilterMode>("all");
  const [contextEnabled, setContextEnabled] = useState(true);
  const [minorEnabled, setMinorEnabled] = useState(false);
  const [ignoreWs, setIgnoreWs] = useState(false);
  const [ignoreCase, setIgnoreCase] = useState(false);
  const [ignoreBlank, setIgnoreBlank] = useState(false);
  const [algorithm, setAlgorithm] = useState<"myers" | "patience">("myers");
  const [encoding, setEncoding] = useState("UTF-8");
  const [eol, setEol] = useState("UNIX");
  const encodingRef = useRef(encoding);
  const eolRef = useRef(eol);
  encodingRef.current = encoding;
  eolRef.current = eol;

  // Selection, editing, modals, find
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);
  const [editing, setEditing] = useState<EditingCell | null>(null);
  const [rulesOpen, setRulesOpen] = useState(false);
  const [formatOpen, setFormatOpen] = useState(false);
  const [findOpen, setFindOpen] = useState(false);
  const [findQuery, setFindQuery] = useState("");
  const [findIdx, setFindIdx] = useState(0);

  const scrollRef = useRef<HTMLDivElement | null>(null);
  const findInputRef = useRef<HTMLInputElement | null>(null);
  const undoStackRef = useRef<Array<{ rows: readonly TextRowDto[]; label: string }>>([]);
  const redoStackRef = useRef<Array<{ rows: readonly TextRowDto[]; label: string }>>([]);
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
            encoding: encodingRef.current,
            eol: eolRef.current,
          });
        }
        setSummary(dto);
        const fetched = await fetchSessionRows(dto.session, dto.rows);
        undoStackRef.current = [];
        redoStackRef.current = [];
        setRows(fetched);
        if (dto.encoding && dto.encoding !== encodingRef.current) setEncoding(dto.encoding);
        if (dto.eol && dto.eol !== eolRef.current) setEol(dto.eol);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setLoading(false);
      }
    },
    [ignoreWs, ignoreCase, ignoreBlank, minorEnabled, algorithm],
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
    const map = new Map<number, HunkMeta>();
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

  const { startIndex, endIndex, totalHeight, offsetTop, scrollToPosition } = useVirtualRows({
    count: visibleRows.length,
    rowHeight: ROW_HEIGHT,
    overscan: 16,
    scrollRef,
  });

  const windowedRows = useMemo(
    () => visibleRows.slice(startIndex, endIndex),
    [visibleRows, startIndex, endIndex],
  );

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

  const scrollToRow = useCallback(
    (rowIndex: number): void => {
      setSelectedIndex(rowIndex);
      const pos = visibleRows.findIndex((r) => r.index === rowIndex);
      if (pos >= 0) scrollToPosition(pos);
    },
    [visibleRows, scrollToPosition],
  );

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
        const prev =
          [...hunks].reverse().find((h) => h.end < selectedIndex) ?? hunks[hunks.length - 1];
        scrollToRow(prev.start);
      }
    },
    [hunks, selectedIndex, scrollToRow],
  );

  const pushUndoSnapshot = useCallback((snapshotRows: readonly TextRowDto[], label: string): void => {
    undoStackRef.current = [...undoStackRef.current.slice(-29), { rows: snapshotRows, label }];
    redoStackRef.current = [];
  }, []);

  const handleUndo = useCallback((): void => {
    const prev = undoStackRef.current.pop();
    if (!prev) return;
    redoStackRef.current = [...redoStackRef.current.slice(-29), { rows, label: prev.label }];
    setRows(prev.rows);
    onDirtyChange(undoStackRef.current.length > 0);
    setNotice(`Undid ${prev.label}`);
  }, [rows, onDirtyChange]);

  const handleRedo = useCallback((): void => {
    const next = redoStackRef.current.pop();
    if (!next) return;
    undoStackRef.current = [...undoStackRef.current.slice(-29), { rows, label: next.label }];
    setRows(next.rows);
    onDirtyChange(true);
    setNotice(`Redid ${next.label}`);
  }, [rows, onDirtyChange]);

  useEffect(() => {
    const onUndoKey = (e: KeyboardEvent): void => {
      const mod = e.metaKey || e.ctrlKey;
      if (!mod) return;
      const target = e.target;
      if (target instanceof HTMLElement && (target.tagName === "INPUT" || target.tagName === "TEXTAREA")) {
        return;
      }
      const key = e.key.toLowerCase();
      if (key === "z" && !e.shiftKey) {
        if (undoStackRef.current.length > 0) {
          e.preventDefault();
          handleUndo();
        }
      } else if ((key === "z" && e.shiftKey) || key === "y") {
        if (redoStackRef.current.length > 0) {
          e.preventDefault();
          handleRedo();
        }
      }
    };
    window.addEventListener("keydown", onUndoKey);
    return () => window.removeEventListener("keydown", onUndoKey);
  }, [handleUndo, handleRedo]);

  const handleCopyHunk = useCallback(
    async (hunk: TextHunk, dir: TextSide): Promise<void> => {
      if (!summary) return;
      const label = `copy section ${hunk.ordinal + 1} (${dir === "left" ? "L → R" : "R → L"})`;
      const snapshot = rows;
      try {
        await textApplyHunk(summary.session, hunk.start, hunk.end, dir);
        const fetched = await fetchSessionRows(summary.session, Math.max(rows.length, summary.rows));
        pushUndoSnapshot(snapshot, label);
        setRows(fetched);
        onDirtyChange(true);
        setNotice(`Copied section ${hunk.ordinal + 1} (${dir === "left" ? "L → R" : "R → L"})`);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    },
    [summary, rows, onDirtyChange, pushUndoSnapshot],
  );

  const handleCommitEdit = useCallback(async (): Promise<void> => {
    if (!editing || !summary) return;
    const label = `edit line ${editing.index + 1} (${editing.side})`;
    const snapshot = rows;
    try {
      await textApplyEdit(summary.session, editing.index, editing.side, editing.text);
      const fetched = await fetchSessionRows(summary.session, Math.max(rows.length, summary.rows));
      pushUndoSnapshot(snapshot, label);
      setRows(fetched);
      onDirtyChange(true);
      setEditing(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [editing, summary, rows, onDirtyChange, pushUndoSnapshot]);

  const handleSaveSide = useCallback(
    async (side: TextSide): Promise<void> => {
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
    },
    [summary, leftDraft, rightDraft, onDirtyChange],
  );

  const handleStartEdit = useCallback((index: number, side: TextSide, text: string): void => {
    setEditing({ index, side, text });
  }, []);

  const handleEditChange = useCallback((text: string): void => {
    setEditing((prev) => (prev ? { ...prev, text } : null));
  }, []);

  const handleCancelEdit = useCallback((): void => setEditing(null), []);

  const handleCopyHunkSync = useCallback(
    (hunk: TextHunk, dir: TextSide): void => {
      void handleCopyHunk(hunk, dir);
    },
    [handleCopyHunk],
  );

  const handleCommitEditSync = useCallback((): void => {
    void handleCommitEdit();
  }, [handleCommitEdit]);

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
      copySelectedRight: () => {
        if (selectedHunk) void handleCopyHunk(selectedHunk, "right");
      },
      hasSections: hunks.length > 0,
      prevSection: () => stepSection(-1),
      nextSection: () => stepSection(1),
      swapSides: () => onPathsChange(rightDraft, leftDraft),
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

  return (
    <div
      data-testid="v2/text-view"
      className="flex h-full min-h-0 flex-col overflow-hidden bg-[var(--v2-surface)]"
    >
      <V2DualPathBar
        leftValue={leftDraft}
        rightValue={rightDraft}
        onLeftChange={setLeftDraft}
        onRightChange={setRightDraft}
        onSubmit={onPathsChange}
        leftPlaceholder="Left file path…"
        rightPlaceholder="Right file path…"
        splitPct={splitPct}
        onSaveSide={(side) => void handleSaveSide(side)}
        onBrowseUnavailable={() =>
          setNotice("Native file picker requires the Tauri desktop app — type a path and press Enter.")
        }
        leftMeta={
          <>
            {summary?.leftMeta?.mtimeUnix ? <span>{formatMtime(summary.leftMeta.mtimeUnix)}</span> : null}
            {summary?.leftMeta?.bytes !== undefined ? <span>{formatBytes(summary.leftMeta.bytes)}</span> : null}
            {summary?.leftLines !== undefined ? <span>{summary.leftLines} lines</span> : null}
            <span>{encoding}</span>
            <span>{eol}</span>
          </>
        }
        rightMeta={
          <>
            {summary?.rightMeta?.mtimeUnix ? <span>{formatMtime(summary.rightMeta.mtimeUnix)}</span> : null}
            {summary?.rightMeta?.bytes !== undefined ? <span>{formatBytes(summary.rightMeta.bytes)}</span> : null}
            {summary?.rightLines !== undefined ? <span>{summary.rightLines} lines</span> : null}
            <span>{encoding}</span>
            <span>{eol}</span>
          </>
        }
      />

      {findOpen ? (
        <V2TextFindBar
          inputRef={findInputRef}
          query={findQuery}
          hitCount={findHits.length}
          activeHit={findIdx}
          onQueryChange={(next) => {
            setFindQuery(next);
            setFindIdx(0);
          }}
          onStep={(delta) => {
            if (findHits.length === 0) return;
            const next = (findIdx + delta + findHits.length) % findHits.length;
            setFindIdx(next);
            scrollToRow(findHits[next]);
          }}
          onClose={() => setFindOpen(false)}
        />
      ) : null}

      <V2ErrorBanner message={error} onDismiss={() => setError(null)} />

      {/* Main Workspace: Far-Left Overview Minimap + Virtualized 7-Column Diff Grid */}
      <div className="flex min-h-0 flex-1 overflow-hidden">
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
            <div
              className="v2-mono relative min-w-full text-[12px] leading-[21px]"
              style={{ height: totalHeight }}
            >
              <div style={{ transform: `translateY(${offsetTop}px)` }}>
                {windowedRows.map((row) => {
                  const nos = lineNosByIndex.get(row.index) ?? { left: null, right: null };
                  const isEditingThis = editing?.index === row.index;
                  return (
                    <V2TextDiffRow
                      key={row.index}
                      row={row}
                      meta={hunkMetaByIndex.get(row.index)}
                      leftLineNo={nos.left}
                      rightLineNo={nos.right}
                      isSelected={selectedIndex === row.index}
                      editingSide={isEditingThis ? editing.side : null}
                      editingText={isEditingThis ? editing.text : ""}
                      splitPct={splitPct}
                      dragging={dragging}
                      onSelect={setSelectedIndex}
                      onCopyHunk={handleCopyHunkSync}
                      onStartEdit={handleStartEdit}
                      onEditChange={handleEditChange}
                      onCommitEdit={handleCommitEditSync}
                      onCancelEdit={handleCancelEdit}
                      onStartDrag={startDrag}
                      onResetSplit={resetSplit}
                    />
                  );
                })}
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Bottom Stacked Line Detail Pane */}
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

      <V2TextRulesModal
        open={rulesOpen}
        ignoreWs={ignoreWs}
        ignoreCase={ignoreCase}
        ignoreBlank={ignoreBlank}
        algorithm={algorithm}
        onIgnoreWsChange={setIgnoreWs}
        onIgnoreCaseChange={setIgnoreCase}
        onIgnoreBlankChange={setIgnoreBlank}
        onAlgorithmChange={setAlgorithm}
        onClose={() => setRulesOpen(false)}
        onApply={() => {
          setRulesOpen(false);
          void runCompare(leftDraft, rightDraft);
        }}
      />

      <V2TextFormatModal
        open={formatOpen}
        encoding={encoding}
        eol={eol}
        onEncodingChange={setEncoding}
        onEolChange={setEol}
        onClose={() => setFormatOpen(false)}
        onApply={() => {
          setFormatOpen(false);
          void runCompare(leftDraft, rightDraft);
        }}
      />
    </div>
  );
}
