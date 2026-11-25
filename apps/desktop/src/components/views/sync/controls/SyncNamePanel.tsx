/**
 * Wave-234: sync name filter migrated to ui/DockedBar.
 * In-flow collapsible sync name filter bar (never absolute). Purely presentational:
 * query state and step match math live in SyncView.
 * Testids byte-identical, two-stage Esc preserved.
 */
import { useEffect } from "react";
import { DockedBar, Input } from "../../../ui";

export interface SyncNamePanelProps {
  readonly open: boolean;
  readonly query: string;
  readonly onQueryChange: (q: string) => void;
  readonly onClose: () => void;
}

export default function SyncNamePanel(props: SyncNamePanelProps): React.JSX.Element | null {
  const { open, query, onQueryChange, onClose } = props;

  // Wave-64 / Wave-73 two-stage Esc contract:
  // When query is non-empty, Esc clears query first without closing the panel.
  // Esc on empty query falls through to DockedBar document keydown listener to close.
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent): void => {
      if (e.key === "Escape" && query !== "") {
        e.stopImmediatePropagation();
        e.stopPropagation();
        onQueryChange("");
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [open, query, onQueryChange]);

  return (
    <DockedBar
      open={open}
      onClose={onClose}
      label="Filter steps"
      testid="syncview/name-panel"
      closeTestId="syncview/name-close"
    >
      <Input
        id="sync-name-box"
        controlSize="sm"
        data-testid="syncview/name-filter"
        value={query}
        onChange={(event) => onQueryChange(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            if (query !== "") {
              event.stopPropagation();
              onQueryChange("");
            } else {
              onClose();
            }
          }
        }}
        placeholder="file or folder name"
        autoComplete="off"
        spellCheck={false}
        className="w-64 ac-code"
      />
    </DockedBar>
  );
}
