/**
 * Wave-79: BC-style far-left overview strip (presentational ONLY).
 *
 * Wave-196: 4px far-left rail beside the text rows with one per-hunk marker.
 * Markers derive from the SAME `hunks` array the per-pane gutter arrows
 * use (`collectTextHunks` over the loaded rows), so strip markers line up
 * 1:1 with gutter hunk ordinals. Clicking marker N jumps through the SAME
 * `onJump` hunk-nav path the toolbar prev/next buttons use (select +
 * scroll + focus, no new IPC). Colors reuse existing diff/ruler tokens
 * (no new hues); geometry is a 4px relative rail with absolute markers.
 */

export interface OverviewStripHunk {
  readonly start: number;
  readonly end: number;
  readonly kind: "changed" | "leftOnly" | "rightOnly" | "mixed";
}

export interface OverviewStripProps {
  readonly hunks: readonly OverviewStripHunk[];
  /** Full comparison row total (marker geometry denominator). */
  readonly total: number;
  /** Ordinal of the hunk holding the current selection (null = none). */
  readonly activeOrdinal: number | null;
  /** Jump to hunk ordinal (existing hunk-nav path, no new IPC). */
  readonly onJump: (ordinal: number) => void;
}

export default function OverviewStrip({ hunks, total, activeOrdinal, onJump }: OverviewStripProps): React.JSX.Element {
  const denom = total > 0 ? total : 1;
  return (
    <div
      data-testid="textview/overview-strip"
      className="tv-overview-strip"
      role="navigation"
      aria-label={`Overview: ${hunks.length} ${hunks.length === 1 ? "difference" : "differences"} — activate a marker to jump`}
    >
      {hunks.map((hunk, ordinal) => {
        const top = (hunk.start / denom) * 100;
        const height = Math.max(((hunk.end - hunk.start + 1) / denom) * 100, 0);
        const active = activeOrdinal === ordinal;
        return (
          <button
            key={`${hunk.start}-${hunk.end}`}
            type="button"
            data-testid={`textview/overview-hunk-${ordinal}`}
            data-kind={hunk.kind}
            data-active={active ? "true" : "false"}
            aria-label={`Jump to difference ${ordinal + 1} of ${hunks.length}`}
            title={`Difference ${ordinal + 1} of ${hunks.length} (rows ${hunk.start + 1}–${hunk.end + 1})`}
            aria-current={active ? "true" : undefined}
            onClick={() => onJump(ordinal)}
            className="tv-overview-hunk"
            style={{ top: `${top}%`, height: `${height}%` }}
          />
        );
      })}
    </div>
  );
}
