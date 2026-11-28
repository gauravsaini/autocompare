import React, { useCallback, useEffect, useMemo, useState } from "react";
import { FolderOpen, GitMerge, Play } from "lucide-react";
import { mergeFolderApply, mergeFolderPlan } from "../../ipc/merge";
import type { FolderMergeApplyReport, TreeMergeAction, TreeMergePlan, TreeMergeStep } from "../../ipc/merge";
import V2ErrorBanner from "../components/V2ErrorBanner";
import { isMissingFixtureError } from "../utils/demoGuard";
import { browsePath } from "../utils/dialog";
import type { FilterMode, V2StatusInfo, V2ViewCommands } from "../types";

export interface V2FolderMergeProps {
  readonly leftPath: string;
  readonly rightPath: string;
  readonly onPathsChange: (left: string, right: string) => void;
  readonly onOpenTextMerge?: (left: string, right: string, title: string) => void;
  readonly onRegisterCommands: (cmds: V2ViewCommands) => void;
  readonly onStatusChange: (status: V2StatusInfo) => void;
}

type StepOverride = "TakeLeft" | "TakeRight" | "Skip";

function actionStyle(action: TreeMergeAction | "Skip"): { label: string; bg: string; fg: string } {
  switch (action) {
    case "TakeLeft":
      return { label: "Take Left", bg: "var(--v2-accent-soft)", fg: "var(--v2-accent)" };
    case "TakeRight":
      return { label: "Take Right", bg: "var(--v2-ins-bg)", fg: "var(--v2-ins-fg)" };
    case "MergedText":
      return { label: "Auto-Merged", bg: "var(--v2-chg-bg)", fg: "var(--v2-chg-fg)" };
    case "Conflict":
      return { label: "Conflict", bg: "var(--v2-del-bg)", fg: "var(--v2-del-fg)" };
    case "MakeDir":
      return { label: "Create Dir", bg: "var(--v2-seg-track)", fg: "var(--v2-fg-secondary)" };
    case "Skip":
      return { label: "Skipped", bg: "var(--v2-seg-track)", fg: "var(--v2-fg-muted)" };
  }
}

