import React, { useEffect, useState } from "react";
import { ArrowLeftRight, Plus, Trash2 } from "lucide-react";
import V2Modal from "../V2Modal";
import { DEFAULT_UNIMPORTANT_PRESETS } from "../../utils/textReplacements";
import type { TextReplacementRule, UnimportantRule } from "../../utils/textReplacements";

export type RulesDialogTab = "specs" | "format" | "importance" | "alignment" | "replacements";
export type AlignmentMode = "unaligned" | "standard" | "myers" | "patience";

export interface TextSessionRules {
  // Format tab
  readonly leftFormat: string;
  readonly rightFormat: string;
  readonly sameFormat: boolean;
  readonly leftEncoding: string;
  readonly rightEncoding: string;
  readonly sameEncoding: boolean;
  readonly eol: string;
  // Importance tab
  readonly ignoreWs: boolean;
  readonly ignoreCase: boolean;
  readonly ignoreBlank: boolean;
  readonly minorEnabled: boolean;
  readonly unimportantRules: readonly UnimportantRule[];
  // Alignment tab
  readonly alignmentMode: AlignmentMode;
  readonly neverAlignDifferences: boolean;
  readonly skewTolerance: number;
  readonly useClosenessMatching: boolean;
  // Replacements tab
  readonly replacements: readonly TextReplacementRule[];
}

export const FILE_FORMATS: readonly string[] = [
  "Detected (Everything Else)",
  "Plain Text",
  "Source Code",
  "JSON",
  "YAML",
  "XML / HTML",
  "CSV / TSV",
  "Markdown",
];

export const ENCODING_OPTIONS: readonly string[] = [
  "None",
  "Current Locale (UTF-8)",
  "UTF-8",
  "UTF-16LE",
  "UTF-16BE",
  "ASCII",
  "Windows-1252",
  "ISO-8859-1",
  "GB18030",
  "Shift-JIS",
  "Windows-1256",
];

export interface V2TextRulesDialogProps {
  readonly open: boolean;
  readonly initialTab: RulesDialogTab;
  readonly rules: TextSessionRules;
  readonly leftPath: string;
  readonly rightPath: string;
  readonly onClose: () => void;
  readonly onApply: (
    next: TextSessionRules,
    scope: "view" | "defaults",
    paths?: { readonly left: string; readonly right: string },
  ) => void;
}

