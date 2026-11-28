import { endSpan, isTelemetryEnabled, startRootSpan } from "../../observability/tracer.ts";

export interface FileFingerprint {
  readonly bytes?: number;
  readonly mtimeUnix?: number;
}

/**
 * ODD Phase 4 Leaf A — explicit file-watch state-transition event.
 *
 * Emits a short correlated span (`ui.reload_prompt`, `ts.ui/fileWatch`)
 * parent-linked to the ambient trace (the view's last compare root span)
 * so reload prompts / auto-reloads / dismissals are traceable without
 * altering behavior. No-op when telemetry is off or `parent` is absent;
 * never throws.
 */
export function emitFileWatchTransition(
  viewId: string,
  transition: "prompt_shown" | "reload_accepted" | "reload_dismissed" | "auto_reloaded" | "baseline_reset",
  detail?: Record<string, unknown>,
  parent?: unknown,
): void {
  try {
    if (!isTelemetryEnabled()) return;
    if (parent === null || parent === undefined) return;
    const span = startRootSpan({
      op: "ui.reload_prompt",
      component: "ts.ui/fileWatch",
      view_id: viewId,
      parent,
      attributes: { event: transition, ...(detail ?? {}) },
    });
    endSpan(span);
  } catch {
    // Instrumentation never breaks the UI.
  }
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