export default function V2FolderMerge({
  leftPath,
  rightPath,
  onPathsChange,
  onOpenTextMerge,
  onRegisterCommands,
  onStatusChange,
}: V2FolderMergeProps): React.JSX.Element {
  const [baseDraft, setBaseDraft] = useState("");
  const [leftDraft, setLeftDraft] = useState(leftPath);
  const [rightDraft, setRightDraft] = useState(rightPath);
  const [outPath, setOutPath] = useState("");
  const [filterMode, setFilterMode] = useState<FilterMode>("all");
  const [plan, setPlan] = useState<TreeMergePlan | null>(null);
  const [overrides, setOverrides] = useState<Partial<Record<string, StepOverride>>>({});
  const [report, setReport] = useState<FolderMergeApplyReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [applying, setApplying] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setLeftDraft(leftPath);
    setRightDraft(rightPath);
  }, [leftPath, rightPath]);

  const runPlan = useCallback(async (b: string, l: string, r: string): Promise<void> => {
    setLoading(true);
    setError(null);
    setReport(null);
    setOverrides({});
    const lEff = l || "tools/parity-harness/fixtures/folder/left";
    const rEff = r || "tools/parity-harness/fixtures/folder/right";
    const bEff = b || lEff;
    try {
      const p = await mergeFolderPlan(bEff, lEff, rEff);
      if (p && Array.isArray(p.steps)) {
        setPlan({
          steps: p.steps,
          conflicts: Array.isArray(p.conflicts) ? p.conflicts : [],
          skipped: Array.isArray(p.skipped) ? p.skipped : [],
        });
      } else {
        setPlan({ steps: [], conflicts: [], skipped: [] });
      }
    } catch (e) {
      if (!b && !l && !r && isMissingFixtureError(e)) {
        setPlan(null);
      } else {
        setError(e instanceof Error ? e.message : String(e));
      }
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void runPlan(baseDraft, leftPath, rightPath);
  }, [baseDraft, leftPath, rightPath, runPlan]);

  const effectiveSteps = useMemo(() => {
    const raw = Array.isArray(plan?.steps) ? plan.steps : [];
    return raw.map((step) => {
      const ov: StepOverride | undefined = overrides[step.rel_path];
      const effectiveAction: TreeMergeAction | "Skip" = ov ?? step.action;
      return { step, effectiveAction, overridden: ov !== undefined };
    });
  }, [plan, overrides]);

  const visibleSteps = useMemo(() => {
    if (filterMode === "all") return effectiveSteps;
    if (filterMode === "diffs") {
      return effectiveSteps.filter(
        ({ effectiveAction }) => effectiveAction === "Conflict" || effectiveAction === "MergedText",
      );
    }
    return effectiveSteps.filter(({ effectiveAction }) => effectiveAction !== "Conflict");
  }, [effectiveSteps, filterMode]);

  const unresolvedConflicts = useMemo(
    () => effectiveSteps.filter(({ effectiveAction }) => effectiveAction === "Conflict").length,
    [effectiveSteps],
  );

  useEffect(() => {
    onRegisterCommands({
      filterMode,
      setFilterMode,
      swapSides: () => onPathsChange(rightDraft, leftDraft),
      reload: () => void runPlan(baseDraft, leftDraft, rightDraft),
    });
  }, [onRegisterCommands, filterMode, onPathsChange, baseDraft, leftDraft, rightDraft, runPlan]);

  useEffect(() => {
    const steps = effectiveSteps.length;
    const skipped =
      (Array.isArray(plan?.skipped) ? plan.skipped.length : 0) +
      Object.values(overrides).filter((v) => v === "Skip").length;
    onStatusChange({
      left: `${steps} planned steps · ${unresolvedConflicts} unresolved conflicts`,
      center: report
        ? `Wrote ${report.written} files, ${report.dirsCreated} dirs → ${report.outPath}`
        : "Double-click any file row to open in 3-Way Text Merge",
      right: `${skipped} skipped`,
    });
  }, [onStatusChange, effectiveSteps.length, unresolvedConflicts, plan, overrides, report]);

  const setRowOverride = (relPath: string, choice: StepOverride): void => {
    setOverrides((prev) => {
      if (prev[relPath] === choice) {
        const next = { ...prev };
        delete next[relPath];
        return next;
      }
      return { ...prev, [relPath]: choice };
    });
  };

  const resolveAllConflicts = (choice: "TakeLeft" | "TakeRight"): void => {
    if (!plan) return;
    setOverrides((prev) => {
      const next = { ...prev };
      for (const step of plan.steps) {
        if (step.action === "Conflict") next[step.rel_path] = choice;
      }
      return next;
    });
  };

  const handleApply = async (): Promise<void> => {
    if (!plan) return;
    if (!outPath.trim()) {
      setError("Enter an empty output directory path before applying the folder merge.");
      return;
    }
    setApplying(true);
    setError(null);
    const lEff = leftDraft || "tools/parity-harness/fixtures/folder/left";
    const rEff = rightDraft || "tools/parity-harness/fixtures/folder/right";

    const nextSteps: TreeMergeStep[] = [];
    const nextSkipped: Array<readonly [string, string]> = [
      ...(Array.isArray(plan.skipped) ? plan.skipped : []),
    ];
    for (const { step, effectiveAction } of effectiveSteps) {
      if (effectiveAction === "Skip") {
        nextSkipped.push([step.rel_path, "Skipped by user"]);
      } else {
        nextSteps.push({ ...step, action: effectiveAction });
      }
    }
    const nextPlan: TreeMergePlan = {
      steps: nextSteps,
      conflicts: nextSteps.filter((s) => s.action === "Conflict").map((s) => s.rel_path),
      skipped: nextSkipped,
    };

    try {
      const rep = await mergeFolderApply(lEff, rEff, nextPlan, outPath.trim());
      setReport(rep);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setApplying(false);
    }
  };

  const handleBrowse = async (which: "base" | "left" | "right" | "out"): Promise<void> => {
    const picked = await browsePath(true);
    if (!picked) return;
    if (which === "base") setBaseDraft(picked);
    if (which === "out") setOutPath(picked);
    if (which === "left") {
      setLeftDraft(picked);
      onPathsChange(picked, rightDraft);
    }
    if (which === "right") {
      setRightDraft(picked);
      onPathsChange(leftDraft, picked);
    }
  };

  const openInTextMerge = (relPath: string): void => {
    if (!onOpenTextMerge) return;
    const lBase = (leftDraft || "tools/parity-harness/fixtures/folder/left").replace(/\/$/, "");
    const rBase = (rightDraft || "tools/parity-harness/fixtures/folder/right").replace(/\/$/, "");
    const name = relPath.split("/").pop() ?? relPath;
    onOpenTextMerge(`${lBase}/${relPath}`, `${rBase}/${relPath}`, `Merge: ${name}`);
  };

  return (
    <div
      data-testid="v2/foldermerge-view"
      className="flex h-full min-h-0 flex-col overflow-hidden bg-[var(--v2-surface)]"
    >
      {/* 3-Pane Folder Path Header */}
      <form
        onSubmit={(e) => {
          e.preventDefault();
          onPathsChange(leftDraft.trim(), rightDraft.trim());
          void runPlan(baseDraft.trim(), leftDraft.trim(), rightDraft.trim());
        }}
        className="grid shrink-0 grid-cols-3 border-b border-[var(--v2-border)] bg-[var(--v2-surface-subtle)]"
      >
        {(
          [
            ["left", "Left Folder", leftDraft, setLeftDraft],
            ["base", "Base Folder", baseDraft, setBaseDraft],
            ["right", "Right Folder", rightDraft, setRightDraft],
          ] as const
        ).map(([key, label, val, setter], idx) => (
          <div
            key={key}
            className={`flex flex-col gap-1 px-2.5 py-1.5 ${
              idx < 2 ? "border-r border-[var(--v2-border)]" : ""
            }`}
          >
            <span className="text-[10.5px] font-semibold uppercase tracking-wide text-[var(--v2-fg-muted)]">
              {label}
            </span>
            <div className="flex items-center gap-1">
              <input
                type="text"
                value={val}
                onChange={(e) => setter(e.target.value)}
                placeholder={`${label} path…`}
                className="v2-input v2-mono min-w-0 flex-1 text-[11.5px]"
              />
              <button
                type="button"
                onClick={() => void handleBrowse(key)}
                className="v2-btn px-1.5"
                title={`Browse ${label}`}
              >
                <FolderOpen size={12} />
              </button>
            </div>
          </div>
        ))}
        <button type="submit" className="sr-only" tabIndex={-1}>
          Compute folder merge plan
        </button>
      </form>

      {/* Output Target & Bulk Conflict Resolution Bar */}
      <div className="flex shrink-0 flex-wrap items-center justify-between gap-2 border-b border-[var(--v2-border)] bg-[var(--v2-chrome)] px-3 py-1.5">
        <div className="flex min-w-0 flex-1 items-center gap-2">
          <span className="shrink-0 text-[11px] font-semibold text-[var(--v2-fg-secondary)]">
            Merge Output Folder
          </span>
          <input
            type="text"
            value={outPath}
            onChange={(e) => setOutPath(e.target.value)}
            placeholder="/path/to/empty-output-folder"
            className="v2-input v2-mono min-w-0 max-w-md flex-1 text-[11.5px]"
          />
          <button
            type="button"
            onClick={() => void handleBrowse("out")}
            className="v2-btn px-2"
            title="Browse output folder"
          >
            <FolderOpen size={12} />
          </button>
        </div>
        <div className="flex flex-wrap items-center gap-1.5">
          {unresolvedConflicts > 0 ? (
            <>
              <span className="rounded-full bg-[var(--v2-del-bg)] px-2 py-0.5 text-[10.5px] font-semibold text-[var(--v2-del-fg)]">
                {unresolvedConflicts} {unresolvedConflicts === 1 ? "conflict" : "conflicts"}
              </span>
              <button
                type="button"
                onClick={() => resolveAllConflicts("TakeLeft")}
                className="v2-btn h-[22px] text-[11px]"
              >
                Conflicts → Left
              </button>
              <button
                type="button"
                onClick={() => resolveAllConflicts("TakeRight")}
                className="v2-btn h-[22px] text-[11px]"
              >
                Conflicts → Right
              </button>
            </>
          ) : null}
          {Object.keys(overrides).length > 0 ? (
            <button
              type="button"
              onClick={() => setOverrides({})}
              className="v2-btn h-[22px] text-[11px]"
            >
              Reset
            </button>
          ) : null}
          <button
            type="button"
            disabled={applying || !plan || effectiveSteps.length === 0}
            onClick={() => void handleApply()}
            className="v2-btn v2-btn-primary"
          >
            <Play size={12} />
            {applying ? "Merging…" : "Apply Tree Merge"}
          </button>
        </div>
      </div>

      <V2ErrorBanner message={error} onDismiss={() => setError(null)} />

      {/* Merge Steps Table with Per-Row Conflict Overrides */}
      <div className="min-h-0 flex-1 overflow-y-auto text-[12px]">
        {loading ? (
          <div className="flex h-full items-center justify-center text-[var(--v2-fg-muted)]">
            Planning 3-way folder merge…
          </div>
        ) : visibleSteps.length === 0 ? (
          <div className="flex h-full items-center justify-center text-[var(--v2-fg-muted)]">
            No tree merge steps to display.
          </div>
        ) : (
          <div>
            <div className="sticky top-0 grid grid-cols-[130px_1fr_220px] border-b border-[var(--v2-border)] bg-[var(--v2-surface-subtle)] px-3 py-1 text-[11px] font-semibold text-[var(--v2-fg-secondary)]">
              <span>Merge Action</span>
              <span>Relative Path</span>
              <span className="text-right">Override / Resolve</span>
            </div>
            {visibleSteps.map(({ step, effectiveAction, overridden }, i) => {
              const badge = actionStyle(effectiveAction);
              const isDir = step.action === "MakeDir";
              return (
                <div
                  key={`${step.rel_path}-${i}`}
                  onDoubleClick={() => {
                    if (!isDir) openInTextMerge(step.rel_path);
                  }}
                  className="grid h-[28px] grid-cols-[130px_1fr_220px] items-center border-b border-[var(--v2-border-subtle)]/60 px-3 hover:bg-[var(--v2-btn-hover)]/40"
                >
                  <div>
                    <span
                      style={{ backgroundColor: badge.bg, color: badge.fg }}
                      className="inline-flex rounded px-1.5 py-0.5 text-[10.5px] font-semibold"
                    >
                      {badge.label}
                      {overridden ? " *" : ""}
                    </span>
                  </div>
                  <span className="v2-mono truncate pr-2 text-[var(--v2-fg)]">{step.rel_path}</span>
                  <div className="flex items-center justify-end gap-1">
                    {!isDir ? (
                      <>
                        <button
                          type="button"
                          onClick={() => setRowOverride(step.rel_path, "TakeLeft")}
                          className={`v2-btn h-[20px] px-1.5 text-[10.5px] ${
                            overrides[step.rel_path] === "TakeLeft" ? "v2-btn-primary" : ""
                          }`}
                        >
                          Left
                        </button>
                        <button
                          type="button"
                          onClick={() => setRowOverride(step.rel_path, "TakeRight")}
                          className={`v2-btn h-[20px] px-1.5 text-[10.5px] ${
                            overrides[step.rel_path] === "TakeRight" ? "v2-btn-primary" : ""
                          }`}
                        >
                          Right
                        </button>
                        <button
                          type="button"
                          onClick={() => setRowOverride(step.rel_path, "Skip")}
                          className={`v2-btn h-[20px] px-1.5 text-[10.5px] ${
                            overrides[step.rel_path] === "Skip" ? "v2-btn-primary" : ""
                          }`}
                        >
                          Skip
                        </button>
                        {onOpenTextMerge ? (
                          <button
                            type="button"
                            title="Open in 3-Way Text Merge"
                            onClick={() => openInTextMerge(step.rel_path)}
                            className="v2-btn h-[20px] px-1.5 text-[10.5px]"
                          >
                            <GitMerge size={10} />
                          </button>
                        ) : null}
                      </>
                    ) : null}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
