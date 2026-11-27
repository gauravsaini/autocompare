import React, { useEffect, useRef } from "react";
import type { InlineSpanDto, TextRowDto, TextSide } from "../../../ipc/text";
import { tokenizeLine } from "../../utils/syntaxHighlight";
import type { SyntaxLang, SyntaxTokenType } from "../../utils/syntaxHighlight";

export type HunkRole = "single" | "start" | "middle" | "end";

export interface TextHunk {
  readonly ordinal: number;
  readonly start: number;
  readonly end: number;
}

export interface HunkMeta {
  readonly ordinal: number;
  readonly role: HunkRole;
  readonly hunk: TextHunk;
}

export interface ActiveCaret {
  readonly rowIndex: number;
  readonly side: TextSide;
  readonly col: number;
}

const SYNTAX_COLOR: Record<SyntaxTokenType, string | undefined> = {
  plain: undefined,
  keyword: "var(--v2-syn-keyword)",
  string: "var(--v2-syn-string)",
  comment: "var(--v2-syn-comment)",
  number: "var(--v2-syn-number)",
  key: "var(--v2-syn-key)",
  tag: "var(--v2-syn-tag)",
};

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

function SingleLineArrow({ dir }: { dir: TextSide }): React.JSX.Element {
  return (
    <svg
      width="11"
      height="11"
      viewBox="0 0 12 12"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {dir === "left" ? (
        <path d="M4 2.5 8 6 4 9.5" />
      ) : (
        <path d="M8 2.5 4 6 8 9.5" />
      )}
    </svg>
  );
}

function renderSyntaxSlice(text: string, syntaxLang?: SyntaxLang, keyPrefix = "s"): React.ReactNode {
  if (!syntaxLang || syntaxLang === "plain" || text.length === 0) return text;
  const tokens = tokenizeLine(text, syntaxLang);
  if (tokens.length === 0 || (tokens.length === 1 && tokens[0].type === "plain")) return text;
  return tokens.map((tok, i) => {
    const color = SYNTAX_COLOR[tok.type];
    const slice = text.slice(tok.start, tok.end);
    if (!color) return <React.Fragment key={`${keyPrefix}-${i}`}>{slice}</React.Fragment>;
    return (
      <span
        key={`${keyPrefix}-${i}`}
        style={{ color, fontStyle: tok.type === "comment" ? "italic" : undefined }}
      >
        {slice}
      </span>
    );
  });
}

