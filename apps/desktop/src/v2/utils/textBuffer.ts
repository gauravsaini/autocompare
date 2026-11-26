export interface BufferEditResult {
  readonly lines: string[];
  readonly nextLine: number;
  readonly nextCol: number;
}

export interface MinimalTextRow {
  readonly index: number;
  readonly left: string | null;
  readonly right: string | null;
}

/** Extracts all non-null display lines for `side` from aligned diff rows. */
export function extractSideLines(
  rows: readonly MinimalTextRow[],
  side: "left" | "right",
): string[] {
  const out: string[] = [];
  for (const r of rows) {
    const val = side === "left" ? r.left : r.right;
    if (val !== null) out.push(val);
  }
  return out;
}

/** Maps an aligned row `index` to its 0-based line index on `side` (-1 if orphan/missing). */
export function rowIndexToSideLineIndex(
  rows: readonly MinimalTextRow[],
  rowIndex: number,
  side: "left" | "right",
): number {
  let count = 0;
  for (const r of rows) {
    const val = side === "left" ? r.left : r.right;
    if (r.index === rowIndex) {
      return val !== null ? count : -1;
    }
    if (val !== null) count += 1;
  }
  return -1;
}

/** Maps a 0-based line index on `side` back to the aligned row `index` (-1 if out of range). */
export function sideLineIndexToRowIndex(
  rows: readonly MinimalTextRow[],
  sideLineIdx: number,
  side: "left" | "right",
): number {
  if (sideLineIdx < 0) return -1;
  let count = 0;
  for (const r of rows) {
    const val = side === "left" ? r.left : r.right;
    if (val !== null) {
      if (count === sideLineIdx) return r.index;
      count += 1;
    }
  }
  return -1;
}

/** Splits `lines[lineIdx]` at `col` (Enter key in notepad editing). */
export function splitLineAt(
  lines: readonly string[],
  lineIdx: number,
  col: number,
): BufferEditResult {
  const next = [...lines];
  const current = next[lineIdx] ?? "";
  const clamped = Math.max(0, Math.min(current.length, col));
  const head = current.slice(0, clamped);
  const tail = current.slice(clamped);
  next.splice(lineIdx, 1, head, tail);
  return { lines: next, nextLine: lineIdx + 1, nextCol: 0 };
}

/** Joins `lines[lineIdx]` onto the end of `lines[lineIdx - 1]` (Backspace at col 0). */
export function joinWithPreviousLine(
  lines: readonly string[],
  lineIdx: number,
): BufferEditResult {
  if (lineIdx <= 0 || lineIdx >= lines.length) {
    return { lines: [...lines], nextLine: Math.max(0, lineIdx), nextCol: 0 };
  }
  const next = [...lines];
  const prev = next[lineIdx - 1];
  const curr = next[lineIdx];
  next.splice(lineIdx - 1, 2, prev + curr);
  return { lines: next, nextLine: lineIdx - 1, nextCol: prev.length };
}

/** Joins `lines[lineIdx + 1]` onto the end of `lines[lineIdx]` (Delete at end of line). */
export function joinWithNextLine(
  lines: readonly string[],
  lineIdx: number,
): BufferEditResult {
  if (lineIdx < 0 || lineIdx + 1 >= lines.length) {
    const len = lines[lineIdx]?.length ?? 0;
    return { lines: [...lines], nextLine: Math.max(0, lineIdx), nextCol: len };
  }
  const next = [...lines];
  const curr = next[lineIdx];
  const after = next[lineIdx + 1];
  next.splice(lineIdx, 2, curr + after);
  return { lines: next, nextLine: lineIdx, nextCol: curr.length };
}

/** Inserts single- or multi-line pasted text into `lines[lineIdx]` replacing `[selStart, selEnd]`. */
export function insertPastedText(
  lines: readonly string[],
  lineIdx: number,
  selStart: number,
  selEnd: number,
  pasted: string,
): BufferEditResult {
  const next = [...lines];
  const current = next[lineIdx] ?? "";
  const s = Math.max(0, Math.min(current.length, selStart));
  const e = Math.max(s, Math.min(current.length, selEnd));
  const head = current.slice(0, s);
  const tail = current.slice(e);
  const parts = pasted.replace(/\r\n/g, "\n").replace(/\r/g, "\n").split("\n");
  if (parts.length === 1) {
    next[lineIdx] = head + parts[0] + tail;
    return { lines: next, nextLine: lineIdx, nextCol: head.length + parts[0].length };
  }
  const inserted = [
    head + parts[0],
    ...parts.slice(1, -1),
    parts[parts.length - 1] + tail,
  ];
  next.splice(lineIdx, 1, ...inserted);
  return {
    lines: next,
    nextLine: lineIdx + parts.length - 1,
    nextCol: parts[parts.length - 1].length,
  };
}
