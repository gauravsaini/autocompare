import React, { useEffect, useState } from "react";
import V2Modal from "../V2Modal";

export type FolderRulesTab = "comparison" | "filters";

export interface FolderSessionRules {
  readonly contentCompare: boolean;
  readonly timestampToleranceSec: number;
  readonly masks: string;
}

export interface V2FolderRulesModalProps {
  readonly open: boolean;
  readonly initialTab: FolderRulesTab;
  readonly rules: FolderSessionRules;
  readonly onClose: () => void;
  readonly onApply: (next: FolderSessionRules) => void;
}

const MASK_PRESETS: readonly { readonly label: string; readonly token: string }[] = [
  { label: "-node_modules", token: "-node_modules/*" },
  { label: "-target", token: "-target/*" },
  { label: "-.git", token: "-.git/*" },
  { label: "Source (*.rs;*.ts;*.tsx)", token: "*.rs;*.ts;*.tsx" },
];

export default function V2FolderRulesModal({
  open,
  initialTab,
  rules,
  onClose,
  onApply,
}: V2FolderRulesModalProps): React.JSX.Element {
  const [tab, setTab] = useState<FolderRulesTab>(initialTab);
  const [draft, setDraft] = useState<FolderSessionRules>(rules);

  useEffect(() => {
    if (open) {
      setTab(initialTab);
      setDraft(rules);
    }
  }, [open, initialTab, rules]);

  const patch = (partial: Partial<FolderSessionRules>): void => {
    setDraft((prev) => ({ ...prev, ...partial }));
  };

  const appendMaskToken = (token: string): void => {
    const existing = draft.masks
      .split(";")
      .map((s) => s.trim())
      .filter(Boolean);
    if (!existing.includes(token)) {
      patch({ masks: [...existing, token].join("; ") });
    }
  };

  return (
    <V2Modal
      open={open}
      title="Folder Compare Session Settings"
      subtitle="Configure comparison criteria and semicolon-separated file name filters"
      widthClass="max-w-lg"
      onClose={onClose}
      footer={
        <>
          <button type="button" onClick={onClose} className="v2-btn">
            Cancel
          </button>
          <button
            type="button"
            data-testid="v2/folder-rules-ok"
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
                ["comparison", "Comparison"],
                ["filters", "Name Filters"],
              ] as const
            ).map(([id, label]) => (
              <button
                key={id}
                type="button"
                role="tab"
                aria-selected={tab === id}
                data-testid={`v2/folder-rules-tab-${id}`}
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

        {/* 1. COMPARISON TAB */}
        {tab === "comparison" ? (
          <div className="space-y-3 py-1">
            <label className="flex cursor-pointer items-start gap-2">
              <input
                type="radio"
                name="v2-folder-compare-mode"
                checked={!draft.contentCompare}
                onChange={() => patch({ contentCompare: false })}
                className="mt-0.5 accent-[var(--v2-accent)]"
              />
              <div>
                <div className="font-medium">Quick comparison (Size &amp; Timestamp)</div>
                <div className="text-[11px] text-[var(--v2-fg-muted)]">
                  Fastest scan — compares file byte sizes and modification timestamps.
                </div>
              </div>
            </label>

            <label className="flex cursor-pointer items-start gap-2">
              <input
                type="radio"
                name="v2-folder-compare-mode"
                checked={draft.contentCompare}
                onChange={() => patch({ contentCompare: true })}
                className="mt-0.5 accent-[var(--v2-accent)]"
              />
              <div>
                <div className="font-medium">Binary content comparison</div>
                <div className="text-[11px] text-[var(--v2-fg-muted)]">
                  Reads and compares same-size files byte-for-byte to catch silent edits.
                </div>
              </div>
            </label>

            <div className="pt-1">
              <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-secondary)]">
                Timestamp tolerance:
              </span>
              <div className="flex items-center gap-2">
                <input
                  type="number"
                  aria-label="Timestamp tolerance in seconds"
                  min={0}
                  max={3600}
                  value={draft.timestampToleranceSec}
                  onChange={(e) =>
                    patch({ timestampToleranceSec: Math.max(0, Number(e.target.value) || 0) })
                  }
                  className="v2-input v2-mono w-24"
                />
                <span className="text-[11px] text-[var(--v2-fg-muted)]">
                  seconds (default 2s for FAT/network drives)
                </span>
              </div>
            </div>
          </div>
        ) : null}

        {/* 2. NAME FILTERS TAB */}
        {tab === "filters" ? (
          <div className="space-y-3 py-1">
            <div>
              <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-secondary)]">
                Include / Exclude File Masks (semicolon-separated, prefix `-` to exclude):
              </span>
              <input
                type="text"
                aria-label="Include or exclude file masks"
                data-testid="v2/folder-masks-input"
                value={draft.masks}
                onChange={(e) => patch({ masks: e.target.value })}
                placeholder="e.g. *.rs; *.ts; -target/*; -node_modules/*"
                className="v2-input v2-mono w-full"
              />
            </div>

            <div className="flex flex-wrap items-center gap-1.5">
              {MASK_PRESETS.map((p) => (
                <button
                  key={p.label}
                  type="button"
                  onClick={() => appendMaskToken(p.token)}
                  className="v2-btn h-[22px] px-2 text-[11px]"
                >
                  + {p.label}
                </button>
              ))}
              {draft.masks.trim() ? (
                <button
                  type="button"
                  onClick={() => patch({ masks: "" })}
                  className="v2-btn h-[22px] px-2 text-[11px] text-[var(--v2-del-fg)]"
                >
                  Clear
                </button>
              ) : null}
            </div>
          </div>
        ) : null}
      </div>
    </V2Modal>
  );
}
