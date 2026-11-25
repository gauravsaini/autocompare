import React, { useCallback, useRef } from "react";

export type DiffRowKind = "same" | "del" | "ins" | "chg";

/** Backend `text_compare` row kinds (see `ipc/text.ts` `TextRowKind`). */
export type TextCompareRowKind = "equal" | "leftOnly" | "rightOnly" | "changed";

export interface DiffRow {
  kind: DiffRowKind;
}

/**
 * Same-row mapping `TextView` uses to paint rows (`TextView.tsx`
 * `rowClass`: equal -> same, leftOnly -> del, rightOnly -> ins,
 * changed -> chg). The ruler reuses it so markers line up 1:1 with
 * the virtualized pane rows.
 */
export function textCompareKindToDiffKind(kind: TextCompareRowKind): DiffRowKind {
  switch (kind) {
    case "equal":
      return "same";
    case "leftOnly":
      return "del";
    case "rightOnly":
      return "ins";
    case "changed":
      return "chg";
  }
}

/** Contiguous run of non-`"same"` rows (a Beyond Compare style hunk). */
export interface RulerHunk {
  readonly start: number;
  readonly end: number;
  readonly kind: DiffRowKind;
}

/** Group consecutive non-`"same"` rows into hunks. Resolves [] for empty input. */
export function collectHunks(rows: readonly DiffRow[]): RulerHunk[] {
  const hunks: RulerHunk[] = [];
  let start = -1;
  for (let i = 0; i < rows.length; i += 1) {
    const kind = rows[i]?.kind ?? "same";
    if (kind === "same") {
      if (start >= 0) {
        hunks.push({ start, end: i - 1, kind: rows[start]?.kind ?? "chg" });
        start = -1;
      }
    } else if (start < 0) {
      start = i;
    }
  }
  if (start >= 0) hunks.push({ start, end: rows.length - 1, kind: rows[start]?.kind ?? "chg" });
  return hunks;
}

/** Hard cap on rendered ruler markers. Past this many rows contiguous row
 * indices are partitioned into buckets (one marker per bucket) so a
 * 100k+ row diff never mounts 100k+ DOM nodes. The click path stays
 * fractional (`fractionToRowIndex`), so bucketing keeps click accuracy.
 */
export const MAX_RULER_BUCKETS = 400;

/** One rendered ruler marker covering rows `start..end` (inclusive). */
export interface RulerBucket {
  readonly start: number;
  readonly end: number;
  readonly kind: DiffRowKind;
}

/** Severity rank: chg (3) > del/ins (2) > same (1). */
function kindRank(kind: DiffRowKind): number {
  switch (kind) {
    case "chg":
      return 3;
    case "del":
    case "ins":
      return 2;
    default:
      return 1;
  }
}

/** Worst (highest-rank) kind in rows `start..end` (inclusive). */
export function worstKindInRange(rows: readonly DiffRow[], start: number, end: number): DiffRowKind {
  let worst: DiffRowKind = "same";
  for (let i = start; i <= end; i += 1) {
    const kind = rows[i]?.kind ?? "same";
    if (kindRank(kind) > kindRank(worst)) {
      worst = kind;
      if (worst === "chg") break;
    }
  }
  return worst;
}

/**
 * Partition row indices into at most `maxBuckets` contiguous buckets.
 * Small diffs (<= maxBuckets rows) map 1:1 (one bucket per row); large
 * diffs use `ceil(total / maxBuckets)`-sized buckets. Each bucket keeps
 * the first row index as its identity (`ruler/row-<start>`) and the
 * worst kind in its range, so hunk markers survive compression.
 */
export function compressRowsToBuckets(
  rows: readonly DiffRow[],
  maxBuckets: number = MAX_RULER_BUCKETS,
): RulerBucket[] {
  const total = rows.length;
  if (total === 0) return [];
  const cap = Math.max(1, Math.floor(maxBuckets));
  if (total <= cap) {
    return rows.map((row, i) => ({ start: i, end: i, kind: row?.kind ?? "same" }));
  }
  const size = Math.ceil(total / cap);
  const buckets: RulerBucket[] = [];
  for (let start = 0; start < total; start += size) {
    const end = Math.min(total - 1, start + size - 1);
    buckets.push({ start, end, kind: worstKindInRange(rows, start, end) });
  }
  return buckets;
}

