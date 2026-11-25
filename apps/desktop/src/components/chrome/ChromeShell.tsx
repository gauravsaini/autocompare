import React from "react";
import { TabBar, type ChromeTab } from "./TabBar";
import { Toolbar, type ToolbarAction } from "./Toolbar";
import { PathBar } from "./PathBar";
import { StatusBar } from "./StatusBar";
import { OverviewRuler, type DiffRow } from "./OverviewRuler";
import { useWebShortcuts } from "./shortcuts";

export interface ChromeShellStatus {
  left: string;
  right: string;
}

export interface ChromeShellProps {
  tabs: readonly ChromeTab[];
  activeTabId: string;
  onSelectTab: (id: string) => void;
  onCloseTab: (id: string) => void;
  onNewTab: (kind: string) => void;
  toolbar: { actions: readonly ToolbarAction[] };
  leftPath: string;
  rightPath: string;
  /**
   * Wave-123: PathBar submit passthrough (prop only). App wires this to
   * `runComparePaths`; submit behavior stays in PathBar (wave-124).
   * Wave-134: side widens to `"base"` for the merge ancestor field.
   */
  onSubmitPath?: (side: "left" | "right" | "base", path: string) => void;
  /**
   * Wave-124: PathBar compare submit handler. Forwarded to PathBar.
   * Wave-134: side widens to `"base"` for the merge ancestor field.
   */
  onSubmit?: (side: "left" | "right" | "base", path: string) => void;
  /**
   * Wave-124: Pending / comparing state forwarded to PathBar.
   */
  isComparing?: boolean;
  pending?: boolean;
  disabled?: boolean;
  /**
   * Wave-124: Optional note forwarded to PathBar.
   */
  note?: string | null;
  /**
   * Wave-134: merge ancestor passthrough. `basePath` feeds the 3rd
   * PathBar field; rendered only when `showBase` (merge kind) is true.
   */
  basePath?: string;
  showBase?: boolean;
  /**
   * Wave-140: dual-drop passthrough (BC Item0 -> left, Item1 -> right).
   * App wires this to `handlePaneDropPaths` so a 2-file PathBar/strip
   * drop stages both sides + single auto-compare (no double compare).
   */
  onDropPaths?: (paths: readonly string[]) => void;
  status: ChromeShellStatus;
  statusMessage: string;
  diffRows: readonly DiffRow[];
  /**
   * Wave-13: ruler click/drag/keyboard target owned by the scroller
   * (`App` jumps the virtualized pane via `rowIndexToScrollTop`).
   * Absent = visual-only ruler (pre-wave-13 behaviour).
   */
  onJump?: (index: number) => void;
  /** Wave-13: currently visible top row highlighted by the ruler. */
  activeIndex?: number;
  /**
   * Wave-21: bottom detail zone (Beyond Compare style line detail pane).
   * Rendered between the view and the status bar; absent/null renders
   * nothing so non-text kinds show no pane without layout breakage.
   */
  detail?: React.ReactNode;
  children: React.ReactNode;
}

export function ChromeShell({
  tabs,
  activeTabId,
  onSelectTab,
  onCloseTab,
  onNewTab,
  toolbar,
  leftPath,
  rightPath,
  onSubmitPath,
  onSubmit,
  isComparing,
  pending,
  disabled,
  note,
  basePath,
  showBase,
  onDropPaths,
  status,
  statusMessage,
  diffRows,
  onJump,
  activeIndex,
  detail,
  children,
}: ChromeShellProps): React.JSX.Element {
  // Wave-142: web fallback for native menu accelerators (Cmd+1..7 / N /
  // O / S / D / E). Emits the same `menu-event` the Tauri shell emits;
  // view-local chords (F/G, l/r, arrows, Enter/Esc) pass through.
  useWebShortcuts();
  const [submittedPath, setSubmittedPath] = React.useState<string | null>(null);

  const handleSubmit = (side: "left" | "right" | "base", path: string) => {
    setSubmittedPath(path);
    (onSubmit ?? onSubmitPath)?.(side, path);
  };

  const comparing = Boolean(isComparing || pending || disabled);

  const displayMessage =
    submittedPath && !statusMessage.includes(submittedPath)
      ? `${statusMessage} • ${submittedPath}`
      : statusMessage;

  return (
    <div className="flex min-h-screen flex-col bg-[var(--ac-bg)] text-[var(--ac-fg)]">
      {/*
       * Wave-174: BC chrome budget — ONE shared band for tabs + toolbar
       * (was: tabbar row 40 + toolbar row 44 + pathbar row 32 = 3 stacked
       * shell rows; view toolbars made 4). The band is a single 44px row:
       * TabBar (40, capped 280px, internal scroll) + Toolbar (rest,
       * internal scroll, single-row guarantee intact) side-by-side, then
       * PathBar as the second row (32). Shell above panes: 2 rows, 76px.
       * Wave-190: cap was 42%, which starved the toolbar (17x44px
       * actions need 812px+gaps at 1280; merge overflowed ~28px).
       * Wave-207: cap 34% -> 280px. With 2 tabs the TabBar's intrinsic
       * 356px still starved the top action row (973px of actions in a
       * 924px slot), so the trailing `toolbar/action-merge` rect sat
       * ~40px past the band edge and the full-page overlap gate flagged
       * `chrome/band OVERLAPS toolbar/action-merge`. 280px leaves the
       * toolbar 1000px at 1280 (27px slack, no clip, no scroll); chips
       * keep the W186 shrink-to-fit + ellipsis, so 3 tabs fit.
       * Chips keep the W186 shrink-to-fit + ellipsis, so 3 tabs fit.
       * In-view toolbars (Text/Table/Hex/Merge) are views-owned (off
       * limits here) and already single-row + bus-unified (W161/W170);
       * Find/Goto ride overlays, no new stacked row anywhere. All
       * testids, heights-per-component, labels, wiring byte-identical —
       * placement only.
       */}
      <div
        data-testid="chrome/band"
        className="flex h-[var(--ac-toolbar-h)] items-center border-b border-[var(--ac-border)] bg-[var(--ac-tab-bg)]"
      >
        <TabBar
          tabs={tabs}
          activeTabId={activeTabId}
          onSelectTab={onSelectTab}
          onCloseTab={onCloseTab}
          onNewTab={onNewTab}
          className="min-w-0 max-w-[280px] shrink"
        />
        <Toolbar actions={toolbar.actions} className="min-w-0 flex-1" />
      </div>
      <PathBar
        leftPath={leftPath}
        rightPath={rightPath}
        onSubmit={handleSubmit}
        onSubmitPath={onSubmitPath}
        isComparing={comparing}
        pending={pending}
        disabled={disabled}
        note={note}
        basePath={basePath}
        showBase={showBase}
        onDropPaths={onDropPaths}
      />
      <div className="flex min-h-0 flex-1">
        <OverviewRuler rows={diffRows} onJump={onJump} activeIndex={activeIndex} />
        <main className="min-w-0 flex-1">{children}</main>
      </div>
      {detail}
      <StatusBar left={status.left} right={status.right} message={displayMessage} />
    </div>
  );
}
