import React, { useCallback, useEffect, useState } from "react";
import {
  AlignJustify,
  ArrowLeftRight,
  ChevronDown,
  ChevronUp,
  Copy,
  Equal,
  Filter,
  FolderOpen,
  Home,
  List,
  Moon,
  Plus,
  RefreshCw,
  Search,
  SlidersHorizontal,
  Sparkles,
  Sun,
  Type,
  X,
} from "lucide-react";
import { sessionSave } from "../ipc/session";
import TableView from "../components/views/TableView";
import HexView from "../components/views/HexView";
import MergeView from "../components/views/MergeView";
import SyncView from "../components/views/SyncView";
import MediaView from "../components/views/MediaView";
import V2Modal from "./components/V2Modal";
import V2Home from "./views/V2Home";
import V2TextCompare from "./views/V2TextCompare";
import V2FolderCompare from "./views/V2FolderCompare";
import type { SessionKind, V2StatusInfo, V2Tab, V2ViewCommands } from "./types";

const HOME_TAB: V2Tab = {
  id: "home",
  kind: "home",
  title: "Home",
  leftPath: "",
  rightPath: "",
};

interface ToolbarBtnProps {
  readonly icon: React.ComponentType<{ size?: number; strokeWidth?: number }>;
  readonly label: string;
  readonly active?: boolean;
  readonly disabled?: boolean;
  readonly onClick?: () => void;
}

function TopBarBtn({
  icon: Icon,
  label,
  active,
  disabled,
  onClick,
}: ToolbarBtnProps): React.JSX.Element {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className={`inline-flex h-[38px] min-w-[44px] shrink-0 flex-col items-center justify-center gap-0.5 rounded-[6px] px-2 text-[10.5px] font-medium transition select-none ${
        active
          ? "bg-[var(--v2-accent-soft)] text-[var(--v2-accent)]"
          : "text-[var(--v2-fg)] hover:bg-[var(--v2-btn-hover)]"
      } disabled:cursor-not-allowed disabled:opacity-35 disabled:hover:bg-transparent`}
    >
      <Icon size={15} strokeWidth={1.75} />
      <span className="leading-none">{label}</span>
    </button>
  );
}

function BarSep(): React.JSX.Element {
  return <span aria-hidden="true" className="mx-1 h-6 w-px shrink-0 bg-[var(--v2-border)]" />;
}

