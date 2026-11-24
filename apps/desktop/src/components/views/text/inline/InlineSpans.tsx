import type { ReactNode } from "react";
import type { InlineSpanDto } from "../../../../ipc/text";

/**
 * Wave-66S BC-parity inline word highlights (plug & play extension).
 * Pure renderer, no IPC/state. Reuses LineDetailPane sideRanges logic:
 * half-open char ranges sorted/clamped/merged, plain segments stay bare.
 * TextView integration comes in the follow-up wave (RowLine wiring).
 */
export function sideRanges(
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

export function InlineSpans({
  text,
  ranges,
  side,
}: {
  text: string;
  ranges: ReadonlyArray<readonly [number, number]>;
  side: "left" | "right";
}): React.JSX.Element {
  if (ranges.length === 0) return <>{text}</>;
  const parts: ReactNode[] = [];
  let at = 0;
  ranges.forEach(([start, end], i) => {
    if (start > at) parts.push(text.slice(at, start));
    parts.push(
      <mark
        key={`${side}-w-${i}`}
        data-testid="inline/hl"
        data-side={side}
        className="tv-w-hl rounded-sm px-px"
      >
        {text.slice(start, end)}
      </mark>,
    );
    at = end;
  });
  if (at < text.length) parts.push(text.slice(at));
  return <>{parts}</>;
}
