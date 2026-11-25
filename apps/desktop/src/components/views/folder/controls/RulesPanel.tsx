/**
 * Wave-233: Rules file-mask panel migrated onto ui/Modal primitive (centered card on scrim,
 * role=dialog aria-modal, autofocus, Esc+backdrop+close single onClose).
 * All testids byte-identical (folderview/rules-panel, folderview/mask-input,
 * folderview/mask-apply, folderview/mask-clear, folderview/rules-close).
 * Wave-C2d adds the engine content-compare toggle
 * (testid folderview/content-compare-toggle) without touching legacy testids.
 */
import { useEffect, useState } from "react";
import { Button, Input, Modal } from "../../../ui";

/** Window event carrying the top-bar Rules request (BC parity). */
export const VIEW_RULES_EVENT = "autocompare:view-rules";

/** Dispatch a Rules-panel open request (top Toolbar -> FolderView). */
export function dispatchViewRules(): void {
  window.dispatchEvent(new CustomEvent(VIEW_RULES_EVENT));
}

export interface RulesPanelProps {
  readonly open: boolean;
  readonly masks: string;
  readonly busy: boolean;
  readonly onApply: (masks: string) => void;
  readonly onClose: () => void;
  /** Wave-C2d: engine content-compare flag (byte-compare same-size files). */
  readonly contentCompare: boolean;
  readonly onToggleContentCompare: (next: boolean) => void;
}

export default function RulesPanel(props: RulesPanelProps): React.JSX.Element | null {
  const { open, masks, busy, onApply, onClose, contentCompare, onToggleContentCompare } = props;
  const [draft, setDraft] = useState(masks);

  useEffect(() => {
    if (open) setDraft(masks);
  }, [open, masks]);

  const apply = (): void => {
    onApply(draft.trim());
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      label="File masks"
      testid="folderview/rules-panel"
      closeTestid="folderview/rules-close"
      closeAriaLabel="Dismiss rules (Esc)"
    >
      <div className="flex flex-col gap-3 pt-1">
        <label htmlFor="folder-mask-box" className="ac-meta font-normal text-[var(--ac-fg-muted)]">
          Semicolon list, * / ? wildcards, - prefix excludes
        </label>
        <Input
          id="folder-mask-box"
          controlSize="sm"
          data-testid="folderview/mask-input"
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") apply();
            else if (event.key === "Escape") onClose();
          }}
          placeholder="*.ts;-*.min.js"
          title="Semicolon list, * / ? wildcards, - prefix excludes"
          autoComplete="off"
          spellCheck={false}
          disabled={busy}
          className="w-full ac-code"
          autoFocus
        />
        <label className="flex items-center gap-2 ac-meta font-normal text-[var(--ac-fg-muted)]">
          <input
            type="checkbox"
            data-testid="folderview/content-compare-toggle"
            checked={contentCompare}
            disabled={busy}
            onChange={(event) => onToggleContentCompare(event.target.checked)}
            aria-label="Compare file contents (byte compare)"
            title="Compare file contents byte-by-byte (off = size/mtime fast path)"
          />
          <span>Compare contents (byte-by-byte)</span>
        </label>
        <div className="flex items-center justify-end gap-2">
          {masks !== "" ? (
            <Button
              size="sm"
              variant="secondary"
              data-testid="folderview/mask-clear"
              title="Clear file masks (compare everything)"
              disabled={busy}
              onClick={() => onApply("")}
            >
              Clear
            </Button>
          ) : null}
          <Button
            size="sm"
            variant="secondary"
            data-testid="folderview/mask-apply"
            title="Compare with these file masks"
            disabled={busy}
            onClick={apply}
          >
            Apply masks
          </Button>
        </div>
      </div>
    </Modal>
  );
}
