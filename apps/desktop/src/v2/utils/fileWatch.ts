export interface FileFingerprint {
  readonly bytes?: number;
  readonly mtimeUnix?: number;
}

/**
 * Pure comparison of baseline vs current file fingerprints.
 * Returns the subset of `paths` whose size or modification timestamp changed.
 */
export function detectChangedPaths(
  paths: readonly string[],
  baseline: readonly (FileFingerprint | null)[],
  current: readonly (FileFingerprint | null)[],
): string[] {
  const changed: string[] = [];
  for (let i = 0; i < paths.length; i += 1) {
    const p = paths[i];
    if (!p) continue;
    const prev = baseline[i];
    const next = current[i];
    if (!prev || !next) continue;
    const bytesChanged =
      prev.bytes !== undefined && next.bytes !== undefined && prev.bytes !== next.bytes;
    const mtimeChanged =
      prev.mtimeUnix !== undefined &&
      next.mtimeUnix !== undefined &&
      prev.mtimeUnix !== next.mtimeUnix;
    if (bytesChanged || mtimeChanged) {
      changed.push(p);
    }
  }
  return changed;
}
