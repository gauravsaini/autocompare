import React, { useCallback, useEffect, useMemo, useState } from "react";
import { Check, FolderOpen } from "lucide-react";
import { mergeTextApply, mergeTextPlan } from "../../ipc/merge";
import type { TextChoice, TextMergePlan, TextResolution } from "../../ipc/merge";
import type { V2StatusInfo, V2ViewCommands } from "../types";

export interface V2MergeCompareProps {
  readonly leftPath: string;
  readonly rightPath: string;
  readonly onPathsChange: (left: string, right: string) => void;
  readonly onRegisterCommands: (cmds: V2ViewCommands) => void;
  readonly onStatusChange: (status: V2StatusInfo) => void;
}

function renderChoiceLines(
  conflict: { base: readonly string[]; left: readonly string[]; right: readonly string[] },
  choice: TextChoice | undefined,
): string {
  if (!choice) {
    return [
      "<<<<<<< left",
      ...conflict.left,
      "=======",
      ...conflict.right,
      ">>>>>>> right",
    ].join("\n");
  }
  if (choice === "left") return conflict.left.join("\n");
  if (choice === "right") return conflict.right.join("\n");
  if (choice === "base") return conflict.base.join("\n");
  if (choice === "bothLeftFirst") return [...conflict.left, ...conflict.right].join("\n");
  if (choice === "bothRightFirst") return [...conflict.right, ...conflict.left].join("\n");
  return choice.custom.join("\n");
}

