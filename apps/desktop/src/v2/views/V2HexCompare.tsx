import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Copy, RotateCcw, Search, X } from "lucide-react";
import { hexApplyEdit, hexCompare, hexFind, hexGetPage, hexSaveSide } from "../../ipc/hex";
import type { HexPage, HexSummary } from "../../ipc/hex";
import { clearActiveTrace, setActiveTrace } from "../../ipc/client";
import { endSpan, isTelemetryEnabled, startRootSpan, type Span } from "../../observability/tracer";
import { emitFileWatchTransition } from "../utils/fileWatch";
import { emitBufferEditTransition } from "../utils/textBuffer";
import V2ErrorBanner from "../components/V2ErrorBanner";
import V2ReloadPromptBanner from "../components/V2ReloadPromptBanner";
import V2DualPathBar from "../components/V2DualPathBar";
import { isMissingFixtureError } from "../utils/demoGuard";
import {
  asciiToHexQuery,
  decodeHexInspector,
  formatBytesAsAscii,
  formatBytesAsHex,
} from "../utils/hexInspector";
import { useSplitPane } from "../hooks/useSplitPane";
import { useVirtualRows } from "../hooks/useVirtualRows";
import { useExternalChangeWatch } from "../hooks/useExternalChangeWatch";
import type { FilterMode, V2StatusInfo, V2ViewCommands } from "../types";

export interface V2HexCompareProps {
  readonly leftPath: string;
  readonly rightPath: string;
  readonly paneOrientation?: "side-by-side" | "over-under";
  readonly onPathsChange: (left: string, right: string) => void;
  readonly onRegisterCommands: (cmds: V2ViewCommands) => void;
  readonly onStatusChange: (status: V2StatusInfo) => void;
}

const BYTES_PER_ROW = 16;
const HEX_ROW_HEIGHT = 21;
const PAGE_BYTES = 16384; // 1,024 virtualized hex rows per page

interface EditingByte {
  readonly side: "left" | "right";
  readonly absOffset: number;
  readonly draft: string;
}

interface BytePatchEntry {
  readonly key: string;
  readonly prev: number | undefined;
  readonly next: number | undefined;
}

function hex2(b: number): string {
  return b.toString(16).padStart(2, "0").toUpperCase();
}

function asciiChar(b: number): string {
  return b >= 0x20 && b <= 0x7e ? String.fromCharCode(b) : "·";
}

function byteKey(side: "left" | "right", absOffset: number): string {
  return `${side}:${absOffset}`;
}