export function V2TextRulesModal({
  open,
  initialTab,
  rules,
  leftPath,
  rightPath,
  onClose,
  onApply,
}: V2TextRulesDialogProps): React.JSX.Element {
  const [tab, setTab] = useState<RulesDialogTab>(initialTab);
  const [draft, setDraft] = useState<TextSessionRules>(rules);
  const [leftSpec, setLeftSpec] = useState(leftPath);
  const [rightSpec, setRightSpec] = useState(rightPath);
  const [scope, setScope] = useState<"view" | "defaults">("view");
  const [selectedRuleId, setSelectedRuleId] = useState<string | null>(null);
  const [selectedUnimpId, setSelectedUnimpId] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setTab(initialTab);
      setDraft(rules);
      setLeftSpec(leftPath);
      setRightSpec(rightPath);
      setSelectedRuleId(rules.replacements[0]?.id ?? null);
      setSelectedUnimpId(rules.unimportantRules[0]?.id ?? null);
    }
  }, [open, initialTab, rules, leftPath, rightPath]);

  const patch = (partial: Partial<TextSessionRules>): void => {
    setDraft((prev) => ({ ...prev, ...partial }));
  };

  const addUnimportant = (preset?: Omit<UnimportantRule, "id">): void => {
    const id = `unimp-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
    const nextRule: UnimportantRule = preset
      ? { id, ...preset }
      : {
          id,
          name: "Custom Rule",
          pattern: "",
          regex: true,
          caseSensitive: false,
          enabled: true,
        };
    patch({
      minorEnabled: true,
      unimportantRules: [...draft.unimportantRules, nextRule],
    });
    setSelectedUnimpId(id);
  };

  const updateUnimportant = (id: string, partial: Partial<UnimportantRule>): void => {
    patch({
      unimportantRules: draft.unimportantRules.map((r) => (r.id === id ? { ...r, ...partial } : r)),
    });
  };

  const removeSelectedUnimportant = (): void => {
    if (!selectedUnimpId) return;
    const next = draft.unimportantRules.filter((r) => r.id !== selectedUnimpId);
    patch({ unimportantRules: next });
    setSelectedUnimpId(next[0]?.id ?? null);
  };

  const addReplacement = (): void => {
    const id = `rep-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
    const nextRule: TextReplacementRule = {
      id,
      left: "",
      right: "",
      regex: false,
      caseSensitive: false,
      enabled: true,
    };
    patch({ replacements: [...draft.replacements, nextRule] });
    setSelectedRuleId(id);
  };

  const updateReplacement = (id: string, partial: Partial<TextReplacementRule>): void => {
    patch({
      replacements: draft.replacements.map((r) => (r.id === id ? { ...r, ...partial } : r)),
    });
  };

  const removeSelectedReplacement = (): void => {
    if (!selectedRuleId) return;
    const next = draft.replacements.filter((r) => r.id !== selectedRuleId);
    patch({ replacements: next });
    setSelectedRuleId(next[0]?.id ?? null);
  };

  return (
    <V2Modal
      open={open}
      title="Text Compare Session Settings"
      subtitle="Configure specs, format, importance, alignment, and replacement rules"
      widthClass="max-w-xl"
      onClose={onClose}
      footer={
        <div className="flex w-full items-center justify-between gap-2">
          <select
            aria-label="Settings application scope"
            value={scope}
            onChange={(e) => setScope(e.target.value as "view" | "defaults")}
            className="v2-input text-[11.5px]"
          >
            <option value="view">Use for this view only</option>
            <option value="defaults">Also update session defaults</option>
          </select>
          <div className="flex items-center gap-2">
            <button type="button" onClick={onClose} className="v2-btn">
              Cancel
            </button>
            <button
              type="button"
              data-testid="v2/rules-ok"
              onClick={() => {
                const pathsChanged = leftSpec !== leftPath || rightSpec !== rightPath;
                onApply(
                  draft,
                  scope,
                  pathsChanged ? { left: leftSpec, right: rightSpec } : undefined,
                );
              }}
              className="v2-btn v2-btn-primary"
            >
              OK
            </button>
          </div>
        </div>
      }
    >
      <div className="space-y-4 text-[12px]">
        {/* Beyond Compare–style 5-Tab Segmented Strip */}
        <div className="flex justify-center">
          <div
            role="tablist"
            className="inline-flex rounded-[7px] bg-[var(--v2-seg-track)] p-0.5"
          >
            {(
              [
                ["specs", "Specs"],
                ["format", "Format"],
                ["importance", "Importance"],
                ["alignment", "Alignment"],
                ["replacements", "Replacements"],
              ] as const
            ).map(([id, label]) => {
              const active = tab === id;
              return (
                <button
                  key={id}
                  type="button"
                  role="tab"
                  aria-selected={active}
                  data-testid={`v2/rules-tab-${id}`}
                  onClick={() => setTab(id)}
                  className={`h-[24px] rounded-[6px] px-3 text-[12px] font-medium transition ${
                    active
                      ? "bg-[var(--v2-surface)] text-[var(--v2-fg)] shadow-xs"
                      : "text-[var(--v2-fg-secondary)] hover:text-[var(--v2-fg)]"
                  }`}
                >
                  {label}
                </button>
              );
            })}
          </div>
        </div>

        {/* 1. SPECS TAB */}
        {tab === "specs" ? (
          <div className="space-y-3 py-1">
            <div>
              <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-secondary)]">
                Left file:
              </span>
              <input
                type="text"
                data-testid="v2/rules-spec-left"
                value={leftSpec}
                onChange={(e) => setLeftSpec(e.target.value)}
                placeholder="/path/to/left.txt"
                className="v2-input v2-mono w-full"
              />
            </div>

            <div>
              <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-secondary)]">
                Right file:
              </span>
              <input
                type="text"
                data-testid="v2/rules-spec-right"
                value={rightSpec}
                onChange={(e) => setRightSpec(e.target.value)}
                placeholder="/path/to/right.txt"
                className="v2-input v2-mono w-full"
              />
            </div>

            <div className="flex items-center justify-between pt-1">
              <button
                type="button"
                onClick={() => {
                  setLeftSpec(rightSpec);
                  setRightSpec(leftSpec);
                }}
                className="v2-btn h-[24px] px-2.5"
              >
                <ArrowLeftRight size={12} />
                Swap Sides
              </button>
              <span className="text-[11px] text-[var(--v2-fg-muted)]">
                Changes reload both files when you click OK.
              </span>
            </div>
          </div>
        ) : null}

        {/* 2. FORMAT TAB */}
        {tab === "format" ? (
          <div className="space-y-3">
            <div>
              <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-secondary)]">
                Left file format:
              </span>
              <select
                aria-label="Left file format"
                value={draft.leftFormat}
                onChange={(e) =>
                  patch({
                    leftFormat: e.target.value,
                    ...(draft.sameFormat ? { rightFormat: e.target.value } : {}),
                  })
                }
                className="v2-input w-full"
              >
                {FILE_FORMATS.map((f) => (
                  <option key={f} value={f}>
                    {f}
                  </option>
                ))}
              </select>
            </div>

            <div>
              <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-secondary)]">
                Right file format:
              </span>
              <select
                aria-label="Right file format"
                disabled={draft.sameFormat}
                value={draft.sameFormat ? draft.leftFormat : draft.rightFormat}
                onChange={(e) => patch({ rightFormat: e.target.value })}
                className="v2-input w-full disabled:opacity-50"
              >
                {FILE_FORMATS.map((f) => (
                  <option key={f} value={f}>
                    {f}
                  </option>
                ))}
              </select>
              <label className="mt-1.5 flex cursor-pointer items-center gap-2">
                <input
                  type="checkbox"
                  checked={draft.sameFormat}
                  onChange={(e) =>
                    patch({
                      sameFormat: e.target.checked,
                      rightFormat: e.target.checked ? draft.leftFormat : draft.rightFormat,
                    })
                  }
                  className="accent-[var(--v2-accent)]"
                />
                <span>Same as left file format</span>
              </label>
            </div>

            <div className="grid grid-cols-2 gap-3 pt-1">
              <div>
                <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-secondary)]">
                  Left encoding override:
                </span>
                <select
                  aria-label="Left encoding override"
                  data-testid="v2/rules-left-encoding"
                  value={draft.leftEncoding}
                  onChange={(e) =>
                    patch({
                      leftEncoding: e.target.value,
                      ...(draft.sameEncoding ? { rightEncoding: e.target.value } : {}),
                    })
                  }
                  className="v2-input w-full"
                >
                  {ENCODING_OPTIONS.map((enc) => (
                    <option key={enc} value={enc}>
                      {enc}
                    </option>
                  ))}
                </select>
              </div>

              <div>
                <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-secondary)]">
                  Right encoding override:
                </span>
                <select
                  aria-label="Right encoding override"
                  data-testid="v2/rules-right-encoding"
                  disabled={draft.sameEncoding}
                  value={draft.sameEncoding ? draft.leftEncoding : draft.rightEncoding}
                  onChange={(e) => patch({ rightEncoding: e.target.value })}
                  className="v2-input w-full disabled:opacity-50"
                >
                  {ENCODING_OPTIONS.map((enc) => (
                    <option key={enc} value={enc}>
                      {enc}
                    </option>
                  ))}
                </select>
              </div>
            </div>

            <label className="flex cursor-pointer items-center gap-2">
              <input
                type="checkbox"
                checked={draft.sameEncoding}
                onChange={(e) =>
                  patch({
                    sameEncoding: e.target.checked,
                    rightEncoding: e.target.checked ? draft.leftEncoding : draft.rightEncoding,
                  })
                }
                className="accent-[var(--v2-accent)]"
              />
              <span>Same as left encoding</span>
            </label>

            <div>
              <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-secondary)]">
                Line Endings (EOL):
              </span>
              <select
                aria-label="Line Endings (EOL)"
                value={draft.eol}
                onChange={(e) => patch({ eol: e.target.value })}
                className="v2-input w-full"
              >
                <option value="UNIX">UNIX (LF)</option>
                <option value="DOS">Windows (CRLF)</option>
                <option value="MIX">Mixed</option>
              </select>
            </div>
          </div>
        ) : null}

        {/* 3. IMPORTANCE TAB (with Unimportant Grammar / Regex Rules) */}
        {tab === "importance" ? (
          <div className="space-y-3 py-1">
            <div className="grid grid-cols-2 gap-2">
              <label className="flex cursor-pointer items-center gap-2">
                <input
                  type="checkbox"
                  checked={draft.ignoreWs}
                  onChange={(e) => patch({ ignoreWs: e.target.checked })}
                  className="accent-[var(--v2-accent)]"
                />
                <span>Ignore whitespace changes</span>
              </label>
              <label className="flex cursor-pointer items-center gap-2">
                <input
                  type="checkbox"
                  checked={draft.ignoreCase}
                  onChange={(e) => patch({ ignoreCase: e.target.checked })}
                  className="accent-[var(--v2-accent)]"
                />
                <span>Ignore character case</span>
              </label>
              <label className="flex cursor-pointer items-center gap-2">
                <input
                  type="checkbox"
                  checked={draft.ignoreBlank}
                  onChange={(e) => patch({ ignoreBlank: e.target.checked })}
                  className="accent-[var(--v2-accent)]"
                />
                <span>Ignore blank lines</span>
              </label>
              <label className="flex cursor-pointer items-center gap-2">
                <input
                  type="checkbox"
                  checked={draft.minorEnabled}
                  onChange={(e) => patch({ minorEnabled: e.target.checked })}
                  className="accent-[var(--v2-accent)]"
                />
                <span>Ignore unimportant text</span>
              </label>
            </div>

            <div className="space-y-2 pt-1">
              <div className="flex items-center justify-between">
                <span className="text-[11px] font-medium text-[var(--v2-fg-secondary)]">
                  Unimportant text / grammar rules:
                </span>
                <div className="flex flex-wrap items-center gap-1">
                  {DEFAULT_UNIMPORTANT_PRESETS.map((preset, idx) => (
                    <button
                      key={preset.name}
                      type="button"
                      data-testid={`v2/rules-add-preset-${idx}`}
                      onClick={() => addUnimportant(preset)}
                      className="v2-btn h-[20px] px-1.5 text-[10.5px]"
                    >
                      + {preset.name}
                    </button>
                  ))}
                </div>
              </div>

              <div className="overflow-hidden rounded-[6px] border border-[var(--v2-border)] bg-[var(--v2-surface-subtle)]">
                <div className="grid grid-cols-[24px_110px_1fr_48px_48px] border-b border-[var(--v2-border)] bg-[var(--v2-chrome)] px-2 py-1 text-[11px] font-semibold text-[var(--v2-fg-secondary)]">
                  <span />
                  <span>Name</span>
                  <span>Pattern</span>
                  <span className="text-center">Regex</span>
                  <span className="text-center">Case</span>
                </div>
                <div className="max-h-36 min-h-[84px] overflow-y-auto divide-y divide-[var(--v2-border-subtle)]">
                  {draft.unimportantRules.length === 0 ? (
                    <div className="flex h-[84px] items-center justify-center text-[11px] text-[var(--v2-fg-muted)]">
                      Add a preset above (Timestamps, Comments, UUIDs) or click + Add.
                    </div>
                  ) : (
                    draft.unimportantRules.map((rule) => {
                      const selected = selectedUnimpId === rule.id;
                      return (
                        <div
                          key={rule.id}
                          onClick={() => setSelectedUnimpId(rule.id)}
                          className={`grid grid-cols-[24px_110px_1fr_48px_48px] items-center gap-1.5 px-2 py-1 ${
                            selected ? "bg-[var(--v2-accent-soft)]" : ""
                          }`}
                        >
                          <input
                            type="checkbox"
                            aria-label={`Enable unimportant rule ${rule.name || rule.id}`}
                            checked={rule.enabled}
                            onChange={(e) =>
                              updateUnimportant(rule.id, { enabled: e.target.checked })
                            }
                            className="accent-[var(--v2-accent)]"
                          />
                          <input
                            type="text"
                            aria-label={`Rule name for ${rule.name || rule.id}`}
                            value={rule.name}
                            onChange={(e) => updateUnimportant(rule.id, { name: e.target.value })}
                            className="v2-input h-[22px] w-full text-[11px]"
                          />
                          <input
                            type="text"
                            aria-label={`Rule pattern for ${rule.name || rule.id}`}
                            value={rule.pattern}
                            placeholder="Regex or literal…"
                            onChange={(e) =>
                              updateUnimportant(rule.id, { pattern: e.target.value })
                            }
                            className="v2-input v2-mono h-[22px] w-full text-[11px]"
                          />
                          <div className="flex justify-center">
                            <input
                              type="checkbox"
                              aria-label={`Use regex for ${rule.name || rule.id}`}
                              checked={rule.regex}
                              onChange={(e) =>
                                updateUnimportant(rule.id, { regex: e.target.checked })
                              }
                              className="accent-[var(--v2-accent)]"
                            />
                          </div>
                          <div className="flex justify-center">
                            <input
                              type="checkbox"
                              aria-label={`Case sensitive for ${rule.name || rule.id}`}
                              checked={rule.caseSensitive}
                              onChange={(e) =>
                                updateUnimportant(rule.id, { caseSensitive: e.target.checked })
                              }
                              className="accent-[var(--v2-accent)]"
                            />
                          </div>
                        </div>
                      );
                    })
                  )}
                </div>
              </div>

              <div className="flex items-center gap-1.5">
                <button
                  type="button"
                  onClick={() => addUnimportant()}
                  className="v2-btn h-[22px] px-2 text-[11px]"
                >
                  <Plus size={11} />
                  Add
                </button>
                <button
                  type="button"
                  disabled={!selectedUnimpId}
                  onClick={removeSelectedUnimportant}
                  className="v2-btn h-[22px] px-2 text-[11px]"
                >
                  <Trash2 size={11} />
                  Remove
                </button>
              </div>
            </div>
          </div>
        ) : null}

        {/* 4. ALIGNMENT TAB */}
        {tab === "alignment" ? (
          <div className="grid grid-cols-2 gap-6 py-1">
            <div className="space-y-2.5">
              {(
                [
                  ["unaligned", "Unaligned"],
                  ["standard", "Standard alignment"],
                  ["myers", "Myers O(ND) alignment"],
                  ["patience", "Patience Diff alignment"],
                ] as const
              ).map(([mode, label]) => (
                <label key={mode} className="flex cursor-pointer items-center gap-2">
                  <input
                    type="radio"
                    name="v2-alignment-mode"
                    data-testid={`v2/rules-align-${mode}`}
                    checked={draft.alignmentMode === mode}
                    onChange={() => patch({ alignmentMode: mode })}
                    className="accent-[var(--v2-accent)]"
                  />
                  <span>{label}</span>
                </label>
              ))}
            </div>

            <div className="space-y-3">
              <label className="flex cursor-pointer items-center gap-2">
                <input
                  type="checkbox"
                  data-testid="v2/rules-never-align"
                  checked={draft.neverAlignDifferences}
                  onChange={(e) => patch({ neverAlignDifferences: e.target.checked })}
                  className="accent-[var(--v2-accent)]"
                />
                <span>Never align differences</span>
              </label>

              <div>
                <span className="mb-1 block text-[11px] text-[var(--v2-fg-secondary)]">
                  Skew tolerance:
                </span>
                <div className="flex items-center gap-2">
                  <input
                    type="number"
                    aria-label="Skew tolerance in lines"
                    min={0}
                    max={50000}
                    step={100}
                    value={draft.skewTolerance}
                    onChange={(e) =>
                      patch({ skewTolerance: Math.max(0, Number(e.target.value) || 0) })
                    }
                    className="v2-input v2-mono w-24"
                  />
                  <span className="text-[11px] text-[var(--v2-fg-muted)]">
                    lines (default 2000)
                  </span>
                </div>
              </div>

              <label className="flex cursor-pointer items-center gap-2">
                <input
                  type="checkbox"
                  checked={draft.useClosenessMatching}
                  onChange={(e) => patch({ useClosenessMatching: e.target.checked })}
                  className="accent-[var(--v2-accent)]"
                />
                <span>Use closeness matching</span>
              </label>
            </div>
          </div>
        ) : null}

        {/* 5. REPLACEMENTS TAB */}
        {tab === "replacements" ? (
          <div className="space-y-2.5">
            <div className="overflow-hidden rounded-[6px] border border-[var(--v2-border)] bg-[var(--v2-surface-subtle)]">
              <div className="grid grid-cols-[26px_1fr_1fr_54px_54px] border-b border-[var(--v2-border)] bg-[var(--v2-chrome)] px-2 py-1 text-[11px] font-semibold text-[var(--v2-fg-secondary)]">
                <span />
                <span>Left</span>
                <span>Right</span>
                <span className="text-center">Regex</span>
                <span className="text-center">Case</span>
              </div>
              <div className="max-h-52 min-h-[140px] overflow-y-auto divide-y divide-[var(--v2-border-subtle)]">
                {draft.replacements.length === 0 ? (
                  <div className="flex h-[140px] items-center justify-center text-[11.5px] text-[var(--v2-fg-muted)]">
                    No replacement rules. Click + to map Left text to Right text.
                  </div>
                ) : (
                  draft.replacements.map((rule) => {
                    const selected = selectedRuleId === rule.id;
                    return (
                      <div
                        key={rule.id}
                        onClick={() => setSelectedRuleId(rule.id)}
                        className={`grid grid-cols-[26px_1fr_1fr_54px_54px] items-center gap-1.5 px-2 py-1 ${
                          selected ? "bg-[var(--v2-accent-soft)]" : ""
                        }`}
                      >
                        <input
                          type="checkbox"
                          aria-label={`Enable replacement rule ${rule.left || rule.id}`}
                          checked={rule.enabled}
                          onChange={(e) =>
                            updateReplacement(rule.id, { enabled: e.target.checked })
                          }
                          className="accent-[var(--v2-accent)]"
                        />
                        <input
                          type="text"
                          aria-label={`Replacement match pattern ${rule.id}`}
                          value={rule.left}
                          placeholder="Left text or regex…"
                          onChange={(e) => updateReplacement(rule.id, { left: e.target.value })}
                          className="v2-input v2-mono h-[22px] w-full text-[11.5px]"
                        />
                        <input
                          type="text"
                          aria-label={`Replacement output text ${rule.id}`}
                          value={rule.right}
                          placeholder="Right replacement…"
                          onChange={(e) => updateReplacement(rule.id, { right: e.target.value })}
                          className="v2-input v2-mono h-[22px] w-full text-[11.5px]"
                        />
                        <div className="flex justify-center">
                          <input
                            type="checkbox"
                            aria-label={`Use regex for replacement ${rule.id}`}
                            checked={rule.regex}
                            onChange={(e) =>
                              updateReplacement(rule.id, { regex: e.target.checked })
                            }
                            className="accent-[var(--v2-accent)]"
                          />
                        </div>
                        <div className="flex justify-center">
                          <input
                            type="checkbox"
                            aria-label={`Case sensitive for replacement ${rule.id}`}
                            checked={rule.caseSensitive}
                            onChange={(e) =>
                              updateReplacement(rule.id, { caseSensitive: e.target.checked })
                            }
                            className="accent-[var(--v2-accent)]"
                          />
                        </div>
                      </div>
                    );
                  })
                )}
              </div>
            </div>

            <div className="flex items-center gap-1.5">
              <button
                type="button"
                data-testid="v2/rules-add-replacement"
                title="Add replacement rule"
                onClick={addReplacement}
                className="v2-btn h-[24px] px-2"
              >
                <Plus size={12} />
                Add
              </button>
              <button
                type="button"
                disabled={!selectedRuleId}
                title="Remove selected replacement rule"
                onClick={removeSelectedReplacement}
                className="v2-btn h-[24px] px-2"
              >
                <Trash2 size={12} />
                Remove
              </button>
            </div>
          </div>
        ) : null}
      </div>
    </V2Modal>
  );
}
