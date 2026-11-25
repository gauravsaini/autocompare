/**
 * Wave-234: folder name filter migrated to ui/DockedBar.
 * In-flow collapsible name filter bar (never absolute). Purely presentational:
 * query state and match math live in FolderView.
 * Testids byte-identical.
 */
import { useEffect } from "react";
import { DockedBar, Input } from "../../../ui";

export interface FolderNamePanelProps {
  readonly open: boolean;
  readonly query: string;
  readonly onQueryChange: (q: string) => void;
  readonly onClose: () => void;
}

export default function FolderNamePanel(props: FolderNamePanelProps): React.JSX.Element | null {
  const { open, query, onQueryChange, onClose } = props;

  // Wave-63 / Wave-182c contract:
  // Esc with focus in the input box clears the query (63).
  // Esc from outside (e.g. table rows) collapses without clearing (98/113b).
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent): void => {
      if (e.key === "Escape") {
        const t = e.target;
        if (t instanceof HTMLElement && t.id === "folder-name-box") {
          onQueryChange("");
        }
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [open, onQueryChange]);

  return (
    <DockedBar
      open={open}
      onClose={onClose}
      label="Filter by name"
      testid="folderview/name-panel"
      closeTestId="folderview/name-close"
    >
      <Input
        id="folder-name-box"
        controlSize="sm"
        data-testid="folder-name-filter"
        value={query}
        onChange={(event) => onQueryChange(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            onQueryChange("");
            onClose();
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
