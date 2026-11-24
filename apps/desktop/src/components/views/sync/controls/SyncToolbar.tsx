/**
 * Wave-73 / Wave-B3: single-row sync controls toolbar (Beyond Compare Folder Sync style).
 *
 * Collapses the stacked sync chrome (direction group + prune row + sync
 * actions group) into one toolbar row: the three direction buttons
 * (Update right/left, Newer wins), the prune-orphans checkbox, a
 * name-filter toggle, and the dry-run / real apply buttons. Purely
 * presentational: direction/prune/apply/filter state and every IPC call
 * stay owned by `SyncView` (waves 47/62/64/66N logic untouched). All
 * legacy testids ride the same controls so specs keep passing unmodified.
 */
import { Search } from "lucide-react";
import { Button, Checkbox, SegmentedControl, ToolbarButton } from "../../../ui";
import type { SyncDirection } from "../../../../ipc/sync";

export interface SyncToolbarProps {
  readonly direction: SyncDirection;
  readonly onDirection: (d: SyncDirection) => void;
  readonly prune: boolean;
  readonly onPruneChange: (v: boolean) => void;
  readonly filterOpen: boolean;
  readonly onToggleFilter: () => void;
  readonly applying: boolean;
  readonly canApply: boolean;
  readonly disabledReason?: string | null;
  readonly onDry: () => void;
  readonly onReal: () => void;
}

export default function SyncToolbar(props: SyncToolbarProps): React.JSX.Element {
  const {
    direction,
    onDirection,
    prune,
    onPruneChange,
    filterOpen,
    onToggleFilter,
    applying,
    canApply,
    disabledReason,
    onDry,
    onReal,
  } = props;
  return (
    <div role="toolbar" aria-label="Sync controls" data-testid="syncview/toolbar" className="tv-controls-bar">
      <SegmentedControl<SyncDirection>
        label="Sync direction"
        options={[
          { value: "LeftToRight", label: "Update right (left wins)", testid: "syncview/dir-left", pressed: direction === "LeftToRight", onSelect: () => onDirection("LeftToRight") },
          { value: "RightToLeft", label: "Update left (right wins)", testid: "syncview/dir-right", pressed: direction === "RightToLeft", onSelect: () => onDirection("RightToLeft") },
          { value: "NewerWins", label: "Newer wins", testid: "syncview/dir-newer", pressed: direction === "NewerWins", onSelect: () => onDirection("NewerWins") },
        ]}
      />
      <span className="sy-prune-label">
        <Checkbox id="sync-prune-box" data-testid="syncview/prune-toggle" label="Prune orphans (delete extra files on target)" checked={prune} onChange={(event) => onPruneChange(event.target.checked)} />
      </span>
      <span aria-hidden="true" className="tv-controls-sep" />
      <ToolbarButton
        data-testid="syncview/toolbar-filter"
        label={filterOpen ? "Hide name filter (Esc)" : "Show name filter (Ctrl+F)"}
        title="Filter steps by name (Ctrl+F)"
        icon={Search}
        expanded={filterOpen}
        onClick={onToggleFilter}
      >
        <span>Name filter</span>
      </ToolbarButton>
      <span aria-hidden="true" className="tv-controls-sep" />
      <Button
        data-testid="syncview/apply-dry"
        size="sm"
        onClick={onDry}
        disabled={applying || !canApply}
        data-disabled-reason={!canApply ? (disabledReason ?? "no steps to apply") : applying ? "sync in progress" : undefined}
        title={!canApply ? (disabledReason ?? "no steps to apply") : applying ? "sync in progress" : "Preview apply (dry run)"}
      >
        Preview apply (dry run)
      </Button>
      <Button
        data-testid="syncview/apply-real"
        variant="primary"
        size="sm"
        onClick={onReal}
        disabled={applying || !canApply}
        data-disabled-reason={!canApply ? (disabledReason ?? "no steps to apply") : applying ? "sync in progress" : undefined}
        title={!canApply ? (disabledReason ?? "no steps to apply") : applying ? "sync in progress" : "Apply for real"}
      >
        Apply for real
      </Button>
    </div>
  );
}
