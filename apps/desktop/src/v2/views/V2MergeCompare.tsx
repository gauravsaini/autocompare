import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Check, ChevronDown, ChevronUp, Edit3, FolderOpen, RotateCcw } from "lucide-react";
import { mergeTextApply, mergeTextPlan } from "../../ipc/merge";
import type { TextChoice, TextMergePlan, TextResolution } from "../../ipc/merge";
import V2ErrorBanner from "../components/V2ErrorBanner";
import { isMissingFixtureError } from "../utils/demoGuard";
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

function isCustomChoice(choice: TextChoice | undefined): choice is { readonly custom: readonly string[] } {
  return typeof choice === "object" && choice !== null && "custom" in choice;
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
  const [editingConflictId, setEditingConflictId] = useState<number | null>(null);
  const [outputText, setOutputText] = useState("");
  const [manualOutputEdited, setManualOutputEdited] = useState(false);
  const [selectedConflict, setSelectedConflict] = useState(0);
  const [loading, setLoading] = useState(true);
  const [applying, setApplying] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [reportNote, setReportNote] = useState<string | null>(null);

  const cardRefs = useRef<Record<number, HTMLDivElement | null>>({});
  const paneScrollRefs = useRef<Record<string, HTMLPreElement | null>>({});
  const syncingScrollRef = useRef(false);

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
      setEditingConflictId(null);
      setManualOutputEdited(false);
      setSelectedConflict(0);
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

  const jumpToConflict = useCallback(
    (nextIdx: number): void => {
      if (!plan || plan.conflicts.length === 0) return;
      const clamped = (nextIdx + plan.conflicts.length) % plan.conflicts.length;
      setSelectedConflict(clamped);
      const id = plan.conflicts[clamped]?.id;
      if (id !== undefined) {
        cardRefs.current[id]?.scrollIntoView({ block: "nearest", behavior: "smooth" });
      }
    },
    [plan],
  );

  const setChoice = useCallback((id: number, choice: TextChoice): void => {
    setManualOutputEdited(false);
    setChoices((prev) => ({ ...prev, [id]: choice }));
  }, []);

  const applyAll = useCallback(
    (choice: "left" | "right" | "base" | "bothLeftFirst"): void => {
      if (!plan) return;
      const next: Record<number, TextChoice> = {};
      for (const c of plan.conflicts) next[c.id] = choice;
      setManualOutputEdited(false);
      setChoices(next);
      setReportNote(`Applied "${choice}" to all ${plan.conflicts.length} conflict(s)`);
    },
    [plan],
  );

  // Sync horizontal scroll across Left | Base | Right panes within each conflict card
  const handleSyncHorizontalScroll = useCallback((conflictId: number, sourceKey: string, scrollLeft: number): void => {
    if (syncingScrollRef.current) return;
    syncingScrollRef.current = true;
    for (const side of ["left", "base", "right"] as const) {
      const k = `${conflictId}:${side}`;
      if (k === sourceKey) continue;
      const el = paneScrollRefs.current[k];
      if (el) el.scrollLeft = scrollLeft;
    }
    syncingScrollRef.current = false;
  }, []);

  // Keyboard shortcuts for rapid conflict resolution (Alt+1/2/3/4, Alt+N/P)
  useEffect(() => {
    const onKey = (e: KeyboardEvent): void => {
      if (!e.altKey || !plan || plan.conflicts.length === 0) return;
      const active = plan.conflicts[selectedConflict];
      if (!active) return;
      const key = e.key.toLowerCase();
      if (key === "1") {
        e.preventDefault();
        setChoice(active.id, "left");
      } else if (key === "2") {
        e.preventDefault();
        setChoice(active.id, "base");
      } else if (key === "3") {
        e.preventDefault();
        setChoice(active.id, "right");
      } else if (key === "4") {
        e.preventDefault();
        setChoice(active.id, "bothLeftFirst");
      } else if (key === "n") {
        e.preventDefault();
        jumpToConflict(selectedConflict + 1);
      } else if (key === "p") {
        e.preventDefault();
        jumpToConflict(selectedConflict - 1);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [plan, selectedConflict, setChoice, jumpToConflict]);

  useEffect(() => {
    const count = plan?.conflicts.length ?? 0;
    onRegisterCommands({
      hasSections: count > 0,
      prevSection: () => jumpToConflict(selectedConflict - 1),
      nextSection: () => jumpToConflict(selectedConflict + 1),
      swapSides: () => onPathsChange(rightDraft, leftDraft),
      reload: () => void runPlan(baseDraft, leftDraft, rightDraft),
    });
  }, [onRegisterCommands, plan, selectedConflict, jumpToConflict, onPathsChange, baseDraft, leftDraft, rightDraft, runPlan]);

  useEffect(() => {
    const total = plan?.conflictCount ?? 0;
    onStatusChange({
      left: `${total} conflict ${total === 1 ? "section" : "sections"} · ${resolvedCount} resolved`,
      center: reportNote ?? (manualOutputEdited ? "Output manually edited" : "Alt+1/2/3/4 resolve · Alt+N/P jump"),
      right: `${plan?.cleanBlockCount ?? 0} clean blocks`,
    });
  }, [onStatusChange, plan, resolvedCount, reportNote, manualOutputEdited]);

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

  const handleBrowse = async (which: "base" | "left" | "right" | "out"): Promise<void> => {
    try {
      const mod = await import("@tauri-apps/plugin-dialog");
      if (which === "out") {
        const saved = await mod.save({ title: "Save Merged Output" });
        if (typeof saved === "string" && saved) setOutPath(saved);
        return;
      }
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
    <div data-testid="v2/merge-view" className="flex h-full min-h-0 flex-col overflow-hidden bg-[var(--v2-surface)]">
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
      </form>

      {/* Conflict Navigation & Bulk Resolution Bar */}
      {plan && plan.conflicts.length > 0 ? (
        <div className="flex shrink-0 flex-wrap items-center justify-between gap-2 border-b border-[var(--v2-border)] bg-[var(--v2-chrome)] px-3 py-1 text-[11.5px]">
          <div className="flex items-center gap-1.5">
            <span className="font-medium text-[var(--v2-fg)]">
              Conflict {selectedConflict + 1} / {plan.conflicts.length}
            </span>
            <span className="text-[var(--v2-fg-muted)]">
              ({resolvedCount} resolved)
            </span>
            <button
              type="button"
              onClick={() => jumpToConflict(selectedConflict - 1)}
              title="Previous Conflict (Alt+P)"
              className="v2-btn h-[21px] px-1.5"
            >
              <ChevronUp size={12} />
            </button>
            <button
              type="button"
              onClick={() => jumpToConflict(selectedConflict + 1)}
              title="Next Conflict (Alt+N)"
              className="v2-btn h-[21px] px-1.5"
            >
              <ChevronDown size={12} />
            </button>
          </div>
          <div className="flex flex-wrap items-center gap-1.5">
            <button type="button" onClick={() => applyAll("left")} className="v2-btn h-[21px] text-[11px]">
              Take All Left
            </button>
            <button type="button" onClick={() => applyAll("base")} className="v2-btn h-[21px] text-[11px]">
              Take All Base
            </button>
            <button type="button" onClick={() => applyAll("right")} className="v2-btn h-[21px] text-[11px]">
              Take All Right
            </button>
            <button type="button" onClick={() => applyAll("bothLeftFirst")} className="v2-btn h-[21px] text-[11px]">
              Take All Both
            </button>
            <button
              type="button"
              onClick={() => {
                void navigator.clipboard?.writeText(outputText).then(
                  () => setReportNote("Copied merged output with conflict markers"),
                  () => setReportNote("Could not access clipboard"),
                );
              }}
              title="Copy merged output (including Git conflict markers for unresolved conflicts)"
              className="v2-btn h-[21px] text-[11px]"
            >
              Copy Markers
            </button>
            <button
              type="button"
              onClick={() => {
                setChoices({});
                setEditingConflictId(null);
                setManualOutputEdited(false);
              }}
              title="Reset all conflict choices"
              className="v2-btn h-[21px] px-2 text-[11px]"
            >
              <RotateCcw size={11} />
              Reset
            </button>
          </div>
        </div>
      ) : null}

      <V2ErrorBanner message={error} onDismiss={() => setError(null)} />

      {/* Top Bank: 3-Way Synchronized Conflict Cards with Inline Custom Editor */}
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
              const isEditingCustom = editingConflictId === conflict.id;
              const customLinesText = isCustomChoice(active)
                ? active.custom.join("\n")
                : renderChoiceLines(conflict, active);

              return (
                <div
                  key={conflict.id}
                  ref={(el) => {
                    cardRefs.current[conflict.id] = el;
                  }}
                  onClick={() => setSelectedConflict(idx)}
                  className={`overflow-hidden rounded-[8px] border bg-[var(--v2-surface)] shadow-xs ${
                    isSelected
                      ? "border-[var(--v2-accent)] ring-1 ring-[var(--v2-accent-soft)]"
                      : "border-[var(--v2-border)]"
                  }`}
                >
                  <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[var(--v2-border-subtle)] bg-[var(--v2-surface-subtle)] px-3 py-1.5">
                    <div className="flex items-center gap-2">
                      <span className="text-[12px] font-semibold text-[var(--v2-fg)]">
                        Conflict #{idx + 1}
                      </span>
                      <span
                        className={`rounded-full px-2 py-0.5 text-[10px] font-medium ${
                          active
                            ? "bg-[var(--v2-ins-bg)] text-[var(--v2-ins-fg)]"
                            : "bg-[var(--v2-chg-bg)] text-[var(--v2-chg-fg)]"
                        }`}
                      >
                        {active ? (isCustomChoice(active) ? "Custom" : "Resolved") : "Unresolved"}
                      </span>
                    </div>
                    <div className="flex flex-wrap items-center gap-1.5">
                      {(
                        [
                          ["left", "Take Left", "Alt+1"],
                          ["base", "Take Base", "Alt+2"],
                          ["right", "Take Right", "Alt+3"],
                          ["bothLeftFirst", "Both (L→R)", "Alt+4"],
                          ["bothRightFirst", "Both (R→L)", ""],
                        ] as const
                      ).map(([val, label, hint]) => {
                        const picked = active === val;
                        return (
                          <button
                            key={val}
                            type="button"
                            title={hint ? `${label} (${hint})` : label}
                            onClick={(e) => {
                              e.stopPropagation();
                              setEditingConflictId(null);
                              setChoice(conflict.id, val);
                            }}
                            className={`v2-btn h-[22px] text-[11px] ${picked ? "v2-btn-primary" : ""}`}
                          >
                            {picked ? <Check size={11} /> : null}
                            {label}
                          </button>
                        );
                      })}
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          if (!isEditingCustom && !isCustomChoice(active)) {
                            setChoice(conflict.id, {
                              custom: renderChoiceLines(conflict, active).split("\n"),
                            });
                          }
                          setEditingConflictId(isEditingCustom ? null : conflict.id);
                        }}
                        className={`v2-btn h-[22px] text-[11px] ${
                          isEditingCustom || isCustomChoice(active) ? "v2-btn-primary" : ""
                        }`}
                      >
                        <Edit3 size={11} />
                        Edit Inline
                      </button>
                    </div>
                  </div>

                  {/* Synchronized 3-Pane Conflict Body */}
                  <div className="v2-mono grid grid-cols-3 text-[11.5px] leading-[19px]">
                    <pre
                      ref={(el) => {
                        paneScrollRefs.current[`${conflict.id}:left`] = el;
                      }}
                      onScroll={(e) =>
                        handleSyncHorizontalScroll(
                          conflict.id,
                          `${conflict.id}:left`,
                          e.currentTarget.scrollLeft,
                        )
                      }
                      className="m-0 overflow-x-auto border-r border-[var(--v2-border-subtle)] p-2.5"
                      style={{ backgroundColor: "var(--v2-del-bg)", color: "var(--v2-del-fg)" }}
                    >
                      {conflict.left.join("\n") || "(empty)"}
                    </pre>
                    <pre
                      ref={(el) => {
                        paneScrollRefs.current[`${conflict.id}:base`] = el;
                      }}
                      onScroll={(e) =>
                        handleSyncHorizontalScroll(
                          conflict.id,
                          `${conflict.id}:base`,
                          e.currentTarget.scrollLeft,
                        )
                      }
                      className="m-0 overflow-x-auto border-r border-[var(--v2-border-subtle)] bg-[var(--v2-surface)] p-2.5 text-[var(--v2-fg-secondary)]"
                    >
                      {conflict.base.join("\n") || "(empty)"}
                    </pre>
                    <pre
                      ref={(el) => {
                        paneScrollRefs.current[`${conflict.id}:right`] = el;
                      }}
                      onScroll={(e) =>
                        handleSyncHorizontalScroll(
                          conflict.id,
                          `${conflict.id}:right`,
                          e.currentTarget.scrollLeft,
                        )
                      }
                      className="m-0 overflow-x-auto p-2.5"
                      style={{ backgroundColor: "var(--v2-ins-bg)", color: "var(--v2-ins-fg)" }}
                    >
                      {conflict.right.join("\n") || "(empty)"}
                    </pre>
                  </div>

                  {/* Per-Conflict Inline Custom Resolution Editor */}
                  {isEditingCustom ? (
                    <div className="border-t border-[var(--v2-border)] bg-[var(--v2-surface-subtle)] p-2.5">
                      <div className="mb-1 flex items-center justify-between text-[11px] text-[var(--v2-fg-secondary)]">
                        <span className="font-medium">Custom resolution for Conflict #{idx + 1}</span>
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            setEditingConflictId(null);
                          }}
                          className="v2-btn h-[20px] px-2 text-[10.5px]"
                        >
                          Done
                        </button>
                      </div>
                      <textarea
                        value={customLinesText}
                        onClick={(e) => e.stopPropagation()}
                        onChange={(e) =>
                          setChoice(conflict.id, { custom: e.target.value.split("\n") })
                        }
                        rows={Math.max(3, Math.min(10, customLinesText.split("\n").length + 1))}
                        spellCheck={false}
                        className="v2-input v2-mono h-auto w-full resize-y py-1.5 text-[11.5px] leading-[18px]"
                      />
                    </div>
                  ) : null}
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
          <div className="flex items-center gap-1.5">
            <input
              type="text"
              value={outPath}
              onChange={(e) => setOutPath(e.target.value)}
              placeholder="Output file path…"
              className="v2-input v2-mono w-64 text-[11.5px]"
            />
            <button
              type="button"
              onClick={() => void handleBrowse("out")}
              title="Choose output file"
              className="v2-btn px-1.5"
            >
              <FolderOpen size={12} />
            </button>
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
