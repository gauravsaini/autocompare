import { useState } from "react";
import { Search } from "lucide-react";
import Button from "./Button";
import SegmentedControl from "./SegmentedControl";
import ToolbarButton from "./ToolbarButton";
import Checkbox from "./Checkbox";
import Input from "./Input";
import Select from "./Select";
import Field, { LabeledSelect } from "./Field";
import StatusText from "./StatusText";
import DotButton from "./DotButton";
import Modal from "./Modal";
import DockedBar from "./DockedBar";
import EmptyState from "./EmptyState";
import InlineNotice from "./InlineNotice";
import { UnifiedCopyCluster, UnifiedDiffOnlyCluster, UnifiedFilterSegment, UnifiedNavCluster, useUnifiedEsc } from "./UnifiedViewControls";

/**
 * Wave-232: UI primitive gallery (dev-only, `/?gallery=1`).
 * Renders every `ui/` export with `gallery/*` testids so the
 * gallery gate pins drift in both color schemes. No IPC, no views.
 */
export function Gallery(): React.JSX.Element {
  const [seg, setSeg] = useState("all");
  const [modalOpen, setModalOpen] = useState(false);
  const [dockOpen, setDockOpen] = useState(true);
  // Wave-239: UnifiedViewControls showcase state (presentational only).
  const [unifiedFilter, setUnifiedFilter] = useState("all");
  const [unifiedDiffsOnly, setUnifiedDiffsOnly] = useState(false);
  const [unifiedContext, setUnifiedContext] = useState(3);
  const [unifiedNote, setUnifiedNote] = useState("Unified clusters idle.");
  const [unifiedPanelOpen, setUnifiedPanelOpen] = useState(true);
  useUnifiedEsc(unifiedPanelOpen ? () => setUnifiedPanelOpen(false) : undefined, unifiedPanelOpen);
  return (
    <main data-testid="gallery/root" className="mx-auto flex w-full max-w-3xl flex-col gap-6 bg-[var(--ac-bg)] px-8 py-10 text-[var(--ac-fg)]">
      <h1 data-testid="gallery/title" className="ac-display font-semibold">
        UI Gallery
      </h1>

      <section aria-label="Button" data-testid="gallery/section-button" className="flex flex-col gap-2">
        <h2 className="ac-title font-medium">Button</h2>
        <div className="flex flex-wrap items-center gap-2">
          <Button variant="primary" size="sm" data-testid="gallery/button-primary-sm">
            Primary sm
          </Button>
          <Button variant="secondary" size="sm" data-testid="gallery/button-secondary-sm">
            Secondary sm
          </Button>
          <Button variant="ghost" size="md" data-testid="gallery/button-ghost-md">
            Ghost md
          </Button>
          <Button variant="danger" size="md" data-testid="gallery/button-danger-md">
            Danger md
          </Button>
        </div>
      </section>

      <section aria-label="SegmentedControl" data-testid="gallery/section-segmented" className="flex flex-col gap-2">
        <h2 className="ac-title font-medium">SegmentedControl</h2>
        <SegmentedControl
          label="Filter"
          testid="gallery/segmented"
          options={[
            { value: "all", label: "All", testid: "gallery/segmented-all", pressed: seg === "all", onSelect: () => setSeg("all") },
            { value: "diffs", label: "Diffs", testid: "gallery/segmented-diffs", pressed: seg === "diffs", onSelect: () => setSeg("diffs") },
          ]}
        />
      </section>

      <section aria-label="ToolbarButton" data-testid="gallery/section-toolbar" className="flex flex-col gap-2">
        <h2 className="ac-title font-medium">ToolbarButton</h2>
        <div className="flex flex-wrap items-center gap-2">
          <ToolbarButton label="Find" icon={Search} data-testid="gallery/toolbar-btn">
            Find
          </ToolbarButton>
        </div>
      </section>

      <section aria-label="Checkbox" data-testid="gallery/section-checkbox" className="flex flex-col gap-2">
        <h2 className="ac-title font-medium">Checkbox</h2>
        <Checkbox label="Ignore whitespace" defaultChecked data-testid="gallery/checkbox" />
      </section>

      <section aria-label="Input and Select" data-testid="gallery/section-input" className="flex flex-col gap-2">
        <h2 className="ac-title font-medium">Input / Select / Field</h2>
        <div className="flex flex-wrap items-center gap-2">
          <Input aria-label="Find text" placeholder="Find" defaultValue="demo" data-testid="gallery/input-sm" />
          <Input aria-label="Find text large" placeholder="Find" controlSize="md" defaultValue="demo" data-testid="gallery/input-md" />
          <Select aria-label="Encoding" defaultValue="utf8" data-testid="gallery/select">
            <option value="utf8">UTF-8</option>
            <option value="latin1">Latin-1</option>
          </Select>
        </div>
        <Field label="Goto line" htmlFor="gallery-goto">
          <Input id="gallery-goto" aria-label="Goto line" placeholder="1" data-testid="gallery/field-input" />
        </Field>
        <LabeledSelect label="Format" id="gallery-format" defaultValue="unix" data-testid="gallery/labeled-select">
          <option value="unix">LF</option>
          <option value="dos">CRLF</option>
        </LabeledSelect>
      </section>

      <section aria-label="StatusText" data-testid="gallery/section-status" className="flex flex-col gap-2">
        <h2 className="ac-title font-medium">StatusText</h2>
        <StatusText data-testid="gallery/status-meta">Ready — 200 / 644 rows</StatusText>
        <StatusText variant="code" data-testid="gallery/status-code">
          Ln 1, Col 1
        </StatusText>
      </section>

      <section aria-label="DotButton" data-testid="gallery/section-dot" className="flex flex-col gap-2">
        <h2 className="ac-title font-medium">DotButton</h2>
        <div className="flex items-center gap-2">
          <DotButton aria-label="Conflict 1" data-testid="gallery/dot-off" />
          <DotButton active aria-label="Conflict 2" data-testid="gallery/dot-on" />
        </div>
      </section>

      <section aria-label="Modal" data-testid="gallery/section-modal" className="flex flex-col gap-2">
        <h2 className="ac-title font-medium">Modal</h2>
        <div>
          <Button variant="secondary" size="sm" onClick={() => setModalOpen(true)} data-testid="gallery/open-modal">
            Open modal
          </Button>
        </div>
        <Modal open={modalOpen} onClose={() => setModalOpen(false)} label="Gallery modal" testid="gallery/modal">
          <StatusText data-testid="gallery/modal-body">Centered, Esc or backdrop dismisses.</StatusText>
        </Modal>
      </section>

      <section aria-label="DockedBar" data-testid="gallery/section-docked" className="flex flex-col gap-2">
        <h2 className="ac-title font-medium">DockedBar</h2>
        <div>
          <Button variant="secondary" size="sm" onClick={() => setDockOpen((v) => !v)} data-testid="gallery/toggle-dock">
            Toggle bar
          </Button>
        </div>
        <DockedBar open={dockOpen} onClose={() => setDockOpen(false)} label="Gallery bar" testid="gallery/dock" closeTestId="gallery/dock-close">
          <Input aria-label="Docked find" placeholder="Find in bar" data-testid="gallery/dock-input" />
        </DockedBar>
      </section>

      <section aria-label="EmptyState" data-testid="gallery/section-empty" className="flex flex-col gap-2">
        <h2 className="ac-title font-medium">EmptyState</h2>
        <EmptyState
          title="Nothing to compare"
          body="Open two paths to start."
          testid="gallery/empty"
          action={
            <Button variant="primary" size="sm" data-testid="gallery/empty-action">
              Open session
            </Button>
          }
        />
      </section>

      <section aria-label="InlineNotice" data-testid="gallery/section-notice" className="flex flex-col gap-2">
        <h2 className="ac-title font-medium">InlineNotice</h2>
        <InlineNotice testid="gallery/notice">Minor toggle applies to text and table only.</InlineNotice>
      </section>

      <section aria-label="UnifiedViewControls" data-testid="gallery/section-unified" className="flex flex-col gap-2">
        <h2 className="ac-title font-medium">UnifiedViewControls</h2>
        <div className="flex flex-wrap items-center gap-2" data-testid="gallery/unified-nav">
          <UnifiedNavCluster
            prevTestid="gallery/unified-nav-prev"
            nextTestid="gallery/unified-nav-next"
            onPrev={() => setUnifiedNote("Unified nav: prev")}
            onNext={() => setUnifiedNote("Unified nav: next")}
          />
        </div>
        <div className="flex flex-wrap items-center gap-2" data-testid="gallery/unified-filter">
          <UnifiedFilterSegment
            label="Unified filter"
            testid="gallery/unified-filter-group"
            options={[
              { value: "all", label: "All", testid: "gallery/unified-filter-all", pressed: unifiedFilter === "all", onSelect: () => { setUnifiedFilter("all"); setUnifiedNote("Unified filter: all"); } },
              { value: "diffs", label: "Diffs", testid: "gallery/unified-filter-diffs", pressed: unifiedFilter === "diffs", onSelect: () => { setUnifiedFilter("diffs"); setUnifiedNote("Unified filter: diffs"); } },
              { value: "same", label: "Same", testid: "gallery/unified-filter-same", pressed: unifiedFilter === "same", onSelect: () => { setUnifiedFilter("same"); setUnifiedNote("Unified filter: same"); } },
            ]}
          />
        </div>
        <div className="flex flex-wrap items-center gap-2" data-testid="gallery/unified-diffonly">
          <UnifiedDiffOnlyCluster
            toggleTestid="gallery/unified-diffsonly"
            checked={unifiedDiffsOnly}
            onToggle={setUnifiedDiffsOnly}
            contextTestid="gallery/unified-context"
            contextValue={unifiedContext}
            onContext={setUnifiedContext}
            contextDisabled={!unifiedDiffsOnly}
          />
        </div>
        <div className="flex flex-wrap items-center gap-2" data-testid="gallery/unified-copy">
          <UnifiedCopyCluster
            leftTestid="gallery/unified-copy-left"
            rightTestid="gallery/unified-copy-right"
            leftTitle="Copy left (demo)"
            rightTitle="Copy right (demo)"
            onCopyLeft={() => setUnifiedNote("Unified copy: left")}
            onCopyRight={() => setUnifiedNote("Unified copy: right")}
          />
        </div>
        <StatusText data-testid="gallery/unified-note" role="status">
          {unifiedNote}
        </StatusText>
        {unifiedPanelOpen ? (
          <div data-testid="gallery/unified-dismissable" className="flex items-center gap-2">
            <StatusText data-testid="gallery/unified-dismissable-note">Esc dismisses this cluster (DockedBar contract).</StatusText>
            <Button variant="secondary" size="sm" onClick={() => setUnifiedPanelOpen(false)} data-testid="gallery/unified-close">
              Dismiss unified
            </Button>
          </div>
        ) : (
          <div>
            <Button variant="secondary" size="sm" onClick={() => setUnifiedPanelOpen(true)} data-testid="gallery/unified-reopen">
              Reopen unified
            </Button>
          </div>
        )}
      </section>
    </main>
  );
}

export default Gallery;
