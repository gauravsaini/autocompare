import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Search, X } from "lucide-react";
import { hexCompare, hexFind, hexGetPage } from "../../ipc/hex";
import type { HexPage, HexSummary } from "../../ipc/hex";
import V2ErrorBanner from "../components/V2ErrorBanner";
import V2DualPathBar from "../components/V2DualPathBar";
import { isMissingFixtureError } from "../utils/demoGuard";
import { useSplitPane } from "../hooks/useSplitPane";
import type { FilterMode, V2StatusInfo, V2ViewCommands } from "../types";

export interface V2HexCompareProps {
  readonly leftPath: string;
  readonly rightPath: string;
  readonly onPathsChange: (left: string, right: string) => void;
  readonly onRegisterCommands: (cmds: V2ViewCommands) => void;
  readonly onStatusChange: (status: V2StatusInfo) => void;
}

const BYTES_PER_ROW = 16;
const PAGE_BYTES = 2048;

function hex2(b: number): string {
  return b.toString(16).padStart(2, "0").toUpperCase();
}

function asciiChar(b: number): string {
  return b >= 0x20 && b <= 0x7e ? String.fromCharCode(b) : "·";
}

export default function V2HexCompare({
  leftPath,
  rightPath,
  onPathsChange,
  onRegisterCommands,
  onStatusChange,
}: V2HexCompareProps): React.JSX.Element {
  const [leftDraft, setLeftDraft] = useState(leftPath);
  const [rightDraft, setRightDraft] = useState(rightPath);
  const [summary, setSummary] = useState<HexSummary | null>(null);
  const [page, setPage] = useState<HexPage | null>(null);
  const [offset, setOffset] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filterMode, setFilterMode] = useState<FilterMode>("all");
  const [findOpen, setFindOpen] = useState(false);
  const [findHex, setFindHex] = useState("");
  const [findNote, setFindNote] = useState<string | null>(null);
  const findInputRef = useRef<HTMLInputElement | null>(null);
  const { containerRef, splitPct, dragging, startDrag, resetSplit } = useSplitPane(50);

  useEffect(() => {
    setLeftDraft(leftPath);
    setRightDraft(rightPath);
  }, [leftPath, rightPath]);

  const loadPage = useCallback(async (sessionId: string, off: number): Promise<void> => {
    const pg = await hexGetPage(sessionId, off, PAGE_BYTES);
    setPage(pg);
    setOffset(pg.offset);
  }, []);

  const runCompare = useCallback(
    async (l: string, r: string): Promise<void> => {
      setLoading(true);
      setError(null);
      const lEff = l || "tools/parity-harness/fixtures/t_left.txt";
      const rEff = r || "tools/parity-harness/fixtures/t_right.txt";
      try {
        const dto = await hexCompare(lEff, rEff);
        setSummary(dto);
        await loadPage(dto.sessionId, 0);
      } catch (e) {
        if (!l && !r && isMissingFixtureError(e)) {
          setSummary(null);
          setPage(null);
        } else {
          setError(e instanceof Error ? e.message : String(e));
        }
      } finally {
        setLoading(false);
      }
    },
    [loadPage],
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

  const rows = useMemo(() => {
    if (!page) return [];
    const maxLen = Math.max(page.left.length, page.right.length);
    const out: { rowOffset: number; left: (number | null)[]; right: (number | null)[]; differs: boolean }[] = [];
    for (let i = 0; i < maxLen; i += BYTES_PER_ROW) {
      const left: (number | null)[] = [];
      const right: (number | null)[] = [];
      let differs = false;
      for (let j = 0; j < BYTES_PER_ROW; j += 1) {
        const lb = i + j < page.left.length ? page.left[i + j] : null;
        const rb = i + j < page.right.length ? page.right[i + j] : null;
        if (lb !== rb) differs = true;
        left.push(lb);
        right.push(rb);
      }
      out.push({ rowOffset: page.offset + i, left, right, differs });
    }
    return out;
  }, [page]);

  const visibleRows = useMemo(() => {
    if (filterMode === "all") return rows;
    if (filterMode === "diffs") return rows.filter((r) => r.differs);
    return rows.filter((r) => !r.differs);
  }, [rows, filterMode]);

  useEffect(() => {
    onRegisterCommands({
      filterMode,
      setFilterMode,
      openFind: () => {
        setFindOpen(true);
        setTimeout(() => findInputRef.current?.focus(), 20);
      },
      hasSections: summary?.firstDiff !== null && summary?.firstDiff !== undefined,
      nextSection: () => {
        if (summary && summary.firstDiff !== null) {
          void loadPage(summary.sessionId, Math.floor(summary.firstDiff / BYTES_PER_ROW) * BYTES_PER_ROW);
        }
      },
      swapSides: () => onPathsChange(rightDraft, leftDraft),
      reload: () => void runCompare(leftDraft, rightDraft),
    });
  }, [onRegisterCommands, filterMode, summary, loadPage, onPathsChange, leftDraft, rightDraft, runCompare]);

  useEffect(() => {
    onStatusChange({
      left: summary
        ? summary.equal
          ? "Binary files are identical"
          : `First diff at 0x${(summary.firstDiff ?? 0).toString(16).toUpperCase().padStart(8, "0")}`
        : "Ready",
      center: `Window 0x${offset.toString(16).toUpperCase().padStart(8, "0")}`,
      right: summary ? `${summary.leftSize} B / ${summary.rightSize} B` : "",
    });
  }, [onStatusChange, summary, offset]);

  const handleFindHex = async (): Promise<void> => {
    if (!summary || !findHex.trim()) return;
    try {
      const hit = await hexFind(summary.sessionId, findHex.trim(), offset);
      if (hit === null) {
        setFindNote("Hex pattern not found");
      } else {
        setFindNote(`Found at 0x${hit.toString(16).toUpperCase().padStart(8, "0")}`);
        await loadPage(summary.sessionId, Math.floor(hit / BYTES_PER_ROW) * BYTES_PER_ROW);
      }
    } catch (e) {
      setFindNote(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div data-testid="v2/hex-view" className="flex h-full min-h-0 flex-col overflow-hidden bg-[var(--v2-surface)]">
      <V2DualPathBar
        leftValue={leftDraft}
        rightValue={rightDraft}
        onLeftChange={setLeftDraft}
        onRightChange={setRightDraft}
        onSubmit={onPathsChange}
        leftPlaceholder="Left binary path…"
        rightPlaceholder="Right binary path…"
        splitPct={splitPct}
      />

      {/* Docked Hex Find Bar */}
      {findOpen ? (
        <div className="flex shrink-0 items-center gap-2 border-b border-[var(--v2-border)] bg-[var(--v2-chrome)] px-3 py-1.5">
          <Search size={13} className="text-[var(--v2-fg-muted)]" />
          <input
            ref={findInputRef}
            type="text"
            value={findHex}
            onChange={(e) => setFindHex(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void handleFindHex();
            }}
            placeholder="Hex bytes (e.g. 61 6C 70 68 61)…"
            className="v2-input v2-mono w-64"
          />
          <button type="button" onClick={() => void handleFindHex()} className="v2-btn">
            Find Hex
          </button>
          {findNote ? <span className="text-[11px] text-[var(--v2-fg-muted)]">{findNote}</span> : null}
          <button
            type="button"
            aria-label="Close hex find"
            onClick={() => setFindOpen(false)}
            className="ml-auto inline-flex h-5 w-5 items-center justify-center rounded text-[var(--v2-fg-muted)] hover:bg-[var(--v2-btn-hover)]"
          >
            <X size={13} />
          </button>
        </div>
      ) : null}

      <V2ErrorBanner message={error} onDismiss={() => setError(null)} />

      {/* Dual-Pane Hex + ASCII Table */}
      <div ref={containerRef} className="v2-mono min-h-0 flex-1 overflow-auto text-[11.5px] leading-[21px]">
        {loading ? (
          <div className="flex h-full items-center justify-center text-[var(--v2-fg-muted)]">
            Comparing binary streams…
          </div>
        ) : visibleRows.length === 0 ? (
          <div className="flex h-full items-center justify-center text-[var(--v2-fg-muted)]">
            No hex bytes to display.
          </div>
        ) : (
          <div className="min-w-[980px]">
            <div
              style={{ gridTemplateColumns: `78px minmax(0, ${splitPct}fr) 132px 6px minmax(0, ${100 - splitPct}fr) 132px` }}
              className="sticky top-0 z-10 grid border-b border-[var(--v2-border)] bg-[var(--v2-chrome)] px-2 py-1 text-[10.5px] font-semibold text-[var(--v2-fg-secondary)]"
            >
              <span>Offset</span>
              <span>Left Hex (00–0F)</span>
              <span>Left ASCII</span>
              <span
                role="separator"
                aria-orientation="vertical"
                title="Drag to resize panes (double-click to reset 50/50)"
                onMouseDown={startDrag}
                onDoubleClick={resetSplit}
                className="cursor-col-resize bg-[var(--v2-splitter)]/40 hover:bg-[var(--v2-accent)]/60"
              />
              <span className="pl-2">Right Hex (00–0F)</span>
              <span>Right ASCII</span>
            </div>
            {visibleRows.map((row) => (
              <div
                key={row.rowOffset}
                style={{ gridTemplateColumns: `78px minmax(0, ${splitPct}fr) 132px 6px minmax(0, ${100 - splitPct}fr) 132px` }}
                className="grid h-[21px] items-center px-2 hover:bg-[var(--v2-btn-hover)]/40"
              >
                <span className="select-none text-[var(--v2-fg-muted)]">
                  {row.rowOffset.toString(16).toUpperCase().padStart(8, "0")}
                </span>

                {/* Left Hex */}
                <div className="flex flex-wrap gap-1 pr-2">
                  {row.left.map((b, idx) => {
                    const rb = row.right[idx];
                    const diff = b !== rb;
                    return (
                      <span
                        key={idx}
                        style={
                          b !== null && diff
                            ? { backgroundColor: "var(--v2-chg-bg)", color: "var(--v2-chg-fg)" }
                            : undefined
                        }
                        className={`inline-block w-[18px] rounded-[2px] text-center ${
                          b === null ? "v2-hatch text-transparent" : ""
                        } ${idx === 7 ? "mr-1.5" : ""}`}
                      >
                        {b === null ? ".." : hex2(b)}
                      </span>
                    );
                  })}
                </div>

                {/* Left ASCII */}
                <div className="truncate tracking-tight text-[var(--v2-fg-secondary)]">
                  {row.left.map((b, idx) => (
                    <span
                      key={idx}
                      style={
                        b !== null && b !== row.right[idx]
                          ? { backgroundColor: "var(--v2-chg-bg)", color: "var(--v2-chg-fg)" }
                          : undefined
                      }
                    >
                      {b === null ? " " : asciiChar(b)}
                    </span>
                  ))}
                </div>

                {/* Draggable Splitter */}
                <span
                  role="separator"
                  aria-orientation="vertical"
                  title="Drag to resize panes (double-click to reset 50/50)"
                  onMouseDown={startDrag}
                  onDoubleClick={resetSplit}
                  className="group relative h-full cursor-col-resize"
                >
                  <span
                    className={`absolute inset-y-0 left-1/2 -translate-x-1/2 transition-colors ${
                      dragging
                        ? "w-[2px] bg-[var(--v2-accent)]"
                        : "w-px bg-[var(--v2-splitter)] group-hover:w-[2px] group-hover:bg-[var(--v2-accent)]"
                    }`}
                  />
                </span>

                {/* Right Hex */}
                <div className="flex flex-wrap gap-1 pl-2 pr-2">
                  {row.right.map((b, idx) => {
                    const lb = row.left[idx];
                    const diff = b !== lb;
                    return (
                      <span
                        key={idx}
                        style={
                          b !== null && diff
                            ? { backgroundColor: "var(--v2-chg-bg)", color: "var(--v2-chg-fg)" }
                            : undefined
                        }
                        className={`inline-block w-[18px] rounded-[2px] text-center ${
                          b === null ? "v2-hatch text-transparent" : ""
                        } ${idx === 7 ? "mr-1.5" : ""}`}
                      >
                        {b === null ? ".." : hex2(b)}
                      </span>
                    );
                  })}
                </div>

                {/* Right ASCII */}
                <div className="truncate tracking-tight text-[var(--v2-fg-secondary)]">
                  {row.right.map((b, idx) => (
                    <span
                      key={idx}
                      style={
                        b !== null && b !== row.left[idx]
                          ? { backgroundColor: "var(--v2-chg-bg)", color: "var(--v2-chg-fg)" }
                          : undefined
                      }
                    >
                      {b === null ? " " : asciiChar(b)}
                    </span>
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Pagination Footer when files exceed 2KB */}
      {page && page.total > PAGE_BYTES ? (
        <div className="flex shrink-0 items-center justify-between border-t border-[var(--v2-border)] bg-[var(--v2-surface-subtle)] px-3 py-1 text-[11px]">
          <button
            type="button"
            disabled={offset === 0 || !summary}
            onClick={() => summary && void loadPage(summary.sessionId, Math.max(0, offset - PAGE_BYTES))}
            className="v2-btn"
          >
            Prev Page
          </button>
          <span className="v2-mono text-[var(--v2-fg-muted)]">
            Showing {offset} – {Math.min(page.total, offset + PAGE_BYTES)} of {page.total} bytes
          </span>
          <button
            type="button"
            disabled={offset + PAGE_BYTES >= page.total || !summary}
            onClick={() => summary && void loadPage(summary.sessionId, offset + PAGE_BYTES)}
            className="v2-btn"
          >
            Next Page
          </button>
        </div>
      ) : null}
    </div>
  );
}