export default function V2MergeCompare({
  leftPath,
  rightPath,
  onPathsChange,
  onRegisterCommands,
  onStatusChange,
}: V2MergeCompareProps): React.JSX.Element {
  const [baseDraft, setBaseDraft] = useState("");
  const [leftDraft, setLeftDraft] = useState(leftPath);
  const [rightDraft, setRightDraft] = useState(rightPath);
  const [outPath, setOutPath] = useState("");

  const [plan, setPlan] = useState<TextMergePlan | null>(null);
  const [choices, setChoices] = useState<Record<number, TextChoice>>({});
  const [outputText, setOutputText] = useState("");
  const [manualOutputEdited, setManualOutputEdited] = useState(false);
  const [selectedConflict, setSelectedConflict] = useState(0);
  const [loading, setLoading] = useState(true);
  const [applying, setApplying] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [reportNote, setReportNote] = useState<string | null>(null);

  useEffect(() => {
    setLeftDraft(leftPath);
    setRightDraft(rightPath);
  }, [leftPath, rightPath]);

  const runPlan = useCallback(async (b: string, l: string, r: string): Promise<void> => {
    setLoading(true);
    setError(null);
    setReportNote(null);
    const lEff = l || "tools/parity-harness/fixtures/t_left.txt";
    const rEff = r || "tools/parity-harness/fixtures/t_right.txt";
    const bEff = b || lEff;
    try {
      const p = await mergeTextPlan(bEff, lEff, rEff);
      setPlan(p);
      setChoices({});
      setManualOutputEdited(false);
      setSelectedConflict(0);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void runPlan(baseDraft, leftPath, rightPath);
  }, [baseDraft, leftPath, rightPath, runPlan]);

  // Recompute live editable merged preview when conflict choices change
  useEffect(() => {
    if (!plan || manualOutputEdited) return;
    if (plan.conflicts.length === 0) {
      setOutputText("Clean merge — zero conflicts.");
      return;
    }
    const merged = plan.conflicts
      .map((c) => renderChoiceLines(c, choices[c.id]))
      .join("\n");
    setOutputText(merged);
  }, [plan, choices, manualOutputEdited]);

  const resolvedCount = useMemo(
    () => (plan ? plan.conflicts.filter((c) => choices[c.id] !== undefined).length : 0),
    [plan, choices],
  );

  useEffect(() => {
    const count = plan?.conflicts.length ?? 0;
    onRegisterCommands({
      hasSections: count > 0,
      prevSection: () => setSelectedConflict((i) => (count > 0 ? (i - 1 + count) % count : 0)),
      nextSection: () => setSelectedConflict((i) => (count > 0 ? (i + 1) % count : 0)),
      swapSides: () => onPathsChange(rightDraft, leftDraft),
      reload: () => void runPlan(baseDraft, leftDraft, rightDraft),
    });
  }, [onRegisterCommands, plan, onPathsChange, baseDraft, leftDraft, rightDraft, runPlan]);

  useEffect(() => {
    const total = plan?.conflictCount ?? 0;
    onStatusChange({
      left: `${total} conflict ${total === 1 ? "section" : "sections"} · ${resolvedCount} resolved`,
      center: reportNote ?? (manualOutputEdited ? "Output manually edited" : "Live 3-way merge"),
      right: `${plan?.cleanBlockCount ?? 0} clean blocks`,
    });
  }, [onStatusChange, plan, resolvedCount, reportNote, manualOutputEdited]);

  const setChoice = (id: number, choice: TextChoice): void => {
    setManualOutputEdited(false);
    setChoices((prev) => ({ ...prev, [id]: choice }));
  };

  const handleApply = async (): Promise<void> => {
    if (!plan) return;
    if (!outPath.trim()) {
      setError("Enter an output file path before applying the merge.");
      return;
    }
    setApplying(true);
    setError(null);
    try {
      const resolutions: TextResolution[] = plan.conflicts
        .filter((c) => choices[c.id] !== undefined)
        .map((c) => ({ conflictId: c.id, choice: choices[c.id] }));
      const lEff = leftDraft || "tools/parity-harness/fixtures/t_left.txt";
      const rEff = rightDraft || "tools/parity-harness/fixtures/t_right.txt";
      const bEff = baseDraft || lEff;
      const rep = await mergeTextApply(bEff, lEff, rEff, resolutions, outPath.trim());
      setReportNote(
        `Wrote ${rep.writtenLines} lines to ${rep.outPath} (${rep.resolvedCount}/${rep.conflictCount} resolved)`,
      );
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setApplying(false);
    }
  };

  const handleBrowse = async (which: "base" | "left" | "right"): Promise<void> => {
    try {
      const mod = await import("@tauri-apps/plugin-dialog");
      const picked = await mod.open({ multiple: false, directory: false });
      if (typeof picked === "string" && picked) {
        if (which === "base") setBaseDraft(picked);
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
      {/* 3-Pane Path Header: Left | Base (Center) | Right */}
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
            ["left", "Left (Theirs)", leftDraft, setLeftDraft],
            ["base", "Center (Base)", baseDraft, setBaseDraft],
            ["right", "Right (Mine)", rightDraft, setRightDraft],
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

      {error ? (
        <div className="border-b border-red-300 bg-red-50 px-3 py-1.5 text-[12px] text-red-700">{error}</div>
      ) : null}

      {/* Top Bank: 3-Way Conflict Cards */}
      <div className="min-h-0 flex-1 overflow-y-auto bg-[var(--v2-bg)] p-3">
        {loading ? (
          <div className="flex h-full items-center justify-center text-[12px] text-[var(--v2-fg-muted)]">
            Planning 3-way merge…
          </div>
        ) : !plan || plan.conflicts.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center gap-1 text-[var(--v2-fg-muted)]">
            <p className="text-[13px] font-medium text-[var(--v2-fg)]">No merge conflicts</p>
            <p className="text-[12px]">All {plan?.cleanBlockCount ?? 0} blocks merge cleanly.</p>
          </div>
        ) : (
          <div className="space-y-3">
            {plan.conflicts.map((conflict, idx) => {
              const active = choices[conflict.id];
              const isSelected = idx === selectedConflict;
              return (
                <div
                  key={conflict.id}
                  onClick={() => setSelectedConflict(idx)}
                  className={`overflow-hidden rounded-[8px] border bg-[var(--v2-surface)] shadow-xs ${
                    isSelected ? "border-[var(--v2-accent)] ring-1 ring-[var(--v2-accent-soft)]" : "border-[var(--v2-border)]"
                  }`}
                >
                  <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[var(--v2-border-subtle)] bg-[var(--v2-surface-subtle)] px-3 py-1.5">
                    <span className="text-[12px] font-semibold text-[var(--v2-fg)]">
                      Conflict #{idx + 1}
                    </span>
                    <div className="flex flex-wrap items-center gap-1.5">
                      {(
                        [
                          ["left", "Take Left"],
                          ["base", "Take Base"],
                          ["right", "Take Right"],
                          ["bothLeftFirst", "Both (L→R)"],
                          ["bothRightFirst", "Both (R→L)"],
                        ] as const
                      ).map(([val, label]) => {
                        const picked = active === val;
                        return (
                          <button
                            key={val}
                            type="button"
                            onClick={(e) => {
                              e.stopPropagation();
                              setChoice(conflict.id, val);
                            }}
                            className={`v2-btn h-[22px] text-[11px] ${picked ? "v2-btn-primary" : ""}`}
                          >
                            {picked ? <Check size={11} /> : null}
                            {label}
                          </button>
                        );
                      })}
                    </div>
                  </div>
                  <div className="v2-mono grid grid-cols-3 text-[11.5px] leading-[19px]">
                    <pre
                      className="m-0 overflow-x-auto border-r border-[var(--v2-border-subtle)] p-2.5"
                      style={{ backgroundColor: "var(--v2-del-bg)", color: "var(--v2-del-fg)" }}
                    >
                      {conflict.left.join("\n") || "(empty)"}
                    </pre>
                    <pre className="m-0 overflow-x-auto border-r border-[var(--v2-border-subtle)] bg-[var(--v2-surface)] p-2.5 text-[var(--v2-fg-secondary)]">
                      {conflict.base.join("\n") || "(empty)"}
                    </pre>
                    <pre
                      className="m-0 overflow-x-auto p-2.5"
                      style={{ backgroundColor: "var(--v2-ins-bg)", color: "var(--v2-ins-fg)" }}
                    >
                      {conflict.right.join("\n") || "(empty)"}
                    </pre>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* Bottom Editable Output Pane (BC 4th Pane) */}
      <div className="flex h-[210px] shrink-0 flex-col border-t border-[var(--v2-border)] bg-[var(--v2-surface)]">
        <div className="flex shrink-0 items-center justify-between gap-2 border-b border-[var(--v2-border-subtle)] bg-[var(--v2-chrome)] px-3 py-1.5">
          <span className="text-[11px] font-semibold uppercase tracking-wide text-[var(--v2-fg-secondary)]">
            Merged Output (Editable)
          </span>
          <div className="flex items-center gap-2">
            <input
              type="text"
              value={outPath}
              onChange={(e) => setOutPath(e.target.value)}
              placeholder="Output file path…"
              className="v2-input v2-mono w-64 text-[11.5px]"
            />
            <button
              type="button"
              disabled={applying}
              onClick={() => void handleApply()}
              className="v2-btn v2-btn-primary"
            >
              {applying ? "Saving…" : "Save Output"}
            </button>
          </div>
        </div>
        <textarea
          value={outputText}
          onChange={(e) => {
            setManualOutputEdited(true);
            setOutputText(e.target.value);
          }}
          spellCheck={false}
          className="v2-mono min-h-0 flex-1 resize-none border-0 bg-[var(--v2-surface)] p-3 text-[12px] leading-[19px] text-[var(--v2-fg)] outline-none"
        />
      </div>
    </div>
  );
}
