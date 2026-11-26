import React from "react";
import type { InlineSpanDto, TextRowDto, TextSide } from "../../../ipc/text";

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

export interface EditingCell {
  readonly index: number;
  readonly side: TextSide;
  readonly text: string;
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

export function renderHighlightedLine(
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

export interface V2TextDiffRowProps {
  readonly row: TextRowDto;
  readonly meta?: HunkMeta;
  readonly leftLineNo: number | null;
  readonly rightLineNo: number | null;
  readonly isSelected: boolean;
  readonly editingSide: TextSide | null;
  readonly editingText: string;
  readonly splitPct: number;
  readonly dragging: boolean;
  readonly onSelect: (index: number) => void;
  readonly onCopyHunk: (hunk: TextHunk, dir: TextSide) => void;
  readonly onCopyLine: (index: number, dir: TextSide) => void;
  readonly onStartEdit: (index: number, side: TextSide, text: string) => void;
  readonly onEditChange: (text: string) => void;
  readonly onCommitEdit: () => void;
  readonly onCancelEdit: () => void;
  readonly onStartDrag: (e: React.MouseEvent) => void;
  readonly onResetSplit: () => void;
}

const V2TextDiffRow = React.memo(function V2TextDiffRow({
  row,
  meta,
  leftLineNo,
  rightLineNo,
  isSelected,
  editingSide,
  editingText,
  splitPct,
  dragging,
  onSelect,
  onCopyHunk,
  onCopyLine,
  onStartEdit,
  onEditChange,
  onCommitEdit,
  onCancelEdit,
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
      data-testid={`v2/text-row-${row.index}`}
      data-row-index={row.index}
      onClick={() => onSelect(row.index)}
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
            className="relative z-10 inline-flex h-[15px] w-[15px] items-center justify-center rounded bg-[var(--v2-surface-subtle)] text-[var(--v2-amber)] hover:bg-[var(--v2-btn-hover)]"
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
      <span className="select-none border-r border-[var(--v2-border-subtle)] bg-[var(--v2-surface-subtle)] pr-2 text-right text-[11px] text-[var(--v2-fg-muted)]">
        {leftLineNo ?? ""}
      </span>

      {/* Left Code Cell */}
      <div
        onDoubleClick={() => {
          if (row.left !== null) onStartEdit(row.index, "left", row.left);
        }}
        style={{ backgroundColor: leftBg, color: leftFg }}
        className={`truncate whitespace-pre px-2 ${
          row.left === null && !isSelected ? "v2-hatch" : ""
        }`}
      >
        {editingSide === "left" ? (
          <input
            autoFocus
            value={editingText}
            onChange={(e) => onEditChange(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") onCommitEdit();
              if (e.key === "Escape") onCancelEdit();
            }}
            onBlur={onCancelEdit}
            className="h-[19px] w-full bg-[var(--v2-surface)] px-1 text-[var(--v2-fg)] outline outline-1 outline-[var(--v2-accent)]"
          />
        ) : row.left !== null ? (
          renderHighlightedLine(row.left, row.inline, "left", "var(--v2-chg-inline)")
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
            className="relative z-10 inline-flex h-[15px] w-[15px] items-center justify-center rounded bg-[var(--v2-surface-subtle)] text-[var(--v2-amber)] hover:bg-[var(--v2-btn-hover)]"
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
      <span className="select-none border-r border-[var(--v2-border-subtle)] bg-[var(--v2-surface-subtle)] pr-2 text-right text-[11px] text-[var(--v2-fg-muted)]">
        {rightLineNo ?? ""}
      </span>

      {/* Right Code Cell */}
      <div
        onDoubleClick={() => {
          if (row.right !== null) onStartEdit(row.index, "right", row.right);
        }}
        style={{ backgroundColor: rightBg, color: rightFg }}
        className={`truncate whitespace-pre px-2 ${
          row.right === null && !isSelected ? "v2-hatch" : ""
        }`}
      >
        {editingSide === "right" ? (
          <input
            autoFocus
            value={editingText}
            onChange={(e) => onEditChange(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") onCommitEdit();
              if (e.key === "Escape") onCancelEdit();
            }}
            onBlur={onCancelEdit}
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
});

export default V2TextDiffRow;
