import { useEffect, useRef, useState } from "react";
import { ArrowLeft, ArrowRight, Trash2 } from "lucide-react";
import { syncApply, syncPlan, type SyncDirection, type SyncPlan, type SyncStep } from "../../ipc/sync";
import { Button } from "../ui";
import { SyncNamePanel, SyncToolbar } from "./sync/controls";
import { errString, isRecord } from "./types";
import type { ViewPaths } from "./types";

type Status = "loading" | "ready" | "empty" | "error";

/**
 * Folder sync plan view (Wave 47).
 *
 * IPC: `sync_plan` once per path pair (read-only), then `sync_apply`
 * on demand with an explicit dry-run or real-run receipt. Every payload
 * is validated at runtime and degrades to loading/empty/error states;
 * failures render inline and never throw.
 *
 * Destructive actions stay explicit: dry run previews counts only,
 * the real apply button says what it does. Under `VITE_MOCK_IPC=1` the
 * mock serves a deterministic plan derived from the bundled engine
 * fixtures, so the spec proves the full loop without touching disk.
 */
function actionLabel(action: SyncStep["action"]): string {
  switch (action) {
    case "CopyToRight":
      return "copy to right";
    case "CopyToLeft":
      return "copy to left";
    case "DeleteRight":
      return "delete right";
    case "DeleteLeft":
      return "delete left";
  }
}

/**
 * Wave-147: BC folder-compare action glyphs. Each plan row names its
 * action in words already (`actionLabel`); the glyph mirrors it
 * visually (right-arrow = copy to right, left-arrow = copy to left,
 * trash = delete either side). Purely presentational, aria-hidden —
 * the words stay the accessible name. No IPC, no logic change.
 */
function actionGlyph(action: SyncStep["action"]): "to-right" | "to-left" | "delete" {
  switch (action) {
    case "CopyToRight":
      return "to-right";
    case "CopyToLeft":
      return "to-left";
    case "DeleteRight":
    case "DeleteLeft":
      return "delete";
  }
}
function directionLabel(direction: SyncDirection): string {
  switch (direction) {
    case "LeftToRight":
      return "left to right";
    case "RightToLeft":
      return "right to left";
    case "NewerWins":
      return "newer wins";
  }
}

/**
 * Agent-SYNC: BC Folder Sync mode name derived from the locked
 * direction+prune contract (no IPC change). BC offers Mirror /
 * Update Left / Update Right / Update Both; the engine expresses those
 * as direction+prune combos (`gui_sync_options` in
 * `crates/compare/src/sync.rs`), so the label here reuses the same
 * mapping purely for display. Prune on a both-ways plan is engine-
 * inert (prune only deletes one-sided losers of a one-directional
 * sync), so the Both name wins regardless of the prune flag.
 */
function bcModeName(direction: SyncDirection, prune: boolean): string {
  switch (direction) {
    case "NewerWins":
      return "Update both";
    case "RightToLeft":
      return prune ? "Mirror right to left" : "Update left";
    case "LeftToRight":
      return prune ? "Mirror left to right" : "Update right";
  }
}

function isStep(v: unknown): v is SyncStep {
  if (!isRecord(v)) return false;
  return (
    typeof v["rel_path"] === "string" &&
    typeof v["is_dir"] === "boolean" &&
    (v["action"] === "CopyToRight" ||
      v["action"] === "CopyToLeft" ||
      v["action"] === "DeleteRight" ||
      v["action"] === "DeleteLeft")
  );
}

