import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  textApplyHunk,
  textCompare,
  textCompareDemo,
  textGetRows,
  textRestoreRows,
  textSaveSide,
  textUpdateSide,
} from "../../ipc/text";
import type { TextCompareDto, TextRowDto, TextSide } from "../../ipc/text";
import V2ErrorBanner from "../components/V2ErrorBanner";
import V2ReloadPromptBanner from "../components/V2ReloadPromptBanner";
import V2DualPathBar from "../components/V2DualPathBar";
import V2TextFindBar from "../components/text/V2TextFindBar";
import type { ReplaceTargetSide } from "../components/text/V2TextFindBar";
import V2TextDiffRow, {
  renderHighlightedLine,
} from "../components/text/V2TextDiffRow";
import type { HunkMeta, HunkRole, TextHunk } from "../components/text/V2TextDiffRow";
import { V2TextRulesModal } from "../components/text/V2TextModals";
import type { RulesDialogTab, TextSessionRules } from "../components/text/V2TextModals";
import { useSplitPane } from "../hooks/useSplitPane";
import { useVirtualRows } from "../hooks/useVirtualRows";
import { useExternalChangeWatch } from "../hooks/useExternalChangeWatch";
import {
  extractSideLines,
  insertPastedText,
  joinWithNextLine,
  joinWithPreviousLine,
  replaceInLines,
  rowIndexToSideLineIndex,
  sideLineIndexToRowIndex,
  splitLineAt,
} from "../utils/textBuffer";
import { matchesUnderSessionRules } from "../utils/textReplacements";
import { resolveSyntaxLang } from "../utils/syntaxHighlight";
import { addManualAnchor, applyManualAnchors } from "../utils/manualAlign";
import type { ManualAnchor } from "../utils/manualAlign";
import type { FilterMode, V2StatusInfo, V2ViewCommands } from "../types";

const DEFAULT_RULES_STORAGE_KEY = "autocompare.v2.text.defaultRules";

const DEFAULT_TEXT_RULES: TextSessionRules = {
  leftFormat: "Detected (Everything Else)",
  rightFormat: "Detected (Everything Else)",
  sameFormat: true,
  leftEncoding: "None",
  rightEncoding: "None",
  sameEncoding: true,
  eol: "UNIX",
  ignoreWs: false,
  ignoreCase: false,
  ignoreBlank: false,
  minorEnabled: false,
  unimportantRules: [],
  alignmentMode: "standard",
  neverAlignDifferences: false,
  skewTolerance: 2000,
  useClosenessMatching: true,
  replacements: [],
};

function loadDefaultRules(): TextSessionRules {
  try {
    const raw = window.localStorage.getItem(DEFAULT_RULES_STORAGE_KEY);
    if (!raw) return DEFAULT_TEXT_RULES;
    const parsed = JSON.parse(raw) as Partial<TextSessionRules>;
    return { ...DEFAULT_TEXT_RULES, ...parsed };
  } catch {
    return DEFAULT_TEXT_RULES;
  }
}

function saveDefaultRules(rules: TextSessionRules): void {
  try {
    window.localStorage.setItem(DEFAULT_RULES_STORAGE_KEY, JSON.stringify(rules));
  } catch {
    // Ignore storage quota / private-mode errors
  }
}

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

interface ActiveEditor {
  readonly rowIndex: number;
  readonly side: TextSide;
  readonly text: string;
  readonly original: string;
  readonly col: number;
  readonly focusNonce: number;
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

  // Comparison options & BC-style session rules
  const [filterMode, setFilterMode] = useState<FilterMode>("all");
  const [contextEnabled, setContextEnabled] = useState(true);
  const [rules, setRules] = useState<TextSessionRules>(() => loadDefaultRules());
  const [detectedEncoding, setDetectedEncoding] = useState("UTF-8");
  const [detectedEol, setDetectedEol] = useState("UNIX");
  const rulesRef = useRef(rules);
  rulesRef.current = rules;

  const effectiveAlgorithm: "myers" | "patience" =
    rules.alignmentMode === "patience" ? "patience" : "myers";
  const displayEncoding =
    rules.leftEncoding !== "None" ? rules.leftEncoding : detectedEncoding;
  const displayEol = rules.eol || detectedEol;

