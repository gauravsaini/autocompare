import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { KeyRound, Search, X } from "lucide-react";
import { tableApplyEdit, tableCompare, tableFind, tableGetPage, tableSaveSide } from "../../ipc/table";
import type { TableHit, TablePage, TableSummary } from "../../ipc/table";
import { clearActiveTrace, setActiveTrace } from "../../ipc/client";
import { endSpan, isTelemetryEnabled, startRootSpan, type Span } from "../../observability/tracer";
import { emitFileWatchTransition } from "../utils/fileWatch";
import { emitBufferEditTransition, emitManualAlignTransition } from "../utils/textBuffer";
import V2TableRulesModal from "../components/table/V2TableRulesModal";
import type { TableRulesTab } from "../components/table/V2TableRulesModal";
import V2ErrorBanner from "../components/V2ErrorBanner";
import V2ReloadPromptBanner from "../components/V2ReloadPromptBanner";
import V2DualPathBar from "../components/V2DualPathBar";
import { isMissingFixtureError } from "../utils/demoGuard";
import { useSplitPane } from "../hooks/useSplitPane";
import { useVirtualRows } from "../hooks/useVirtualRows";
import { useExternalChangeWatch } from "../hooks/useExternalChangeWatch";
import type { FilterMode, V2StatusInfo, V2ViewCommands } from "../types";

const TABLE_ROW_HEIGHT = 22;

export interface V2TableCompareProps {
  readonly leftPath: string;
  readonly rightPath: string;
  readonly onPathsChange: (left: string, right: string) => void;
  readonly onRegisterCommands: (cmds: V2ViewCommands) => void;
  readonly onStatusChange: (status: V2StatusInfo) => void;
}

