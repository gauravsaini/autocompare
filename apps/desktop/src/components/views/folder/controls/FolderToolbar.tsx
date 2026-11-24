import { Code, Search } from "lucide-react";
import { SegmentedControl, ToolbarButton } from "../../../ui";

/**
 * Wave-67B / Wave-B3: single-row folder controls toolbar (Beyond Compare style).
 *
 * Replaces the stacked chrome block with one toolbar row:
 * all/diffs/same/changes-only unified into ONE SegmentedControl, and a
 * name-filter toggle. Purely presentational: every value and callback is
 * owned by `FolderView` (filter/changes-only/name logic untouched).
 * All legacy testids ride the same buttons so specs keep passing unmodified.
 */
export interface FolderToolbarProps {
  // Wave-L3: widened for the L2 `view-filter` bus `context` state
  // (diffs + neighbors, TableView twin). The SegmentedControl keeps
  // its four legacy options/testids byte-identical; under `context`
  // none of the three row options reads pressed.
  readonly rowFilter: "all" | "diffs" | "same" | "context";
  readonly onRowFilter: (f: "all" | "diffs" | "same") => void;
  readonly changesOnly: boolean;
  readonly onToggleChangesOnly: () => void;
  readonly loadedCount: number;
  readonly diffsCount: number;
  readonly sameCount: number;
  readonly nameFilterOpen: boolean;
  readonly onToggleNameFilter: () => void;
  /** Wave-164: Rules (file-mask) popover toggle. Same collapse pattern
      as the name filter; masks + re-compare owned by `FolderView`. */
  readonly rulesOpen: boolean;
  readonly onToggleRules: () => void;
}

export default function FolderToolbar(props: FolderToolbarProps): React.JSX.Element {
  const {
    rowFilter,
    onRowFilter,
    changesOnly,
    onToggleChangesOnly,
    loadedCount,
    diffsCount,
    sameCount,
    nameFilterOpen,
    onToggleNameFilter,
    rulesOpen,
    onToggleRules,
  } = props;
  return (
    <div
      role="toolbar"
      aria-label="Folder controls"
      data-testid="folderview/toolbar"
      className="tv-controls-bar"
    >
      <SegmentedControl<"all" | "diffs" | "same" | "changes-only">
        label="Row filter"
        options={[
          { value: "all", label: `All (${loadedCount})`, testid: "folder-filter-all", pressed: rowFilter === "all", onSelect: () => onRowFilter("all") },
          { value: "diffs", label: `Diffs (${diffsCount})`, testid: "folder-filter-diffs", pressed: rowFilter === "diffs", onSelect: () => onRowFilter("diffs") },
          { value: "same", label: `Same (${sameCount})`, testid: "folder-filter-same", pressed: rowFilter === "same", onSelect: () => onRowFilter("same") },
          { value: "changes-only", label: `Changes only (${diffsCount})`, testid: "folder-changesonly-toggle", pressed: changesOnly, onSelect: onToggleChangesOnly },
        ]}
      />
      <span aria-hidden="true" className="tv-controls-sep" />
      <ToolbarButton
        data-testid="folderview/toolbar-rules"
        label={rulesOpen ? "Hide rules (Esc)" : "Show rules"}
        title="Rules: file masks for this compare"
        icon={Code}
        expanded={rulesOpen}
        onClick={onToggleRules}
      >
        <span>Rules</span>
      </ToolbarButton>
      <ToolbarButton
        data-testid="folderview/toolbar-namefilter"
        label={nameFilterOpen ? "Hide name filter (Esc)" : "Show name filter (Ctrl+F)"}
        title="Filter by name (Ctrl+F)"
        icon={Search}
        expanded={nameFilterOpen}
        onClick={onToggleNameFilter}
      >
        <span>Name filter</span>
      </ToolbarButton>
    </div>
  );
}
