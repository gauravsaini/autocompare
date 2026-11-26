export interface ManualAnchor {
  readonly leftLineIdx: number;
  readonly rightLineIdx: number;
}

export interface AlignedRowLike {
  readonly index: number;
  readonly kind: "equal" | "changed" | "leftOnly" | "rightOnly";
  readonly left: string | null;
  readonly right: string | null;
  readonly inline: readonly {
    readonly aStart: number;
    readonly aEnd: number;
    readonly bStart: number;
    readonly bEnd: number;
  }[];
}

/**
 * Inserts a manual alignment anchor while pruning any crossing/conflicting
 * anchors so the list remains strictly monotonic in both Left and Right indices.
 */
export function addManualAnchor(
  existing: readonly ManualAnchor[],
  next: ManualAnchor,
): ManualAnchor[] {
  const compatible = existing.filter(
    (a) =>
      (a.leftLineIdx < next.leftLineIdx && a.rightLineIdx < next.rightLineIdx) ||
      (a.leftLineIdx > next.leftLineIdx && a.rightLineIdx > next.rightLineIdx),
  );
  return [...compatible, next].sort((a, b) => a.leftLineIdx - b.leftLineIdx);
}

function simpleInlineSpans(
  left: string,
  right: string,
): Array<{ aStart: number; aEnd: number; bStart: number; bEnd: number }> {
  if (left === right) return [];
  const a = Array.from(left);
  const b = Array.from(right);
  let prefix = 0;
  while (prefix < a.length && prefix < b.length && a[prefix] === b[prefix]) {
    prefix += 1;
  }
  let suffix = 0;
  while (
    suffix < a.length - prefix &&
    suffix < b.length - prefix &&
    a[a.length - 1 - suffix] === b[b.length - 1 - suffix]
  ) {
    suffix += 1;
  }
  return [
    {
      aStart: prefix,
      aEnd: a.length - suffix,
      bStart: prefix,
      bEnd: b.length - suffix,
    },
  ];
}

/**
 * Aligns one segment between two manual anchors using LCS for moderate segments
 * and positional pairing for very large segments (keeps UI instant).
 */
function alignSegment(
  leftSeg: readonly string[],
  rightSeg: readonly string[],
  out: AlignedRowLike[],
): void {
  const n = leftSeg.length;
  const m = rightSeg.length;
  if (n === 0) {
    for (const r of rightSeg) {
      out.push({ index: out.length, kind: "rightOnly", left: null, right: r, inline: [] });
    }
    return;
  }
  if (m === 0) {
    for (const l of leftSeg) {
      out.push({ index: out.length, kind: "leftOnly", left: l, right: null, inline: [] });
    }
    return;
  }

  // Exact DP LCS when segment area is bounded (<= 250_000 cells)
  if (n * m <= 250_000) {
    const dp: Int32Array[] = Array.from({ length: n + 1 }, () => new Int32Array(m + 1));
    for (let i = n - 1; i >= 0; i -= 1) {
      for (let j = m - 1; j >= 0; j -= 1) {
        if (leftSeg[i] === rightSeg[j]) {
          dp[i][j] = dp[i + 1][j + 1] + 1;
        } else {
          dp[i][j] = dp[i + 1][j] >= dp[i][j + 1] ? dp[i + 1][j] : dp[i][j + 1];
        }
      }
    }
    let i = 0;
    let j = 0;
    while (i < n || j < m) {
      if (i < n && j < m && leftSeg[i] === rightSeg[j]) {
        out.push({
          index: out.length,
          kind: "equal",
          left: leftSeg[i],
          right: rightSeg[j],
          inline: [],
        });
        i += 1;
        j += 1;
      } else {
        // Collect mismatch block until next LCS match
        const delStart = i;
        const insStart = j;
        while (i < n || j < m) {
          if (i < n && j < m && leftSeg[i] === rightSeg[j]) break;
          if (i < n && (j === m || dp[i + 1][j] >= dp[i][j + 1])) {
            i += 1;
          } else {
            j += 1;
          }
        }
        const delLen = i - delStart;
        const insLen = j - insStart;
        const paired = Math.min(delLen, insLen);
        for (let k = 0; k < paired; k += 1) {
          const lv = leftSeg[delStart + k];
          const rv = rightSeg[insStart + k];
          out.push({
            index: out.length,
            kind: "changed",
            left: lv,
            right: rv,
            inline: simpleInlineSpans(lv, rv),
          });
        }
        for (let k = paired; k < delLen; k += 1) {
          out.push({
            index: out.length,
            kind: "leftOnly",
            left: leftSeg[delStart + k],
            right: null,
            inline: [],
          });
        }
        for (let k = paired; k < insLen; k += 1) {
          out.push({
            index: out.length,
            kind: "rightOnly",
            left: null,
            right: rightSeg[insStart + k],
            inline: [],
          });
        }
      }
    }
    return;
  }

  // Fallback for huge segments: positional pairing
  const maxLen = Math.max(n, m);
  for (let k = 0; k < maxLen; k += 1) {
    const lv = k < n ? leftSeg[k] : null;
    const rv = k < m ? rightSeg[k] : null;
    if (lv !== null && rv !== null) {
      out.push({
        index: out.length,
        kind: lv === rv ? "equal" : "changed",
        left: lv,
        right: rv,
        inline: simpleInlineSpans(lv, rv),
      });
    } else if (lv !== null) {
      out.push({ index: out.length, kind: "leftOnly", left: lv, right: null, inline: [] });
    } else if (rv !== null) {
      out.push({ index: out.length, kind: "rightOnly", left: null, right: rv, inline: [] });
    }
  }
}

/**
 * Rebuilds aligned rows pinning every manual `(leftLineIdx, rightLineIdx)`
 * anchor onto the exact same row and aligning segments between anchors.
 */
export function applyManualAnchors(
  leftLines: readonly string[],
  rightLines: readonly string[],
  anchors: readonly ManualAnchor[],
): AlignedRowLike[] {
  const valid = anchors
    .filter(
      (a) =>
        a.leftLineIdx >= 0 &&
        a.leftLineIdx < leftLines.length &&
        a.rightLineIdx >= 0 &&
        a.rightLineIdx < rightLines.length,
    )
    .sort((a, b) => a.leftLineIdx - b.leftLineIdx);

  const out: AlignedRowLike[] = [];
  let lCursor = 0;
  let rCursor = 0;

  for (const anchor of valid) {
    if (anchor.leftLineIdx < lCursor || anchor.rightLineIdx < rCursor) continue;
    alignSegment(
      leftLines.slice(lCursor, anchor.leftLineIdx),
      rightLines.slice(rCursor, anchor.rightLineIdx),
      out,
    );
    const lv = leftLines[anchor.leftLineIdx];
    const rv = rightLines[anchor.rightLineIdx];
    out.push({
      index: out.length,
      kind: lv === rv ? "equal" : "changed",
      left: lv,
      right: rv,
      inline: simpleInlineSpans(lv, rv),
    });
    lCursor = anchor.leftLineIdx + 1;
    rCursor = anchor.rightLineIdx + 1;
  }

  alignSegment(leftLines.slice(lCursor), rightLines.slice(rCursor), out);
  return out;
}