/** Clamp a ruler fraction to a row index. */
export function fractionToRowIndex(fraction: number, total: number): number {
  if (total <= 0) return 0;
  if (!Number.isFinite(fraction)) return 0;
  const clamped = Math.min(1, Math.max(0, fraction));
  return Math.min(total - 1, Math.floor(clamped * total));
}

/** Row index back to a ruler fraction (inverse of `fractionToRowIndex`). */
export function rowIndexToFraction(index: number, total: number): number {
  if (total <= 0) return 0;
  return Math.min(1, Math.max(0, index / total));
}

/**
 * Scroll offset for the virtualized pane (`TextView` uses a fixed
 * `ROW_PX` estimate so offsets equal `index * ROW_PX`; equivalently
 * `fraction * (scrollHeight - clientHeight)`). The scroller owner
 * applies `el.scrollTop = rowIndexToScrollTop(...)`.
 */
export function rowIndexToScrollTop(
  index: number,
  total: number,
  scrollHeight: number,
  clientHeight: number,
): number {
  const span = Math.max(0, scrollHeight - clientHeight);
  return rowIndexToFraction(index, total) * span;
}

/** First hunk starting strictly after `from` (wraps to first hunk). Null when no hunks. */
export function findNextHunk(rows: readonly DiffRow[], from: number): number | null {
  const hunks = collectHunks(rows);
  if (hunks.length === 0) return null;
  for (const hunk of hunks) {
    if (hunk.start > from) return hunk.start;
  }
  return hunks[0]?.start ?? null;
}

/** Last hunk starting strictly before `from` (wraps to last hunk). Null when no hunks. */
export function findPrevHunk(rows: readonly DiffRow[], from: number): number | null {
  const hunks = collectHunks(rows);
  if (hunks.length === 0) return null;
  for (let i = hunks.length - 1; i >= 0; i -= 1) {
    const hunk = hunks[i];
    if (hunk !== undefined && hunk.start < from) return hunk.start;
  }
  return hunks[hunks.length - 1]?.start ?? null;
}

export interface OverviewRulerProps {
  rows: readonly DiffRow[];
  /**
   * Currently visible top row (highlights that marker). Owned by the
   * scroller; the ruler never mutates scroll itself so it stays
   * decoupled from the virtualized pane.
   */
  activeIndex?: number;
  /**
   * Called with the target row index on click / drag / keyboard.
   * The scroller owner jumps via `rowIndexToScrollTop`. Absent =
   * visual-only fallback (previous behaviour, no handlers).
   */
  onJump?: (index: number) => void;
  /** Accessible name override. */
  label?: string;
}

const KIND_TO_TOKEN: Record<DiffRowKind, string> = {
  same: "var(--ac-ruler-same)",
  del: "var(--ac-ruler-del)",
  ins: "var(--ac-ruler-ins)",
  chg: "var(--ac-ruler-chg)",
};

function isHunkKind(kind: DiffRowKind): boolean {
  return kind !== "same";
}

