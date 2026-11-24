import { Search } from "lucide-react";
import { SegmentedControl, ToolbarButton } from "../../../ui";

/**
 * Wave-67B: single-row folder controls toolbar (Beyond Compare style).
 *
 * Replaces the ~5-row stacked chrome block (totals + all/diffs/same +
 * changes-only + name filter) with one toolbar row: all/diffs/same
 * filter buttons, changes-only toggle, and a name-filter toggle.
 * Purely presentational: every value and callback is owned by
 * `FolderView` (existing filter/changes-only/name logic untouched).
 * All legacy testids ride the same buttons so wave-28/35/63 specs
 * keep passing unmodified.
 */
export interface FolderToolbarProps {
  readonly rowFilter: "all" | "diffs" | "same";
  readonly onRowFilter: (f: "all" | "diffs" | "same") => void;
  readonly changesOnly: boolean;
  readonly onToggleChangesOnly: () => void;
  readonly loadedCount: number;
  readonly diffsCount: number;
  readonly sameCount: number;
  readonly nameFilterOpen: boolean;
  readonly onToggleNameFilter: () => void;
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
  } = props;
  return (
    <div
      role="toolbar"
      aria-label="Folder controls"
      data-testid="folderview/toolbar"
      className="tv-controls-bar"
    >
      <SegmentedControl<"all" | "diffs" | "same">
        label="Row filter"
        options={[
          { value: "all", label: `All (${loadedCount})`, testid: "folder-filter-all", pressed: rowFilter === "all", onSelect: () => onRowFilter("all") },
          { value: "diffs", label: `Diffs (${diffsCount})`, testid: "folder-filter-diffs", pressed: rowFilter === "diffs", onSelect: () => onRowFilter("diffs") },
          { value: "same", label: `Same (${sameCount})`, testid: "folder-filter-same", pressed: rowFilter === "same", onSelect: () => onRowFilter("same") },
        ]}
      />
      <SegmentedControl<"on" | "off">
        label="Changes only"
        options={[
          { value: changesOnly ? "on" : "off", label: `Changes only (${diffsCount})`, testid: "folder-changesonly-toggle", pressed: changesOnly, onSelect: onToggleChangesOnly },
        ]}
      />
      <span aria-hidden="true" className="tv-controls-sep" />
      <ToolbarButton
        data-testid="folderview/toolbar-namefilter"
        label={nameFilterOpen ? "Hide name filter (Esc)" : "Show name filter (Ctrl+F)"}
        title="Filter by name (Ctrl+F)"
        expanded={nameFilterOpen}
        onClick={onToggleNameFilter}
      >
        <Search size={14} strokeWidth={1.75} aria-hidden="true" />
        <span>Name filter</span>
      </ToolbarButton>
    </div>
  );
}
