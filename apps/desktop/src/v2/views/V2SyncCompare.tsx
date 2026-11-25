import React, { useCallback, useEffect, useState } from "react";
import { ArrowLeft, ArrowRight, ArrowLeftRight, Play, Trash2 } from "lucide-react";
import { syncApply, syncPlan } from "../../ipc/sync";
import type { SyncApplyReport, SyncDirection, SyncPlan, SyncStep } from "../../ipc/sync";
import V2ErrorBanner from "../components/V2ErrorBanner";
import V2DualPathBar from "../components/V2DualPathBar";
import { isMissingFixtureError } from "../utils/demoGuard";
import type { V2StatusInfo, V2ViewCommands } from "../types";

export interface V2SyncCompareProps {
  readonly leftPath: string;
  readonly rightPath: string;
  readonly onPathsChange: (left: string, right: string) => void;
  readonly onRegisterCommands: (cmds: V2ViewCommands) => void;
  readonly onStatusChange: (status: V2StatusInfo) => void;
}

function actionBadge(action: SyncStep["action"]): { label: string; color: string; bg: string } {
  switch (action) {
    case "CopyToRight":
      return { label: "Copy → Right", color: "#15803d", bg: "var(--v2-ins-bg)" };
    case "CopyToLeft":
      return { label: "Copy ← Left", color: "#2563eb", bg: "var(--v2-accent-soft)" };
    case "DeleteRight":
      return { label: "Delete Right", color: "#b91c1c", bg: "var(--v2-del-bg)" };
    case "DeleteLeft":
      return { label: "Delete Left", color: "#b91c1c", bg: "var(--v2-del-bg)" };
  }
}

