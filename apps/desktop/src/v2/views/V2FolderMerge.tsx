import React, { useCallback, useEffect, useMemo, useState } from "react";
import { FolderOpen, Play } from "lucide-react";
import { mergeFolderApply, mergeFolderPlan } from "../../ipc/merge";
import type { FolderMergeApplyReport, TreeMergeAction, TreeMergePlan } from "../../ipc/merge";
import type { FilterMode, V2StatusInfo, V2ViewCommands } from "../types";

export interface V2FolderMergeProps {
  readonly leftPath: string;
  readonly rightPath: string;
  readonly onPathsChange: (left: string, right: string) => void;
  readonly onRegisterCommands: (cmds: V2ViewCommands) => void;
  readonly onStatusChange: (status: V2StatusInfo) => void;
}

function actionStyle(action: TreeMergeAction): { label: string; bg: string; fg: string } {
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
  }
}

export default function V2FolderMerge({
  leftPath,
  rightPath,
  onPathsChange,
  onRegisterCommands,
  onStatusChange,
}: V2FolderMergeProps): React.JSX.Element {
  const [baseDraft, setBaseDraft] = useState("");
  const [leftDraft, setLeftDraft] = useState(leftPath);
  const [rightDraft, setRightDraft] = useState(rightPath);
  const [outPath, setOutPath] = useState("");
  const [filterMode, setFilterMode] = useState<FilterMode>("all");
  const [plan, setPlan] = useState<TreeMergePlan | null>(null);
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
    const lEff = l || "tools/parity-harness/fixtures/folder/left";
    const rEff = r || "tools/parity-harness/fixtures/folder/right";
    const bEff = b || lEff;
    try {
      const p = await mergeFolderPlan(bEff, lEff, rEff);
      setPlan(p);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void runPlan(baseDraft, leftPath, rightPath);
  }, [baseDraft, leftPath, rightPath, runPlan]);

  const visibleSteps = useMemo(() => {
    if (!plan) return [];
    if (filterMode === "all") return plan.steps;
    if (filterMode === "diffs") return plan.steps.filter((s) => s.action === "Conflict" || s.action === "MergedText");
    return plan.steps.filter((s) => s.action !== "Conflict");
  }, [plan, filterMode]);

  useEffect(() => {
    onRegisterCommands({
      filterMode,
      setFilterMode,
      swapSides: () => onPathsChange(rightDraft, leftDraft),
      reload: () => void runPlan(baseDraft, leftDraft, rightDraft),
    });
  }, [onRegisterCommands, filterMode, onPathsChange, baseDraft, leftDraft, rightDraft, runPlan]);

  useEffect(() => {
    const conflicts = plan?.conflicts.length ?? 0;
    const steps = plan?.steps.length ?? 0;
    onStatusChange({
      left: `${steps} planned steps · ${conflicts} conflicts`,
      center: report
        ? `Wrote ${report.written} files, ${report.dirsCreated} dirs → ${report.outPath}`
        : "3-Way Folder Merge",
      right: `${plan?.skipped.length ?? 0} skipped`,
    });
  }, [onStatusChange, plan, report]);

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
    try {
      const rep = await mergeFolderApply(lEff, rEff, plan, outPath.trim());
      setReport(rep);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setApplying(false);
    }
  };

  const handleBrowse = async (which: "base" | "left" | "right" | "out"): Promise<void> => {
    try {
      const mod = await import("@tauri-apps/plugin-dialog");
      const picked = await mod.open({ multiple: false, directory: true });
      if (typeof picked === "string" && picked) {
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
      }
    } catch {
      // Browser fallback
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden bg-[var(--v2-surface)]">
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
              <button type="button" onClick={() => void handleBrowse(key)} className="v2-btn px-1.5" title={`Browse ${label}`}>
                <FolderOpen size={12} />
              </button>
            </div>
          </div>
        ))}
      </form>

      {/* Output Target Bar */}
      <div className="flex shrink-0 items-center justify-between gap-2 border-b border-[var(--v2-border)] bg-[var(--v2-chrome)] px-3 py-1.5">
        <div className="flex min-w-0 flex-1 items-center gap-2">
          <span className="shrink-0 text-[11px] font-semibold text-[var(--v2-fg-secondary)]">
            Merge Output Folder
          </span>
          <input
            type="text"
            value={outPath}
            onChange={(e) => setOutPath(e.target.value)}
            placeholder="/path/to/empty-output-folder"
            className="v2-input v2-mono min-w-0 flex-1 max-w-md text-[11.5px]"
          />
          <button type="button" onClick={() => void handleBrowse("out")} className="v2-btn px-2" title="Browse output folder">
            <FolderOpen size={12} />
          </button>
        </div>
        <button
          type="button"
          disabled={applying || !plan || plan.steps.length === 0}
          onClick={() => void handleApply()}
          className="v2-btn v2-btn-primary"
        >
          <Play size={12} />
          {applying ? "Merging…" : "Apply Tree Merge"}
        </button>
      </div>

      {error ? (
        <div className="border-b border-red-300 bg-red-50 px-3 py-1.5 text-[12px] text-red-700">{error}</div>
      ) : null}

      {/* Merge Steps Table */}
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
            <div className="sticky top-0 grid grid-cols-[130px_1fr] border-b border-[var(--v2-border)] bg-[var(--v2-surface-subtle)] px-3 py-1 text-[11px] font-semibold text-[var(--v2-fg-secondary)]">
              <span>Merge Action</span>
              <span>Relative Path</span>
            </div>
            {visibleSteps.map((step, i) => {
              const badge = actionStyle(step.action);
              return (
                <div
                  key={`${step.rel_path}-${i}`}
                  className="grid h-[26px] grid-cols-[130px_1fr] items-center border-b border-[var(--v2-border-subtle)]/60 px-3 hover:bg-[var(--v2-btn-hover)]/40"
                >
                  <div>
                    <span
                      style={{ backgroundColor: badge.bg, color: badge.fg }}
                      className="inline-flex rounded px-1.5 py-0.5 text-[10.5px] font-semibold"
                    >
                      {badge.label}
                    </span>
                  </div>
                  <span className="v2-mono truncate text-[var(--v2-fg)]">{step.rel_path}</span>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
