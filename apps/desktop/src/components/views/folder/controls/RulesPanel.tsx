/**
 * Wave-164: Rules file-mask panel (Beyond Compare style Rules button).
 * Collapsible in-flow panel (never `position: absolute`, so the chrome
 * budget holds with the panel open): closed by default, toggled from
 * the toolbar Rules button or the top-bar Rules action (bus). Purely
 * presentational: committed mask state and the re-compare live in
 * `FolderView`; the typed `folderCompare` wrapper already sends
 * `masks`, honored at compare time by the W165 engine (mock too).
 *
 * Grammar (engine `parse_file_masks`): semicolon list, `*`/`?`
 * wildcards matched against the file name and `/`-separated relPath,
 * `-` prefix excludes (e.g. `*.ts;-*.min.js`). Empty clears filtering.
 */
import { useEffect, useState } from "react";
import { X } from "lucide-react";
import { Button, Input, ToolbarButton } from "../../../ui";

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
}

export default function RulesPanel(props: RulesPanelProps): React.JSX.Element | null {
  const { open, masks, busy, onApply, onClose } = props;
  const [draft, setDraft] = useState(masks);
  // Re-seed the draft whenever the panel opens or the committed value
  // changes underneath (apply from the top bar, session switch).
  useEffect(() => {
    if (open) setDraft(masks);
  }, [open, masks]);
  if (!open) return null;
  const apply = (): void => {
    onApply(draft.trim());
  };
  return (
    <section data-testid="folderview/rules-panel" className="tv-controls-panel">
      <div className="ac-control-group" role="search" aria-label="rules file masks">
        <label htmlFor="folder-mask-box" className="ac-meta font-normal text-[var(--ac-fg-muted)]">
          File masks
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
          className="w-64 ac-code"
        />
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
        <ToolbarButton
          size="sm"
          data-testid="folderview/rules-close"
          label="Dismiss rules (Esc)"
          aria-label="Dismiss rules (Esc)"
          title="Dismiss rules (Esc)"
          icon={X}
          onClick={onClose}
        />
      </div>
    </section>
  );
}