export function renderHighlightedLine(
  text: string,
  spans: readonly InlineSpanDto[],
  side: TextSide,
  inlineBgVar: string,
  syntaxLang?: SyntaxLang,
): React.ReactNode {
  if (!spans || spans.length === 0) {
    return renderSyntaxSlice(text, syntaxLang, "eq");
  }
  const chars = Array.from(text);
  const ranges = spans
    .map((s) => ({
      start: Math.max(0, Math.min(chars.length, side === "left" ? s.aStart : s.bStart)),
      end: Math.max(0, Math.min(chars.length, side === "left" ? s.aEnd : s.bEnd)),
    }))
    .filter((r) => r.end > r.start)
    .sort((a, b) => a.start - b.start);

  if (ranges.length === 0) {
    return renderSyntaxSlice(text, syntaxLang, "no");
  }

  const nodes: React.ReactNode[] = [];
  let cursor = 0;
  ranges.forEach((r, idx) => {
    if (r.start > cursor) {
      const seg = chars.slice(cursor, r.start).join("");
      nodes.push(<span key={`t-${idx}`}>{renderSyntaxSlice(seg, syntaxLang, `t${idx}`)}</span>);
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
    const tail = chars.slice(cursor).join("");
    nodes.push(<span key="tail">{renderSyntaxSlice(tail, syntaxLang, "tail")}</span>);
  }
  return nodes;
}

interface SeamlessLineInputProps {
  readonly value: string;
  readonly targetCol: number;
  readonly focusNonce: number;
  readonly onChange: (text: string, col: number) => void;
  readonly onCaretMove: (col: number) => void;
  readonly onSplitEnter: (col: number) => void;
  readonly onBackspaceStart: () => void;
  readonly onDeleteEnd: () => void;
  readonly onArrowVertical: (delta: -1 | 1, col: number) => void;
  readonly onMultiLinePaste: (selStart: number, selEnd: number, text: string) => void;
  readonly onBlurCommit: () => void;
}

function SeamlessLineInput({
  value,
  targetCol,
  focusNonce,
  onChange,
  onCaretMove,
  onSplitEnter,
  onBackspaceStart,
  onDeleteEnd,
  onArrowVertical,
  onMultiLinePaste,
  onBlurCommit,
}: SeamlessLineInputProps): React.JSX.Element {
  const ref = useRef<HTMLInputElement | null>(null);
  const targetColRef = useRef(targetCol);
  targetColRef.current = targetCol;

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    el.focus();
    const clamped = Math.max(0, Math.min(el.value.length, targetColRef.current));
    el.setSelectionRange(clamped, clamped);
  }, [focusNonce]);

  return (
    <input
      ref={ref}
      type="text"
      spellCheck={false}
      autoComplete="off"
      value={value}
      onChange={(e) => onChange(e.target.value, e.target.selectionStart ?? e.target.value.length)}
      onSelect={(e) => onCaretMove(e.currentTarget.selectionStart ?? 0)}
      onKeyDown={(e) => {
        const el = e.currentTarget;
        const start = el.selectionStart ?? 0;
        const end = el.selectionEnd ?? start;
        if (e.key === "Enter") {
          e.preventDefault();
          onSplitEnter(start);
        } else if (e.key === "Backspace" && start === 0 && end === 0) {
          e.preventDefault();
          onBackspaceStart();
        } else if (e.key === "Delete" && start === el.value.length && end === el.value.length) {
          e.preventDefault();
          onDeleteEnd();
        } else if (e.key === "ArrowUp") {
          e.preventDefault();
          onArrowVertical(-1, start);
        } else if (e.key === "ArrowDown") {
          e.preventDefault();
          onArrowVertical(1, start);
        } else if (e.key === "Escape") {
          el.blur();
        }
      }}
      onPaste={(e) => {
        const text = e.clipboardData?.getData("text/plain") ?? "";
        if (text.includes("\n") || text.includes("\r")) {
          e.preventDefault();
          const el = e.currentTarget;
          onMultiLinePaste(el.selectionStart ?? 0, el.selectionEnd ?? 0, text);
        }
      }}
      onBlur={onBlurCommit}
      className="v2-mono h-[21px] w-full border-0 bg-transparent p-0 text-[12px] leading-[21px] text-inherit outline-none"
    />
  );
}

export interface V2TextDiffRowProps {
  readonly row: TextRowDto;
  readonly meta?: HunkMeta;
  readonly leftLineNo: number | null;
  readonly rightLineNo: number | null;
  readonly isSelected: boolean;
  readonly isUnimportant?: boolean;
  readonly isManualAnchor?: boolean;
  readonly leftSyntaxLang?: SyntaxLang;
  readonly rightSyntaxLang?: SyntaxLang;
  readonly alignPickTargetSide?: TextSide | null;
  readonly activeSide: TextSide | null;
  readonly activeText: string;
  readonly activeCol: number;
  readonly focusNonce: number;
  readonly splitPct: number;
  readonly dragging: boolean;
  readonly onSelect: (index: number, shiftKey: boolean) => void;
  readonly onFocusCell: (index: number, side: TextSide, text: string, col: number) => void;
  readonly onCellChange: (text: string, col: number) => void;
  readonly onCaretMove: (col: number) => void;
  readonly onSplitEnter: (index: number, side: TextSide, col: number) => void;
  readonly onBackspaceStart: (index: number, side: TextSide) => void;
  readonly onDeleteEnd: (index: number, side: TextSide) => void;
  readonly onArrowVertical: (index: number, side: TextSide, delta: -1 | 1, col: number) => void;
  readonly onMultiLinePaste: (
    index: number,
    side: TextSide,
    selStart: number,
    selEnd: number,
    pasted: string,
  ) => void;
  readonly onBlurCommit: () => void;
  readonly onCopyHunk: (hunk: TextHunk, dir: TextSide) => void;
  readonly onCopyLine: (index: number, dir: TextSide) => void;
  readonly onContextMenuCell?: (e: React.MouseEvent, index: number, side: TextSide) => void;
  readonly onPickAlignTarget?: (index: number, side: TextSide) => void;
  readonly onStartDrag: (e: React.MouseEvent) => void;
  readonly onResetSplit: () => void;
}

