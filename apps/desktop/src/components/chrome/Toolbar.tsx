import React, { useCallback, useEffect, useRef, useState } from "react";
import { Loader2, RefreshCw, Square, type LucideIcon } from "lucide-react";
import { ToolbarButton } from "../ui";

export interface ToolbarAction {
  id: string;
  icon: LucideIcon;
  label: string;
  enabled: boolean;
  onClick: () => void;
  /**
   * Wave-29: per-action working flag. Renders a spinner with
   * `aria-busy="true"` instead of the static icon. Optional so every
   * pre-wave-29 caller (which passes no flag) keeps compiling and
   * rendering exactly as before.
   */
  busy?: boolean;
  /**
   * Wave-29: honest reason surfaced in the `title` tooltip (and the
   * accessible description). Optional; when absent the label itself —
   * which already carries reasons like `Compare (needs two paths)` —
   * doubles as the title.
   */
  disabledReason?: string | null;
}

export interface ToolbarProps {
  actions: readonly ToolbarAction[];
  /**
   * Wave-174: optional band-siting hook. `ChromeShell` renders Toolbar +
   * TabBar side-by-side in ONE 44px chrome band (BC parity); the caller
   * passes flex sizing (`min-w-0 flex-1` + internal `overflow-x-auto`
   * keeps the single-row guarantee). Heights/testids/wiring untouched.
   */
  className?: string;
  /**
   * Wave-29: external busy override for the compare lifecycle.
   * Absent (the `App` case) = the toolbar's internal run state.
   */
  busy?: boolean;
  /**
   * Wave-29: external inline note override. Absent = the internal
   * note (stop explanation / run failure). Rendered inline, never
   * thrown.
   */
  note?: string | null;
}

/**
 * Wave-29: minimum visible busy dwell. Keeps the compare spinner
 * cycle observable (and avoids flicker on instant mock IPC) while the
 * view settles after a re-run.
 */
export const COMPARE_MIN_BUSY_MS = 600;

/**
 * Wave-29: upper bound for the busy state while waiting for the view
 * to re-render after a compare re-run. The compare IPC surface has no
 * completion callback at toolbar level, so busy resolves on the first
 * view mutation after the click, or on this timeout — whichever wins.
 */
export const COMPARE_SETTLE_TIMEOUT_MS = 6000;

/**
 * Wave-29: honest stop note. The existing compare IPC surface
 * (`text_compare`, `folder_compare`, …) exposes no cancel command,
 * so Stop is a best-effort no-op: the running compare finishes on its
 * own and the busy state clears when the view settles.
 */
export const COMPARE_NO_ABORT_NOTE =
  "Stop is best-effort: the compare IPC cannot be cancelled once sent — the running compare finishes on its own.";

/** Reason shown while a compare run is in flight. */
export const COMPARE_BUSY_REASON = "comparing…";

/** Reason shown on Stop/Refresh when no compare is running. */
export const COMPARE_IDLE_STOP_REASON = "no compare running";

const BC_LABELS: Record<string, string> = {
  home: "Home",
  open: "Sessions",
  "filter-all": "All",
  "filter-diffs": "Diffs",
  "filter-same": "Same",
  "filter-context": "Context",
  minor: "Minor",
  rules: "Rules",
  format: "Format",
  copy: "Copy",
  "next-hunk": "Next",
  "prev-hunk": "Prev",
  swap: "Swap",
  compare: "Compare",
};

/**
 * Wave-52 / Wave-206: short display label for icon-above-label buttons.
 * Maps actions to concise Beyond Compare button labels (Home, Sessions,
 * All, Diffs, Same, Context, Minor, Rules, Format, Copy, Next, Prev,
 * Swap, Reload) preventing label truncation in compact buttons; strips
 * parenthesized reason suffix; the full label stays in aria-label + title.
 */
