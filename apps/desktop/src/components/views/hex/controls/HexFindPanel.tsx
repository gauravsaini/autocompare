/**
 * Wave-234: hex find migrated to ui/DockedBar.
 * In-flow collapsible hex find bar (never absolute). Purely presentational:
 * query state, match math and activation live in HexView.
 * Testids byte-identical.
 */
import { Button, DockedBar, Input, StatusText } from "../../../ui";

export interface HexFindPanelProps {
  readonly open?: boolean;
  readonly query: string;
  readonly onQueryChange: (q: string) => void;
  readonly finding: boolean;
  readonly onFindNext: () => void;
  readonly onClose: () => void;
  readonly statusText: string;
}

export default function HexFindPanel(props: HexFindPanelProps): React.JSX.Element | null {
  const { open = true, query, onQueryChange, finding, onFindNext, onClose, statusText } = props;
  return (
    <DockedBar
      open={open}
      onClose={onClose}
      label="Find hex"
      testid="hex/find-panel"
      closeTestId="hex-find-close"
    >
      <Input
        id="hex-find-box"
        data-testid="hex-find-input"
        controlSize="sm"
        value={query}
        onChange={(event) => onQueryChange(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") onFindNext();
          else if (event.key === "Escape") onClose();
        }}
        placeholder="de ad be ef"
        autoComplete="off"
        spellCheck={false}
        className="ac-code min-w-48 flex-1"
      />
      <Button
        data-testid="hex-find-next"
        size="sm"
        onClick={onFindNext}
        disabled={finding}
      >
        {finding ? "Finding…" : "Find next"}
      </Button>
      <StatusText
        data-testid="hex-find-status"
        role="status"
        className="block"
      >
        {statusText}
      </StatusText>
    </DockedBar>
  );
}