export default function V2SyncCompare({
  leftPath,
  rightPath,
  onPathsChange,
  onRegisterCommands,
  onStatusChange,
}: V2SyncCompareProps): React.JSX.Element {
  const [leftDraft, setLeftDraft] = useState(leftPath);
  const [rightDraft, setRightDraft] = useState(rightPath);
  const [direction, setDirection] = useState<SyncDirection>("LeftToRight");
  const [prune, setPrune] = useState(false);
  const [plan, setPlan] = useState<SyncPlan | null>(null);
  const [report, setReport] = useState<SyncApplyReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setLeftDraft(leftPath);
    setRightDraft(rightPath);
  }, [leftPath, rightPath]);

  const runPlan = useCallback(
    async (l: string, r: string): Promise<void> => {
      setLoading(true);
      setError(null);
      setReport(null);
      const lEff = l || "tools/parity-harness/fixtures/folder/left";
      const rEff = r || "tools/parity-harness/fixtures/folder/right";
      try {
        const p = await syncPlan(lEff, rEff, { direction, prune });
        setPlan(p);
      } catch (e) {
        if (!l && !r && isMissingFixtureError(e)) {
          setPlan(null);
        } else {
          setError(e instanceof Error ? e.message : String(e));
        }
      } finally {
        setLoading(false);
      }
    },
    [direction, prune],
  );

  useEffect(() => {
    void runPlan(leftPath, rightPath);
  }, [leftPath, rightPath, runPlan]);

  useEffect(() => {
    onRegisterCommands({
      swapSides: () => onPathsChange(rightDraft, leftDraft),
      reload: () => void runPlan(leftDraft, rightDraft),
    });
  }, [onRegisterCommands, onPathsChange, leftDraft, rightDraft, runPlan]);

  useEffect(() => {
    const steps = plan?.steps.length ?? 0;
    const skipped = plan?.skipped.length ?? 0;
    onStatusChange({
      left: steps === 0 ? "Folders are in sync" : `${steps} planned ${steps === 1 ? "action" : "actions"}`,
      center:
        report !== null
          ? report.dryRun
            ? `Dry run: ${report.plannedCopies} copies, ${report.plannedDeletes} deletes`
            : `Synced: ${report.copied} copied, ${report.deleted} deleted`
          : `${direction}${prune ? " · Mirror/Prune" : ""}`,
      right: `${skipped} skipped`,
    });
  }, [onStatusChange, plan, report, direction, prune]);

  const handleApply = async (dryRun: boolean): Promise<void> => {
    if (!plan) return;
    setRunning(true);
    setError(null);
    const lEff = leftDraft || "tools/parity-harness/fixtures/folder/left";
    const rEff = rightDraft || "tools/parity-harness/fixtures/folder/right";
    try {
      const rep = await syncApply(lEff, rEff, plan, dryRun);
      setReport(rep);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setRunning(false);
    }
  };

  return (
    <div data-testid="v2/sync-view" className="flex h-full min-h-0 flex-col overflow-hidden bg-[var(--v2-surface)]">
      <V2DualPathBar
        leftValue={leftDraft}
        rightValue={rightDraft}
        onLeftChange={setLeftDraft}
        onRightChange={setRightDraft}
        onSubmit={onPathsChange}
        leftPlaceholder="Left folder…"
        rightPlaceholder="Right folder…"
        directory
      />

      {/* Sync Mode Control Strip */}
      <div className="flex shrink-0 flex-wrap items-center justify-between gap-3 border-b border-[var(--v2-border)] bg-[var(--v2-chrome)] px-3 py-2">
        <div className="flex flex-wrap items-center gap-3">
          <div className="inline-flex rounded-[6px] bg-[var(--v2-seg-track)] p-0.5">
            {(
              [
                ["LeftToRight", "Update Right", ArrowRight],
                ["RightToLeft", "Update Left", ArrowLeft],
                ["NewerWins", "Mirror Newer", ArrowLeftRight],
              ] as const
            ).map(([dir, label, Icon]) => {
              const active = direction === dir;
              return (
                <button
                  key={dir}
                  type="button"
                  onClick={() => setDirection(dir)}
                  className={`inline-flex h-[22px] items-center gap-1 rounded-[5px] px-2.5 text-[11.5px] font-medium transition ${
                    active
                      ? "bg-[var(--v2-surface)] text-[var(--v2-fg)] shadow-xs"
                      : "text-[var(--v2-fg-secondary)] hover:text-[var(--v2-fg)]"
                  }`}
                >
                  <Icon size={12} />
                  {label}
                </button>
              );
            })}
          </div>

          <label className="flex cursor-pointer items-center gap-1.5 text-[12px] text-[var(--v2-fg)]">
            <input
              type="checkbox"
              checked={prune}
              onChange={(e) => setPrune(e.target.checked)}
              className="accent-[var(--v2-accent)]"
            />
            <Trash2 size={12} className="text-[var(--v2-fg-muted)]" />
            <span>Delete orphans (Prune)</span>
          </label>
        </div>

        <div className="flex items-center gap-2">
          <button
            type="button"
            disabled={running || !plan || plan.steps.length === 0}
            onClick={() => void handleApply(true)}
            className="v2-btn"
          >
            Dry Run
          </button>
          <button
            type="button"
            disabled={running || !plan || plan.steps.length === 0}
            onClick={() => void handleApply(false)}
            className="v2-btn v2-btn-primary"
          >
            <Play size={12} />
            Sync Now
          </button>
        </div>
      </div>

      <V2ErrorBanner message={error} onDismiss={() => setError(null)} />

      {/* Planned Sync Steps Table */}
      <div className="min-h-0 flex-1 overflow-y-auto text-[12px]">
        {loading ? (
          <div className="flex h-full items-center justify-center text-[var(--v2-fg-muted)]">
            Computing sync plan…
          </div>
        ) : !plan || plan.steps.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center gap-1 text-[var(--v2-fg-muted)]">
            <p className="text-[13px] font-medium text-[var(--v2-fg)]">Folders are in sync</p>
            <p className="text-[12px]">No copy or delete operations planned for this direction.</p>
          </div>
        ) : (
          <div>
            <div className="sticky top-0 grid grid-cols-[130px_1fr_80px] border-b border-[var(--v2-border)] bg-[var(--v2-surface-subtle)] px-3 py-1 text-[11px] font-semibold text-[var(--v2-fg-secondary)]">
              <span>Action</span>
              <span>Relative Path</span>
              <span className="text-right">Kind</span>
            </div>
            {plan.steps.map((step, i) => {
              const badge = actionBadge(step.action);
              return (
                <div
                  key={`${step.rel_path}-${i}`}
                  className="grid h-[26px] grid-cols-[130px_1fr_80px] items-center border-b border-[var(--v2-border-subtle)]/60 px-3 hover:bg-[var(--v2-btn-hover)]/40"
                >
                  <div>
                    <span
                      style={{ backgroundColor: badge.bg, color: badge.color }}
                      className="inline-flex rounded px-1.5 py-0.5 text-[10.5px] font-semibold"
                    >
                      {badge.label}
                    </span>
                  </div>
                  <span className="v2-mono truncate text-[var(--v2-fg)]">{step.rel_path}</span>
                  <span className="text-right text-[11px] text-[var(--v2-fg-muted)]">
                    {step.is_dir ? "Folder" : "File"}
                  </span>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
