/**
 * Wave-70: single-row merge controls toolbar (Beyond Compare style).
 *
 * Collapses the stacked chrome (conflict-count note + nav/take-all row +
 * find row) into one toolbar row: prev/next conflict nav + position,
 * take-all left/right (+ receipt), and a find toggle. Purely
 * presentational: every value and callback is owned by `MergeView`
 * (existing plan/choice/find logic untouched). All legacy testids ride
 * the same controls so wave-12/22/45/66F/66J specs keep passing.
 *
 * Wave-144 typexquis: nav/take-all arrows ride Lucide icons (no
 * text-arrow glyphs); labels and testids unchanged.
 */
import { ArrowDown, ArrowUp, ChevronLeft, ChevronRight, ChevronsLeft, ChevronsRight, Search } from "lucide-react";
import { Button, DotButton, StatusText, ToolbarButton } from "../../../ui";

export interface MergeToolbarProps {
  readonly position: string;
  readonly onPrev: () => void;
  readonly onNext: () => void;
  readonly onTakeAllLeft: () => void;
  readonly onTakeAllRight: () => void;
  readonly takeAllNote: string | null;
  readonly findOpen: boolean;
  readonly onToggleFind: () => void;
  /** Wave-147: overview ruler ids in conflict order (toolbar-embedded, same row). */
  readonly conflictIds?: readonly number[];
  /** Wave-147: currently selected conflict id (null = none). */
  readonly selectedId?: number | null;
  /** Wave-147: jump to conflict by id (same selectConflict path as nav). */
  readonly onSelectId?: (id: number) => void;
  /**
   * GAP-13 (FIXMERGE): active-conflict pointer + per-hunk Take
   * Left/Center/Right (BC 3-way toolbar parity). `activeId` names the
   * selected conflict, `activePosition` mirrors the view position label
   * (`Conflict i of n`); the three take callbacks resolve the ACTIVE
   * conflict only (per-conflict card buttons stay untouched). All props
   * optional so legacy mounts render byte-identical chrome.
   */
  readonly activeId?: number | null;
  readonly activePosition?: string | null;
  readonly onTakeActiveLeft?: () => void;
  readonly onTakeActiveCenter?: () => void;
  readonly onTakeActiveRight?: () => void;
  readonly takeActiveDisabled?: boolean;
}

export default function MergeToolbar(props: MergeToolbarProps): React.JSX.Element {
  const { position, onPrev, onNext, onTakeAllLeft, onTakeAllRight, takeAllNote, findOpen, onToggleFind, conflictIds, selectedId, onSelectId, activeId, activePosition, onTakeActiveLeft, onTakeActiveCenter, onTakeActiveRight, takeActiveDisabled } = props;
  const showActiveTakes = onTakeActiveLeft !== undefined || onTakeActiveCenter !== undefined || onTakeActiveRight !== undefined;
  const activeDisabled = takeActiveDisabled === true || activeId === null || activeId === undefined;
  return (
    <div
      role="toolbar"
      aria-label="Merge controls"
      data-testid="mergeview/toolbar"
      className="tv-controls-bar"
    >
      <Button size="sm" data-testid="merge-prev" onClick={onPrev}>
        <ChevronLeft size={14} strokeWidth={1.75} aria-hidden="true" /> Prev conflict
      </Button>
      <Button size="sm" data-testid="merge-next" onClick={onNext}>
        Next conflict <ChevronRight size={14} strokeWidth={1.75} aria-hidden="true" />
      </Button>
      <StatusText data-testid="merge-conflict-position">
        {position}
      </StatusText>
      {/* Wave-147: toolbar-embedded overview ruler (BC right-edge ruler
          folded into the single toolbar row: same row, no new stacked
          rows, chrome budget holds). One 12px DotButton dot per conflict in
          conflict order; click jumps via the same selectConflict path
          as prev/next. Reuses existing accent/border tokens only. */}
      {(conflictIds ?? []).length > 0 && (
        <span
          data-testid="merge-overview"
          role="group"
          aria-label="conflict overview"
          className="ac-control-group--tight"
        >
          {(conflictIds ?? []).map((id) => {
            const active = selectedId === id;
            return (
              <DotButton
                key={id}
                active={active}
                data-testid={`merge-overview-dot-${id}`}
                aria-label={`go to conflict ${id}`}
                aria-current={active ? "true" : undefined}
                title={`go to conflict ${id}`}
                onClick={() => onSelectId?.(id)}
              />
            );
          })}
        </span>
      )}
      <span aria-hidden="true" className="tv-controls-sep" />
      <Button size="sm" data-testid="merge-take-all-left" onClick={onTakeAllLeft}>
        <ChevronsLeft size={14} strokeWidth={1.75} aria-hidden="true" /> Take all left
      </Button>
      <Button size="sm" data-testid="merge-take-all-right" onClick={onTakeAllRight}>
        Take all right <ChevronsRight size={14} strokeWidth={1.75} aria-hidden="true" />
      </Button>
      {takeAllNote === null ? null : (
        <StatusText
          data-testid="merge-take-all-status"
          role="status"
        >
          {takeAllNote}
        </StatusText>
      )}
      {/* GAP-13 (FIXMERGE): active-conflict pointer + per-hunk Take
          Left/Center/Right for the SELECTED conflict (BC 3-way toolbar
          parity). Same row, compact buttons; per-conflict card buttons
          below stay untouched. Renders only when the view wires the
          active-take callbacks, so legacy mounts are byte-identical. */}
      {showActiveTakes ? (
        <>
          <span aria-hidden="true" className="tv-controls-sep" />
          <StatusText
            data-testid="merge-active-conflict"
            data-active-id={activeId ?? ""}
            role="status"
            aria-label={activePosition ?? (activeId === null || activeId === undefined ? "no active conflict" : `conflict ${activeId}`)}
            title={activePosition ?? "no active conflict"}
          >
            {activePosition ?? "no active conflict"}
          </StatusText>
          <Button
            size="sm"
            data-testid="merge-take-active-left"
            onClick={onTakeActiveLeft}
            disabled={activeDisabled || onTakeActiveLeft === undefined}
            title="Take Left for the active conflict"
          >
            <ChevronLeft size={14} strokeWidth={1.75} aria-hidden="true" /> Take Left
          </Button>
          <Button
            size="sm"
            data-testid="merge-take-active-center"
            onClick={onTakeActiveCenter}
            disabled={activeDisabled || onTakeActiveCenter === undefined}
            title="Take Center (common ancestor) for the active conflict"
          >
            Take Center
          </Button>
          <Button
            size="sm"
            data-testid="merge-take-active-right"
            onClick={onTakeActiveRight}
            disabled={activeDisabled || onTakeActiveRight === undefined}
            title="Take Right for the active conflict"
          >
            Take Right <ChevronRight size={14} strokeWidth={1.75} aria-hidden="true" />
          </Button>
        </>
      ) : null}
      <span aria-hidden="true" className="tv-controls-sep" />
      <ToolbarButton
        data-testid="mergeview/toolbar-find"
        label={findOpen ? "Hide find (Ctrl+F)" : "Show find (Ctrl+F)"}
        title="Find in merge (Ctrl+F)"
        expanded={findOpen}
        icon={Search}
        onClick={onToggleFind}
      >
        <span>Find</span>
      </ToolbarButton>
      <StatusText className="ac-control-group--tight">
        (keys: j/k, n/p, <ArrowUp size={14} strokeWidth={1.75} aria-hidden="true" />/<ArrowDown size={14} strokeWidth={1.75} aria-hidden="true" />)
      </StatusText>
    </div>
  );
}
