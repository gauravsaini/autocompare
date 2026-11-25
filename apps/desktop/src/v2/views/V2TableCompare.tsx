import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { FolderOpen, KeyRound, Search, X } from "lucide-react";
import { tableCompare, tableFind, tableGetPage } from "../../ipc/table";
import type { TableHit, TablePage, TableSummary } from "../../ipc/table";
import V2Modal from "../components/V2Modal";
import type { FilterMode, V2StatusInfo, V2ViewCommands } from "../types";

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
  const [rulesOpen, setRulesOpen] = useState(false);

  // Selection & Find
  const [selectedRow, setSelectedRow] = useState<number | null>(null);
  const [findOpen, setFindOpen] = useState(false);
  const [findQuery, setFindQuery] = useState("");
  const [findHits, setFindHits] = useState<readonly TableHit[]>([]);
  const [findIdx, setFindIdx] = useState(0);
  const findInputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    setLeftDraft(leftPath);
    setRightDraft(rightPath);
  }, [leftPath, rightPath]);

  const runCompare = useCallback(
    async (l: string, r: string): Promise<void> => {
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
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
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

  const diffList = useMemo(() => Array.from(diffRowIndices).sort((a, b) => a - b), [diffRowIndices]);

  const stepDiff = useCallback(
    (delta: 1 | -1): void => {
      if (diffList.length === 0) return;
      if (selectedRow === null) {
        setSelectedRow(delta === 1 ? diffList[0] : diffList[diffList.length - 1]);
        return;
      }
      if (delta === 1) {
        setSelectedRow(diffList.find((r) => r > selectedRow) ?? diffList[0]);
      } else {
        setSelectedRow([...diffList].reverse().find((r) => r < selectedRow) ?? diffList[diffList.length - 1]);
      }
    },
    [diffList, selectedRow],
  );

  useEffect(() => {
    onRegisterCommands({
      filterMode,
      setFilterMode,
      openRules: () => setRulesOpen(true),
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
      center: byKey ? `Key-aligned (col ${keyColumns.map((k) => k + 1).join(",")})` : "Positional alignment",
      right: `${summary?.leftRows ?? rowCount} / ${summary?.rightRows ?? rowCount} rows · ${colCount} cols`,
    });
  }, [onStatusChange, summary, diffRowIndices.size, byKey, keyColumns, rowCount, colCount]);

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

  const handleBrowse = async (side: "left" | "right"): Promise<void> => {
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
      // Browser fallback
    }
  };

  const toggleKeyCol = (colIdx: number): void => {
    setByKey(true);
    setKeyColumns((prev) =>
      prev.includes(colIdx) ? (prev.length > 1 ? prev.filter((c) => c !== colIdx) : prev) : [...prev, colIdx].sort((a, b) => a - b),
    );
  };

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden bg-[var(--v2-surface)]">
      {/* Dual-Pane Path Bars */}
      <form
        onSubmit={(e) => {
          e.preventDefault();
          onPathsChange(leftDraft.trim(), rightDraft.trim());
        }}
        className="grid shrink-0 grid-cols-2 border-b border-[var(--v2-border)] bg-[var(--v2-surface-subtle)]"
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
                placeholder={side === "left" ? "Left CSV / table path…" : "Right CSV / table path…"}
                className="v2-input v2-mono min-w-0 flex-1 text-[11.5px]"
              />
              <button type="button" onClick={() => void handleBrowse(side)} className="v2-btn px-2" title={`Browse ${side}`}>
                <FolderOpen size={13} />
              </button>
            </div>
          );
        })}
      </form>

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

      {error ? (
        <div className="border-b border-red-300 bg-red-50 px-3 py-1.5 text-[12px] text-red-700">{error}</div>
      ) : null}

      {/* Dual-Pane Table Grid */}
      <div className="min-h-0 flex-1 overflow-auto">
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
            <div className="sticky top-0 z-10 grid grid-cols-[minmax(0,1fr)_5px_minmax(0,1fr)] border-b border-[var(--v2-border)] bg-[var(--v2-chrome)] text-[11px] font-semibold text-[var(--v2-fg-secondary)]">
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
              <div className="bg-[var(--v2-splitter)]/40" />
              <div className="grid" style={{ gridTemplateColumns: `42px repeat(${colCount}, minmax(90px, 1fr))` }}>
                <div className="border-r border-[var(--v2-border-subtle)] px-2 py-1 text-right text-[var(--v2-fg-muted)]">#</div>
                {Array.from({ length: colCount }, (_, c) => (
                  <div key={`rh-${c}`} className="truncate border-r border-[var(--v2-border-subtle)] px-2 py-1">
                    {page.headerR[c] ?? `Col ${c + 1}`}
                  </div>
                ))}
              </div>
            </div>

            {/* Rows */}
            {visibleRowIndices.map((rIdx) => {
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
                  }}
                  className="grid h-[22px] grid-cols-[minmax(0,1fr)_5px_minmax(0,1fr)] border-b border-[var(--v2-border-subtle)]/60 leading-[22px]"
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
                        return (
                          <div
                            key={`lc-${c}`}
                            style={
                              !isSelected && cellDiff
                                ? { backgroundColor: "var(--v2-del-bg)", color: "var(--v2-del-fg)" }
                                : undefined
                            }
                            className="truncate border-r border-[var(--v2-border-subtle)]/60 px-2"
                          >
                            {lv}
                          </div>
                        );
                      })
                    ) : (
                      <div className="v2-hatch" style={{ gridColumn: `span ${colCount}` }} />
                    )}
                  </div>

                  {/* Splitter */}
                  <span className="relative bg-[var(--v2-surface-subtle)]">
                    <span className="absolute inset-y-0 left-1/2 w-px -translate-x-1/2 bg-[var(--v2-splitter)]" />
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
                        return (
                          <div
                            key={`rc-${c}`}
                            style={
                              !isSelected && cellDiff
                                ? { backgroundColor: "var(--v2-ins-bg)", color: "var(--v2-ins-fg)" }
                                : undefined
                            }
                            className="truncate border-r border-[var(--v2-border-subtle)]/60 px-2"
                          >
                            {rv}
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
        )}
      </div>

      {/* Centered Table Rules Modal */}
      <V2Modal
        open={rulesOpen}
        title="Table Comparison Rules"
        widthClass="max-w-sm"
        onClose={() => setRulesOpen(false)}
        footer={
          <>
            <button type="button" onClick={() => setRulesOpen(false)} className="v2-btn">
              Cancel
            </button>
            <button
              type="button"
              onClick={() => {
                setRulesOpen(false);
                void runCompare(leftDraft, rightDraft);
              }}
              className="v2-btn v2-btn-primary"
            >
              Apply
            </button>
          </>
        }
      >
        <div className="space-y-3 text-[12px]">
          <label className="flex cursor-pointer items-center gap-2">
            <input type="checkbox" checked={byKey} onChange={(e) => setByKey(e.target.checked)} className="accent-[var(--v2-accent)]" />
            <span>Align rows by key column(s)</span>
          </label>
          <label className="flex cursor-pointer items-center gap-2">
            <input type="checkbox" checked={ignoreCase} onChange={(e) => setIgnoreCase(e.target.checked)} className="accent-[var(--v2-accent)]" />
            <span>Ignore case in cells</span>
          </label>
          <label className="flex cursor-pointer items-center gap-2">
            <input type="checkbox" checked={ignoreSpace} onChange={(e) => setIgnoreSpace(e.target.checked)} className="accent-[var(--v2-accent)]" />
            <span>Trim whitespace in cells</span>
          </label>
          <div className="grid grid-cols-2 gap-2">
            <div>
              <span className="mb-1 block text-[11px] text-[var(--v2-fg-muted)]">Delimiter</span>
              <input value={delimiter} onChange={(e) => setDelimiter(e.target.value)} maxLength={1} className="v2-input v2-mono w-full" />
            </div>
            <div>
              <span className="mb-1 block text-[11px] text-[var(--v2-fg-muted)]">Numeric tolerance</span>
              <input
                value={numericTolerance}
                onChange={(e) => setNumericTolerance(e.target.value)}
                placeholder="e.g. 0.01"
                className="v2-input v2-mono w-full"
              />
            </div>
          </div>
        </div>
      </V2Modal>
    </div>
  );
}
