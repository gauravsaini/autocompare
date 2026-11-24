/**
 * Wave-73 / Wave-B3: collapsible sync name-filter panel (Beyond Compare style).
 *
 * In-flow collapsible (never `position: absolute`, so the overlap gate
 * holds with the panel open): open on load for backward compat,
 * toggled from the toolbar, Ctrl+F, or Esc. Esc is two-stage: it clears
 * a non-empty query first (wave-64 refill contract keeps the box
 * mounted) and collapses the panel once the box is empty. Purely
 * presentational: query state and list-only match math live in
 * `SyncView` (wave-64 logic byte-identical, same testids/aria/ids).
 */
import { X } from "lucide-react";
import { Input, ToolbarButton } from "../../../ui";

export interface SyncNamePanelProps {
  readonly open: boolean;
  readonly query: string;
  readonly onQueryChange: (q: string) => void;
  readonly onClose: () => void;
}

export default function SyncNamePanel(props: SyncNamePanelProps): React.JSX.Element | null {
  const { open, query, onQueryChange, onClose } = props;
  if (!open) return null;
  return (
    <section data-testid="syncview/name-panel" className="tv-controls-panel">
      <div className="flex flex-wrap items-center gap-[var(--ac-space-2)]" role="search" aria-label="filter steps by name">
        <label
          htmlFor="sync-name-box"
          className="text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]"
        >
          Filter steps
        </label>
        <Input
          id="sync-name-box"
          data-testid="syncview/name-filter"
          value={query}
          onChange={(event) => onQueryChange(event.target.value)}
          onKeyDown={(event) => {
            // Wave-64 contract first: Esc clears the query so the box
            // stays mounted for an immediate refill; Esc on an empty box
            // collapses the panel (wave-73 chrome collapse).
            if (event.key === "Escape") {
              if (query !== "") onQueryChange("");
              else onClose();
            }
          }}
          placeholder="file or folder name"
          autoComplete="off"
          spellCheck={false}
          className="w-64 ac-mono text-[length:var(--ac-font-size-code)]"
        />
        <ToolbarButton
          data-testid="syncview/name-close"
          label="Dismiss name filter (Esc)"
          aria-label="Dismiss name filter (Esc)"
          title="Dismiss name filter (Esc)"
          icon={X}
          onClick={onClose}
        />
      </div>
    </section>
  );
}
