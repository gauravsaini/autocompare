import React, { useEffect, useState } from "react";
import { KeyRound } from "lucide-react";
import V2Modal from "../V2Modal";

export type TableRulesTab = "columns" | "format" | "importance";

export interface TableSessionRules {
  readonly byKey: boolean;
  readonly keyColumns: readonly number[];
  readonly delimiter: string;
  readonly ignoreCase: boolean;
  readonly ignoreSpace: boolean;
  readonly numericTolerance: string;
}

export interface V2TableRulesModalProps {
  readonly open: boolean;
  readonly initialTab: TableRulesTab;
  readonly rules: TableSessionRules;
  readonly columnNames: readonly string[];
  readonly onClose: () => void;
  readonly onApply: (next: TableSessionRules) => void;
}

const DELIMITER_PRESETS: readonly { readonly label: string; readonly value: string }[] = [
  { label: "Comma (,)", value: "," },
  { label: "Tab (\\t)", value: "\t" },
  { label: "Semicolon (;)", value: ";" },
  { label: "Pipe (|)", value: "|" },
];

export default function V2TableRulesModal({
  open,
  initialTab,
  rules,
  columnNames,
  onClose,
  onApply,
}: V2TableRulesModalProps): React.JSX.Element {
  const [tab, setTab] = useState<TableRulesTab>(initialTab);
  const [draft, setDraft] = useState<TableSessionRules>(rules);

  useEffect(() => {
    if (open) {
      setTab(initialTab);
      setDraft(rules);
    }
  }, [open, initialTab, rules]);

  const patch = (partial: Partial<TableSessionRules>): void => {
    setDraft((prev) => ({ ...prev, ...partial }));
  };

  const toggleKeyCol = (idx: number): void => {
    const exists = draft.keyColumns.includes(idx);
    const next = exists
      ? draft.keyColumns.length > 1
        ? draft.keyColumns.filter((c) => c !== idx)
        : draft.keyColumns
      : [...draft.keyColumns, idx].sort((a, b) => a - b);
    patch({ byKey: true, keyColumns: next });
  };

  return (
    <V2Modal
      open={open}
      title="Table Compare Session Settings"
      subtitle="Configure key columns, delimiter format, and cell importance"
      widthClass="max-w-lg"
      onClose={onClose}
      footer={
        <>
          <button type="button" onClick={onClose} className="v2-btn">
            Cancel
          </button>
          <button
            type="button"
            data-testid="v2/table-rules-ok"
            onClick={() => onApply(draft)}
            className="v2-btn v2-btn-primary"
          >
            OK
          </button>
        </>
      }
    >
      <div className="space-y-4 text-[12px]">
        <div className="flex justify-center">
          <div role="tablist" className="inline-flex rounded-[7px] bg-[var(--v2-seg-track)] p-0.5">
            {(
              [
                ["columns", "Columns"],
                ["format", "Format"],
                ["importance", "Importance"],
              ] as const
            ).map(([id, label]) => (
              <button
                key={id}
                type="button"
                role="tab"
                aria-selected={tab === id}
                data-testid={`v2/table-rules-tab-${id}`}
                onClick={() => setTab(id)}
                className={`h-[24px] rounded-[6px] px-3 text-[12px] font-medium transition ${
                  tab === id
                    ? "bg-[var(--v2-surface)] text-[var(--v2-fg)] shadow-xs"
                    : "text-[var(--v2-fg-secondary)] hover:text-[var(--v2-fg)]"
                }`}
              >
                {label}
              </button>
            ))}
          </div>
        </div>

        {/* 1. COLUMNS TAB */}
        {tab === "columns" ? (
          <div className="space-y-3">
            <label className="flex cursor-pointer items-center gap-2">
              <input
                type="checkbox"
                checked={draft.byKey}
                onChange={(e) => patch({ byKey: e.target.checked })}
                className="accent-[var(--v2-accent)]"
              />
              <span>Align rows by key column(s) instead of row position</span>
            </label>

            <div className="overflow-hidden rounded-[6px] border border-[var(--v2-border)] bg-[var(--v2-surface-subtle)]">
              <div className="border-b border-[var(--v2-border)] bg-[var(--v2-chrome)] px-2.5 py-1 text-[11px] font-semibold text-[var(--v2-fg-secondary)]">
                Key Columns ({draft.keyColumns.length} selected)
              </div>
              <div className="max-h-44 overflow-y-auto divide-y divide-[var(--v2-border-subtle)]">
                {columnNames.length === 0 ? (
                  <div className="px-3 py-6 text-center text-[11.5px] text-[var(--v2-fg-muted)]">
                    Load a table to pick specific key columns.
                  </div>
                ) : (
                  columnNames.map((name, idx) => {
                    const isKey = draft.keyColumns.includes(idx);
                    return (
                      <label
                        key={`${idx}-${name}`}
                        className="flex cursor-pointer items-center justify-between px-2.5 py-1.5 hover:bg-[var(--v2-btn-hover)]"
                      >
                        <div className="flex items-center gap-2">
                          <input
                            type="checkbox"
                            checked={isKey}
                            onChange={() => toggleKeyCol(idx)}
                            className="accent-[var(--v2-accent)]"
                          />
                          <span className="v2-mono text-[11px] text-[var(--v2-fg-muted)]">
                            #{idx + 1}
                          </span>
                          <span>{name}</span>
                        </div>
                        {isKey ? <KeyRound size={12} className="text-[var(--v2-accent)]" /> : null}
                      </label>
                    );
                  })
                )}
              </div>
            </div>
          </div>
        ) : null}

        {/* 2. FORMAT TAB */}
        {tab === "format" ? (
          <div className="space-y-3">
            <div>
              <span className="mb-1.5 block text-[11px] font-medium text-[var(--v2-fg-secondary)]">
                Column delimiter:
              </span>
              <div className="flex flex-wrap gap-1.5">
                {DELIMITER_PRESETS.map((p) => (
                  <button
                    key={p.label}
                    type="button"
                    onClick={() => patch({ delimiter: p.value })}
                    className={`v2-btn h-[24px] px-2.5 text-[11.5px] ${
                      draft.delimiter === p.value
                        ? "border-[var(--v2-accent)] bg-[var(--v2-accent-soft)] text-[var(--v2-accent)]"
                        : ""
                    }`}
                  >
                    {p.label}
                  </button>
                ))}
              </div>
            </div>

            <div>
              <span className="mb-1 block text-[11px] text-[var(--v2-fg-secondary)]">
                Custom delimiter character:
              </span>
              <input
                aria-label="Custom delimiter character"
                value={draft.delimiter === "\t" ? "\\t" : draft.delimiter}
                onChange={(e) => {
                  const raw = e.target.value;
                  patch({ delimiter: raw === "\\t" ? "\t" : raw.slice(0, 1) });
                }}
                maxLength={2}
                className="v2-input v2-mono w-24"
              />
            </div>
          </div>
        ) : null}

        {/* 3. IMPORTANCE TAB */}
        {tab === "importance" ? (
          <div className="space-y-3 py-1">
            <label className="flex cursor-pointer items-center gap-2">
              <input
                type="checkbox"
                checked={draft.ignoreCase}
                onChange={(e) => patch({ ignoreCase: e.target.checked })}
                className="accent-[var(--v2-accent)]"
              />
              <span>Ignore character case in cell values</span>
            </label>
            <label className="flex cursor-pointer items-center gap-2">
              <input
                type="checkbox"
                checked={draft.ignoreSpace}
                onChange={(e) => patch({ ignoreSpace: e.target.checked })}
                className="accent-[var(--v2-accent)]"
              />
              <span>Trim leading/trailing & collapse extra whitespace</span>
            </label>

            <div className="pt-1">
              <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-secondary)]">
                Numeric tolerance (± epsilon):
              </span>
              <div className="flex items-center gap-2">
                <input
                  value={draft.numericTolerance}
                  onChange={(e) => patch({ numericTolerance: e.target.value })}
                  placeholder="Exact (blank) or e.g. 0.01"
                  className="v2-input v2-mono w-44"
                />
                {(["0.01", "0.001", "0.0001"] as const).map((tol) => (
                  <button
                    key={tol}
                    type="button"
                    onClick={() => patch({ numericTolerance: tol })}
                    className="v2-btn h-[22px] px-2 text-[10.5px]"
                  >
                    ±{tol}
                  </button>
                ))}
              </div>
            </div>
          </div>
        ) : null}
      </div>
    </V2Modal>
  );
}
