import { Code, Columns3, Play, RefreshCw, Search } from "lucide-react";
import { ToolbarButton, UnifiedFilterSegment } from "../../../ui";

/**
 * Wave-67B / Wave-B3: single-row folder controls toolbar (Beyond Compare style).
 *
 * Replaces the stacked chrome block with one toolbar row:
 * all/diffs/same/changes-only unified into ONE SegmentedControl, and a
 * name-filter toggle. Purely presentational: every value and callback is
 * owned by `FolderView` (filter/changes-only/name logic untouched).
 * All legacy testids ride the same buttons so specs keep passing unmodified.
 * Wave-216 R4: Minor/unimportant toggle is top-bar-owned (view-filter bus); no view-level toggle by design.
 * Wave-WA (folder toolbar unify, BC parity): filter segment rides the
 * shared ui/SegmentedControl path via UnifiedFilterSegment (size sm =
 * 22px token, testids byte-identical); the name-filter text box itself
 * lives in FolderNamePanel on shared ui/Input (controlSize sm = 22px
 * token). This toolbar keeps zero raw buttons/inputs — toggle only.
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
  /** Optional Columns panel toggle (Beyond Compare parity). */
  readonly columnsOpen?: boolean;
  readonly onToggleColumns?: () => void;
  /** Optional Reload / Refresh compare action. */
  readonly onReload?: () => void;
  readonly onRefresh?: () => void;
  readonly reloading?: boolean;
  /** Optional Compare action. */
  readonly onCompare?: () => void;
  readonly comparing?: boolean;
  /**
   * Wave-228: ViewCommandBus hook for diff nav parity with TableToolbar/HexToolbar.
   * When provided, diff navigation commands dispatch through the SAME App
   * `handleViewCommand` bus as the top Toolbar.
   */
  readonly onBusCommand?: (cmd: "prev" | "next") => void;
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
    columnsOpen,
    onToggleColumns,
    onReload,
    onRefresh,
    reloading,
    onCompare,
    comparing,
    onBusCommand,
  } = props;
  void onBusCommand;
  const handleReload = onReload ?? onRefresh;
  return (
    <div
      role="toolbar"
      aria-label="Folder controls"
      data-testid="folderview/toolbar"
      className="tv-controls-bar"
    >
      {/* Wave-239: All/Diffs/Same + changes-only rides the shared UnifiedFilterSegment (same testids). */}
      <UnifiedFilterSegment<"all" | "diffs" | "same" | "changes-only">
        size="sm"
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
        size="sm"
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
        size="sm"
        data-testid="folderview/toolbar-namefilter"
        label={nameFilterOpen ? "Hide name filter (Esc)" : "Show name filter (Ctrl+F)"}
        title="Filter by name (Ctrl+F)"
        icon={Search}
        expanded={nameFilterOpen}
        onClick={onToggleNameFilter}
      >
        <span>Name filter</span>
      </ToolbarButton>
      {onToggleColumns ? (
        <ToolbarButton
          size="sm"
          data-testid="folderview/toolbar-columns"
          label={columnsOpen ? "Hide columns panel" : "Show columns panel"}
          title="Columns (Beyond Compare parity)"
          icon={Columns3}
          expanded={columnsOpen}
          onClick={onToggleColumns}
        >
          <span>Columns</span>
        </ToolbarButton>
      ) : null}
      {handleReload ? (
        <ToolbarButton
          size="sm"
          data-testid="folderview/toolbar-reload"
          label={reloading ? "Reloading…" : "Reload compare"}
          title="Reload compare"
          icon={RefreshCw}
          disabled={reloading}
          onClick={handleReload}
        >
          <span>Reload</span>
        </ToolbarButton>
      ) : null}
      {onCompare ? (
        <ToolbarButton
          size="sm"
          data-testid="folderview/toolbar-compare"
          label={comparing ? "Comparing…" : "Compare"}
          title="Compare folders"
          icon={Play}
          disabled={comparing}
          onClick={onCompare}
        >
          <span>Compare</span>
        </ToolbarButton>
      ) : null}
    </div>
  );
}