  // Selection (Shift+Click range) + continuous notepad editor state
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);
  const [selectionAnchor, setSelectionAnchor] = useState<number | null>(null);
  const [activeEdit, setActiveEdit] = useState<ActiveEditor | null>(null);
  const activeEditRef = useRef<ActiveEditor | null>(null);
  activeEditRef.current = activeEdit;
  const rowsRef = useRef<readonly TextRowDto[]>(rows);
  rowsRef.current = rows;

  // Manual Line Alignment (Align With... / F7) + Context Menu
  const [manualAnchors, setManualAnchors] = useState<readonly ManualAnchor[]>([]);
  const [alignSource, setAlignSource] = useState<{
    readonly side: TextSide;
    readonly sideLineIdx: number;
    readonly rowIndex: number;
  } | null>(null);
  const [contextMenu, setContextMenu] = useState<{
    readonly x: number;
    readonly y: number;
    readonly rowIndex: number;
    readonly side: TextSide;
  } | null>(null);

  // Modals & Find / Replace (Cmd+F / Cmd+H)
  const [rulesModalTab, setRulesModalTab] = useState<RulesDialogTab | null>(null);
  const [findOpen, setFindOpen] = useState(false);
  const [replaceOpen, setReplaceOpen] = useState(false);
  const [findQuery, setFindQuery] = useState("");
  const [replaceQuery, setReplaceQuery] = useState("");
  const [findMatchCase, setFindMatchCase] = useState(false);
  const [replaceTargetSide, setReplaceTargetSide] = useState<ReplaceTargetSide>("left");
  const [findIdx, setFindIdx] = useState(0);

  const scrollRef = useRef<HTMLDivElement | null>(null);
  const findInputRef = useRef<HTMLInputElement | null>(null);
  const replaceInputRef = useRef<HTMLInputElement | null>(null);
  const liveDiffTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const undoStackRef = useRef<Array<{ rows: readonly TextRowDto[]; label: string }>>([]);
  const redoStackRef = useRef<Array<{ rows: readonly TextRowDto[]; label: string }>>([]);
  const { containerRef, splitPct, dragging, startDrag, resetSplit } = useSplitPane(50);

  const watchedPaths = useMemo(
    () => [leftDraft.trim(), rightDraft.trim()].filter((p) => p.length > 0),
    [leftDraft, rightDraft],
  );
  const { changedPaths, dismiss: dismissReload, resetBaseline } = useExternalChangeWatch({
    paths: watchedPaths,
    enabled: !loading && watchedPaths.length > 0,
  });

  useEffect(() => {
    setLeftDraft(leftPath);
    setRightDraft(rightPath);
  }, [leftPath, rightPath]);

  useEffect(() => {
    return () => {
      if (liveDiffTimerRef.current !== null) clearTimeout(liveDiffTimerRef.current);
    };
  }, []);

  const runCompare = useCallback(
    async (l: string, r: string): Promise<void> => {
      setLoading(true);
      setError(null);
      setNotice(null);
      setActiveEdit(null);
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
          const cur = rulesRef.current;
          const normEnc = (v: string): string | undefined =>
            v !== "None" ? (v === "Current Locale (UTF-8)" ? "UTF-8" : v) : undefined;
          const leftEnc = normEnc(cur.leftEncoding);
          const rightEnc = cur.sameEncoding ? leftEnc : normEnc(cur.rightEncoding);
          dto = await textCompare(l, r, {
            ignoreWhitespace: cur.ignoreWs,
            ignoreCase: cur.ignoreCase,
            ignoreBlankLines: cur.ignoreBlank,
            minorUnimportant: cur.minorEnabled,
            algorithm: cur.alignmentMode === "patience" ? "patience" : "myers",
            alignmentMode: cur.alignmentMode,
            neverAlignDifferences: cur.neverAlignDifferences,
            skewTolerance: cur.skewTolerance,
            useClosenessMatching: cur.useClosenessMatching,
            encoding: leftEnc,
            leftEncoding: leftEnc,
            rightEncoding: rightEnc,
            eol: cur.eol,
          });
        }
        setSummary(dto);
        const fetched = await fetchSessionRows(dto.session, dto.rows);
        undoStackRef.current = [];
        redoStackRef.current = [];
        setManualAnchors([]);
        setAlignSource(null);
        setRows(fetched);
        if (dto.encoding) setDetectedEncoding(dto.encoding);
        if (dto.eol) setDetectedEol(dto.eol);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setLoading(false);
      }
    },
    [
      rules.ignoreWs,
      rules.ignoreCase,
      rules.ignoreBlank,
      rules.minorEnabled,
      rules.alignmentMode,
      rules.neverAlignDifferences,
      rules.skewTolerance,
      rules.useClosenessMatching,
      rules.leftEncoding,
      rules.rightEncoding,
      rules.sameEncoding,
      rules.eol,
    ],
  );

  useEffect(() => {
    void runCompare(leftPath, rightPath);
  }, [leftPath, rightPath, runCompare]);

  const leftSyntaxLang = useMemo(
    () => resolveSyntaxLang(rules.leftFormat, leftDraft),
    [rules.leftFormat, leftDraft],
  );
  const rightSyntaxLang = useMemo(
    () => resolveSyntaxLang(rules.sameFormat ? rules.leftFormat : rules.rightFormat, rightDraft),
    [rules.sameFormat, rules.leftFormat, rules.rightFormat, rightDraft],
  );

  // Apply manual alignment anchors (F7 / Align With...) if any are active
  const anchoredRows = useMemo(() => {
    if (manualAnchors.length === 0) return rows;
    const leftLines = extractSideLines(rows, "left");
    const rightLines = extractSideLines(rows, "right");
    return applyManualAnchors(leftLines, rightLines, manualAnchors) as readonly TextRowDto[];
  }, [rows, manualAnchors]);

  const manualAnchorRowIndices = useMemo(() => {
    const set = new Set<number>();
    if (manualAnchors.length === 0) return set;
    let lIdx = 0;
    let rIdx = 0;
    for (const row of anchoredRows) {
      const curL = row.left !== null ? lIdx++ : -1;
      const curR = row.right !== null ? rIdx++ : -1;
      if (manualAnchors.some((a) => a.leftLineIdx === curL && a.rightLineIdx === curR)) {
        set.add(row.index);
      }
    }
    return set;
  }, [anchoredRows, manualAnchors]);

  // Classify Unimportant differences (Blue tint when Minor OFF, folded to equal when Minor ON)
  const { effectiveRows, unimportantIndices } = useMemo(() => {
    const hasCustomUnimp = rules.unimportantRules.some((r) => r.enabled && r.pattern.length > 0);
    const hasRep = rules.replacements.some((r) => r.enabled && r.left.length > 0);
    const blueSet = new Set<number>();

    const nextRows = anchoredRows.map((row) => {
      if (row.kind !== "changed" || row.left === null || row.right === null) return row;

      const matchedByRules =
        (hasCustomUnimp || hasRep) &&
        matchesUnderSessionRules(
          row.left,
          row.right,
          rules.unimportantRules,
          rules.replacements,
          true,
        );
      const wsOrCaseOnly =
        !hasCustomUnimp &&
        !hasRep &&
        row.left.trim().replace(/\s+/g, " ").toLowerCase() ===
          row.right.trim().replace(/\s+/g, " ").toLowerCase();

      if (matchedByRules || wsOrCaseOnly) {
        if (rules.minorEnabled) {
          return { ...row, kind: "equal" as const, inline: [] };
        }
        blueSet.add(row.index);
      }
      return row;
    });
    return { effectiveRows: nextRows, unimportantIndices: blueSet };
  }, [anchoredRows, rules.unimportantRules, rules.replacements, rules.minorEnabled]);

  const startAlignWith = useCallback(
    (rowIndex: number, preferredSide: TextSide): void => {
      const rowObj = effectiveRows.find((r) => r.index === rowIndex);
      if (!rowObj) return;
      const side: TextSide =
        preferredSide === "left"
          ? rowObj.left !== null
            ? "left"
            : "right"
          : rowObj.right !== null
            ? "right"
            : "left";
      const sideLineIdx = rowIndexToSideLineIndex(effectiveRows, rowIndex, side);
      if (sideLineIdx < 0) return;
      setActiveEdit(null);
      setContextMenu(null);
      setAlignSource({ side, sideLineIdx, rowIndex });
    },
    [effectiveRows],
  );

  const handlePickAlignTarget = useCallback(
    (rowIndex: number, side: TextSide): void => {
      if (!alignSource || alignSource.side === side) return;
      const targetSideIdx = rowIndexToSideLineIndex(effectiveRows, rowIndex, side);
      if (targetSideIdx < 0) return;
      const nextAnchor: ManualAnchor =
        alignSource.side === "left"
          ? { leftLineIdx: alignSource.sideLineIdx, rightLineIdx: targetSideIdx }
          : { leftLineIdx: targetSideIdx, rightLineIdx: alignSource.sideLineIdx };
      setManualAnchors((prev) => addManualAnchor(prev, nextAnchor));
      setAlignSource(null);
      setNotice(
        `Aligned Left line ${nextAnchor.leftLineIdx + 1} with Right line ${nextAnchor.rightLineIdx + 1}`,
      );
    },
    [alignSource, effectiveRows],
  );

  const handleContextMenuCell = useCallback(
    (e: React.MouseEvent, rowIndex: number, side: TextSide): void => {
      e.preventDefault();
      setSelectionAnchor(rowIndex);
      setSelectedIndex(rowIndex);
      setContextMenu({ x: e.clientX, y: e.clientY, rowIndex, side });
    },
    [],
  );

  // Global Cmd+F, F7 (Align With...), and Escape
  useEffect(() => {
    const onKey = (e: KeyboardEvent): void => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "f") {
        e.preventDefault();
        setFindOpen(true);
        setTimeout(() => findInputRef.current?.focus(), 20);
      } else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "h") {
        e.preventDefault();
        setFindOpen(true);
        setReplaceOpen(true);
        setTimeout(() => findInputRef.current?.focus(), 20);
      } else if (e.key === "F7") {
        e.preventDefault();
        const targetIdx = activeEditRef.current?.rowIndex ?? selectedIndex;
        if (targetIdx !== null) {
          startAlignWith(targetIdx, activeEditRef.current?.side ?? "left");
        }
      } else if (e.key === "Escape") {
        if (contextMenu) setContextMenu(null);
        else if (alignSource) setAlignSource(null);
        else if (findOpen) setFindOpen(false);
      }
    };
    const onWinClick = (): void => {
      if (contextMenu) setContextMenu(null);
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("click", onWinClick);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("click", onWinClick);
    };
  }, [findOpen, alignSource, contextMenu, selectedIndex, startAlignWith]);

  const hunks = useMemo(() => collectHunks(effectiveRows), [effectiveRows]);

  const hunkMetaByIndex = useMemo(() => {
    const map = new Map<number, HunkMeta>();
    for (const h of hunks) {
      const members = effectiveRows.filter(
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
  }, [hunks, effectiveRows]);

  const lineNosByIndex = useMemo(() => {
    let l = 0;
    let r = 0;
    const map = new Map<number, { left: number | null; right: number | null }>();
    for (const row of effectiveRows) {
      map.set(row.index, {
        left: row.left !== null ? ++l : null,
        right: row.right !== null ? ++r : null,
      });
    }
    return map;
  }, [effectiveRows]);

  const visibleRows = useMemo(() => {
    if (filterMode === "all") return effectiveRows;
    if (filterMode === "same") return effectiveRows.filter((r) => r.kind === "equal");
    if (!contextEnabled) return effectiveRows.filter((r) => r.kind !== "equal");
    const keep = new Set<number>();
    effectiveRows.forEach((r, idx) => {
      if (r.kind === "equal") return;
      for (let d = -3; d <= 3; d += 1) {
        const nb = effectiveRows[idx + d];
        if (nb) keep.add(nb.index);
      }
    });
    return effectiveRows.filter((r) => keep.has(r.index));
  }, [effectiveRows, filterMode, contextEnabled]);

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
    const raw = findQuery;
    if (!raw.trim()) return [] as number[];
    const q = findMatchCase ? raw : raw.toLowerCase();
    return visibleRows
      .filter((r) => {
        const l = findMatchCase ? (r.left ?? "") : (r.left ?? "").toLowerCase();
        const ri = findMatchCase ? (r.right ?? "") : (r.right ?? "").toLowerCase();
        return l.includes(q) || ri.includes(q);
      })
      .map((r) => r.index);
  }, [findQuery, findMatchCase, visibleRows]);

  const handleSelectRow = useCallback(
    (rowIndex: number, shiftKey: boolean): void => {
      if (shiftKey && selectionAnchor !== null) {
        setSelectedIndex(rowIndex);
      } else {
        setSelectionAnchor(rowIndex);
        setSelectedIndex(rowIndex);
      }
    },
    [selectionAnchor],
  );

  const selectionRange = useMemo(() => {
    if (selectedIndex === null) return null;
    const anchor = selectionAnchor ?? selectedIndex;
    return {
      start: Math.min(anchor, selectedIndex),
      end: Math.max(anchor, selectedIndex),
    };
  }, [selectedIndex, selectionAnchor]);

  const selectedRangeCount = selectionRange
    ? selectionRange.end - selectionRange.start + 1
    : 0;

  const selectionHasDiff = useMemo(() => {
    if (!selectionRange) return false;
    return effectiveRows.some(
      (r) => r.index >= selectionRange.start && r.index <= selectionRange.end && r.kind !== "equal",
    );
  }, [effectiveRows, selectionRange]);

  const scrollToRow = useCallback(
    (rowIndex: number): void => {
      setSelectionAnchor(rowIndex);
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

  const handleUndo = useCallback(async (): Promise<void> => {
    const prev = undoStackRef.current.pop();
    if (!prev) return;
    setActiveEdit(null);
    redoStackRef.current = [...redoStackRef.current.slice(-29), { rows: rowsRef.current, label: prev.label }];
    setRows(prev.rows);
    onDirtyChange(undoStackRef.current.length > 0);
    setNotice(`Undid ${prev.label}`);
    if (summary) {
      try {
        await textRestoreRows(summary.session, prev.rows);
      } catch {
        // Local snapshot already restored
      }
    }
  }, [summary, onDirtyChange]);

  const handleRedo = useCallback(async (): Promise<void> => {
    const next = redoStackRef.current.pop();
    if (!next) return;
    setActiveEdit(null);
    undoStackRef.current = [...undoStackRef.current.slice(-29), { rows: rowsRef.current, label: next.label }];
    setRows(next.rows);
    onDirtyChange(true);
    setNotice(`Redid ${next.label}`);
    if (summary) {
      try {
        await textRestoreRows(summary.session, next.rows);
      } catch {
        // Local snapshot already restored
      }
    }
  }, [summary, onDirtyChange]);

  useEffect(() => {
    const onUndoKey = (e: KeyboardEvent): void => {
      const mod = e.metaKey || e.ctrlKey;
      if (!mod) return;
      const key = e.key.toLowerCase();
      if (key === "z" && !e.shiftKey) {
        if (undoStackRef.current.length > 0) {
          e.preventDefault();
          void handleUndo();
        }
      } else if ((key === "z" && e.shiftKey) || key === "y") {
        if (redoStackRef.current.length > 0) {
          e.preventDefault();
          void handleRedo();
        }
      }
    };
    window.addEventListener("keydown", onUndoKey);
    return () => window.removeEventListener("keydown", onUndoKey);
  }, [handleUndo, handleRedo]);

  // Apply updated full side buffer and live re-diff via Rust `text_update_side`
  const applySideBuffer = useCallback(
    async (
      side: TextSide,
      nextLines: readonly string[],
      label: string,
      focusTarget?: { sideLineIdx: number; col: number },
    ): Promise<void> => {
      if (!summary) return;
      const snapshot = rowsRef.current;
      try {
        const total = await textUpdateSide(summary.session, side, nextLines);
        const fetched = await fetchSessionRows(summary.session, Math.max(total, nextLines.length));
        pushUndoSnapshot(snapshot, label);
        setRows(fetched);
        onDirtyChange(true);
        if (focusTarget) {
          const targetRow = sideLineIndexToRowIndex(fetched, focusTarget.sideLineIdx, side);
          if (targetRow >= 0) {
            const rowObj = fetched.find((r) => r.index === targetRow);
            const text = (side === "left" ? rowObj?.left : rowObj?.right) ?? "";
            setSelectionAnchor(targetRow);
            setSelectedIndex(targetRow);
            setActiveEdit((prev) => ({
              rowIndex: targetRow,
              side,
              text,
              original: text,
              col: focusTarget.col,
              focusNonce: (prev?.focusNonce ?? 0) + 1,
            }));
          } else {
            setActiveEdit(null);
          }
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    },
    [summary, onDirtyChange, pushUndoSnapshot],
  );

  // Flush current line edits to backend immediately
  const flushActiveEdit = useCallback(
    async (ed: ActiveEditor | null): Promise<readonly TextRowDto[]> => {
      if (liveDiffTimerRef.current !== null) {
        clearTimeout(liveDiffTimerRef.current);
        liveDiffTimerRef.current = null;
      }
      if (!ed || !summary || ed.text === ed.original) return rowsRef.current;
      const sideIdx = rowIndexToSideLineIndex(rowsRef.current, ed.rowIndex, ed.side);
      if (sideIdx < 0) return rowsRef.current;
      const lines = extractSideLines(rowsRef.current, ed.side);
      lines[sideIdx] = ed.text;
      const snapshot = rowsRef.current;
      try {
        const total = await textUpdateSide(summary.session, ed.side, lines);
        const fetched = await fetchSessionRows(summary.session, Math.max(total, lines.length));
        pushUndoSnapshot(snapshot, `edit line ${sideIdx + 1} (${ed.side})`);
        setRows(fetched);
        onDirtyChange(true);
        return fetched;
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
        return rowsRef.current;
      }
    },
    [summary, onDirtyChange, pushUndoSnapshot],
  );

  const handleFocusCell = useCallback(
    (rowIndex: number, side: TextSide, text: string, col: number): void => {
      const prev = activeEditRef.current;
      if (prev && (prev.rowIndex !== rowIndex || prev.side !== side) && prev.text !== prev.original) {
        void flushActiveEdit(prev);
      }
      setActiveEdit((prev) => ({
        rowIndex,
        side,
        text,
        original: text,
        col,
        focusNonce: (prev?.focusNonce ?? 0) + 1,
      }));
    },
    [flushActiveEdit],
  );

  const handleCellChange = useCallback(
    (text: string, col: number): void => {
      setActiveEdit((prev) => (prev ? { ...prev, text, col } : null));
      onDirtyChange(true);
      if (liveDiffTimerRef.current !== null) clearTimeout(liveDiffTimerRef.current);
      liveDiffTimerRef.current = setTimeout(() => {
        const cur = activeEditRef.current;
        if (!cur || !summary || cur.text === cur.original) return;
        const sideIdx = rowIndexToSideLineIndex(rowsRef.current, cur.rowIndex, cur.side);
        if (sideIdx < 0) return;
        const lines = extractSideLines(rowsRef.current, cur.side);
        lines[sideIdx] = cur.text;
        void applySideBuffer(cur.side, lines, `edit line ${sideIdx + 1} (${cur.side})`, {
          sideLineIdx: sideIdx,
          col: cur.col,
        });
      }, 240);
    },
    [summary, onDirtyChange, applySideBuffer],
  );

  const handleCaretMove = useCallback((col: number): void => {
    setActiveEdit((prev) => (prev ? { ...prev, col } : null));
  }, []);

  const handleSplitEnter = useCallback(
    (rowIndex: number, side: TextSide, col: number): void => {
      if (liveDiffTimerRef.current !== null) {
        clearTimeout(liveDiffTimerRef.current);
        liveDiffTimerRef.current = null;
      }
      const sideIdx = rowIndexToSideLineIndex(rowsRef.current, rowIndex, side);
      if (sideIdx < 0) return;
      const lines = extractSideLines(rowsRef.current, side);
      if (activeEditRef.current?.rowIndex === rowIndex && activeEditRef.current.side === side) {
        lines[sideIdx] = activeEditRef.current.text;
      }
      const res = splitLineAt(lines, sideIdx, col);
      void applySideBuffer(side, res.lines, `insert line (${side})`, {
        sideLineIdx: res.nextLine,
        col: res.nextCol,
      });
    },
    [applySideBuffer],
  );

  const handleBackspaceStart = useCallback(
    (rowIndex: number, side: TextSide): void => {
      if (liveDiffTimerRef.current !== null) {
        clearTimeout(liveDiffTimerRef.current);
        liveDiffTimerRef.current = null;
      }
      const sideIdx = rowIndexToSideLineIndex(rowsRef.current, rowIndex, side);
      if (sideIdx <= 0) return;
      const lines = extractSideLines(rowsRef.current, side);
      if (activeEditRef.current?.rowIndex === rowIndex && activeEditRef.current.side === side) {
        lines[sideIdx] = activeEditRef.current.text;
      }
      const res = joinWithPreviousLine(lines, sideIdx);
      void applySideBuffer(side, res.lines, `join lines (${side})`, {
        sideLineIdx: res.nextLine,
        col: res.nextCol,
      });
    },
    [applySideBuffer],
  );

  const handleDeleteEnd = useCallback(
    (rowIndex: number, side: TextSide): void => {
      if (liveDiffTimerRef.current !== null) {
        clearTimeout(liveDiffTimerRef.current);
        liveDiffTimerRef.current = null;
      }
      const sideIdx = rowIndexToSideLineIndex(rowsRef.current, rowIndex, side);
      if (sideIdx < 0) return;
      const lines = extractSideLines(rowsRef.current, side);
      if (activeEditRef.current?.rowIndex === rowIndex && activeEditRef.current.side === side) {
        lines[sideIdx] = activeEditRef.current.text;
      }
      if (sideIdx + 1 >= lines.length) return;
      const res = joinWithNextLine(lines, sideIdx);
      void applySideBuffer(side, res.lines, `join lines (${side})`, {
        sideLineIdx: res.nextLine,
        col: res.nextCol,
      });
    },
    [applySideBuffer],
  );

  const handleArrowVertical = useCallback(
    (rowIndex: number, side: TextSide, delta: -1 | 1, col: number): void => {
      const cur = activeEditRef.current;
      if (cur && cur.text !== cur.original) {
        void flushActiveEdit(cur);
      }
      const sideIdx = rowIndexToSideLineIndex(rowsRef.current, rowIndex, side);
      if (sideIdx < 0) return;
      const nextSideIdx = sideIdx + delta;
      const nextRowIdx = sideLineIndexToRowIndex(rowsRef.current, nextSideIdx, side);
      if (nextRowIdx < 0) return;
      const rowObj = rowsRef.current.find((r) => r.index === nextRowIdx);
      const nextText = (side === "left" ? rowObj?.left : rowObj?.right) ?? "";
      setSelectionAnchor(nextRowIdx);
      setSelectedIndex(nextRowIdx);
      setActiveEdit((prev) => ({
        rowIndex: nextRowIdx,
        side,
        text: nextText,
        original: nextText,
        col: Math.min(col, nextText.length),
        focusNonce: (prev?.focusNonce ?? 0) + 1,
      }));
      const pos = visibleRows.findIndex((r) => r.index === nextRowIdx);
      if (pos >= 0 && (pos < startIndex + 2 || pos > endIndex - 4)) {
        scrollToPosition(pos);
      }
    },
    [flushActiveEdit, visibleRows, startIndex, endIndex, scrollToPosition],
  );

  const handleMultiLinePaste = useCallback(
    (rowIndex: number, side: TextSide, selStart: number, selEnd: number, pasted: string): void => {
      if (liveDiffTimerRef.current !== null) {
        clearTimeout(liveDiffTimerRef.current);
        liveDiffTimerRef.current = null;
      }
      const sideIdx = rowIndexToSideLineIndex(rowsRef.current, rowIndex, side);
      if (sideIdx < 0) return;
      const lines = extractSideLines(rowsRef.current, side);
      if (activeEditRef.current?.rowIndex === rowIndex && activeEditRef.current.side === side) {
        lines[sideIdx] = activeEditRef.current.text;
      }
      const res = insertPastedText(lines, sideIdx, selStart, selEnd, pasted);
      void applySideBuffer(side, res.lines, `paste (${side})`, {
        sideLineIdx: res.nextLine,
        col: res.nextCol,
      });
    },
    [applySideBuffer],
  );

  const handleBlurCommit = useCallback((): void => {
    const cur = activeEditRef.current;
    setActiveEdit(null);
    if (cur && cur.text !== cur.original) {
      void flushActiveEdit(cur);
    }
  }, [flushActiveEdit]);

  const handleCopyHunk = useCallback(
    async (hunk: TextHunk, dir: TextSide): Promise<void> => {
      if (!summary) return;
      setActiveEdit(null);
      const label = `copy section ${hunk.ordinal + 1} (${dir === "left" ? "L → R" : "R → L"})`;
      const snapshot = rowsRef.current;
      try {
        await textApplyHunk(summary.session, hunk.start, hunk.end, dir);
        const fetched = await fetchSessionRows(summary.session, Math.max(snapshot.length, summary.rows));
        pushUndoSnapshot(snapshot, label);
        setRows(fetched);
        onDirtyChange(true);
        setNotice(`Copied section ${hunk.ordinal + 1} (${dir === "left" ? "L → R" : "R → L"})`);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    },
    [summary, onDirtyChange, pushUndoSnapshot],
  );

  const handleCopyRange = useCallback(
    async (start: number, end: number, dir: TextSide): Promise<void> => {
      if (!summary) return;
      setActiveEdit(null);
      const count = end - start + 1;
      const label =
        count === 1
          ? `copy line ${start + 1} (${dir === "left" ? "L → R" : "R → L"})`
          : `copy lines ${start + 1}–${end + 1} (${dir === "left" ? "L → R" : "R → L"})`;
      const snapshot = rowsRef.current;
      try {
        await textApplyHunk(summary.session, start, end, dir);
        const fetched = await fetchSessionRows(summary.session, Math.max(snapshot.length, summary.rows));
        pushUndoSnapshot(snapshot, label);
        setRows(fetched);
        onDirtyChange(true);
        setNotice(
          count === 1
            ? `Copied line ${start + 1} (${dir === "left" ? "L → R" : "R → L"})`
            : `Copied ${count} lines (${start + 1}–${end + 1}, ${dir === "left" ? "L → R" : "R → L"})`,
        );
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    },
    [summary, onDirtyChange, pushUndoSnapshot],
  );

  const handleCopyLine = useCallback(
    async (rowIndex: number, dir: TextSide): Promise<void> => {
      await handleCopyRange(rowIndex, rowIndex, dir);
    },
    [handleCopyRange],
  );

  const handleReplace = useCallback(
    async (replaceAll: boolean): Promise<void> => {
      if (!summary || !findQuery) return;
      const sides: readonly TextSide[] =
        replaceTargetSide === "both" ? ["left", "right"] : [replaceTargetSide];
      let totalReplaced = 0;
      const activeRowIndex = !replaceAll && findHits.length > 0 ? findHits[findIdx % findHits.length] : undefined;

      for (const side of sides) {
        const sideLines = extractSideLines(rowsRef.current, side);
        const onlyLineIndex =
          activeRowIndex !== undefined
            ? rowIndexToSideLineIndex(rowsRef.current, activeRowIndex, side)
            : undefined;
        if (onlyLineIndex !== undefined && onlyLineIndex < 0) continue;
        const res = replaceInLines(sideLines, findQuery, replaceQuery, {
          matchCase: findMatchCase,
          onlyLineIndex,
        });
        if (res.count > 0) {
          totalReplaced += res.count;
          await applySideBuffer(
            side,
            res.lines,
            `replace ${res.count} occurrence${res.count === 1 ? "" : "s"} (${side})`,
          );
        }
      }
      setNotice(
        totalReplaced > 0
          ? `Replaced ${totalReplaced} occurrence${totalReplaced === 1 ? "" : "s"}`
          : `No matches to replace on ${replaceTargetSide} side`,
      );
    },
    [summary, findQuery, replaceQuery, findMatchCase, replaceTargetSide, findHits, findIdx, applySideBuffer],
  );

  const handleSaveSide = useCallback(
    async (side: TextSide): Promise<void> => {
      if (!summary) return;
      if (activeEditRef.current) {
        await flushActiveEdit(activeEditRef.current);
      }
      const targetPath = side === "left" ? leftDraft : rightDraft;
      if (!targetPath) {
        setNotice(`No file path set for ${side} side.`);
        return;
      }
      try {
        const saved = await textSaveSide(summary.session, side, targetPath);
        onDirtyChange(false);
        setNotice(`Saved ${side} (${saved.lines} lines, ${formatBytes(saved.bytes)})`);
        void resetBaseline();
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    },
    [summary, leftDraft, rightDraft, onDirtyChange, flushActiveEdit, resetBaseline],
  );

  const handleCopyHunkSync = useCallback(
    (hunk: TextHunk, dir: TextSide): void => {
      void handleCopyHunk(hunk, dir);
    },
    [handleCopyHunk],
  );

  const handleCopyLineSync = useCallback(
    (rowIndex: number, dir: TextSide): void => {
      void handleCopyLine(rowIndex, dir);
    },
    [handleCopyLine],
  );

  const selectedRow = useMemo(
    () =>
      selectedIndex === null
        ? null
        : (effectiveRows.find((r) => r.index === selectedIndex) ?? null),
    [effectiveRows, selectedIndex],
  );

  const selectedHunk = useMemo(() => {
    if (selectedIndex === null) return null;
    return hunkMetaByIndex.get(selectedIndex)?.hunk ?? null;
  }, [selectedIndex, hunkMetaByIndex]);

  const caretCoordLabel = useMemo(() => {
    if (activeEdit) {
      const nos = lineNosByIndex.get(activeEdit.rowIndex);
      const ln = (activeEdit.side === "left" ? nos?.left : nos?.right) ?? activeEdit.rowIndex + 1;
      return `${ln}: ${activeEdit.col + 1}`;
    }
    if (selectedIndex !== null) {
      const nos = lineNosByIndex.get(selectedIndex);
      const ln = nos?.left ?? nos?.right ?? selectedIndex + 1;
      return `${ln}: 1`;
    }
    return "";
  }, [activeEdit, selectedIndex, lineNosByIndex]);

  // Register unified top toolbar commands
  useEffect(() => {
    onRegisterCommands({
      filterMode,
      setFilterMode,
      contextEnabled,
      toggleContext: () => setContextEnabled((v) => !v),
      minorEnabled: rules.minorEnabled,
      toggleMinor: () => setRules((prev) => ({ ...prev, minorEnabled: !prev.minorEnabled })),
      openRules: () => setRulesModalTab("importance"),
      openFormat: () => setRulesModalTab("format"),
      openFind: () => {
        setFindOpen(true);
        setTimeout(() => findInputRef.current?.focus(), 20);
      },
      canCopy: selectionHasDiff || selectedHunk !== null,
      copySelected: () => {
        if (selectionRange && selectedRangeCount > 1 && selectionHasDiff) {
          void handleCopyRange(selectionRange.start, selectionRange.end, "left");
        } else if (selectedHunk) {
          void handleCopyHunk(selectedHunk, "left");
        }
      },
      copySelectedRight: () => {
        if (selectionRange && selectedRangeCount > 1 && selectionHasDiff) {
          void handleCopyRange(selectionRange.start, selectionRange.end, "right");
        } else if (selectedHunk) {
          void handleCopyHunk(selectedHunk, "right");
        }
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
    rules.minorEnabled,
    selectionRange,
    selectedRangeCount,
    selectionHasDiff,
    selectedHunk,
    handleCopyRange,
    handleCopyHunk,
    hunks.length,
    stepSection,
    onPathsChange,
    leftDraft,
    rightDraft,
    runCompare,
  ]);

  // Publish clean 3-zone status bar
  const activeReplacementCount = useMemo(
    () => rules.replacements.filter((r) => r.enabled && r.left.length > 0).length,
    [rules.replacements],
  );

  useEffect(() => {
    const alignLabel =
      rules.alignmentMode === "patience"
        ? "Patience"
        : rules.alignmentMode === "unaligned"
          ? "Unaligned"
          : effectiveAlgorithm === "myers" && rules.alignmentMode === "myers"
            ? "Myers O(ND)"
            : "Standard";
    const repLabel =
      activeReplacementCount > 0
        ? ` · ${activeReplacementCount} replacement${activeReplacementCount === 1 ? "" : "s"}`
        : "";
    onStatusChange({
      left:
        hunks.length === 0
          ? "Files are identical"
          : `${hunks.length} difference ${hunks.length === 1 ? "section" : "sections"}`,
      center: notice ?? `${alignLabel} · ${displayEncoding} · ${displayEol}${repLabel}`,
      right: `${caretCoordLabel ? `${caretCoordLabel}  ·  ` : ""}${
        summary ? `${summary.leftLines} / ${summary.rightLines} lines` : `${rows.length} rows`
      }`,
    });
  }, [
    onStatusChange,
    hunks.length,
    notice,
    rules.alignmentMode,
    effectiveAlgorithm,
    displayEncoding,
    displayEol,
    activeReplacementCount,
    caretCoordLabel,
    summary,
    rows.length,
  ]);

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
            <span>{displayEncoding}</span>
            <span>{displayEol}</span>
          </>
        }
        rightMeta={
          <>
            {summary?.rightMeta?.mtimeUnix ? <span>{formatMtime(summary.rightMeta.mtimeUnix)}</span> : null}
            {summary?.rightMeta?.bytes !== undefined ? <span>{formatBytes(summary.rightMeta.bytes)}</span> : null}
            {summary?.rightLines !== undefined ? <span>{summary.rightLines} lines</span> : null}
            <span>{rules.sameEncoding ? displayEncoding : rules.rightEncoding !== "None" ? rules.rightEncoding : detectedEncoding}</span>
            <span>{displayEol}</span>
          </>
        }
      />

      {findOpen ? (
        <V2TextFindBar
          inputRef={findInputRef}
          replaceInputRef={replaceInputRef}
          query={findQuery}
          replaceQuery={replaceQuery}
          replaceOpen={replaceOpen}
          matchCase={findMatchCase}
          targetSide={replaceTargetSide}
          hitCount={findHits.length}
          activeHit={findIdx}
          onQueryChange={(next) => {
            setFindQuery(next);
            setFindIdx(0);
          }}
          onReplaceQueryChange={setReplaceQuery}
          onToggleReplaceOpen={() => setReplaceOpen((v) => !v)}
          onToggleMatchCase={() => setFindMatchCase((v) => !v)}
          onTargetSideChange={setReplaceTargetSide}
          onStep={(delta) => {
            if (findHits.length === 0) return;
            const next = (findIdx + delta + findHits.length) % findHits.length;
            setFindIdx(next);
            scrollToRow(findHits[next]);
          }}
          onReplaceOne={() => void handleReplace(false)}
          onReplaceAll={() => void handleReplace(true)}
          onClose={() => {
            setFindOpen(false);
            setReplaceOpen(false);
          }}
        />
      ) : null}

      {alignSource || manualAnchors.length > 0 ? (
        <div
          data-testid="v2/align-banner"
          className="flex shrink-0 items-center justify-between gap-2 border-b border-[var(--v2-border)] bg-[var(--v2-minor-bg)] px-3 py-1 text-[11.5px] text-[var(--v2-minor-fg)]"
        >
          <span>
            {alignSource
              ? `Align With: click a line on the ${alignSource.side === "left" ? "right" : "left"} side to pin with ${alignSource.side} line ${alignSource.sideLineIdx + 1} (Esc to cancel)`
              : `${manualAnchors.length} manual alignment${manualAnchors.length === 1 ? "" : "s"} active`}
          </span>
          <div className="flex items-center gap-1.5">
            {alignSource ? (
              <button
                type="button"
                onClick={() => setAlignSource(null)}
                className="v2-btn h-[20px] px-2 text-[11px]"
              >
                Cancel
              </button>
            ) : null}
            {manualAnchors.length > 0 ? (
              <button
                type="button"
                onClick={() => {
                  setManualAnchors([]);
                  setAlignSource(null);
                  setNotice("Cleared manual alignments");
                }}
                className="v2-btn h-[20px] px-2 text-[11px]"
              >
                Clear Alignments
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

      {/* Main Workspace: Far-Left Overview Minimap + Virtualized Continuous Notepad Diff Grid */}
      <div className="flex min-h-0 flex-1 overflow-hidden">
        <div
          aria-label="Difference overview map"
          className="relative w-[14px] shrink-0 cursor-pointer border-r border-[var(--v2-border)] bg-[var(--v2-surface-subtle)]"
          onClick={(e) => {
            if (effectiveRows.length === 0) return;
            const rect = e.currentTarget.getBoundingClientRect();
            const ratio = Math.max(0, Math.min(1, (e.clientY - rect.top) / rect.height));
            const idx = Math.min(effectiveRows.length - 1, Math.floor(ratio * effectiveRows.length));
            scrollToRow(effectiveRows[idx].index);
          }}
        >
          {effectiveRows.length > 0 &&
            hunks.map((h) => {
              const firstRow = effectiveRows.find((r) => r.index === h.start);
              const topPct = (h.start / effectiveRows.length) * 100;
              const heightPct = Math.max(1.2, ((h.end - h.start + 1) / effectiveRows.length) * 100);
              const isMinorHunk = firstRow ? unimportantIndices.has(firstRow.index) : false;
              const color = isMinorHunk
                ? "var(--v2-ruler-minor)"
                : firstRow?.kind === "leftOnly"
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
                  const isActiveRow = activeEdit?.rowIndex === row.index;
                  const isInSelection =
                    selectionRange !== null &&
                    row.index >= selectionRange.start &&
                    row.index <= selectionRange.end;
                  return (
                    <V2TextDiffRow
                      key={row.index}
                      row={row}
                      meta={hunkMetaByIndex.get(row.index)}
                      leftLineNo={nos.left}
                      rightLineNo={nos.right}
                      isSelected={isInSelection}
                      isUnimportant={unimportantIndices.has(row.index)}
                      isManualAnchor={manualAnchorRowIndices.has(row.index)}
                      leftSyntaxLang={leftSyntaxLang}
                      rightSyntaxLang={rightSyntaxLang}
                      alignPickTargetSide={
                        alignSource ? (alignSource.side === "left" ? "right" : "left") : null
                      }
                      activeSide={isActiveRow ? activeEdit.side : null}
                      activeText={isActiveRow ? activeEdit.text : ""}
                      activeCol={isActiveRow ? activeEdit.col : 0}
                      focusNonce={isActiveRow ? activeEdit.focusNonce : 0}
                      splitPct={splitPct}
                      dragging={dragging}
                      onSelect={handleSelectRow}
                      onFocusCell={handleFocusCell}
                      onCellChange={handleCellChange}
                      onCaretMove={handleCaretMove}
                      onSplitEnter={handleSplitEnter}
                      onBackspaceStart={handleBackspaceStart}
                      onDeleteEnd={handleDeleteEnd}
                      onArrowVertical={handleArrowVertical}
                      onMultiLinePaste={handleMultiLinePaste}
                      onBlurCommit={handleBlurCommit}
                      onCopyHunk={handleCopyHunkSync}
                      onCopyLine={handleCopyLineSync}
                      onContextMenuCell={handleContextMenuCell}
                      onPickAlignTarget={handlePickAlignTarget}
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

      {contextMenu ? (
        <div
          style={{ left: contextMenu.x, top: contextMenu.y }}
          className="fixed z-50 min-w-[190px] rounded-[7px] border border-[var(--v2-border)] bg-[var(--v2-chrome-elevated)] py-1 text-[12px] shadow-lg"
          onClick={(e) => e.stopPropagation()}
        >
          <button
            type="button"
            onClick={() => startAlignWith(contextMenu.rowIndex, contextMenu.side)}
            className="flex w-full items-center justify-between px-3 py-1.5 text-left hover:bg-[var(--v2-accent-soft)]"
          >
            <span>Align With…</span>
            <span className="text-[10.5px] text-[var(--v2-fg-muted)]">F7</span>
          </button>
          <button
            type="button"
            onClick={() => {
              void handleCopyLine(contextMenu.rowIndex, "left");
              setContextMenu(null);
            }}
            className="flex w-full items-center justify-between px-3 py-1.5 text-left hover:bg-[var(--v2-accent-soft)]"
          >
            <span>Copy Line Left → Right</span>
          </button>
          <button
            type="button"
            onClick={() => {
              void handleCopyLine(contextMenu.rowIndex, "right");
              setContextMenu(null);
            }}
            className="flex w-full items-center justify-between px-3 py-1.5 text-left hover:bg-[var(--v2-accent-soft)]"
          >
            <span>Copy Line Right → Left</span>
          </button>
          {manualAnchors.length > 0 ? (
            <button
              type="button"
              onClick={() => {
                setManualAnchors([]);
                setAlignSource(null);
                setContextMenu(null);
                setNotice("Cleared manual alignments");
              }}
              className="mt-0.5 flex w-full items-center justify-between border-t border-[var(--v2-border-subtle)] px-3 py-1.5 text-left text-[var(--v2-del-fg)] hover:bg-[var(--v2-accent-soft)]"
            >
              <span>Clear Manual Alignments ({manualAnchors.length})</span>
            </button>
          ) : null}
        </div>
      ) : null}

      {/* Bottom Stacked Line Detail Pane with Selective Single-Line & Range Merge */}
      <div className="v2-mono shrink-0 border-t border-[var(--v2-border)] bg-[var(--v2-surface-subtle)] text-[11.5px]">
        <div className="grid grid-cols-[26px_1fr_auto] items-center border-b border-[var(--v2-border-subtle)] leading-[22px]">
          <span className="flex h-full items-center justify-center border-r border-[var(--v2-border-subtle)] text-[10px] font-semibold text-[var(--v2-fg-muted)]">
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
              : "Click anywhere to type directly, or Shift+Click a range to merge selectively"}
          </div>
          {selectionRange && selectionHasDiff ? (
            <button
              type="button"
              data-testid="v2/detail-copy-line-left"
              title={
                selectedRangeCount > 1
                  ? `Copy selected ${selectedRangeCount} lines Left → Right`
                  : "Copy only this selected line Left → Right"
              }
              onClick={() => void handleCopyRange(selectionRange.start, selectionRange.end, "left")}
              className="v2-btn mx-1.5 h-[18px] px-2 font-sans text-[10.5px]"
            >
              {selectedRangeCount > 1
                ? `Copy ${selectedRangeCount} Lines →`
                : selectedRow?.left === null
                  ? "Delete R Line"
                  : "Copy Line →"}
            </button>
          ) : null}
        </div>
        <div className="grid grid-cols-[26px_1fr_auto] items-center leading-[22px]">
          <span className="flex h-full items-center justify-center border-r border-[var(--v2-border-subtle)] text-[10px] font-semibold text-[var(--v2-fg-muted)]">
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
          {selectionRange && selectionHasDiff ? (
            <button
              type="button"
              data-testid="v2/detail-copy-line-right"
              title={
                selectedRangeCount > 1
                  ? `Copy selected ${selectedRangeCount} lines Right → Left`
                  : "Copy only this selected line Right → Left"
              }
              onClick={() => void handleCopyRange(selectionRange.start, selectionRange.end, "right")}
              className="v2-btn mx-1.5 h-[18px] px-2 font-sans text-[10.5px]"
            >
              {selectedRangeCount > 1
                ? `← Copy ${selectedRangeCount} Lines`
                : selectedRow?.right === null
                  ? "Delete L Line"
                  : "← Copy Line"}
            </button>
          ) : null}
        </div>
      </div>

      <V2TextRulesModal
        open={rulesModalTab !== null}
        initialTab={rulesModalTab ?? "importance"}
        rules={rules}
        leftPath={leftDraft}
        rightPath={rightDraft}
        onClose={() => setRulesModalTab(null)}
        onApply={(next, scope, paths) => {
          rulesRef.current = next;
          setRules(next);
          if (scope === "defaults") {
            saveDefaultRules(next);
          }
          if (paths) {
            onPathsChange(paths.left, paths.right);
          }
          setRulesModalTab(null);
          setNotice(
            scope === "defaults"
              ? "Applied rules and updated session defaults"
              : "Applied session rules to current view",
          );
        }}
      />
    </div>
  );
}