export function OverviewRuler({ rows, activeIndex, onJump, label }: OverviewRulerProps): React.JSX.Element {
  const trackRef = useRef<HTMLDivElement | null>(null);
  const draggingRef = useRef(false);
  const interactive = typeof onJump === "function";
  const total = rows.length;
  const current = activeIndex ?? 0;

  const jumpFromClientY = useCallback(
    (clientY: number): void => {
      if (onJump === undefined) return;
      const el = trackRef.current;
      if (el === null || total === 0) return;
      const rect = el.getBoundingClientRect();
      if (rect.height <= 0) return;
      onJump(fractionToRowIndex((clientY - rect.top) / rect.height, total));
    },
    [onJump, total],
  );

  function handleKeyDown(event: React.KeyboardEvent): void {
    if (onJump === undefined || total === 0) return;
    switch (event.key) {
      case "ArrowDown":
      case "ArrowRight": {
        event.preventDefault();
        const next = findNextHunk(rows, current) ?? Math.min(total - 1, current + 1);
        onJump(next);
        break;
      }
      case "ArrowUp":
      case "ArrowLeft": {
        event.preventDefault();
        const prev = findPrevHunk(rows, current) ?? Math.max(0, current - 1);
        onJump(prev);
        break;
      }
      case "Home":
        event.preventDefault();
        onJump(0);
        break;
      case "End":
        event.preventDefault();
        onJump(total - 1);
        break;
      default:
        break;
    }
  }

  const hunkStarts = React.useMemo(() => {
    const starts = new Set<number>();
    let inHunk = false;
    for (let i = 0; i < rows.length; i += 1) {
      const hunk = isHunkKind(rows[i]?.kind ?? "same");
      if (hunk && !inHunk) starts.add(i);
      inHunk = hunk;
    }
    return starts;
  }, [rows]);

  const buckets = React.useMemo(() => compressRowsToBuckets(rows), [rows]);

  const name = label ?? `Overview ruler: ${total} rows`;

  return (
    <div
      ref={trackRef}
      data-testid="ruler"
      role={interactive ? "slider" : "img"}
      aria-label={name}
      aria-orientation={interactive ? "vertical" : undefined}
      aria-valuemin={interactive ? 0 : undefined}
      aria-valuemax={interactive ? Math.max(0, total - 1) : undefined}
      aria-valuenow={interactive ? Math.min(current, Math.max(0, total - 1)) : undefined}
      aria-valuetext={interactive ? `row ${Math.min(current, Math.max(0, total - 1)) + 1} of ${total}` : undefined}
      tabIndex={interactive ? 0 : undefined}
      onKeyDown={interactive ? handleKeyDown : undefined}
      onPointerDown={
        interactive
          ? (event) => {
              draggingRef.current = true;
              (event.target as HTMLElement | null)?.setPointerCapture?.(event.pointerId);
              jumpFromClientY(event.clientY);
            }
          : undefined
      }
      onPointerMove={
        interactive
          ? (event) => {
              if (draggingRef.current) jumpFromClientY(event.clientY);
            }
          : undefined
      }
      onPointerUp={
        interactive
          ? () => {
              draggingRef.current = false;
            }
          : undefined
      }
      onPointerCancel={
        interactive
          ? () => {
              draggingRef.current = false;
            }
          : undefined
      }
      className="flex w-[var(--ac-space-4)] shrink-0 cursor-pointer flex-col gap-0 self-stretch border-l border-[var(--ac-border)] bg-[var(--ac-ruler-track)] py-[var(--ac-space-1)]"
    >
      {/* Wave-130: zero-row honest affordance — no fake differences.
          Wave-225: the track is a 16px column, so the "0 rows" label
          rides `vertical-rl` (no squeezed wrapping in 8px of content
          width) — testid + text + aria-label byte-identical, still
          zero `[data-kind]` markers. */}
      {total === 0 ? (
        <span
          data-testid="ruler/empty"
          style={{
            writingMode: "vertical-rl",
            whiteSpace: "nowrap",
            alignSelf: "center",
            marginTop: "auto",
            marginBottom: "auto",
            padding: "8px 0",
            fontSize: "10px",
            lineHeight: 1,
            letterSpacing: "0.04em",
            color: "var(--ac-fg-muted)",
            userSelect: "none",
          }}
        >
          0 rows
        </span>
      ) : null}
      {buckets.map((bucket) => (
        <div
          key={bucket.start}
          data-testid={`ruler/row-${bucket.start}`}
          data-kind={bucket.kind}
          data-hunk={isHunkKind(bucket.kind) ? "true" : "false"}
          data-hunk-start={hunkStarts.has(bucket.start) ? "true" : "false"}
          data-active={current >= bucket.start && current <= bucket.end && interactive ? "true" : "false"}
          title={
            bucket.start === bucket.end
              ? `${bucket.kind} · row ${bucket.start + 1}`
              : `${bucket.kind} · rows ${bucket.start + 1}–${bucket.end + 1}`
          }
          style={{
            background: KIND_TO_TOKEN[bucket.kind],
            height: 4,
            outline:
              current >= bucket.start && current <= bucket.end && interactive
                ? "1px solid var(--ac-focus-ring)"
                : undefined,
          }}
          className="w-full shrink-0"
        />
      ))}
    </div>
  );
}