export default function SyncView({ leftPath, rightPath }: ViewPaths): React.JSX.Element {
  const [status, setStatus] = useState<Status>("loading");
  const [error, setError] = useState<string | null>(null);
  const [plan, setPlan] = useState<SyncPlan | null>(null);
  const [receipt, setReceipt] = useState<string | null>(null);
  const [applying, setApplying] = useState(false);
  // Wave-64: step filter (folder-63 pattern in sync). Substring match
  // over the planned rel_path, case-insensitive. List-only: preview and
  // real apply run the included subset (name filter never trims apply;
  // per-item Exclude does). No new IPC; reset on session.
  const [nameQuery, setNameQuery] = useState("");
  // Wave-66N: sync direction + prune (BC Update Left/Right idiom mapped
  // onto the locked 3-direction contract). Defaults hold the wire
  // byte-identical to today (no opts sent); any deviation re-plans with
  // explicit opts. Settings survive session/paths changes (BC keeps sync
  // settings per session); only plan/receipt reset on re-plan.
  const [direction, setDirection] = useState<SyncDirection>("LeftToRight");
  const [prune, setPrune] = useState(false);
  // Agent-SYNC: per-item include/exclude (BC Folder Sync idiom). Keys are
  // rel_paths so exclusions survive direction/prune re-plans and name-
  // filter edits; they reset only when the path pair changes (new
  // session). List-only until apply: `runApply` sends the included
  // subset, so dry-run/real counts stay honest.
  const [excluded, setExcluded] = useState<ReadonlySet<string>>(new Set());
  // Wave-228: diff selection cursor for prev/next ViewCommandBus stepping.
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  // Wave-73: collapsible name-filter panel (BC Folder Sync style).
  // Boots open for backward compat (the wave-64 filter box stays visible
  // on load, mirroring folder-67B/media-72); an effect below reopens it
  // whenever a filter is active so a live filter is never hidden by
  // default. Toolbar toggle / Esc collapse it; Esc also clears the query
  // (wave-64 note-hides contract). Ctrl/Cmd+F reopens + focuses.
  const [filterOpen, setFilterOpen] = useState(true);
  const pathsRef = useRef({ left: leftPath, right: rightPath });

  useEffect(() => {
    let cancelled = false;
    // Wave-130: no paths = pick-two-paths empty (App.hasPaths gate parity:
    // both sides required). No sync_plan fires, no error.
    if (leftPath === "" || rightPath === "") {
      setStatus("empty");
      setPlan(null);
      return;
    }
    const prev = pathsRef.current;
    const pathsChanged = prev.left !== leftPath || prev.right !== rightPath;
    pathsRef.current = { left: leftPath, right: rightPath };
    setStatus("loading");
    setError(null);
    setPlan(null);
    setReceipt(null);
    if (pathsChanged) setNameQuery("");
    if (pathsChanged) setExcluded(new Set());
    if (pathsChanged) setSelectedPath(null);
    if (pathsChanged) setStepFilter("all");
    // Defaults (left-wins, no prune) send NO opts: byte-identical IPC to
    // the pre-66N wire. Anything else sends explicit opts.
    const opts =
      direction === "LeftToRight" && prune === false ? undefined : { direction, prune };
    // Headless-proof hook (table-bykey pattern): mirrors the exact opts
    // handed to invoke. The mock sync_plan ignores direction (fixed
    // left-wins plan), so specs prove the REQUEST path here.
    (window as unknown as Record<string, unknown>)["__SYNC_OPTS_LAST__"] = opts ?? null;
    (async () => {
      try {
        const raw = (await syncPlan(leftPath, rightPath, opts)) as unknown;
        if (cancelled) return;
        if (!isRecord(raw) || !Array.isArray(raw["steps"])) {
          setError("sync plan has an unexpected shape");
          setStatus("error");
          return;
        }
        const steps = (raw["steps"] as unknown[]).filter(isStep);
        const next: SyncPlan = {
          steps,
          skipped: Array.isArray(raw["skipped"]) ? (raw["skipped"] as SyncPlan["skipped"]) : [],
          truncated: raw["truncated"] === true,
        };
        setPlan(next);
        setStatus(steps.length === 0 ? "empty" : "ready");
      } catch (e) {
        if (!cancelled) {
          setError(errString(e));
          setStatus("error");
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [leftPath, rightPath, direction, prune]);

  async function runApply(dryRun: boolean): Promise<void> {
    if (plan === null || applying) return;
    setApplying(true);
    setReceipt(null);
    try {
      // Agent-SYNC: apply only the included subset. The mock
      // (`VITE_MOCK_IPC=1`) echoes the full fixture plan, so the dry-run
      // line is computed locally from the sent subset to stay honest;
      // the real backend plans from what we send.
      const included = plan.steps.filter((s) => !excluded.has(s.rel_path));
      const sent: SyncPlan = { steps: included, skipped: plan.skipped, truncated: plan.truncated };
      const plannedCopies = included.filter((s) => s.action === "CopyToRight" || s.action === "CopyToLeft").length;
      const plannedDeletes = included.length - plannedCopies;
      const report = await syncApply(leftPath, rightPath, sent, dryRun);
      const failed = report.failed.length > 0 ? `, ${report.failed.length} failed` : "";
      const excludedNote = excluded.size > 0 ? ` (${excluded.size} excluded, not applied)` : "";
      setReceipt(
        dryRun
          ? `dry run: ${plannedCopies} copies, ${plannedDeletes} deletes planned${excludedNote}${failed} — nothing touched`
          : `applied: ${report.copied} copied, ${report.deleted} deleted${failed}${excludedNote}`,
      );
    } catch (e) {
      setReceipt(`apply failed: ${errString(e)}`);
    } finally {
      setApplying(false);
    }
  }

  function toggleInclude(relPath: string): void {
    // A toggle invalidates any previous receipt (it described a
    // different included set); the next dry run / apply re-proves.
    setReceipt(null);
    setExcluded((prev) => {
      const next = new Set(prev);
      if (next.has(relPath)) next.delete(relPath);
      else next.add(relPath);
      return next;
    });
  }

  /* Wave-62 pattern in sync (extended wave-73): Ctrl/Cmd+F focuses the
     step filter. The panel may be collapsed, so open it first, then
     focus the box. Only the active view is mounted (ViewRouter), so no
     cross-view fight. */
  useEffect(() => {
    // Wave-127: full-session find (menu `session.find`) opens the step
    // filter FIRST, then focuses — same open-then-focus as Cmd+F below.
    // The panel commits after `setFilterOpen`, so focus retries bounded
    // until the box mounts (no focus-first query on unmounted input).
    const openStepFilter = (): void => {
      setFilterOpen(true);
      let attempts = 0;
      const tick = (): void => {
        attempts += 1;
        const box = document.getElementById("sync-name-box");
        if (box instanceof HTMLElement) {
          box.focus();
          return;
        }
        if (attempts < 20) window.setTimeout(tick, 50);
      };
      window.setTimeout(tick, 0);
    };
    const onSessionFind = (): void => {
      openStepFilter();
    };
    const onKey = (e: KeyboardEvent): void => {
      if ((e.metaKey || e.ctrlKey) && (e.key === "f" || e.key === "F")) {
        e.preventDefault();
        openStepFilter();
      }
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("autocompare:session-find", onSessionFind);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("autocompare:session-find", onSessionFind);
    };
  }, []);

  // Wave-SYNCL3: L2 `autocompare:view-filter` bus (Beyond Compare View menu
  // parity: Show All / Differences / Same / Context, dispatched by the top
  // Toolbar via `handleViewFilter`). Applies only when the event
  // targets this kind (`viewKind === "sync"` or an unkinded event);
  // other kinds are ignored so folder/text/hex filters never leak across
  // views.
  const [stepFilter, setStepFilter] = useState<"all" | "diffs" | "same" | "context">("all");

  useEffect(() => {
    const onViewFilter = (e: Event): void => {
      const detail = (e as CustomEvent<{ filter?: unknown; viewKind?: unknown }>).detail;
      const kind = detail?.viewKind;
      if (typeof kind === "string" && kind !== "" && kind !== "sync") return;
      const f = detail?.filter;
      if (f === "all" || f === "diffs" || f === "same" || f === "context") {
        setStepFilter(f);
      }
    };
    window.addEventListener("autocompare:view-filter", onViewFilter);
    return () => {
      window.removeEventListener("autocompare:view-filter", onViewFilter);
    };
  }, []);

  useEffect(() => {
    (window as unknown as Record<string, unknown>)["__SYNC_FILTER_LAST__"] = stepFilter;
  }, [stepFilter]);

  /* Wave-73: an active filter is never hidden by default -- if a filter
     value lands while the panel is collapsed (e.g. kept across a
     direction/prune re-plan after the user collapsed it), reopen. */
  const steps = plan?.steps ?? [];
  // Wave-SYNCL3: filter steps based on L2 view-filter bus.
  // In sync plans, every step is a diff action (copy or delete to synchronize).
  // "same" produces 0 steps as identical items have no planned action.
  // "diffs", "context", and "all" keep the planned diff actions.
  const stepFiltered = steps.filter((_step) => {
    if (stepFilter === "same") return false;
    return true;
  });
  const nameNeedle = nameQuery.trim().toLowerCase();
  const visibleSteps =
    nameNeedle === "" ? stepFiltered : stepFiltered.filter((s) => s.rel_path.toLowerCase().includes(nameNeedle));

  const stepDiffRef = useRef<((dir: 1 | -1) => void) | null>(null);
  stepDiffRef.current = (dir: 1 | -1) => {
    if (visibleSteps.length === 0) return;
    const currentIdx = selectedPath ? visibleSteps.findIndex((s) => s.rel_path === selectedPath) : -1;
    let nextIdx: number;
    if (currentIdx === -1) {
      nextIdx = dir === 1 ? 0 : visibleSteps.length - 1;
    } else {
      nextIdx = Math.min(visibleSteps.length - 1, Math.max(0, currentIdx + dir));
    }
    const next = visibleSteps[nextIdx];
    if (next) {
      setSelectedPath(next.rel_path);
      const el =
        typeof CSS !== "undefined" && typeof CSS.escape === "function"
          ? document.querySelector(`[data-path="${CSS.escape(next.rel_path)}"]`)
          : null;
      if (el instanceof HTMLElement) {
        el.scrollIntoView({ block: "nearest" });
      }
    }
  };

  /* Wave-180 / Wave-228: `autocompare:view-command` bus (unified top Toolbar
     parity). Connects to ViewCommandBus: prev/next step through the
     planned diff steps without crashing. Kind-scoped so other view
     commands do not leak. Never throws. */
  useEffect(() => {
    const onBusCmd = (e: Event): void => {
      const detail = (e as CustomEvent<{ cmd?: unknown; viewKind?: unknown }>).detail;
      const kind = detail?.viewKind;
      if (typeof kind === "string" && kind !== "" && kind !== "sync") return;
      const cmd = detail?.cmd;
      if (cmd === "prev") {
        stepDiffRef.current?.(-1);
      } else if (cmd === "next") {
        stepDiffRef.current?.(1);
      }
    };
    window.addEventListener("autocompare:view-command", onBusCmd);
    return () => {
      window.removeEventListener("autocompare:view-command", onBusCmd);
    };
  }, []);
  // Agent-SYNC honest counts: apply runs the included subset only, so
  // the preview counts below (and the dry-run line) are computed over
  // included steps, never the raw plan.
  const includedSteps = steps.filter((s) => !excluded.has(s.rel_path));
  const copies = includedSteps.filter((s) => s.action === "CopyToRight" || s.action === "CopyToLeft").length;
  const deletes = includedSteps.length - copies;
  // Wave-73 status bar: per-op direction counts reused from the existing
  // plan steps (no backend change -- the sync_plan contract only carries
  // step actions, so the bar honestly reports those action counts).
  const toRight = steps.filter((s) => s.action === "CopyToRight").length;
  const toLeft = steps.filter((s) => s.action === "CopyToLeft").length;
  const delRight = steps.filter((s) => s.action === "DeleteRight").length;
  const delLeft = steps.filter((s) => s.action === "DeleteLeft").length;

  const applyDisabledReason = applying
    ? "sync in progress"
    : steps.length === 0
      ? "folders are in sync — no steps planned"
      : includedSteps.length === 0
        ? "all steps excluded — include at least one step to apply"
        : null;
  const canApply = steps.length > 0 && includedSteps.length > 0 && !applying;

  function closeNamePanel(): void {
    // Esc / dismiss clears the query (wave-64 note-hides contract) and
    // collapses the panel.
    setNameQuery("");
    setFilterOpen(false);
  }

  return (
    <section aria-label="folder sync plan" data-filter={stepFilter}>
      {status === "loading" && (
        <p data-testid="syncview/status" className="px-[var(--ac-space-4)] py-[var(--ac-space-2)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]">
          Planning sync…
        </p>
      )}
      {status === "error" && (
        <p data-testid="syncview/error" role="alert" className="px-[var(--ac-space-4)] py-[var(--ac-space-2)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg)]">
          {error ?? "sync plan failed"}
        </p>
      )}
      {status === "empty" && (leftPath === "" || rightPath === "") && (
        <div
          data-testid="workspace/blank-empty"
          className="mx-auto flex w-full max-w-3xl flex-col items-center gap-3 px-8 py-16 text-center"
        >
          <p
            data-testid="syncview/empty"
            className="ac-body text-[var(--ac-fg-muted)]"
          >
            Pick two folders to sync — needs two paths.
          </p>
        </div>
      )}
      {status === "empty" && leftPath !== "" && rightPath !== "" && (
        <p
          data-testid="syncview/empty"
          className="px-[var(--ac-space-4)] py-[var(--ac-space-8)] text-center text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]"
        >
          Folders are in sync — no steps planned.
        </p>
      )}
      {(status === "ready" || status === "empty") && plan !== null && (
        <>
          {/* Wave-73: chrome collapse (BC Folder Sync style). One toolbar
              row (directions + prune + name-filter toggle + dry/real
              apply) plus a collapsible in-flow name panel and a bottom
              status line. All legacy testids ride the same controls;
              plan/direction/prune/filter/apply logic byte-identical. */}
          <SyncToolbar
            direction={direction}
            onDirection={setDirection}
            prune={prune}
            onPruneChange={setPrune}
            filterOpen={filterOpen}
            onToggleFilter={() => {
              const next = !filterOpen;
              setFilterOpen(next);
              if (next) {
                window.setTimeout(() => {
                  const box = document.getElementById("sync-name-box");
                  if (box instanceof HTMLElement) box.focus();
                }, 0);
              }
            }}
            applying={applying}
            canApply={canApply}
            disabledReason={applyDisabledReason}
            onDry={() => void runApply(true)}
            onReal={() => void runApply(false)}
            onBusCommand={(cmd) => {
              if (cmd === "prev") stepDiffRef.current?.(-1);
              else if (cmd === "next") stepDiffRef.current?.(1);
            }}
          />
          <p data-testid="syncview/totals" className="px-[var(--ac-space-4)] py-[var(--ac-space-2)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)] border-b border-[var(--ac-border-subtle)] bg-[var(--ac-bg)]">
            {excluded.size === 0
              ? `${steps.length} steps (${copies} copies, ${deletes} deletes)`
              : `${includedSteps.length} of ${steps.length} steps included (${copies} copies, ${deletes} deletes, ${excluded.size} excluded)`}
            {plan.truncated ? " — truncated" : ""}
          </p>
          <p data-testid="syncview/mode-note" role="status" className="px-[var(--ac-space-4)] py-[var(--ac-space-1)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]">
            direction: {directionLabel(direction)} &middot; prune: {prune ? "on" : "off"} &middot; mode: {bcModeName(direction, prune)}{stepFilter !== "all" ? ` \u00b7 filter: ${stepFilter}` : ""}
          </p>
          {receipt !== null && (
            <p data-testid="syncview/receipt" role="status" className="px-[var(--ac-space-4)] py-[var(--ac-space-1)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]">
              {receipt}
            </p>
          )}
          <SyncNamePanel
            open={filterOpen}
            query={nameQuery}
            onQueryChange={setNameQuery}
            onClose={closeNamePanel}
          />
          {nameNeedle !== "" && (
            <p data-testid="syncview/name-note" role="status" className="px-[var(--ac-space-4)] py-[var(--ac-space-1)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]">
              {visibleSteps.length} of {steps.length} steps match &ldquo;{nameQuery.trim()}&rdquo; — list only, apply runs the full plan.
            </p>
          )}
          <div data-testid="syncview/list" data-filter={stepFilter}>
            {visibleSteps.length === 0 && stepFilter === "same" && (
              <p
                data-testid="syncview/filter-empty"
                className="px-[var(--ac-space-4)] py-[var(--ac-space-8)] text-center text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]"
              >
                No sync steps match view filter &ldquo;same&rdquo; &mdash; sync plan only contains differences to reconcile.
              </p>
            )}
            {visibleSteps.map((step, index) => {
              const glyph = actionGlyph(step.action);
              const included = !excluded.has(step.rel_path);
              const side =
                step.action === "CopyToRight" || step.action === "DeleteLeft"
                  ? "left-only"
                  : "right-only";
              const sidePlace =
                side === "right-only"
                  ? { width: "50%", marginLeft: "auto", boxSizing: "border-box" as const }
                  : { width: "50%", marginRight: "auto", boxSizing: "border-box" as const };
              return (
                <div
                  key={step.rel_path}
                  data-testid="syncview/row"
                  data-action={step.action}
                  data-path={step.rel_path}
                  data-side={side}
                  data-bracket="single"
                  data-included={included ? "true" : "false"}
                  data-selected={selectedPath === step.rel_path ? "true" : "false"}
                  onClick={() => setSelectedPath(step.rel_path)}
                  className={`sv-row fv-cell-empty fv-rail font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)]${included ? "" : " opacity-50"}${selectedPath === step.rel_path ? " is-selected" : ""}`}
                >
                  <div
                    style={sidePlace}
                    className="flex items-center gap-[var(--ac-space-2)] px-[var(--ac-space-4)] py-[var(--ac-space-1)]"
                  >
                    <span
                      data-testid="syncview/row-glyph"
                      data-glyph={glyph}
                      aria-hidden="true"
                      className="inline-flex shrink-0 items-center text-[var(--ac-fg-muted)]"
                    >
                      {glyph === "to-right" ? (
                        <ArrowRight size={12} />
                      ) : glyph === "to-left" ? (
                        <ArrowLeft size={12} />
                      ) : (
                        <Trash2 size={12} />
                      )}
                    </span>
                    <span className="text-[var(--ac-fg-muted)]">{step.is_dir ? "dir" : "file"}</span>
                    <span className="min-w-0 flex-1 truncate" title={step.rel_path}>
                      {step.rel_path}
                    </span>
                    {/* Wave-185: right-pinned action cluster (BC Folder Sync
                        center-action-column echo). The badge + include toggle
                        ride one nowrap group at the row end so narrow windows
                        keep them together on the row line (uniform-height
                        gate); testids/aria/words byte-identical. */}
                    <span className="ml-auto inline-flex shrink-0 items-center gap-[var(--ac-space-2)]">
                      <span
                        data-testid="syncview/row-action"
                        data-action={step.action}
                        className="whitespace-nowrap text-[var(--ac-fg-muted)]"
                      >
                        [{actionLabel(step.action)}]
                      </span>
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        data-testid={`sync-include-${index}`}
                        aria-pressed={included}
                        aria-label={`${included ? "Exclude" : "Include"} ${step.rel_path} ${included ? "from" : "in"} apply`}
                        title={included ? "Exclude from apply (preview keeps it listed)" : "Include in apply"}
                        onClick={() => toggleInclude(step.rel_path)}
                      >
                        {included ? "Exclude" : "Include"}
                      </Button>
                    </span>
                  </div>
                </div>
              );
            })}
          </div>
          {/* Wave-73 bottom status bar (BC Folder Sync style). Honest
              scope: reuses the existing plan steps + direction/prune
              state only -- per-op counts by step action plus the mode.
              NO backend change: the sync_plan contract carries no other
              per-file direction data, so none is claimed here. */}
          <div
            data-testid="syncview/statusbar"
            role="status"
            className="mt-[var(--ac-space-2)] flex flex-wrap items-center gap-[var(--ac-space-2)] border-t border-[var(--ac-border-subtle)] bg-[var(--ac-bg)] px-[var(--ac-space-4)] py-[var(--ac-space-2)] text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]"
          >
            <span data-testid="syncview/statusbar-counts">
              {steps.length} steps · to right {toRight} · to left {toLeft} · deletes L{delLeft}/R{delRight} · showing {visibleSteps.length}
              {excluded.size === 0 ? "" : ` · ${includedSteps.length} included, ${excluded.size} excluded`}
              {stepFilter !== "all" ? ` · filter: ${stepFilter}` : ""}
            </span>
            <span data-testid="syncview/statusbar-mode">
              direction: {directionLabel(direction)} · prune: {prune ? "on" : "off"} · mode: {bcModeName(direction, prune)}{stepFilter !== "all" ? ` · filter: ${stepFilter}` : ""} · dry run previews only, nothing touched until Apply
            </span>
          </div>
        </>
      )}
    </section>
  );
}