function estimateClickCol(e: React.MouseEvent<HTMLDivElement>, text: string): number {
  const rect = e.currentTarget.getBoundingClientRect();
  const relX = Math.max(0, e.clientX - rect.left - 8);
  const charWidth = 7.22;
  return Math.max(0, Math.min(text.length, Math.round(relX / charWidth)));
}

const V2TextDiffRow = React.memo(function V2TextDiffRow({
  row,
  meta,
  leftLineNo,
  rightLineNo,
  isSelected,
  isUnimportant = false,
  isManualAnchor = false,
  leftSyntaxLang = "plain",
  rightSyntaxLang = "plain",
  alignPickTargetSide = null,
  activeSide,
  activeText,
  activeCol,
  focusNonce,
  splitPct,
  dragging,
  onSelect,
  onFocusCell,
  onCellChange,
  onCaretMove,
  onSplitEnter,
  onBackspaceStart,
  onDeleteEnd,
  onArrowVertical,
  onMultiLinePaste,
  onBlurCommit,
  onCopyHunk,
  onCopyLine,
  onContextMenuCell,
  onPickAlignTarget,
  onStartDrag,
  onResetSplit,
}: V2TextDiffRowProps): React.JSX.Element {
  const showBlockArrow = meta && (meta.role === "single" || meta.role === "start");
  const showLineArrow = meta && (meta.role === "middle" || meta.role === "end");
  const railClass =
    !meta || meta.role === "single"
      ? ""
      : meta.role === "start"
        ? "v2-rail-start"
        : meta.role === "middle"
          ? "v2-rail-middle"
          : "v2-rail-end";

  const inlineBg = isUnimportant ? "var(--v2-minor-inline)" : "var(--v2-chg-inline)";

  const leftBg =
    isSelected && activeSide !== "left"
      ? "var(--v2-sel-bg)"
      : row.left === null
        ? undefined
        : isUnimportant
          ? "var(--v2-minor-bg)"
          : row.kind === "leftOnly"
            ? "var(--v2-del-bg)"
            : row.kind === "changed"
              ? "var(--v2-chg-bg)"
              : undefined;
  const leftFg =
    isSelected && activeSide !== "left"
      ? "var(--v2-sel-fg)"
      : isUnimportant
        ? "var(--v2-minor-fg)"
        : row.kind === "leftOnly"
          ? "var(--v2-del-fg)"
          : row.kind === "changed"
            ? "var(--v2-chg-fg)"
            : "var(--v2-fg)";

  const rightBg =
    isSelected && activeSide !== "right"
      ? "var(--v2-sel-bg)"
      : row.right === null
        ? undefined
        : isUnimportant
          ? "var(--v2-minor-bg)"
          : row.kind === "rightOnly"
            ? "var(--v2-ins-bg)"
            : row.kind === "changed"
              ? "var(--v2-chg-bg)"
              : undefined;
  const rightFg =
    isSelected && activeSide !== "right"
      ? "var(--v2-sel-fg)"
      : isUnimportant
        ? "var(--v2-minor-fg)"
        : row.kind === "rightOnly"
          ? "var(--v2-ins-fg)"
          : row.kind === "changed"
            ? "var(--v2-chg-fg)"
            : "var(--v2-fg)";

  const isLeftPickTarget = alignPickTargetSide === "left" && row.left !== null;
  const isRightPickTarget = alignPickTargetSide === "right" && row.right !== null;

  return (
    <div
      data-testid={`v2/text-row-${row.index}`}
      data-row-index={row.index}
      data-unimportant={isUnimportant ? "true" : undefined}
      style={{
        gridTemplateColumns: `22px 42px minmax(0, ${splitPct}fr) 6px 22px 42px minmax(0, ${100 - splitPct}fr)`,
      }}
      className="group grid h-[21px] items-stretch"
    >
      {/* Left Action Gutter */}
      <span
        className={`relative flex items-center justify-center bg-[var(--v2-surface-subtle)] ${railClass}`}
      >
        {showBlockArrow && meta ? (
          <button
            type="button"
            data-testid={`v2/copy-block-left-${row.index}`}
            title={
              meta.role === "single"
                ? "Copy line Left → Right"
                : "Copy block Left → Right (Option+Click for this line only)"
            }
            onClick={(e) => {
              e.stopPropagation();
              if (e.altKey || meta.role === "single") {
                onCopyLine(row.index, "left");
              } else {
                onCopyHunk(meta.hunk, "left");
              }
            }}
            className={`relative z-10 inline-flex h-[15px] w-[15px] items-center justify-center rounded bg-[var(--v2-surface-subtle)] hover:bg-[var(--v2-btn-hover)] ${
              isUnimportant ? "text-[var(--v2-minor-fg)]" : "text-[var(--v2-amber)]"
            }`}
          >
            <TakeArrow dir="left" />
          </button>
        ) : showLineArrow ? (
          <button
            type="button"
            data-testid={`v2/copy-line-left-${row.index}`}
            title="Copy only this line Left → Right"
            onClick={(e) => {
              e.stopPropagation();
              onCopyLine(row.index, "left");
            }}
            className={`relative z-10 inline-flex h-[15px] w-[15px] items-center justify-center rounded bg-[var(--v2-surface-subtle)] text-[var(--v2-accent)] transition-opacity hover:bg-[var(--v2-btn-hover)] ${
              isSelected ? "opacity-100" : "opacity-0 group-hover:opacity-100"
            }`}
          >
            <SingleLineArrow dir="left" />
          </button>
        ) : null}
      </span>

      {/* Left Line Number */}
      <span
        onClick={(e) => onSelect(row.index, e.shiftKey)}
        className="relative cursor-pointer select-none border-r border-[var(--v2-border-subtle)] bg-[var(--v2-surface-subtle)] pr-2 text-right text-[11px] text-[var(--v2-fg-muted)]"
      >
        {isManualAnchor ? (
          <span
            title="Manually aligned line"
            className="absolute left-1 top-1/2 h-1.5 w-1.5 -translate-y-1/2 rounded-full bg-[var(--v2-accent)]"
          />
        ) : null}
        {leftLineNo ?? ""}
      </span>

      {/* Left Continuous Notepad Cell */}
      <div
        onContextMenu={(e) => {
          if (onContextMenuCell) onContextMenuCell(e, row.index, "left");
        }}
        onClick={(e) => {
          if (isLeftPickTarget && onPickAlignTarget) {
            e.preventDefault();
            onPickAlignTarget(row.index, "left");
            return;
          }
          onSelect(row.index, e.shiftKey);
          if (!e.shiftKey && row.left !== null && activeSide !== "left") {
            onFocusCell(row.index, "left", row.left, estimateClickCol(e, row.left));
          }
        }}
        style={{ backgroundColor: leftBg, color: leftFg }}
        className={`truncate whitespace-pre px-2 ${
          isLeftPickTarget
            ? "cursor-crosshair hover:ring-1 hover:ring-inset hover:ring-[var(--v2-accent)]"
            : row.left === null && !isSelected
              ? "v2-hatch cursor-default"
              : "cursor-text"
        }`}
      >
        {activeSide === "left" ? (
          <SeamlessLineInput
            value={activeText}
            targetCol={activeCol}
            focusNonce={focusNonce}
            onChange={onCellChange}
            onCaretMove={onCaretMove}
            onSplitEnter={(col) => onSplitEnter(row.index, "left", col)}
            onBackspaceStart={() => onBackspaceStart(row.index, "left")}
            onDeleteEnd={() => onDeleteEnd(row.index, "left")}
            onArrowVertical={(delta, col) => onArrowVertical(row.index, "left", delta, col)}
            onMultiLinePaste={(s, end, txt) => onMultiLinePaste(row.index, "left", s, end, txt)}
            onBlurCommit={onBlurCommit}
          />
        ) : row.left !== null ? (
          renderHighlightedLine(
            row.left,
            row.inline,
            "left",
            inlineBg,
            row.kind === "equal" ? leftSyntaxLang : "plain",
          )
        ) : (
          ""
        )}
      </div>

      {/* Draggable Center Splitter */}
      <span
        role="separator"
        aria-orientation="vertical"
        title="Drag to resize panes (double-click to reset 50/50)"
        onMouseDown={onStartDrag}
        onDoubleClick={onResetSplit}
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
        {showBlockArrow && meta ? (
          <button
            type="button"
            data-testid={`v2/copy-block-right-${row.index}`}
            title={
              meta.role === "single"
                ? "Copy line Right → Left"
                : "Copy block Right → Left (Option+Click for this line only)"
            }
            onClick={(e) => {
              e.stopPropagation();
              if (e.altKey || meta.role === "single") {
                onCopyLine(row.index, "right");
              } else {
                onCopyHunk(meta.hunk, "right");
              }
            }}
            className={`relative z-10 inline-flex h-[15px] w-[15px] items-center justify-center rounded bg-[var(--v2-surface-subtle)] hover:bg-[var(--v2-btn-hover)] ${
              isUnimportant ? "text-[var(--v2-minor-fg)]" : "text-[var(--v2-amber)]"
            }`}
          >
            <TakeArrow dir="right" />
          </button>
        ) : showLineArrow ? (
          <button
            type="button"
            data-testid={`v2/copy-line-right-${row.index}`}
            title="Copy only this line Right → Left"
            onClick={(e) => {
              e.stopPropagation();
              onCopyLine(row.index, "right");
            }}
            className={`relative z-10 inline-flex h-[15px] w-[15px] items-center justify-center rounded bg-[var(--v2-surface-subtle)] text-[var(--v2-accent)] transition-opacity hover:bg-[var(--v2-btn-hover)] ${
              isSelected ? "opacity-100" : "opacity-0 group-hover:opacity-100"
            }`}
          >
            <SingleLineArrow dir="right" />
          </button>
        ) : null}
      </span>

      {/* Right Line Number */}
      <span
        onClick={(e) => onSelect(row.index, e.shiftKey)}
        className="relative cursor-pointer select-none border-r border-[var(--v2-border-subtle)] bg-[var(--v2-surface-subtle)] pr-2 text-right text-[11px] text-[var(--v2-fg-muted)]"
      >
        {isManualAnchor ? (
          <span
            title="Manually aligned line"
            className="absolute left-1 top-1/2 h-1.5 w-1.5 -translate-y-1/2 rounded-full bg-[var(--v2-accent)]"
          />
        ) : null}
        {rightLineNo ?? ""}
      </span>

      {/* Right Continuous Notepad Cell */}
      <div
        onContextMenu={(e) => {
          if (onContextMenuCell) onContextMenuCell(e, row.index, "right");
        }}
        onClick={(e) => {
          if (isRightPickTarget && onPickAlignTarget) {
            e.preventDefault();
            onPickAlignTarget(row.index, "right");
            return;
          }
          onSelect(row.index, e.shiftKey);
          if (!e.shiftKey && row.right !== null && activeSide !== "right") {
            onFocusCell(row.index, "right", row.right, estimateClickCol(e, row.right));
          }
        }}
        style={{ backgroundColor: rightBg, color: rightFg }}
        className={`truncate whitespace-pre px-2 ${
          isRightPickTarget
            ? "cursor-crosshair hover:ring-1 hover:ring-inset hover:ring-[var(--v2-accent)]"
            : row.right === null && !isSelected
              ? "v2-hatch cursor-default"
              : "cursor-text"
        }`}
      >
        {activeSide === "right" ? (
          <SeamlessLineInput
            value={activeText}
            targetCol={activeCol}
            focusNonce={focusNonce}
            onChange={onCellChange}
            onCaretMove={onCaretMove}
            onSplitEnter={(col) => onSplitEnter(row.index, "right", col)}
            onBackspaceStart={() => onBackspaceStart(row.index, "right")}
            onDeleteEnd={() => onDeleteEnd(row.index, "right")}
            onArrowVertical={(delta, col) => onArrowVertical(row.index, "right", delta, col)}
            onMultiLinePaste={(s, end, txt) => onMultiLinePaste(row.index, "right", s, end, txt)}
            onBlurCommit={onBlurCommit}
          />
        ) : row.right !== null ? (
          renderHighlightedLine(
            row.right,
            row.inline,
            "right",
            inlineBg,
            row.kind === "equal" ? rightSyntaxLang : "plain",
          )
        ) : (
          ""
        )}
      </div>
    </div>
  );
});

export default V2TextDiffRow;