export default function V2TableCompare({
  leftPath,
  rightPath,
  onPathsChange,
  onRegisterCommands,
  onStatusChange,
}: V2TableCompareProps): React.JSX.Element {
  const [leftDraft, setLeftDraft] = useState(leftPath);
  const [rightDraft, setRightDraft] = useState(rightPath);
  const [summary, setSummary] = useState<TableSummary | null>(null);
  const [page, setPage] = useState<TablePage | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Options
  const [filterMode, setFilterMode] = useState<FilterMode>("all");
  const [byKey, setByKey] = useState(false);
  const [keyColumns, setKeyColumns] = useState<number[]>([0]);
  const [ignoreCase, setIgnoreCase] = useState(false);
  const [ignoreSpace, setIgnoreSpace] = useState(false);
  const [delimiter, setDelimiter] = useState(",");
  const [numericTolerance, setNumericTolerance] = useState("");
  const [rulesTab, setRulesTab] = useState<TableRulesTab | null>(null);

  // Selection, Inline Cell Editor & Find
  const [selectedRow, setSelectedRow] = useState<number | null>(null);
  const [editingCell, setEditingCell] = useState<{
    readonly side: "left" | "right";
    readonly row: number;
    readonly col: number;
    readonly value: string;
  } | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [findOpen, setFindOpen] = useState(false);
  const [findQuery, setFindQuery] = useState("");
  const [findHits, setFindHits] = useState<readonly TableHit[]>([]);
  const [findIdx, setFindIdx] = useState(0);
  const findInputRef = useRef<HTMLInputElement | null>(null);
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const { containerRef, splitPct, dragging, startDrag, resetSplit } = useSplitPane(50);
  // ODD Phase 4 Leaf A: ambient trace of the last compare, so reload prompts,
  // buffer edits, and manual-align overrides link to the same trace.
  const lastTraceRef = useRef<{ trace_id: string; span_id: string; sampled: boolean } | null>(null);
  const watchedPaths = useMemo(
    () => [leftDraft.trim(), rightDraft.trim()].filter((p) => p.length > 0),
    [leftDraft, rightDraft],
  );
  const { changedPaths, dismiss: dismissReload, resetBaseline } = useExternalChangeWatch({
    paths: watchedPaths,
    enabled: !loading && watchedPaths.length > 0,
    onAutoReload: () => void runCompare(leftDraft, rightDraft),
  });

  // ODD Phase 4 Leaf A: file-watch reload prompts emit explicit
  // state-transition events linked to the ambient (last compare) trace.
  const promptEmittedRef = useRef<string>("");
  useEffect(() => {
    if (changedPaths.length === 0) {
      promptEmittedRef.current = "";
      return;
    }
    const key = changedPaths.join("\0");
    if (promptEmittedRef.current === key) return;
    promptEmittedRef.current = key;
    emitFileWatchTransition("table", "prompt_shown", { paths: changedPaths.length }, lastTraceRef.current);
  }, [changedPaths]);

  useEffect(() => {
    setLeftDraft(leftPath);
    setRightDraft(rightPath);
  }, [leftPath, rightPath]);

  const runCompare = useCallback(
    async (l: string, r: string): Promise<void> => {
      // ODD Phase 4 Leaf A: one root span per compare trigger (op table.compare),
      // following the exact V2TextCompare pattern. Telemetry-off runs identically.
      const obs: Span | null = isTelemetryEnabled()
        ? startRootSpan({ op: "table.compare", component: "ts.ui/TableView", view_id: "table" })
        : null;
      if (obs !== null) {
        setActiveTrace(obs.context());
        lastTraceRef.current = obs.context();
      }
      setLoading(true);
      setError(null);
      const lEff = l || "tools/parity-harness/fixtures/tab_left.csv";
      const rEff = r || "tools/parity-harness/fixtures/tab_right.csv";
      try {
        const tol = numericTolerance.trim() ? Number(numericTolerance) : undefined;
        const dto = await tableCompare(lEff, rEff, byKey, byKey ? keyColumns : [], {
          ignoreCase,
          ignoreSpace,
          delimiter: delimiter || ",",
          numericTolerance: tol !== undefined && Number.isFinite(tol) ? tol : undefined,
        });
        setSummary(dto);
        const pg = await tableGetPage(dto.sessionId, 0, 200, 0, 32);
        setPage(pg);
        if (obs !== null) {
          obs.set({
            rows: Math.max(pg.totalRowsL, pg.totalRowsR),
            cols: pg.totalCols,
            hunks: dto.diffRows,
            equal: dto.equal,
            session: dto.sessionId,
          });
        }
      } catch (e) {
        if (obs !== null) {
          obs.error({
            kind: "table_compare_failed",
            component: "ts.ui/TableView",
            message: e instanceof Error ? e.message : String(e),
          });
        }
        if (!l && !r && isMissingFixtureError(e)) {
          setSummary(null);
          setPage(null);
        } else {
          setError(e instanceof Error ? e.message : String(e));
        }
      } finally {
        if (obs !== null) {
          clearActiveTrace();
          endSpan(obs);
        }
        setLoading(false);
      }
    },
    [byKey, keyColumns, ignoreCase, ignoreSpace, delimiter, numericTolerance],
  );

  useEffect(() => {
    void runCompare(leftPath, rightPath);
  }, [leftPath, rightPath, runCompare]);

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

  const rowCount = useMemo(() => {
    if (!page) return 0;
    return Math.max(page.rowsL.length, page.rowsR.length);
  }, [page]);

  const colCount = useMemo(() => {
    if (!page) return 0;
    return Math.max(page.headerL.length, page.headerR.length, page.totalCols);
  }, [page]);

  const diffRowIndices = useMemo(() => {
    if (!page) return new Set<number>();
    const set = new Set<number>();
    for (let i = 0; i < rowCount; i += 1) {
      const rl = page.rowsL[i] ?? [];
      const rr = page.rowsR[i] ?? [];
      let differs = rl.length !== rr.length;
      if (!differs) {
        for (let c = 0; c < colCount; c += 1) {
          if ((rl[c] ?? "") !== (rr[c] ?? "")) {
            differs = true;
            break;
          }
        }
      }
      if (differs) set.add(i);
    }
    return set;
  }, [page, rowCount, colCount]);

  const visibleRowIndices = useMemo(() => {
    const out: number[] = [];
    for (let i = 0; i < rowCount; i += 1) {
      const isDiff = diffRowIndices.has(i);
      if (filterMode === "all" || (filterMode === "diffs" && isDiff) || (filterMode === "same" && !isDiff)) {
        out.push(i);
      }
    }
    return out;
  }, [rowCount, diffRowIndices, filterMode]);

  const { startIndex, endIndex, totalHeight, offsetTop, scrollToPosition } = useVirtualRows({
    count: visibleRowIndices.length,
    rowHeight: TABLE_ROW_HEIGHT,
    overscan: 16,
    scrollRef,
  });

  const windowedRowIndices = useMemo(
    () => visibleRowIndices.slice(startIndex, endIndex),
    [visibleRowIndices, startIndex, endIndex],
  );

  const selectAndScrollRow = useCallback(
    (rowIdx: number): void => {
      setSelectedRow(rowIdx);
      const pos = visibleRowIndices.indexOf(rowIdx);
      if (pos >= 0) scrollToPosition(pos);
    },
    [visibleRowIndices, scrollToPosition],
  );

  const diffList = useMemo(() => Array.from(diffRowIndices).sort((a, b) => a - b), [diffRowIndices]);

  const stepDiff = useCallback(
    (delta: 1 | -1): void => {
      if (diffList.length === 0) return;
      if (selectedRow === null) {
        selectAndScrollRow(delta === 1 ? diffList[0] : diffList[diffList.length - 1]);
        return;
      }
      if (delta === 1) {
        selectAndScrollRow(diffList.find((r) => r > selectedRow) ?? diffList[0]);
      } else {
        selectAndScrollRow(
          [...diffList].reverse().find((r) => r < selectedRow) ?? diffList[diffList.length - 1],
        );
      }
    },
    [diffList, selectedRow, selectAndScrollRow],
  );

  useEffect(() => {
    onRegisterCommands({
      filterMode,
      setFilterMode,
      openRules: () => setRulesTab("columns"),
      openFormat: () => setRulesTab("format"),
      openFind: () => {
        setFindOpen(true);
        setTimeout(() => findInputRef.current?.focus(), 20);
      },
      hasSections: diffList.length > 0,
      prevSection: () => stepDiff(-1),
      nextSection: () => stepDiff(1),
      swapSides: () => onPathsChange(rightDraft, leftDraft),
      reload: () => void runCompare(leftDraft, rightDraft),
    });
  }, [onRegisterCommands, filterMode, diffList.length, stepDiff, onPathsChange, leftDraft, rightDraft, runCompare]);

  useEffect(() => {
    onStatusChange({
      left: `${summary?.diffRows ?? diffRowIndices.size} difference row(s)`,
      center:
        notice ??
        (byKey ? `Key-aligned (col ${keyColumns.map((k) => k + 1).join(",")})` : "Positional alignment"),
      right: `${summary?.leftRows ?? rowCount} / ${summary?.rightRows ?? rowCount} rows · ${colCount} cols`,
    });
  }, [onStatusChange, summary, diffRowIndices.size, notice, byKey, keyColumns, rowCount, colCount]);

  const commitCellEdit = useCallback(async (): Promise<void> => {
    if (!editingCell || !summary || !page) {
      setEditingCell(null);
      return;
    }
    const { side, row, col, value } = editingCell;
    setEditingCell(null);
    // ODD Phase 4 Leaf A: buffer-edit state-transition event linked to ambient trace.
    emitBufferEditTransition("table.compare", "table", "cell_edit", { side, row, col }, lastTraceRef.current);
    try {
      await tableApplyEdit(summary.sessionId, side, row, col, value);
      setPage((prev) => {
        if (!prev) return prev;
        const updateRows = (src: readonly (readonly string[])[]): string[][] =>
          src.map((r, rIdx) => {
            if (rIdx !== row) return [...r];
            const nextRow = [...r];
            while (nextRow.length <= col) nextRow.push("");
            nextRow[col] = value;
            return nextRow;
          });
        return side === "left"
          ? { ...prev, rowsL: updateRows(prev.rowsL) }
          : { ...prev, rowsR: updateRows(prev.rowsR) };
      });
      setNotice(`Edited ${side} row ${row + 1}, col ${col + 1}`);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [editingCell, summary, page]);

  const handleSaveSide = useCallback(
    async (side: "left" | "right"): Promise<void> => {
      if (!summary) return;
      const targetPath = side === "left" ? leftDraft : rightDraft;
      if (!targetPath.trim()) {
        setNotice(`Enter a ${side} CSV file path before saving.`);
        return;
      }
      try {
        const res = await tableSaveSide(summary.sessionId, side, targetPath.trim());
        setNotice(`Saved ${side} CSV (${res.rowsWritten} rows, ${res.bytesWritten} B)`);
        void resetBaseline();
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    },
    [summary, leftDraft, rightDraft, resetBaseline],
  );

  const handleSearch = async (): Promise<void> => {
    if (!summary || !findQuery.trim()) {
      setFindHits([]);
      return;
    }
    try {
      const hits = await tableFind(summary.sessionId, findQuery.trim());
      setFindHits(hits);
      setFindIdx(0);
      if (hits.length > 0) setSelectedRow(hits[0].row);
    } catch {
      setFindHits([]);
    }
  };

  const toggleKeyCol = (colIdx: number): void => {
    // ODD Phase 4 Leaf A: manual-align override event linked to ambient trace.
    emitManualAlignTransition("table.compare", "table", "key_column_toggle", { col: colIdx }, lastTraceRef.current);
    setByKey(true);
    setKeyColumns((prev) =>
      prev.includes(colIdx) ? (prev.length > 1 ? prev.filter((c) => c !== colIdx) : prev) : [...prev, colIdx].sort((a, b) => a - b),
    );
  };

  return (
    <div data-testid="v2/table-view" className="flex h-full min-h-0 flex-col overflow-hidden bg-[var(--v2-surface)]">
      <V2DualPathBar
        leftValue={leftDraft}
        rightValue={rightDraft}
        onLeftChange={setLeftDraft}
        onRightChange={setRightDraft}
        onSubmit={onPathsChange}
        leftPlaceholder="Left CSV / table path…"
        rightPlaceholder="Right CSV / table path…"
        splitPct={splitPct}
        onSaveSide={(side) => void handleSaveSide(side)}
      />

      {/* Docked Find Bar */}
      {findOpen ? (
        <div className="flex shrink-0 items-center gap-2 border-b border-[var(--v2-border)] bg-[var(--v2-chrome)] px-3 py-1.5">
          <Search size={13} className="text-[var(--v2-fg-muted)]" />
          <input
            ref={findInputRef}
            type="text"
            value={findQuery}
            onChange={(e) => setFindQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void handleSearch();
            }}
            placeholder="Find cell value…"
            className="v2-input v2-mono w-60"
          />
          <button type="button" onClick={() => void handleSearch()} className="v2-btn">
            Find
          </button>
          <button
            type="button"
            disabled={findHits.length === 0}
            onClick={() => {
              const next = (findIdx + 1) % findHits.length;
              setFindIdx(next);
              setSelectedRow(findHits[next].row);
            }}
            className="v2-btn"
          >
            Next
          </button>
          <span className="text-[11px] text-[var(--v2-fg-muted)]">
            {findHits.length > 0 ? `${findIdx + 1} of ${findHits.length} hits` : "Press Enter to search"}
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

      <V2ReloadPromptBanner
        changedPaths={changedPaths}
        onReload={() => {
          emitFileWatchTransition("table", "reload_accepted", { paths: changedPaths.length }, lastTraceRef.current);
          void resetBaseline();
          void runCompare(leftDraft, rightDraft);
        }}
        onDismiss={() => {
          emitFileWatchTransition("table", "reload_dismissed", { paths: changedPaths.length }, lastTraceRef.current);
          dismissReload();
        }}
      />
      <V2ErrorBanner message={error} onDismiss={() => setError(null)} />

      {/* Dual-Pane Table Grid */}
      <div
        ref={(el) => {
          scrollRef.current = el;
          (containerRef as React.MutableRefObject<HTMLDivElement | null>).current = el;
        }}
        className="min-h-0 flex-1 overflow-auto"
      >
        {loading ? (
          <div className="flex h-full items-center justify-center text-[12px] text-[var(--v2-fg-muted)]">
            Comparing tables…
          </div>
        ) : !page || colCount === 0 ? (
          <div className="flex h-full items-center justify-center text-[12px] text-[var(--v2-fg-muted)]">
            No table cells to display.
          </div>
        ) : (
          <div className="v2-mono min-w-full text-[12px]">
            {/* Sticky Headers */}
            <div
              style={{ gridTemplateColumns: `minmax(0, ${splitPct}fr) 6px minmax(0, ${100 - splitPct}fr)` }}
              className="sticky top-0 z-10 grid border-b border-[var(--v2-border)] bg-[var(--v2-chrome)] text-[11px] font-semibold text-[var(--v2-fg-secondary)]"
            >
              <div className="grid" style={{ gridTemplateColumns: `42px repeat(${colCount}, minmax(90px, 1fr))` }}>
                <div className="border-r border-[var(--v2-border-subtle)] px-2 py-1 text-right text-[var(--v2-fg-muted)]">#</div>
                {Array.from({ length: colCount }, (_, c) => {
                  const isKey = byKey && keyColumns.includes(c);
                  return (
                    <button
                      key={`lh-${c}`}
                      type="button"
                      onClick={() => toggleKeyCol(c)}
                      title="Click to toggle key-column alignment"
                      className="flex items-center gap-1 truncate border-r border-[var(--v2-border-subtle)] px-2 py-1 text-left hover:bg-[var(--v2-btn-hover)]"
                    >
                      {isKey ? <KeyRound size={11} className="shrink-0 text-[var(--v2-accent)]" /> : null}
                      <span className="truncate">{page.headerL[c] ?? `Col ${c + 1}`}</span>
                    </button>
                  );
                })}
              </div>
              <div
                role="separator"
                aria-orientation="vertical"
                title="Drag to resize panes (double-click to reset 50/50)"
                onMouseDown={startDrag}
                onDoubleClick={resetSplit}
                className="cursor-col-resize bg-[var(--v2-splitter)]/40 hover:bg-[var(--v2-accent)]/60"
              />
              <div className="grid" style={{ gridTemplateColumns: `42px repeat(${colCount}, minmax(90px, 1fr))` }}>
                <div className="border-r border-[var(--v2-border-subtle)] px-2 py-1 text-right text-[var(--v2-fg-muted)]">#</div>
                {Array.from({ length: colCount }, (_, c) => (
                  <div key={`rh-${c}`} className="truncate border-r border-[var(--v2-border-subtle)] px-2 py-1">
                    {page.headerR[c] ?? `Col ${c + 1}`}
                  </div>
                ))}
              </div>
            </div>

            {/* Virtualized Rows */}
            <div className="relative" style={{ height: totalHeight }}>
              <div style={{ transform: `translateY(${offsetTop}px)` }}>
            {windowedRowIndices.map((rIdx) => {
              const rl = page.rowsL[rIdx];
              const rr = page.rowsR[rIdx];
              const isSelected = selectedRow === rIdx;
              return (
                <div
                  key={rIdx}
                  onClick={() => setSelectedRow(rIdx)}
                  style={{
                    backgroundColor: isSelected ? "var(--v2-sel-bg)" : undefined,
                    color: isSelected ? "var(--v2-sel-fg)" : undefined,
                    gridTemplateColumns: `minmax(0, ${splitPct}fr) 6px minmax(0, ${100 - splitPct}fr)`,
                  }}
                  className="grid h-[22px] border-b border-[var(--v2-border-subtle)]/60 leading-[22px]"
                >
                  {/* Left Row */}
                  <div className="grid" style={{ gridTemplateColumns: `42px repeat(${colCount}, minmax(90px, 1fr))` }}>
                    <span className="select-none border-r border-[var(--v2-border-subtle)] bg-[var(--v2-surface-subtle)] px-2 text-right text-[11px] text-[var(--v2-fg-muted)]">
                      {rl ? rIdx + 1 : ""}
                    </span>
                    {rl ? (
                      Array.from({ length: colCount }, (_, c) => {
                        const lv = rl[c] ?? "";
                        const rv = rr?.[c] ?? "";
                        const cellDiff = !rr || lv !== rv;
                        const isEditing =
                          editingCell?.side === "left" && editingCell.row === rIdx && editingCell.col === c;
                        return (
                          <div
                            key={`lc-${c}`}
                            onDoubleClick={(e) => {
                              e.stopPropagation();
                              setEditingCell({ side: "left", row: rIdx, col: c, value: lv });
                            }}
                            style={
                              !isSelected && cellDiff
                                ? { backgroundColor: "var(--v2-del-bg)", color: "var(--v2-del-fg)" }
                                : undefined
                            }
                            className="cursor-text truncate border-r border-[var(--v2-border-subtle)]/60 px-2"
                          >
                            {isEditing ? (
                              <input
                                autoFocus
                                type="text"
                                value={editingCell.value}
                                onClick={(e) => e.stopPropagation()}
                                onChange={(e) =>
                                  setEditingCell((prev) => (prev ? { ...prev, value: e.target.value } : null))
                                }
                                onKeyDown={(e) => {
                                  if (e.key === "Enter") void commitCellEdit();
                                  else if (e.key === "Escape") setEditingCell(null);
                                }}
                                onBlur={() => void commitCellEdit()}
                                className="v2-mono h-[20px] w-full border-0 bg-transparent p-0 text-[12px] text-inherit outline-none"
                              />
                            ) : (
                              lv
                            )}
                          </div>
                        );
                      })
                    ) : (
                      <div className="v2-hatch" style={{ gridColumn: `span ${colCount}` }} />
                    )}
                  </div>

                  {/* Draggable Splitter */}
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

                  {/* Right Row */}
                  <div className="grid" style={{ gridTemplateColumns: `42px repeat(${colCount}, minmax(90px, 1fr))` }}>
                    <span className="select-none border-r border-[var(--v2-border-subtle)] bg-[var(--v2-surface-subtle)] px-2 text-right text-[11px] text-[var(--v2-fg-muted)]">
                      {rr ? rIdx + 1 : ""}
                    </span>
                    {rr ? (
                      Array.from({ length: colCount }, (_, c) => {
                        const lv = rl?.[c] ?? "";
                        const rv = rr[c] ?? "";
                        const cellDiff = !rl || lv !== rv;
                        const isEditing =
                          editingCell?.side === "right" && editingCell.row === rIdx && editingCell.col === c;
                        return (
                          <div
                            key={`rc-${c}`}
                            onDoubleClick={(e) => {
                              e.stopPropagation();
                              setEditingCell({ side: "right", row: rIdx, col: c, value: rv });
                            }}
                            style={
                              !isSelected && cellDiff
                                ? { backgroundColor: "var(--v2-ins-bg)", color: "var(--v2-ins-fg)" }
                                : undefined
                            }
                            className="cursor-text truncate border-r border-[var(--v2-border-subtle)]/60 px-2"
                          >
                            {isEditing ? (
                              <input
                                autoFocus
                                type="text"
                                value={editingCell.value}
                                onClick={(e) => e.stopPropagation()}
                                onChange={(e) =>
                                  setEditingCell((prev) => (prev ? { ...prev, value: e.target.value } : null))
                                }
                                onKeyDown={(e) => {
                                  if (e.key === "Enter") void commitCellEdit();
                                  else if (e.key === "Escape") setEditingCell(null);
                                }}
                                onBlur={() => void commitCellEdit()}
                                className="v2-mono h-[20px] w-full border-0 bg-transparent p-0 text-[12px] text-inherit outline-none"
                              />
                            ) : (
                              rv
                            )}
                          </div>
                        );
                      })
                    ) : (
                      <div className="v2-hatch" style={{ gridColumn: `span ${colCount}` }} />
                    )}
                  </div>
                </div>
              );
            })}
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Tabbed BC-Style Table Rules Modal */}
      <V2TableRulesModal
        open={rulesTab !== null}
        initialTab={rulesTab ?? "columns"}
        columnNames={Array.from({ length: colCount }, (_, c) => page?.headerL[c] ?? `Col ${c + 1}`)}
        rules={{
          byKey,
          keyColumns,
          delimiter,
          ignoreCase,
          ignoreSpace,
          numericTolerance,
        }}
        onClose={() => setRulesTab(null)}
        onApply={(next) => {
          setByKey(next.byKey);
          setKeyColumns([...next.keyColumns]);
          setDelimiter(next.delimiter || ",");
          setIgnoreCase(next.ignoreCase);
          setIgnoreSpace(next.ignoreSpace);
          setNumericTolerance(next.numericTolerance);
          setRulesTab(null);
        }}
      />
    </div>
  );
}