export default function V2HexCompare({
  leftPath,
  rightPath,
  paneOrientation = "side-by-side",
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
  const [notice, setNotice] = useState<string | null>(null);
  const [filterMode, setFilterMode] = useState<FilterMode>("all");
  const [findOpen, setFindOpen] = useState(false);
  const [findMode, setFindMode] = useState<"hex" | "ascii">("hex");
  const [findHex, setFindHex] = useState("");
  const [findNote, setFindNote] = useState<string | null>(null);
  const [selectedByte, setSelectedByte] = useState<{ readonly side: "left" | "right"; readonly absOffset: number } | null>(null);
  const [littleEndian, setLittleEndian] = useState(true);

  // Byte-level inline edits & Undo/Redo stack
  const [byteEdits, setByteEdits] = useState<Map<string, number>>(() => new Map());
  const [editingByte, setEditingByte] = useState<EditingByte | null>(null);
  const undoRef = useRef<BytePatchEntry[]>([]);
  const redoRef = useRef<BytePatchEntry[]>([]);

  const findInputRef = useRef<HTMLInputElement | null>(null);
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const { containerRef, splitPct, dragging, startDrag, resetSplit } = useSplitPane(50);
  // ODD Phase 4 Leaf A: ambient trace of the last compare, so reload prompts
  // and byte edits link to the same trace after the root span ends.
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
    emitFileWatchTransition("hex", "prompt_shown", { paths: changedPaths.length }, lastTraceRef.current);
  }, [changedPaths]);

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
      // ODD Phase 4 Leaf A: one root span per compare trigger (op hex.compare),
      // following the exact V2TextCompare pattern. Telemetry-off runs identically.
      const obs: Span | null = isTelemetryEnabled()
        ? startRootSpan({ op: "hex.compare", component: "ts.ui/HexView", view_id: "hex" })
        : null;
      if (obs !== null) {
        setActiveTrace(obs.context());
        lastTraceRef.current = obs.context();
      }
      setLoading(true);
      setError(null);
      setByteEdits(new Map());
      setEditingByte(null);
      undoRef.current = [];
      redoRef.current = [];
      const lEff = l || "tools/parity-harness/fixtures/t_left.txt";
      const rEff = r || "tools/parity-harness/fixtures/t_right.txt";
      try {
        const dto = await hexCompare(lEff, rEff);
        setSummary(dto);
        await loadPage(dto.sessionId, 0);
        if (obs !== null) {
          obs.set({
            rows: Math.ceil(Math.max(dto.leftSize, dto.rightSize) / BYTES_PER_ROW),
            hunks: dto.firstDiff === null || dto.firstDiff === undefined ? 0 : 1,
            equal: dto.equal,
            left_size: dto.leftSize,
            right_size: dto.rightSize,
            session: dto.sessionId,
          });
        }
      } catch (e) {
        if (obs !== null) {
          obs.error({
            kind: "hex_compare_failed",
            component: "ts.ui/HexView",
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
    [loadPage],
  );

  useEffect(() => {
    void runCompare(leftPath, rightPath);
  }, [leftPath, rightPath, runCompare]);

  const applyByteUndo = useCallback((): void => {
    const entry = undoRef.current.pop();
    if (!entry) return;
    redoRef.current = [...redoRef.current.slice(-49), entry];
    setByteEdits((prev) => {
      const next = new Map(prev);
      if (entry.prev === undefined) next.delete(entry.key);
      else next.set(entry.key, entry.prev);
      return next;
    });
  }, []);

  const applyByteRedo = useCallback((): void => {
    const entry = redoRef.current.pop();
    if (!entry) return;
    undoRef.current = [...undoRef.current.slice(-49), entry];
    setByteEdits((prev) => {
      const next = new Map(prev);
      if (entry.next === undefined) next.delete(entry.key);
      else next.set(entry.key, entry.next);
      return next;
    });
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent): void => {
      const mod = e.metaKey || e.ctrlKey;
      const key = e.key.toLowerCase();
      if (mod && key === "f") {
        e.preventDefault();
        setFindOpen(true);
        setTimeout(() => findInputRef.current?.focus(), 20);
        return;
      }
      if (e.key === "Escape" && findOpen) {
        setFindOpen(false);
        return;
      }
      const target = e.target;
      if (target instanceof HTMLElement && (target.tagName === "INPUT" || target.tagName === "TEXTAREA")) {
        return;
      }
      if (mod && key === "z" && !e.shiftKey && undoRef.current.length > 0) {
        e.preventDefault();
        applyByteUndo();
      } else if (mod && ((key === "z" && e.shiftKey) || key === "y") && redoRef.current.length > 0) {
        e.preventDefault();
        applyByteRedo();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [findOpen, applyByteUndo, applyByteRedo]);

  const commitByteEdit = useCallback((): void => {
    if (!editingByte) return;
    const raw = editingByte.draft.trim();
    if (/^[0-9a-fA-F]{1,2}$/.test(raw)) {
      const parsed = parseInt(raw, 16);
      // ODD Phase 4 Leaf A: byte-edit state-transition event linked to ambient trace.
      emitBufferEditTransition(
        "hex.compare",
        "hex",
        "byte_edit",
        { side: editingByte.side, offset: editingByte.absOffset },
        lastTraceRef.current,
      );
      const key = byteKey(editingByte.side, editingByte.absOffset);
      setByteEdits((prev) => {
        const prevVal = prev.get(key);
        undoRef.current = [...undoRef.current.slice(-49), { key, prev: prevVal, next: parsed }];
        redoRef.current = [];
        const next = new Map(prev);
        next.set(key, parsed);
        return next;
      });
      if (summary) {
        void hexApplyEdit(summary.sessionId, editingByte.side, editingByte.absOffset, parsed).catch(
          (e) => setError(e instanceof Error ? e.message : String(e)),
        );
      }
    }
    setEditingByte(null);
  }, [editingByte, summary]);

  const handleSaveSide = useCallback(
    async (side: "left" | "right"): Promise<void> => {
      if (!summary) return;
      const targetPath = side === "left" ? leftDraft : rightDraft;
      if (!targetPath.trim()) {
        setNotice(`No file path set for ${side} side.`);
        return;
      }
      try {
        const rep = await hexSaveSide(summary.sessionId, side, targetPath.trim());
        setByteEdits((prev) => {
          const next = new Map(prev);
          const prefix = `${side}:`;
          for (const k of Array.from(next.keys())) {
            if (k.startsWith(prefix)) next.delete(k);
          }
          return next;
        });
        setNotice(
          `Saved ${side} (${rep.editsApplied} ${rep.editsApplied === 1 ? "edit" : "edits"}, ${rep.bytesWritten} B)`,
        );
        void resetBaseline();
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    },
    [summary, leftDraft, rightDraft, resetBaseline],
  );

  const rows = useMemo(() => {
    if (!page) return [];
    const maxLen = Math.max(page.left.length, page.right.length);
    const out: {
      rowOffset: number;
      left: (number | null)[];
      right: (number | null)[];
      differs: boolean;
    }[] = [];
    for (let i = 0; i < maxLen; i += BYTES_PER_ROW) {
      const left: (number | null)[] = [];
      const right: (number | null)[] = [];
      let differs = false;
      for (let j = 0; j < BYTES_PER_ROW; j += 1) {
        const absOff = page.offset + i + j;
        const baseL = i + j < page.left.length ? page.left[i + j] : null;
        const baseR = i + j < page.right.length ? page.right[i + j] : null;
        const lb = baseL !== null ? (byteEdits.get(byteKey("left", absOff)) ?? baseL) : null;
        const rb = baseR !== null ? (byteEdits.get(byteKey("right", absOff)) ?? baseR) : null;
        if (lb !== rb) differs = true;
        left.push(lb);
        right.push(rb);
      }
      out.push({ rowOffset: page.offset + i, left, right, differs });
    }
    return out;
  }, [page, byteEdits]);

  const visibleRows = useMemo(() => {
    if (filterMode === "all") return rows;
    if (filterMode === "diffs") return rows.filter((r) => r.differs);
    return rows.filter((r) => !r.differs);
  }, [rows, filterMode]);

  const { startIndex, endIndex, totalHeight, offsetTop, scrollToPosition } = useVirtualRows({
    count: visibleRows.length,
    rowHeight: HEX_ROW_HEIGHT,
    overscan: 16,
    scrollRef,
  });

  const windowedRows = useMemo(
    () => visibleRows.slice(startIndex, endIndex),
    [visibleRows, startIndex, endIndex],
  );

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
          const targetRowOff = Math.floor(summary.firstDiff / BYTES_PER_ROW) * BYTES_PER_ROW;
          const pos = visibleRows.findIndex((r) => r.rowOffset === targetRowOff);
          if (pos >= 0) {
            scrollToPosition(pos);
          } else {
            void loadPage(summary.sessionId, targetRowOff);
          }
        }
      },
      swapSides: () => onPathsChange(rightDraft, leftDraft),
      reload: () => void runCompare(leftDraft, rightDraft),
    });
  }, [
    onRegisterCommands,
    filterMode,
    summary,
    visibleRows,
    scrollToPosition,
    loadPage,
    onPathsChange,
    leftDraft,
    rightDraft,
    runCompare,
  ]);

  useEffect(() => {
    const editedCount = byteEdits.size;
    onStatusChange({
      left: summary
        ? summary.equal && editedCount === 0
          ? "Binary files are identical"
          : `First diff at 0x${(summary.firstDiff ?? 0).toString(16).toUpperCase().padStart(8, "0")}`
        : "Ready",
      center:
        notice ??
        (editedCount > 0
          ? `${editedCount} modified ${editedCount === 1 ? "byte" : "bytes"} · Double-click any byte to edit (⌘Z undo)`
          : `Window 0x${offset.toString(16).toUpperCase().padStart(8, "0")} · Double-click any byte to edit`),
      right: summary ? `${summary.leftSize} B / ${summary.rightSize} B` : "",
    });
  }, [onStatusChange, summary, offset, byteEdits.size]);

  const handleFindHex = async (): Promise<void> => {
    if (!summary || !findHex.trim()) return;
    const query = findMode === "ascii" ? asciiToHexQuery(findHex) : findHex.trim();
    if (!query) return;
    try {
      const hit = await hexFind(summary.sessionId, query, offset);
      if (hit === null) {
        setFindNote(findMode === "ascii" ? "Text pattern not found" : "Hex pattern not found");
      } else {
        setFindNote(`Found at 0x${hit.toString(16).toUpperCase().padStart(8, "0")}`);
        setSelectedByte({ side: "left", absOffset: hit });
        const targetRowOff = Math.floor(hit / BYTES_PER_ROW) * BYTES_PER_ROW;
        const pos = visibleRows.findIndex((r) => r.rowOffset === targetRowOff);
        if (pos >= 0) scrollToPosition(pos);
        else await loadPage(summary.sessionId, targetRowOff);
      }
    } catch (e) {
      setFindNote(e instanceof Error ? e.message : String(e));
    }
  };

  const inspectorState = useMemo(() => {
    if (!selectedByte || !page) return null;
    const rel = selectedByte.absOffset - page.offset;
    const src = selectedByte.side === "left" ? page.left : page.right;
    if (rel < 0 || rel >= src.length) return null;
    const slice: number[] = [];
    for (let i = 0; i < 4 && rel + i < src.length; i += 1) {
      const absOff = selectedByte.absOffset + i;
      slice.push(byteEdits.get(byteKey(selectedByte.side, absOff)) ?? src[rel + i]);
    }
    const rowStart = Math.floor(rel / BYTES_PER_ROW) * BYTES_PER_ROW;
    const rowBytes: number[] = [];
    for (let i = 0; i < BYTES_PER_ROW && rowStart + i < src.length; i += 1) {
      const absOff = page.offset + rowStart + i;
      rowBytes.push(byteEdits.get(byteKey(selectedByte.side, absOff)) ?? src[rowStart + i]);
    }
    return {
      decoded: decodeHexInspector(slice, littleEndian),
      rowBytes,
    };
  }, [selectedByte, page, byteEdits, littleEndian]);

  const copySelectedRow = async (mode: "hex" | "ascii"): Promise<void> => {
    if (!inspectorState) return;
    const text =
      mode === "hex"
        ? formatBytesAsHex(inspectorState.rowBytes)
        : formatBytesAsAscii(inspectorState.rowBytes);
    try {
      await navigator.clipboard.writeText(text);
      setNotice(`Copied ${mode.toUpperCase()} row (${inspectorState.rowBytes.length} bytes)`);
    } catch {
      setNotice("Clipboard unavailable");
    }
  };

  return (
    <div
      data-testid="v2/hex-view"
      data-pane-orientation={paneOrientation}
      className="flex h-full min-h-0 flex-col overflow-hidden bg-[var(--v2-surface)]"
    >
      <V2DualPathBar
        leftValue={leftDraft}
        rightValue={rightDraft}
        onLeftChange={setLeftDraft}
        onRightChange={setRightDraft}
        onSubmit={onPathsChange}
        leftPlaceholder="Left binary path…"
        rightPlaceholder="Right binary path…"
        splitPct={splitPct}
        onSaveSide={(side) => void handleSaveSide(side)}
      />

      {/* Docked Hex / ASCII Find Bar */}
      {findOpen ? (
        <div className="flex shrink-0 items-center gap-2 border-b border-[var(--v2-border)] bg-[var(--v2-chrome)] px-3 py-1.5">
          <Search size={13} className="text-[var(--v2-fg-muted)]" />
          <div className="inline-flex rounded-[5px] bg-[var(--v2-seg-track)] p-0.5 text-[11px]">
            <button
              type="button"
              onClick={() => setFindMode("hex")}
              className={`rounded-[4px] px-2 py-0.5 font-medium ${findMode === "hex" ? "bg-[var(--v2-surface)] text-[var(--v2-fg)]" : "text-[var(--v2-fg-secondary)]"}`}
            >
              Hex
            </button>
            <button
              type="button"
              onClick={() => setFindMode("ascii")}
              className={`rounded-[4px] px-2 py-0.5 font-medium ${findMode === "ascii" ? "bg-[var(--v2-surface)] text-[var(--v2-fg)]" : "text-[var(--v2-fg-secondary)]"}`}
            >
              ASCII
            </button>
          </div>
          <input
            ref={findInputRef}
            type="text"
            value={findHex}
            onChange={(e) => setFindHex(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void handleFindHex();
            }}
            placeholder={findMode === "ascii" ? "UTF-8 / ASCII text (e.g. alpha)…" : "Hex bytes (e.g. 61 6C 70 68 61)…"}
            className="v2-input v2-mono w-64"
          />
          <button type="button" onClick={() => void handleFindHex()} className="v2-btn">
            Find
          </button>
          {findNote ? <span className="text-[11px] text-[var(--v2-fg-muted)]">{findNote}</span> : null}
          {byteEdits.size > 0 ? (
            <button
              type="button"
              onClick={() => {
                setByteEdits(new Map());
                undoRef.current = [];
                redoRef.current = [];
              }}
              className="v2-btn ml-2"
            >
              <RotateCcw size={11} />
              Revert {byteEdits.size} byte edits
            </button>
          ) : null}
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

      <V2ReloadPromptBanner
        changedPaths={changedPaths}
        onReload={() => {
          emitFileWatchTransition("hex", "reload_accepted", { paths: changedPaths.length }, lastTraceRef.current);
          void resetBaseline();
          void runCompare(leftDraft, rightDraft);
        }}
        onDismiss={() => {
          emitFileWatchTransition("hex", "reload_dismissed", { paths: changedPaths.length }, lastTraceRef.current);
          dismissReload();
        }}
      />
      <V2ErrorBanner message={error} onDismiss={() => setError(null)} />

      {/* Virtualized Dual-Pane Hex + ASCII Table */}
      <div
        ref={(el) => {
          scrollRef.current = el;
          (containerRef as React.MutableRefObject<HTMLDivElement | null>).current = el;
        }}
        className="v2-mono min-h-0 flex-1 overflow-auto text-[11.5px] leading-[21px]"
      >
        {loading ? (
          <div className="flex h-full items-center justify-center text-[var(--v2-fg-muted)]">
            Comparing binary streams…
          </div>
        ) : visibleRows.length === 0 ? (
          <div className="flex h-full items-center justify-center text-[var(--v2-fg-muted)]">
            No hex bytes to display.
          </div>
        ) : paneOrientation === "over-under" ? (
          <div className="flex h-full flex-col divide-y divide-[var(--v2-border)]">
            {(["left", "right"] as const).map((side) => (
              <div key={side} className="min-h-0 flex-1 overflow-auto">
                <div className="sticky top-0 z-10 grid grid-cols-[78px_1fr_140px] border-b border-[var(--v2-border)] bg-[var(--v2-chrome)] px-2 py-1 text-[10.5px] font-semibold text-[var(--v2-fg-secondary)]">
                  <span>Offset</span>
                  <span>{side === "left" ? "Top Pane — Left Hex (00–0F)" : "Bottom Pane — Right Hex (00–0F)"}</span>
                  <span>{side === "left" ? "Left ASCII" : "Right ASCII"}</span>
                </div>
                {visibleRows.map((row) => {
                  const bytes = side === "left" ? row.left : row.right;
                  const other = side === "left" ? row.right : row.left;
                  return (
                    <div
                      key={`${side}-${row.rowOffset}`}
                      className="grid h-[21px] grid-cols-[78px_1fr_140px] items-center px-2 hover:bg-[var(--v2-btn-hover)]/40"
                    >
                      <span className="select-none text-[var(--v2-fg-muted)]">
                        {row.rowOffset.toString(16).toUpperCase().padStart(8, "0")}
                      </span>
                      <div className="flex flex-wrap gap-1 pr-2">
                        {bytes.map((b, idx) => {
                          const absOff = row.rowOffset + idx;
                          const diff = b !== other[idx];
                          return (
                            <span
                              key={idx}
                              onClick={() => {
                                if (b !== null) setSelectedByte({ side, absOffset: absOff });
                              }}
                              style={
                                b !== null && diff
                                  ? { backgroundColor: "var(--v2-chg-bg)", color: "var(--v2-chg-fg)" }
                                  : undefined
                              }
                              className={`inline-block w-[18px] cursor-pointer rounded-[2px] text-center ${b === null ? "v2-hatch text-transparent" : ""} ${idx === 7 ? "mr-1.5" : ""}`}
                            >
                              {b === null ? ".." : hex2(b)}
                            </span>
                          );
                        })}
                      </div>
                      <div className="truncate tracking-tight text-[var(--v2-fg-secondary)]">
                        {bytes.map((b, idx) => (
                          <span
                            key={idx}
                            style={
                              b !== null && b !== other[idx]
                                ? { backgroundColor: "var(--v2-chg-bg)", color: "var(--v2-chg-fg)" }
                                : undefined
                            }
                          >
                            {b === null ? " " : asciiChar(b)}
                          </span>
                        ))}
                      </div>
                    </div>
                  );
                })}
              </div>
            ))}
          </div>
        ) : (
          <div className="min-w-[980px]">
            <div
              style={{
                gridTemplateColumns: `78px minmax(0, ${splitPct}fr) 132px 6px minmax(0, ${100 - splitPct}fr) 132px`,
              }}
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

            <div className="relative" style={{ height: totalHeight }}>
              <div style={{ transform: `translateY(${offsetTop}px)` }}>
                {windowedRows.map((row) => (
                  <div
                    key={row.rowOffset}
                    style={{
                      gridTemplateColumns: `78px minmax(0, ${splitPct}fr) 132px 6px minmax(0, ${100 - splitPct}fr) 132px`,
                    }}
                    className="grid h-[21px] items-center px-2 hover:bg-[var(--v2-btn-hover)]/40"
                  >
                    <span className="select-none text-[var(--v2-fg-muted)]">
                      {row.rowOffset.toString(16).toUpperCase().padStart(8, "0")}
                    </span>

                    {/* Left Hex */}
                    <div className="flex flex-wrap gap-1 pr-2">
                      {row.left.map((b, idx) => {
                        const absOff = row.rowOffset + idx;
                        const rb = row.right[idx];
                        const diff = b !== rb;
                        const modified = byteEdits.has(byteKey("left", absOff));
                        const isEditing =
                          editingByte?.side === "left" && editingByte.absOffset === absOff;
                        return isEditing ? (
                          <input
                            key={idx}
                            autoFocus
                            maxLength={2}
                            value={editingByte.draft}
                            onChange={(e) =>
                              setEditingByte({ ...editingByte, draft: e.target.value })
                            }
                            onKeyDown={(e) => {
                              if (e.key === "Enter") commitByteEdit();
                              if (e.key === "Escape") setEditingByte(null);
                            }}
                            onBlur={commitByteEdit}
                            className={`h-[18px] w-[22px] rounded-[2px] bg-[var(--v2-surface)] text-center text-[11px] uppercase text-[var(--v2-fg)] outline outline-1 outline-[var(--v2-accent)] ${
                              idx === 7 ? "mr-1.5" : ""
                            }`}
                          />
                        ) : (
                          <span
                            key={idx}
                            title={b !== null ? `0x${absOff.toString(16).toUpperCase()} — click to inspect, double-click to edit` : undefined}
                            onClick={() => {
                              if (b !== null) setSelectedByte({ side: "left", absOffset: absOff });
                            }}
                            onDoubleClick={() => {
                              if (b !== null) {
                                setEditingByte({ side: "left", absOffset: absOff, draft: hex2(b) });
                              }
                            }}
                            style={
                              b !== null && diff
                                ? { backgroundColor: "var(--v2-chg-bg)", color: "var(--v2-chg-fg)" }
                                : undefined
                            }
                            className={`inline-block w-[18px] cursor-pointer rounded-[2px] text-center ${
                              b === null ? "v2-hatch cursor-default text-transparent" : ""
                            } ${modified || (selectedByte?.side === "left" && selectedByte.absOffset === absOff) ? "ring-1 ring-[var(--v2-accent)]" : ""} ${
                              idx === 7 ? "mr-1.5" : ""
                            }`}
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
                        const absOff = row.rowOffset + idx;
                        const lb = row.left[idx];
                        const diff = b !== lb;
                        const modified = byteEdits.has(byteKey("right", absOff));
                        const isEditing =
                          editingByte?.side === "right" && editingByte.absOffset === absOff;
                        return isEditing ? (
                          <input
                            key={idx}
                            autoFocus
                            maxLength={2}
                            value={editingByte.draft}
                            onChange={(e) =>
                              setEditingByte({ ...editingByte, draft: e.target.value })
                            }
                            onKeyDown={(e) => {
                              if (e.key === "Enter") commitByteEdit();
                              if (e.key === "Escape") setEditingByte(null);
                            }}
                            onBlur={commitByteEdit}
                            className={`h-[18px] w-[22px] rounded-[2px] bg-[var(--v2-surface)] text-center text-[11px] uppercase text-[var(--v2-fg)] outline outline-1 outline-[var(--v2-accent)] ${
                              idx === 7 ? "mr-1.5" : ""
                            }`}
                          />
                        ) : (
                          <span
                            key={idx}
                            title={b !== null ? `0x${absOff.toString(16).toUpperCase()} — click to inspect, double-click to edit` : undefined}
                            onClick={() => {
                              if (b !== null) setSelectedByte({ side: "right", absOffset: absOff });
                            }}
                            onDoubleClick={() => {
                              if (b !== null) {
                                setEditingByte({ side: "right", absOffset: absOff, draft: hex2(b) });
                              }
                            }}
                            style={
                              b !== null && diff
                                ? { backgroundColor: "var(--v2-chg-bg)", color: "var(--v2-chg-fg)" }
                                : undefined
                            }
                            className={`inline-block w-[18px] cursor-pointer rounded-[2px] text-center ${
                              b === null ? "v2-hatch cursor-default text-transparent" : ""
                            } ${modified || (selectedByte?.side === "right" && selectedByte.absOffset === absOff) ? "ring-1 ring-[var(--v2-accent)]" : ""} ${
                              idx === 7 ? "mr-1.5" : ""
                            }`}
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
            </div>
          </div>
        )}
      </div>

      {/* Data Inspector Bar for Selected Byte */}
      {selectedByte && inspectorState ? (
        <div
          data-testid="v2/hex-inspector"
          className="v2-mono flex shrink-0 flex-wrap items-center justify-between gap-2 border-t border-[var(--v2-border)] bg-[var(--v2-chrome)] px-3 py-1.5 text-[11px]"
        >
          <div className="flex flex-wrap items-center gap-3">
            <span className="font-semibold text-[var(--v2-accent)]">
              {selectedByte.side.toUpperCase()} 0x{selectedByte.absOffset.toString(16).toUpperCase().padStart(8, "0")}
            </span>
            <button
              type="button"
              onClick={() => setLittleEndian((v) => !v)}
              className="v2-btn h-[19px] px-1.5 text-[10px]"
              title="Toggle Little-Endian / Big-Endian"
            >
              {littleEndian ? "LE" : "BE"}
            </button>
            <span>U8: <b>{inspectorState.decoded.uint8 ?? "—"}</b></span>
            <span>I8: <b>{inspectorState.decoded.int8 ?? "—"}</b></span>
            <span>U16: <b>{inspectorState.decoded.uint16 ?? "—"}</b></span>
            <span>I16: <b>{inspectorState.decoded.int16 ?? "—"}</b></span>
            <span>I32: <b>{inspectorState.decoded.int32 ?? "—"}</b></span>
            <span>F32: <b>{inspectorState.decoded.float32 ?? "—"}</b></span>
            <span className="text-[var(--v2-fg-muted)]">Bin: {inspectorState.decoded.binary}</span>
          </div>
          <div className="flex items-center gap-1.5">
            <button
              type="button"
              onClick={() => void copySelectedRow("hex")}
              className="v2-btn h-[20px] px-2 text-[10.5px]"
            >
              <Copy size={10} />
              Copy Hex
            </button>
            <button
              type="button"
              onClick={() => void copySelectedRow("ascii")}
              className="v2-btn h-[20px] px-2 text-[10.5px]"
            >
              <Copy size={10} />
              Copy ASCII
            </button>
          </div>
        </div>
      ) : null}

      {/* Pagination Footer when files exceed 16KB */}
      {page && page.total > PAGE_BYTES ? (
        <div className="flex shrink-0 items-center justify-between border-t border-[var(--v2-border)] bg-[var(--v2-surface-subtle)] px-3 py-1 text-[11px]">
          <button
            type="button"
            disabled={offset === 0 || !summary}
            onClick={() => summary && void loadPage(summary.sessionId, Math.max(0, offset - PAGE_BYTES))}
            className="v2-btn"
          >
            Prev 16 KB
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
            Next 16 KB
          </button>
        </div>
      ) : null}
    </div>
  );
}
