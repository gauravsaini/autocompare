import React, { useContext } from "react";

export interface StatusBarProps {
  left: string;
  right: string;
  message: string;
  /**
   * Wave-27: optional structured live diff counts (Beyond Compare style
   * bottom summary). Explicit prop wins over `DiffStatsContext`; absent,
   * null, or zero-total stats render nothing extra so the bar falls back
   * to the honest `message` default (`Ready`) without layout breakage.
   */
  diffStats?: DiffStats | null;
  /** Wave-206: optional file encoding label (e.g. UTF-8) */
  encoding?: string;
  /** Wave-206: optional line/column indicator (e.g. Ln 1, Col 1) */
  lineCol?: string;
  /** Wave-206: optional zoom level indicator (e.g. 100%) */
  zoom?: string;
}

/**
 * Wave-27: live diff counts for the active view kind (`added` = right-only
 * insertions, `removed` = left-only deletions, `changed` = modified rows,
 * `total` = aligned row count). All counts are sanitized (finite,
 * non-negative integers); `total <= 0` means "no stats".
 */
export interface DiffStats {
  readonly added: number;
  readonly removed: number;
  readonly changed: number;
  readonly total: number;
}

/**
 * Wave-27: ambient stats channel. `App` feeds live counts through this
 * provider so they reach `StatusBar` without touching `ChromeShell`
 * (which forwards only `left`/`right`/`message`). An explicit
 * `diffStats` prop always wins over the ambient value.
 */
export const DiffStatsContext: React.Context<DiffStats | null | undefined> =
  React.createContext<DiffStats | null | undefined>(undefined);

/** Coerce any value to a safe non-negative integer count (never throws). */
function toCount(value: unknown): number {
  if (typeof value !== "number" || !Number.isFinite(value)) return 0;
  return Math.max(0, Math.floor(value));
}

/**
 * Normalize raw stats; resolves `null` when there is nothing honest to
 * show (absent/null/zero-total/partial shapes). Never throws, never
 * renders a crash — zero stats fall back to the `Ready` message default.
 */
function normalizeStats(stats: DiffStats | null | undefined): DiffStats | null {
  if (stats === null || stats === undefined) return null;
  try {
    const normalized: DiffStats = {
      added: toCount((stats as DiffStats).added),
      removed: toCount((stats as DiffStats).removed),
      changed: toCount((stats as DiffStats).changed),
      total: toCount((stats as DiffStats).total),
    };
    return normalized.total > 0 ? normalized : null;
  } catch {
    return null;
  }
}

export function StatusBar({
  left,
  right,
  message,
  diffStats,
  encoding,
  lineCol,
  zoom,
}: StatusBarProps): React.JSX.Element {
  const ambient = useContext(DiffStatsContext);
  const stats = normalizeStats(diffStats ?? ambient ?? null);
  const statsLabel =
    stats === null
      ? undefined
      : `${stats.added} added, ${stats.removed} removed, ${stats.changed} changed, ${stats.total} total rows`;
  return (
    <footer
      data-testid="statusbar"
      className="ac-meta flex h-[var(--ac-statusbar-h)] select-none items-center justify-between gap-[var(--ac-space-4)] border-t border-[var(--ac-border)] bg-[var(--ac-statusbar-bg)] px-[var(--ac-space-3)] font-normal text-[var(--ac-statusbar-fg)]"
    >
      <span className="truncate" title={left} data-testid="statusbar/left">
        {left}
      </span>
      <span data-testid="statusbar/message" className="min-w-0 flex-1 truncate text-center" title={message}>
        {message}
      </span>
      {/* Wave-122 / Wave-206: metrics stay right-anchored (shrink-0); alignment only */}
      <span className="shrink-0 truncate text-right flex items-center justify-end gap-[var(--ac-space-2)]" title={statsLabel ?? right}>
        <span data-testid="statusbar/right">{right}</span>
        {encoding && (
          <span data-testid="statusbar/encoding" className="text-[var(--ac-fg-muted)]">
            {encoding}
          </span>
        )}
        {lineCol && (
          <span data-testid="statusbar/line-col" className="text-[var(--ac-fg-muted)]">
            {lineCol}
          </span>
        )}
        {zoom && (
          <span data-testid="statusbar/zoom" className="text-[var(--ac-fg-muted)]">
            {zoom}
          </span>
        )}
        {stats !== null && (
          <span data-testid="statusbar/stats" aria-label={statsLabel}>
            {" \u2022 "}
            <span data-testid="statusbar/stats-added">+{stats.added}</span>{" "}
            <span data-testid="statusbar/stats-removed">-{stats.removed}</span>{" "}
            <span data-testid="statusbar/stats-changed">~{stats.changed}</span>{" "}
            <span data-testid="statusbar/stats-total">{stats.total} rows</span>
          </span>
        )}
      </span>
    </footer>
  );
}
