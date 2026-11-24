import type { ReactNode } from "react";
import type { InlineSpanDto, TextRowDto } from "../../ipc/text";

/**
 * Wave-21 line detail pane (Beyond Compare style bottom pane).
 *
 * Pure props-driven: renders the selected text row stacked left/right
 * with character-level highlights derived from its `inline` ops. No IPC,
 * no state -- the owner (`App`) supplies the already-loaded selected row
 * (or null). Null (no selection, or a non-text kind upstream) renders a
 * collapsed empty placeholder so the layout never breaks.
 */

export interface LineDetailPaneProps {
  /** Already-loaded selected row, or null when nothing is selected. */
  readonly row: TextRowDto | null;
}

/** Half-open char ranges on one side, sorted and clamped to `len`. */
function sideRanges(
  ops: readonly InlineSpanDto[],
  pick: (op: InlineSpanDto) => readonly [number, number],
  len: number,
): Array<readonly [number, number]> {
  const out: Array<readonly [number, number]> = [];
  for (const op of ops) {
    const [rawStart, rawEnd] = pick(op);
    const start = Math.max(0, Math.min(len, rawStart));
    const end = Math.max(0, Math.min(len, rawEnd));
    if (end > start) out.push([start, end] as const);
  }
  out.sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  // Merge overlapping/adjacent ranges so marks never nest.
  const merged: Array<readonly [number, number]> = [];
  for (const range of out) {
    const last = merged[merged.length - 1];
    if (last !== undefined && range[0] <= last[1]) {
      merged[merged.length - 1] = [last[0], Math.max(last[1], range[1])] as const;
    } else {
      merged.push(range);
    }
  }
  return merged;
}

/**
 * Render `text` with `ranges` wrapped in `<mark>` highlights. Plain
 * segments stay bare text so equal characters carry no highlight.
 */
function renderHighlighted(
  text: string,
  ranges: ReadonlyArray<readonly [number, number]>,
  side: "left" | "right",
): ReactNode {
  if (ranges.length === 0) return text;
  const parts: ReactNode[] = [];
  let at = 0;
  ranges.forEach(([start, end], i) => {
    if (start > at) parts.push(text.slice(at, start));
    parts.push(
      <mark
        key={`${side}-hl-${i}`}
        data-testid="line-detail/hl"
        data-side={side}
        className="rounded-sm bg-[var(--ac-diff-chg-bg)] px-px text-[var(--ac-diff-chg-fg)]"
      >
        {text.slice(start, end)}
      </mark>,
    );
    at = end;
  });
  if (at < text.length) parts.push(text.slice(at));
  return parts;
}

export function LineDetailPane({ row }: LineDetailPaneProps): React.JSX.Element {
  if (row === null) {
    return (
      <section aria-label="line detail" data-testid="line-detail" data-empty="true">
        <p data-testid="line-detail/empty" className="ac-chrome-label px-[var(--ac-space-4)] py-[var(--ac-space-2)] font-normal text-[var(--ac-fg-muted)]">
          No line selected.
        </p>
      </section>
    );
  }
  const left = row.left ?? "";
  const right = row.right ?? "";
  const leftRanges = sideRanges(row.inline, (op) => [op.aStart, op.aEnd] as const, left.length);
  const rightRanges = sideRanges(row.inline, (op) => [op.bStart, op.bEnd] as const, right.length);
  return (
    <section
      aria-label="line detail"
      data-testid="line-detail"
      data-empty="false"
      data-kind={row.kind}
      className="border-t border-[var(--ac-border-subtle)] bg-[var(--ac-bg)]"
    >
      <p
        data-testid="line-detail/header"
        className="ac-chrome-label px-[var(--ac-space-4)] pt-[var(--ac-space-2)] font-semibold text-[var(--ac-fg)]"
      >
        Line {row.index + 1} · {row.kind}
      </p>
      <div className="grid grid-cols-1 gap-[var(--ac-space-2)] px-[var(--ac-space-4)] py-[var(--ac-space-2)]">
        <div>
          <p className="ac-meta font-medium text-[var(--ac-fg-muted)]">Left</p>
          <pre data-testid="line-detail/left" className="overflow-x-auto whitespace-pre-wrap ac-mono text-[length:var(--ac-font-size-code)] leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg)]">
            {renderHighlighted(left, leftRanges, "left")}
          </pre>
        </div>
        <div>
          <p className="ac-meta font-medium text-[var(--ac-fg-muted)]">Right</p>
          <pre data-testid="line-detail/right" className="overflow-x-auto whitespace-pre-wrap ac-mono text-[length:var(--ac-font-size-code)] leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg)]">
            {renderHighlighted(right, rightRanges, "right")}
          </pre>
        </div>
      </div>
    </section>
  );
}

export default LineDetailPane;
