/**
 * Wave-67B / Wave-B3: collapsible name-filter panel (Beyond Compare style).
 *
 * In-flow collapsible (never `position: absolute`, so the overlap gate
 * holds with the panel open): open by default for backward compat,
 * toggled from the toolbar, Ctrl+F, or Esc. Purely presentational:
 * query state and match math live in `FolderView` (wave-63 logic
 * byte-identical, same testids/aria).
 */
import { X } from "lucide-react";
import { Input, ToolbarButton } from "../../../ui";

export interface FolderNamePanelProps {
  readonly open: boolean;
  readonly query: string;
  readonly onQueryChange: (q: string) => void;
  readonly onClose: () => void;
}

export default function FolderNamePanel(props: FolderNamePanelProps): React.JSX.Element | null {
  const { open, query, onQueryChange, onClose } = props;
  if (!open) return null;
  return (
    <section data-testid="folderview/name-panel" className="tv-controls-panel">
      <div className="ac-control-group" role="search" aria-label="filter by name">
        <label htmlFor="folder-name-box" className="ac-meta font-normal text-[var(--ac-fg-muted)]">
          Filter by name
        </label>
        <Input
          id="folder-name-box"
          data-testid="folder-name-filter"
          value={query}
          onChange={(event) => onQueryChange(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Escape") onClose();
          }}
          placeholder="file or folder name"
          autoComplete="off"
          spellCheck={false}
          className="w-64 ac-code"
        />
        <ToolbarButton
          data-testid="folderview/name-close"
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