export default function V2App(): React.JSX.Element {
  const [theme, setTheme] = useState<"light" | "dark">(() =>
    typeof window !== "undefined" && window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light",
  );
  const [tabs, setTabs] = useState<V2Tab[]>([HOME_TAB]);
  const [activeId, setActiveId] = useState<string>("home");
  const [commands, setCommands] = useState<V2ViewCommands>({});
  const [status, setStatus] = useState<V2StatusInfo>({ left: "", center: "", right: "" });
  const [newSessionOpen, setNewSessionOpen] = useState(false);
  const [draftKind, setDraftKind] = useState<SessionKind>("text");
  const [draftLeft, setDraftLeft] = useState("");
  const [draftRight, setDraftRight] = useState("");
  const [saveToast, setSaveToast] = useState<string | null>(null);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  const activeTab = tabs.find((t) => t.id === activeId) ?? HOME_TAB;

  const openSession = useCallback(
    (kind: SessionKind, left = "", right = "", customTitle?: string): void => {
      if (kind === "home") {
        setActiveId("home");
        return;
      }
      const id = `${kind}-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
      const baseTitle =
        customTitle ??
        (left && right
          ? `${left.split("/").pop() ?? left} ↔ ${right.split("/").pop() ?? right}`
          : `${kind[0].toUpperCase()}${kind.slice(1)} Compare`);
      const next: V2Tab = {
        id,
        kind,
        title: baseTitle,
        leftPath: left,
        rightPath: right,
        dirty: false,
      };
      setTabs((prev) => [...prev, next]);
      setActiveId(id);
      setCommands({});
    },
    [],
  );

  const closeTab = (id: string, e?: React.MouseEvent): void => {
    e?.stopPropagation();
    if (id === "home") return;
    setTabs((prev) => {
      const idx = prev.findIndex((t) => t.id === id);
      const next = prev.filter((t) => t.id !== id);
      if (activeId === id) {
        const fallback = next[Math.max(0, idx - 1)] ?? HOME_TAB;
        setActiveId(fallback.id);
      }
      return next;
    });
  };

  const updateActivePaths = useCallback(
    (left: string, right: string): void => {
      setTabs((prev) =>
        prev.map((t) => {
          if (t.id !== activeId) return t;
          const title =
            left && right
              ? `${left.split("/").pop() ?? left} ↔ ${right.split("/").pop() ?? right}`
              : t.title;
          return { ...t, leftPath: left, rightPath: right, title };
        }),
      );
    },
    [activeId],
  );

  const updateActiveDirty = useCallback(
    (dirty: boolean): void => {
      setTabs((prev) => prev.map((t) => (t.id === activeId ? { ...t, dirty } : t)));
    },
    [activeId],
  );

  const handleSaveWorkspaceSession = async (): Promise<void> => {
    if (activeTab.kind === "home") return;
    try {
      await sessionSave({
        schema_version: 1,
        name: activeTab.title,
        locked: false,
        active_tab_index: 0,
        tabs: [
          {
            id: activeTab.id,
            title: activeTab.title,
            session_type: activeTab.kind,
            left_path: activeTab.leftPath || null,
            right_path: activeTab.rightPath || null,
            base_path: null,
            output_path: null,
            read_only: false,
            locked: false,
            options: {},
          },
        ],
      });
      setSaveToast("Session saved");
      setTimeout(() => setSaveToast(null), 2200);
    } catch {
      setSaveToast("Could not save session");
      setTimeout(() => setSaveToast(null), 2200);
    }
  };

  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden bg-[var(--v2-bg)] text-[var(--v2-fg)]">
      {/* 1. Crisp macOS Tab Strip — `+` hugs the last tab, theme toggle on far right */}
      <header className="flex h-[36px] shrink-0 items-center justify-between border-b border-[var(--v2-border)] bg-[var(--v2-chrome)] px-2">
        <div className="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto">
          {tabs.map((tab) => {
            const isActive = tab.id === activeTab.id;
            return (
              <div
                key={tab.id}
                role="tab"
                aria-selected={isActive}
                tabIndex={0}
                onClick={() => {
                  setActiveId(tab.id);
                  if (tab.kind === "home") setCommands({});
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    setActiveId(tab.id);
                  }
                }}
                className={`group flex h-[26px] max-w-[220px] shrink-0 cursor-pointer items-center gap-1.5 rounded-[6px] px-2.5 text-[12px] font-medium transition select-none ${
                  isActive
                    ? "border border-[var(--v2-border)] bg-[var(--v2-chrome-elevated)] text-[var(--v2-fg)] shadow-[0_1px_2px_rgba(0,0,0,0.06)]"
                    : "text-[var(--v2-fg-secondary)] hover:bg-[var(--v2-btn-hover)]"
                }`}
              >
                {tab.kind === "home" ? <Home size={12} strokeWidth={2} /> : null}
                <span className="truncate">{tab.title}</span>
                {tab.dirty ? (
                  <span
                    title="Unsaved changes"
                    className="h-1.5 w-1.5 shrink-0 rounded-full bg-[var(--v2-amber-bright)]"
                  />
                ) : null}
                {tab.kind !== "home" ? (
                  <button
                    type="button"
                    aria-label={`Close ${tab.title}`}
                    onClick={(e) => closeTab(tab.id, e)}
                    className="ml-0.5 inline-flex h-4 w-4 items-center justify-center rounded text-[var(--v2-fg-muted)] opacity-60 hover:bg-[var(--v2-btn-active)] hover:text-[var(--v2-fg)] hover:opacity-100"
                  >
                    <X size={11} />
                  </button>
                ) : null}
              </div>
            );
          })}

          <button
            type="button"
            title="New comparison session"
            aria-label="New tab"
            onClick={() => setNewSessionOpen(true)}
            className="inline-flex h-[24px] w-[24px] shrink-0 items-center justify-center rounded-[5px] text-[var(--v2-fg-secondary)] hover:bg-[var(--v2-btn-hover)] hover:text-[var(--v2-fg)]"
          >
            <Plus size={14} strokeWidth={2} />
          </button>
        </div>

        <div className="ml-2 flex shrink-0 items-center gap-1.5">
          {saveToast ? (
            <span className="text-[11px] font-medium text-[var(--v2-accent)]">{saveToast}</span>
          ) : null}
          {activeTab.kind !== "home" ? (
            <button
              type="button"
              onClick={() => void handleSaveWorkspaceSession()}
              className="v2-btn h-[22px] text-[11px]"
            >
              Save Session
            </button>
          ) : null}
          <button
            type="button"
            title={theme === "dark" ? "Switch to Light Mode" : "Switch to Dark Mode"}
            aria-label="Toggle theme"
            onClick={() => setTheme((t) => (t === "dark" ? "light" : "dark"))}
            className="inline-flex h-[24px] w-[24px] items-center justify-center rounded-[5px] text-[var(--v2-fg-secondary)] hover:bg-[var(--v2-btn-hover)] hover:text-[var(--v2-fg)]"
          >
            {theme === "dark" ? <Sun size={14} /> : <Moon size={14} />}
          </button>
        </div>
      </header>

      {/* 2. Context-Aware Beyond Compare Unified Top Toolbar (HIDDEN on Home!) */}
      {activeTab.kind !== "home" ? (
        <div
          role="toolbar"
          aria-label="Comparison actions"
          className="flex h-[46px] shrink-0 items-center gap-0.5 overflow-x-auto border-b border-[var(--v2-border)] bg-[var(--v2-surface-subtle)] px-2.5"
        >
          <TopBarBtn icon={Home} label="Home" onClick={() => setActiveId("home")} />
          <TopBarBtn
            icon={FolderOpen}
            label="Sessions"
            onClick={() => setNewSessionOpen(true)}
          />
          <BarSep />
          <TopBarBtn
            icon={List}
            label="All"
            active={commands.filterMode === "all"}
            disabled={!commands.setFilterMode}
            onClick={() => commands.setFilterMode?.("all")}
          />
          <TopBarBtn
            icon={Filter}
            label="Diffs"
            active={commands.filterMode === "diffs"}
            disabled={!commands.setFilterMode}
            onClick={() => commands.setFilterMode?.("diffs")}
          />
          <TopBarBtn
            icon={Equal}
            label="Same"
            active={commands.filterMode === "same"}
            disabled={!commands.setFilterMode}
            onClick={() => commands.setFilterMode?.("same")}
          />
          <TopBarBtn
            icon={AlignJustify}
            label="Context"
            active={commands.contextEnabled === true}
            disabled={!commands.toggleContext}
            onClick={commands.toggleContext}
          />
          <BarSep />
          <TopBarBtn
            icon={Sparkles}
            label="Minor"
            active={commands.minorEnabled === true}
            disabled={!commands.toggleMinor}
            onClick={commands.toggleMinor}
          />
          <TopBarBtn
            icon={SlidersHorizontal}
            label="Rules"
            disabled={!commands.openRules}
            onClick={commands.openRules}
          />
          <TopBarBtn
            icon={Type}
            label="Format"
            disabled={!commands.openFormat}
            onClick={commands.openFormat}
          />
          <TopBarBtn
            icon={Search}
            label="Find"
            disabled={!commands.openFind}
            onClick={commands.openFind}
          />
          <BarSep />
          <TopBarBtn
            icon={Copy}
            label="Copy"
            disabled={!commands.canCopy}
            onClick={commands.copySelected}
          />
          <TopBarBtn
            icon={ChevronUp}
            label="Prev"
            disabled={!commands.hasSections}
            onClick={commands.prevSection}
          />
          <TopBarBtn
            icon={ChevronDown}
            label="Next"
            disabled={!commands.hasSections}
            onClick={commands.nextSection}
          />
          <BarSep />
          <TopBarBtn
            icon={ArrowLeftRight}
            label="Swap"
            disabled={!commands.swapSides}
            onClick={commands.swapSides}
          />
          <TopBarBtn
            icon={RefreshCw}
            label="Reload"
            disabled={!commands.reload}
            onClick={commands.reload}
          />
        </div>
      ) : null}

      {/* 3. Main Content Surface */}
      <div className="min-h-0 flex-1 overflow-hidden">
        {activeTab.kind === "home" ? (
          <V2Home
            onNewSession={openSession}
            onOpenSessionManager={() => setNewSessionOpen(true)}
          />
        ) : activeTab.kind === "text" ? (
          <V2TextCompare
            key={activeTab.id}
            leftPath={activeTab.leftPath}
            rightPath={activeTab.rightPath}
            onPathsChange={updateActivePaths}
            onDirtyChange={updateActiveDirty}
            onRegisterCommands={setCommands}
            onStatusChange={setStatus}
          />
        ) : activeTab.kind === "folder" ? (
          <V2FolderCompare
            key={activeTab.id}
            leftPath={activeTab.leftPath}
            rightPath={activeTab.rightPath}
            onPathsChange={updateActivePaths}
            onOpenTextPair={(l, r, title) => openSession("text", l, r, title)}
            onRegisterCommands={setCommands}
            onStatusChange={setStatus}
          />
        ) : (
          <div className="h-full overflow-auto bg-[var(--v2-surface)] p-3">
            {activeTab.kind === "table" ? (
              <TableView leftPath={activeTab.leftPath} rightPath={activeTab.rightPath} />
            ) : activeTab.kind === "hex" ? (
              <HexView leftPath={activeTab.leftPath} rightPath={activeTab.rightPath} />
            ) : activeTab.kind === "merge" ? (
              <MergeView leftPath={activeTab.leftPath} rightPath={activeTab.rightPath} />
            ) : activeTab.kind === "sync" ? (
              <SyncView leftPath={activeTab.leftPath} rightPath={activeTab.rightPath} />
            ) : (
              <MediaView leftPath={activeTab.leftPath} rightPath={activeTab.rightPath} />
            )}
          </div>
        )}
      </div>

      {/* 4. Clean 3-Zone Status Bar (No debug strings like "Tab selected: home") */}
      <footer className="flex h-[24px] shrink-0 items-center justify-between border-t border-[var(--v2-border)] bg-[var(--v2-chrome)] px-3 text-[11px] text-[var(--v2-fg-muted)]">
        <span className="truncate">
          {activeTab.kind === "home" ? "Ready" : status.left || "Ready"}
        </span>
        <span className="truncate">{activeTab.kind === "home" ? "" : status.center}</span>
        <span className="v2-mono truncate">
          {activeTab.kind === "home"
            ? `${Math.max(0, tabs.length - 1)} open ${tabs.length - 1 === 1 ? "comparison" : "comparisons"}`
            : status.right}
        </span>
      </footer>

      {/* Standardized Centered New Session Modal */}
      <V2Modal
        open={newSessionOpen}
        title="New Comparison Session"
        subtitle="Pick a mode and optional paths (leave blank for demo)"
        widthClass="max-w-md"
        onClose={() => setNewSessionOpen(false)}
        footer={
          <>
            <button type="button" onClick={() => setNewSessionOpen(false)} className="v2-btn">
              Cancel
            </button>
            <button
              type="button"
              onClick={() => {
                openSession(draftKind, draftLeft.trim(), draftRight.trim());
                setDraftLeft("");
                setDraftRight("");
                setNewSessionOpen(false);
              }}
              className="v2-btn v2-btn-primary"
            >
              Open Comparison
            </button>
          </>
        }
      >
        <div className="space-y-3 text-[12px]">
          <div>
            <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
              Comparison Type
            </span>
            <select
              value={draftKind}
              onChange={(e) => setDraftKind(e.target.value as SessionKind)}
              className="v2-input w-full"
            >
              <option value="text">Text Compare</option>
              <option value="folder">Folder Compare</option>
              <option value="table">Table Compare</option>
              <option value="hex">Hex Compare</option>
              <option value="merge">3-Way Merge</option>
              <option value="sync">Folder Sync</option>
              <option value="media">Media Compare</option>
            </select>
          </div>
          <div>
            <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
              Left Path
            </span>
            <input
              type="text"
              value={draftLeft}
              onChange={(e) => setDraftLeft(e.target.value)}
              placeholder="Optional — leave blank for sample diff"
              className="v2-input v2-mono w-full"
            />
          </div>
          <div>
            <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
              Right Path
            </span>
            <input
              type="text"
              value={draftRight}
              onChange={(e) => setDraftRight(e.target.value)}
              placeholder="Optional — leave blank for sample diff"
              className="v2-input v2-mono w-full"
            />
          </div>
        </div>
      </V2Modal>
    </div>
  );
}