export function displayLabelOf(label: string, id?: string): string {
  if (id && BC_LABELS[id]) {
    return BC_LABELS[id];
  }
  const stripped = label.replace(/\s*\([^)]*\)\s*$/, "").trim() || label;
  if (/^open\s*session/i.test(stripped)) return "Sessions";
  if (/^show\s*all/i.test(stripped)) return "All";
  if (/^show\s*diff/i.test(stripped)) return "Diffs";
  if (/^show\s*same/i.test(stripped)) return "Same";
  if (/^prev(ious)?(\s*hunk|\s*conflict)?/i.test(stripped)) return "Prev";
  if (/^next(\s*hunk|\s*conflict)?/i.test(stripped)) return "Next";
  if (/^swap(\s*sides)?/i.test(stripped)) return "Swap";
  if (/^refresh/i.test(stripped)) return "Reload";
  return stripped;
}

/**
 * Wave-29: derive the honest blocked reason for a disabled compare
 * action. Prefers the explicit `disabledReason`, then falls back to
 * the parenthesized suffix the workspace already embeds in labels
 * (`Compare (needs two paths)`), so pre-wave-29 callers get reasoned
 * titles with zero changes on their side.
 */
export function compareBlockedReasonOf(action: ToolbarAction): string | null {
  if (action.enabled) return null;
  if (action.disabledReason !== undefined && action.disabledReason !== null) {
    return action.disabledReason;
  }
  const match = action.label.match(/\(([^)]+)\)\s*$/);
  return match?.[1] ?? "unavailable";
}

/**
 * Wave-29: resolve the `title` tooltip for an action. When a disabled
 * reason is known and not already embedded in the label, it is
 * appended parenthetically so every disabled state carries its reason
 * in the tooltip.
 */
export function actionTitle(action: ToolbarAction, reason?: string | null): string {
  const r = reason ?? action.disabledReason ?? null;
  if (!r || action.label.includes(r)) return action.label;
  return `${action.label} (${r})`;
}

/**
 * Wave-29: wait until the data view re-renders after a compare
 * re-run. Resolves on the first DOM mutation inside `<main>` (the
 * `ViewRouter` remount / row reload the workspace already performs on
 * `compareTick`), or after `timeoutMs` when no view is mounted (e.g.
 * empty session) or the view renders byte-identical output.
 */
function waitForViewSettle(timeoutMs: number): Promise<void> {
  return new Promise((resolve) => {
    const main = document.querySelector("main");
    if (!main) {
      window.setTimeout(resolve, COMPARE_MIN_BUSY_MS);
      return;
    }
    let done = false;
    let observer: MutationObserver | null = null;
    const finish = (): void => {
      if (done) return;
      done = true;
      observer?.disconnect();
      window.clearTimeout(timer);
      resolve();
    };
    observer = new MutationObserver(() => finish());
    observer.observe(main, { childList: true, subtree: true, characterData: true });
    const timer = window.setTimeout(finish, timeoutMs);
  });
}

/**
 * Wave-29: compare-lifecycle toolbar (Beyond Compare style).
 *
 * Backwards compatible: renders every passed `actions` entry exactly
 * as before (same testids, same native disabled semantics), then
 * synthesizes the compare lifecycle cluster — Stop (`Square`) and
 * Refresh (`RefreshCw`) — from the `compare` entry when one is
 * present. No new IPC: Compare/Refresh re-invoke the host's own
 * kind-matched compare handler (which already drives the existing
 * `*_compare` IPC); failures render inline and never throw.
 *
 * - Compare: runs the active compare, shows a spinner (`busy`) and
 *   disables itself with a `(comparing…)` reason while the run is in
 *   flight; keyboard reachable via the native button.
 * - Stop: enabled only while busy; honest no-op with an inline note
 *   because the backend cannot abort a sent compare.
 * - Refresh: re-runs the current compare; disabled with the compare's
 *   own blocked reason (or `comparing…` while busy).
 * - Empty sessions: all three disable honestly with titled reasons.
 */
