import FolderView from "./FolderView";
import HexView from "./HexView";
import MergeView from "./MergeView";
import SyncView from "./SyncView";
import MediaView from "./MediaView";
import TableView from "./TableView";
import TextView from "./TextView";
import type { TextRowDto } from "../../ipc/text";
import type { ViewPaths } from "./types";

/**
 * Kind-switched data view router (Wave 2B, Wave 5B demo contract).
 *
 * Props mirror the tab descriptor (`leftPath`/`rightPath`, optional
 * `sessionId` forwarded to `TextView` for backend-session reuse with a
 * real-compare fallback). Each branch is wrapped in a `view-{kind}`
 * test id; the router root carries `view-router`. Text mounts with empty
 * paths render the bundled demo pair through the real text IPC path, so
 * first paint already shows real diff rows.
 */

export type DataViewKind = "text" | "folder" | "hex" | "table" | "merge" | "sync" | "media";

export interface ViewRouterProps extends ViewPaths {
  readonly kind: DataViewKind;
  /** Common-ancestor path for `merge`; defaults to `leftPath`. */
  readonly basePath?: string;
  /**
   * Wave-21: controlled text-row selection, forwarded to `TextView` ONLY.
   * `App` owns it so the bottom `LineDetailPane` reuses the selected row.
   */
  readonly selectedIndex?: number | null;
  /** Wave-21: forwarded to `TextView` ONLY (row click / keyboard select). */
  readonly onSelectRow?: (index: number | null, row: TextRowDto | null) => void;
  /**
   * Wave-46: forwarded to `FolderView` ONLY (file-row double-click
   * drilldown). `App` opens the pair as a new session; the view never
   * calls IPC itself.
   */
  readonly onOpenPaths?: (kind: DataViewKind, left: string, right: string) => void;
  /**
   * FIXM GAP-08: forwarded to `MergeView` ONLY (3-way pane-drop
   * staging). `App` routes the triple through `handleOpenSession` /
   * `tabPaths` + merge-plan compare; until wired the view stages the
   * drop honestly inline.
   */
  readonly onDropPaths?: (next: {
    readonly leftPath: string;
    readonly basePath: string;
    readonly rightPath: string;
  }) => void;
  /**
   * FIXM GAP-08: forwarded to `MergeView` ONLY (auto-compare routing
   * for the staged triple). Called with the same triple right after
   * `onDropPaths` so the host can run the merge plan immediately.
   */
  readonly onRequestCompare?: (next: {
    readonly leftPath: string;
    readonly basePath: string;
    readonly rightPath: string;
  }) => void;
  /**
   * FIXM GAP-13: forwarded to `MergeView` ONLY (ViewCommandBus hook
   * for prev/next/copy, `TextView` `onBusCommand` parity). When
   * provided, top-toolbar bus commands route here instead of the
   * internal nav/take handlers; omitted (the `App` default) leaves the
   * internal `autocompare:view-command` listener owning the bus.
   */
  readonly onBusCommand?: (cmd: "prev" | "next" | "copy") => void;
  /**
   * Wave-161: forwarded to `TextView` ONLY (`onBusCommand` parity for
   * prev/next). When provided, the TextToolbar prev/next buttons ride
   * the SAME App `handleViewCommand` bus as the top Toolbar — no
   * duplicate handlers. TextView's internal window listener stays a
   * pure follow-effect (scroll/focus sync). Merge is untouched: it only
   * ever sees `onBusCommand`.
   */
  readonly onTextBusCommand?: (cmd: "prev" | "next") => void;
}

export default function ViewRouter({
  kind,
  leftPath,
  rightPath,
  basePath,
  sessionId,
  selectedIndex,
  onSelectRow,
  onOpenPaths,
  onDropPaths,
  onRequestCompare,
  onBusCommand,
  onTextBusCommand,
}: ViewRouterProps): React.JSX.Element {
  return (
    <div data-testid="view-router">
      {kind === "text" && (
        <div data-testid="view-text">
          <TextView leftPath={leftPath} rightPath={rightPath} sessionId={sessionId} selectedIndex={selectedIndex} onSelectRow={onSelectRow} onBusCommand={onTextBusCommand} />
        </div>
      )}
      {kind === "folder" && (
        <div data-testid="view-folder">
          <FolderView leftPath={leftPath} rightPath={rightPath} onOpenPaths={onOpenPaths} />
        </div>
      )}
      {kind === "hex" && (
        <div data-testid="view-hex">
          <HexView leftPath={leftPath} rightPath={rightPath} />
        </div>
      )}
      {kind === "table" && (
        <div data-testid="view-table">
          <TableView leftPath={leftPath} rightPath={rightPath} />
        </div>
      )}
      {kind === "merge" && (
        <div data-testid="view-merge">
          <MergeView leftPath={leftPath} rightPath={rightPath} basePath={basePath} onDropPaths={onDropPaths} onRequestCompare={onRequestCompare} onBusCommand={onBusCommand} />
        </div>
      )}
      {kind === "sync" && (
        <div data-testid="view-sync">
          <SyncView leftPath={leftPath} rightPath={rightPath} />
        </div>
      )}
      {kind === "media" && (
        <div data-testid="view-media">
          <MediaView leftPath={leftPath} rightPath={rightPath} />
        </div>
      )}
    </div>
  );
}