export function Toolbar({ actions, busy: busyProp, note: noteProp, className }: ToolbarProps): React.JSX.Element {
  const [internalBusy, setInternalBusy] = useState(false);
  const [internalNote, setInternalNote] = useState<string | null>(null);
  const [runs, setRuns] = useState(0);
  const mountedRef = useRef(true);
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const busy = busyProp ?? internalBusy;
  const note = noteProp ?? internalNote;

  const compareAction = actions.find((a) => a.id === "compare") ?? null;
  const blockedReason = compareAction ? compareBlockedReasonOf(compareAction) : "needs an open session";
  const refreshReason: string | null = busy
    ? COMPARE_BUSY_REASON
    : blockedReason;

  const runCompare = useCallback(async (): Promise<void> => {
    if (busyProp ?? internalBusy) return;
    if (!compareAction || !compareAction.enabled) return;
    setInternalBusy(true);
    setInternalNote(null);
    const startedAt = Date.now();
    try {
      // The host handler drives the existing kind-matched compare IPC.
      // It returns void today; awaited defensively so a future
      // promise-returning handler is tracked instead of flashed.
      const result = compareAction.onClick() as unknown;
      if (result instanceof Promise) await result;
      await waitForViewSettle(COMPARE_SETTLE_TIMEOUT_MS);
    } catch (e: unknown) {
      if (mountedRef.current) {
        setInternalNote(e instanceof Error ? e.message : String(e));
      }
    } finally {
      const elapsed = Date.now() - startedAt;
      if (elapsed < COMPARE_MIN_BUSY_MS) {
        await new Promise((r) => window.setTimeout(r, COMPARE_MIN_BUSY_MS - elapsed));
      }
      if (mountedRef.current) {
        setRuns((r) => r + 1);
        setInternalBusy(false);
      }
    }
  }, [busyProp, internalBusy, compareAction]);

  const handleStop = useCallback((): void => {
    if (!(busyProp ?? internalBusy)) return;
    // No abort IPC exists: honest no-op with an inline note. Busy
    // stays until the running compare settles the view on its own.
    setInternalNote(COMPARE_NO_ABORT_NOTE);
  }, [busyProp, internalBusy]);

  // Wave-116: slimmer shared bar (token --ac-toolbar-h, 44px) with tighter
  // side padding. Testids, labels, and wiring untouched — heights/padding only.
  // Wave-L2: the unified BC bar holds more actions (filters + Minor/
  // Rules/Format stubs); it stays ONE row with horizontal scroll instead
  // of stacking a second row (chrome budget).
  const buttonClass = "";

  return (
    <div
      role="toolbar"
      aria-label="Compare actions"
      className={`flex h-[var(--ac-toolbar-h)] items-center gap-[2px] overflow-x-auto overflow-y-hidden whitespace-nowrap border-b [&>*]:shrink-0 border-[var(--ac-border)] bg-[var(--ac-toolbar-bg)] px-[var(--ac-space-2)]${className ? ` ${className}` : ""}`}
    >
      {actions.map((action) => {
        const isCompare = action.id === "compare";
        const actionBusy = action.busy ?? (isCompare && busy);
        const actionEnabled = action.enabled && !(isCompare && busy);
        const reason: string | null = isCompare
          ? busy
            ? COMPARE_BUSY_REASON
            : (action.disabledReason ?? compareBlockedReasonOf(action))
          : (action.disabledReason ?? null);
        const title = isCompare && busy ? `Compare (${COMPARE_BUSY_REASON})` : actionTitle(action, reason);
        // Wave-180: single bus source — Rules/Format events dispatch
        // ONLY from the host `onClick` (App `handleRulesToggle` /
        // `handleFormatToggle` own `dispatchViewRules` /
        // `dispatchViewFormat`). The bar never fires a parallel raw
        // event, so the in-view panel opens exactly once per click.
        // Disabled buttons never fire either path.
        const handleActionClick = actionEnabled ? action.onClick : undefined;
        // Wave-116: idle Prev/Next collapse out of the bar (testid stays
        // mounted, honestly hidden) instead of holding an always-on slot.
        // Wave-206: sync/merge also collapse when idle/disabled so toolbar
        // stays clean and unbloated across views.
        const idleCollapsed =
          !isCompare &&
          !actionEnabled &&
          (action.id === "prev-hunk" ||
            action.id === "next-hunk" ||
            action.id === "copy-right" ||
            action.id === "sync" ||
            action.id === "merge");
        const actionClass = idleCollapsed ? `${buttonClass} !hidden` : buttonClass;
        const Icon = action.icon;
        const compareButton = (
          <ToolbarButton
            key={action.id}
            orientation="vertical"
            data-testid={`toolbar/action-${action.id}`}
            aria-label={action.label}
            aria-busy={isCompare && busy ? true : undefined}
            title={title}
            disabled={!actionEnabled}
            onClick={isCompare ? () => void runCompare() : handleActionClick}
            className={actionClass}
            label={action.label}
          >
            {actionBusy ? (
              <span data-testid="toolbar/compare-busy" className="flex items-center justify-center">
                <Loader2 size={14} strokeWidth={1.75} aria-hidden="true" className="animate-spin" />
              </span>
            ) : (
              <Icon size={14} strokeWidth={1.75} aria-hidden="true" />
            )}
            <span aria-hidden="true" className="ac-meta w-full max-w-24 truncate text-center leading-none">
              {displayLabelOf(action.label, action.id)}
            </span>
          </ToolbarButton>
        );
        if (!isCompare) return compareButton;
        const stopEnabled = busy;
        const refreshEnabled = action.enabled && !busy;
        return (
          <React.Fragment key="wave29-compare-cluster">
            {compareButton}
            <ToolbarButton
              key="compare-stop"
              orientation="vertical"
              data-testid="toolbar/action-compare-stop"
              aria-label={stopEnabled ? "Stop" : `Stop (${COMPARE_IDLE_STOP_REASON})`}
              title={stopEnabled ? "Stop" : `Stop (${COMPARE_IDLE_STOP_REASON})`}
              disabled={!stopEnabled}
              onClick={stopEnabled ? handleStop : undefined}
              // Wave-116: idle Stop collapses out (testid stays mounted,
              // honestly hidden); it reappears while a run is in flight.
              className={stopEnabled ? buttonClass : `${buttonClass} !hidden`}
              label="Stop"
            >
              <Square size={14} strokeWidth={1.75} aria-hidden="true" />
              <span aria-hidden="true" className="ac-meta w-full max-w-24 truncate text-center leading-none">
                Stop
              </span>
            </ToolbarButton>
            <ToolbarButton
              key="compare-refresh"
              orientation="vertical"
              data-testid="toolbar/action-compare-refresh"
              aria-label={refreshEnabled ? "Refresh" : `Refresh (${refreshReason ?? "unavailable"})`}
              title={refreshEnabled ? "Refresh" : `Refresh (${refreshReason ?? "unavailable"})`}
              disabled={!refreshEnabled}
              onClick={refreshEnabled ? () => void runCompare() : undefined}
              className={buttonClass}
              label="Refresh"
            >
              <RefreshCw size={14} strokeWidth={1.75} aria-hidden="true" className={busy ? "animate-spin" : undefined} />
              <span aria-hidden="true" className="ac-meta w-full max-w-24 truncate text-center leading-none">
                Reload
              </span>
            </ToolbarButton>
          </React.Fragment>
        );
      })}
      {/* Wave-29: screen-reader + spec-observable completed-run counter. */}
      <span data-testid="toolbar/compare-runs" className="sr-only" aria-live="polite">
        {`runs: ${runs}`}
      </span>
      {note ? (
        <div
          role="status"
          data-testid="toolbar/compare-note"
          className="ac-meta ml-2 max-w-md truncate text-[var(--ac-fg-muted)]"
          title={note}
        >
          {note}
        </div>
      ) : null}
    </div>
  );
}
