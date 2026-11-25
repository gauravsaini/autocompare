import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  AlignJustify,
  ArrowLeftRight,
  ChevronDown,
  ChevronUp,
  Command,
  Copy,
  Download,
  Equal,
  Filter,
  FolderOpen,
  Home,
  List,
  Moon,
  Plus,
  RefreshCw,
  Search,
  Settings,
  SlidersHorizontal,
  Sparkles,
  Sun,
  Type,
  X,
} from "lucide-react";
import { sessionSave } from "../ipc/session";
import V2Modal from "./components/V2Modal";
import V2SettingsModal from "./components/V2SettingsModal";
import V2ReportExportModal from "./components/V2ReportExportModal";
import V2CommandPalette from "./components/V2CommandPalette";
import type { PaletteCommand } from "./components/V2CommandPalette";
import V2Home from "./views/V2Home";
import V2TextCompare from "./views/V2TextCompare";
import V2FolderCompare from "./views/V2FolderCompare";
import V2TableCompare from "./views/V2TableCompare";
import V2HexCompare from "./views/V2HexCompare";
import V2MergeCompare from "./views/V2MergeCompare";
import V2FolderMerge from "./views/V2FolderMerge";
import V2SyncCompare from "./views/V2SyncCompare";
import V2MediaCompare from "./views/V2MediaCompare";
import type { SessionKind, V2StatusInfo, V2Tab, V2ViewCommands } from "./types";

const HOME_TAB: V2Tab = {
  id: "home",
  kind: "home",
  title: "Home",
  leftPath: "",
  rightPath: "",
};

function inferDropKind(paths: readonly string[]): SessionKind {
  const first = (paths[0] ?? "").toLowerCase();
  if (first.endsWith(".csv") || first.endsWith(".tsv")) return "table";
  if (
    first.endsWith(".png") ||
    first.endsWith(".jpg") ||
    first.endsWith(".jpeg") ||
    first.endsWith(".wav") ||
    first.endsWith(".mp3")
  ) {
    return "media";
  }
  if (first.endsWith(".bin") || first.endsWith(".wasm") || first.endsWith(".exe") || first.endsWith(".dylib")) {
    return "hex";
  }
  const base = first.split("/").pop() ?? "";
  if (first.endsWith("/") || (base !== "" && !base.includes("."))) {
    return "folder";
  }
  return "text";
}

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || target.isContentEditable;
}

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
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [draftKind, setDraftKind] = useState<SessionKind>("text");
  const [draftLeft, setDraftLeft] = useState("");
  const [draftRight, setDraftRight] = useState("");
  const [saveToast, setSaveToast] = useState<string | null>(null);
  const [dropOverlay, setDropOverlay] = useState<"left" | "right" | "both" | null>(null);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  const activeTab = tabs.find((t) => t.id === activeId) ?? HOME_TAB;
  const activeTabRef = useRef<V2Tab>(activeTab);
  activeTabRef.current = activeTab;
  const commandsRef = useRef<V2ViewCommands>(commands);
  commandsRef.current = commands;

  const openSession = useCallback(
    (kind: SessionKind, left = "", right = "", customTitle?: string): void => {
      if (kind === "home") {
        setActiveId("home");
        return;
      }
      const id = `${kind}-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
      const kindLabel =
        kind === "folderMerge" ? "Folder Merge" : `${kind[0].toUpperCase()}${kind.slice(1)} Compare`;
      const baseTitle =
        customTitle ??
        (left && right
          ? `${left.split("/").pop() ?? left} ↔ ${right.split("/").pop() ?? right}`
          : kindLabel);
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

  const closeTab = useCallback(
    (id: string, e?: React.MouseEvent): void => {
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
    },
    [activeId],
  );

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

  const handleSaveWorkspaceSession = useCallback(async (): Promise<void> => {
    const tab = activeTabRef.current;
    if (tab.kind === "home") return;
    try {
      await sessionSave({
        schema_version: 1,
        name: tab.title,
        locked: false,
        active_tab_index: 0,
        tabs: [
          {
            id: tab.id,
            title: tab.title,
            session_type: tab.kind,
            left_path: tab.leftPath || null,
            right_path: tab.rightPath || null,
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
  }, []);

  // Global keyboard shortcuts (Cmd+K palette, Cmd+, preferences, Cmd+W close tab, n/p diff nav, Cmd+D copy)
  useEffect(() => {
    const onKey = (e: KeyboardEvent): void => {
      const mod = e.metaKey || e.ctrlKey;
      const key = e.key.toLowerCase();
      if (mod && key === "k") {
        e.preventDefault();
        setPaletteOpen((v) => !v);
        return;
      }
      if (mod && key === ",") {
        e.preventDefault();
        setSettingsOpen(true);
        return;
      }
      if (mod && key === "w" && activeTabRef.current.kind !== "home") {
        e.preventDefault();
        closeTab(activeTabRef.current.id);
        return;
      }
      if (mod && key === "d" && commandsRef.current.canCopy) {
        e.preventDefault();
        commandsRef.current.copySelected?.();
        return;
      }
      if (!mod && !isEditableTarget(e.target)) {
        if (key === "n" && commandsRef.current.hasSections) {
          e.preventDefault();
          commandsRef.current.nextSection?.();
        } else if (key === "p" && commandsRef.current.hasSections) {
          e.preventDefault();
          commandsRef.current.prevSection?.();
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [closeTab]);

  // Apply dropped file path(s) to current tab or auto-open a matching comparison
  const handleDroppedPaths = useCallback(
    (paths: readonly string[], side: "left" | "right"): void => {
      const clean = paths.map((p) => p.trim()).filter((p) => p.length > 0);
      if (clean.length === 0) return;

      const cur = activeTabRef.current;
      if (clean.length >= 2) {
        const kind = cur.kind === "home" ? inferDropKind(clean) : cur.kind;
        if (cur.kind === "home") {
          openSession(kind, clean[0], clean[1]);
        } else {
          updateActivePaths(clean[0], clean[1]);
        }
        setSaveToast(`Loaded 2 paths (${kind})`);
        setTimeout(() => setSaveToast(null), 2200);
        return;
      }

      const dropped = clean[0];
      if (cur.kind === "home") {
        const kind = inferDropKind([dropped]);
        openSession(kind, side === "left" ? dropped : "", side === "right" ? dropped : "");
      } else {
        const nextL = side === "left" ? dropped : cur.leftPath;
        const nextR = side === "right" ? dropped : cur.rightPath;
        updateActivePaths(nextL, nextR);
      }
      setSaveToast(`Dropped onto ${side} pane`);
      setTimeout(() => setSaveToast(null), 1800);
    },
    [openSession, updateActivePaths],
  );

  // Native Tauri window-level file drag-and-drop listener
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let active = true;

    const attach = async (): Promise<void> => {
      const hasTauri =
        typeof window !== "undefined" &&
        ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);
      if (!hasTauri) return;
      try {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        const fn = await getCurrentWindow().onDragDropEvent((event) => {
          if (!active) return;
          const payload = event.payload;
          if (payload.type === "enter" || payload.type === "over") {
            const x = payload.position?.x ?? 0;
            const dpr = window.devicePixelRatio || 1;
            const logicalX = x > window.innerWidth ? x / dpr : x;
            const count = "paths" in payload && Array.isArray(payload.paths) ? payload.paths.length : 1;
            setDropOverlay(count >= 2 ? "both" : logicalX >= window.innerWidth / 2 ? "right" : "left");
          } else if (payload.type === "drop") {
            setDropOverlay(null);
            const x = payload.position?.x ?? 0;
            const dpr = window.devicePixelRatio || 1;
            const logicalX = x > window.innerWidth ? x / dpr : x;
            const side = logicalX >= window.innerWidth / 2 ? "right" : "left";
            handleDroppedPaths(payload.paths ?? [], side);
          } else {
            setDropOverlay(null);
          }
        });
        if (active) unlisten = fn;
        else fn();
      } catch {
        // Non-Tauri browser preview
      }
    };

    void attach();
    return () => {
      active = false;
      unlisten?.();
    };
  }, [handleDroppedPaths]);

  // Native macOS menu-event listener (all 18 menu IDs in src-tauri/src/menu.rs)
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let active = true;

    const attach = async (): Promise<void> => {
      const hasTauri =
        typeof window !== "undefined" &&
        ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);
      if (!hasTauri) return;
      try {
        const { listen } = await import("@tauri-apps/api/event");
        const fn = await listen<{ id: string; command: string }>("menu-event", (event) => {
          if (!active) return;
          const id = event.payload?.id ?? "";
          switch (id) {
            case "file.open-session":
            case "file.open-workspace":
            case "session.new":
              setNewSessionOpen(true);
              break;
            case "file.open-folder":
              openSession("folder");
              break;
            case "file.save-session":
              void handleSaveWorkspaceSession();
              break;
            case "session.find":
              commandsRef.current.openFind?.();
              break;
            case "session.take-left":
              commandsRef.current.copySelected?.();
              break;
            case "view.kind-text":
              openSession("text");
              break;
            case "view.kind-folder":
              openSession("folder");
              break;
            case "view.kind-hex":
              openSession("hex");
              break;
            case "view.kind-table":
              openSession("table");
              break;
            case "view.kind-media":
              openSession("media");
              break;
            case "view.kind-sync":
              openSession("sync");
              break;
            case "view.kind-merge":
              openSession("merge");
              break;
            case "view.toggle-theme":
              setTheme((t) => (t === "dark" ? "light" : "dark"));
              break;
            case "tools.settings":
              setSettingsOpen(true);
              break;
            default:
              break;
          }
        });
        if (active) unlisten = fn;
        else fn();
      } catch {
        // Non-Tauri browser preview
      }
    };

    void attach();
    return () => {
      active = false;
      unlisten?.();
    };
  }, [openSession, handleSaveWorkspaceSession]);

  const paletteCommands = useMemo<PaletteCommand[]>(() => {
    const list: PaletteCommand[] = [
      { id: "new-text", category: "Compare", label: "New Text Compare", shortcut: "⌘1", run: () => openSession("text") },
      { id: "new-folder", category: "Compare", label: "New Folder Compare", shortcut: "⌘2", run: () => openSession("folder") },
      { id: "new-hex", category: "Compare", label: "New Hex Compare", shortcut: "⌘3", run: () => openSession("hex") },
      { id: "new-table", category: "Compare", label: "New Table Compare", shortcut: "⌘4", run: () => openSession("table") },
      { id: "new-media", category: "Compare", label: "New Media Compare", shortcut: "⌘5", run: () => openSession("media") },
      { id: "new-sync", category: "Compare", label: "New Folder Sync", shortcut: "⌘6", run: () => openSession("sync") },
      { id: "new-merge", category: "Compare", label: "New 3-Way Text Merge", shortcut: "⌘7", run: () => openSession("merge") },
      { id: "new-fmerge", category: "Compare", label: "New 3-Way Folder Merge", run: () => openSession("folderMerge") },
      { id: "go-home", category: "Navigate", label: "Go to Home", run: () => setActiveId("home") },
      { id: "toggle-theme", category: "View", label: `Switch to ${theme === "dark" ? "Light" : "Dark"} Mode`, run: () => setTheme((t) => (t === "dark" ? "light" : "dark")) },
      { id: "open-settings", category: "App", label: "Open Preferences", shortcut: "⌘,", run: () => setSettingsOpen(true) },
    ];
    if (activeTab.kind !== "home") {
      list.push(
        { id: "save-sess", category: "Session", label: "Save Active Session", shortcut: "⌘S", run: () => void handleSaveWorkspaceSession() },
        { id: "export-rep", category: "Report", label: "Export HTML / Markdown / JSON Report", run: () => setExportOpen(true) },
      );
      if (commands.swapSides) {
        list.push({ id: "swap", category: "Action", label: "Swap Left & Right Sides", run: () => commands.swapSides?.() });
      }
      if (commands.reload) {
        list.push({ id: "reload", category: "Action", label: "Reload Comparison", run: () => commands.reload?.() });
      }
    }
    return list;
  }, [activeTab.kind, theme, openSession, handleSaveWorkspaceSession, commands]);

  return (
    <div
      className="relative flex h-screen w-screen flex-col overflow-hidden bg-[var(--v2-bg)] text-[var(--v2-fg)]"
      onDragOver={(e) => {
        e.preventDefault();
        setDropOverlay(e.clientX >= window.innerWidth / 2 ? "right" : "left");
      }}
      onDragLeave={(e) => {
        if (e.currentTarget === e.target) setDropOverlay(null);
      }}
      onDrop={(e) => {
        e.preventDefault();
        setDropOverlay(null);
        const side = e.clientX >= window.innerWidth / 2 ? "right" : "left";
        const files = Array.from(e.dataTransfer?.files ?? []).map(
          (f) => (f as File & { path?: string }).path || f.name,
        );
        if (files.length > 0) handleDroppedPaths(files, side);
      }}
    >
      {/* 1. Crisp macOS Tab Strip */}
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
            <>
              <button
                type="button"
                title="Export comparison report"
                onClick={() => setExportOpen(true)}
                className="v2-btn h-[22px] text-[11px]"
              >
                <Download size={11} />
                Export
              </button>
              <button
                type="button"
                onClick={() => void handleSaveWorkspaceSession()}
                className="v2-btn h-[22px] text-[11px]"
              >
                Save Session
              </button>
            </>
          ) : null}
          <button
            type="button"
            title="Command Palette (Cmd+K)"
            onClick={() => setPaletteOpen(true)}
            className="v2-btn h-[22px] gap-1 px-2 text-[11px] text-[var(--v2-fg-secondary)]"
          >
            <Command size={11} />
            <span>K</span>
          </button>
          <button
            type="button"
            title="Preferences (Cmd+,)"
            aria-label="Preferences"
            onClick={() => setSettingsOpen(true)}
            className="inline-flex h-[24px] w-[24px] items-center justify-center rounded-[5px] text-[var(--v2-fg-secondary)] hover:bg-[var(--v2-btn-hover)] hover:text-[var(--v2-fg)]"
          >
            <Settings size={14} />
          </button>
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
          <TopBarBtn icon={FolderOpen} label="Sessions" onClick={() => setNewSessionOpen(true)} />
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
          <TopBarBtn icon={SlidersHorizontal} label="Rules" disabled={!commands.openRules} onClick={commands.openRules} />
          <TopBarBtn icon={Type} label="Format" disabled={!commands.openFormat} onClick={commands.openFormat} />
          <TopBarBtn icon={Search} label="Find" disabled={!commands.openFind} onClick={commands.openFind} />
          <BarSep />
          <TopBarBtn icon={Copy} label="Copy" disabled={!commands.canCopy} onClick={commands.copySelected} />
          <TopBarBtn icon={ChevronUp} label="Prev" disabled={!commands.hasSections} onClick={commands.prevSection} />
          <TopBarBtn icon={ChevronDown} label="Next" disabled={!commands.hasSections} onClick={commands.nextSection} />
          <BarSep />
          <TopBarBtn icon={ArrowLeftRight} label="Swap" disabled={!commands.swapSides} onClick={commands.swapSides} />
          <TopBarBtn icon={RefreshCw} label="Reload" disabled={!commands.reload} onClick={commands.reload} />
        </div>
      ) : null}

      {/* 3. Main Content Surface (100% Native v2 Views — Zero Legacy Imports!) */}
      <div className="min-h-0 flex-1 overflow-hidden">
        {activeTab.kind === "home" ? (
          <V2Home onNewSession={openSession} onOpenSessionManager={() => setNewSessionOpen(true)} />
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
        ) : activeTab.kind === "table" ? (
          <V2TableCompare
            key={activeTab.id}
            leftPath={activeTab.leftPath}
            rightPath={activeTab.rightPath}
            onPathsChange={updateActivePaths}
            onRegisterCommands={setCommands}
            onStatusChange={setStatus}
          />
        ) : activeTab.kind === "hex" ? (
          <V2HexCompare
            key={activeTab.id}
            leftPath={activeTab.leftPath}
            rightPath={activeTab.rightPath}
            onPathsChange={updateActivePaths}
            onRegisterCommands={setCommands}
            onStatusChange={setStatus}
          />
        ) : activeTab.kind === "merge" ? (
          <V2MergeCompare
            key={activeTab.id}
            leftPath={activeTab.leftPath}
            rightPath={activeTab.rightPath}
            onPathsChange={updateActivePaths}
            onRegisterCommands={setCommands}
            onStatusChange={setStatus}
          />
        ) : activeTab.kind === "folderMerge" ? (
          <V2FolderMerge
            key={activeTab.id}
            leftPath={activeTab.leftPath}
            rightPath={activeTab.rightPath}
            onPathsChange={updateActivePaths}
            onRegisterCommands={setCommands}
            onStatusChange={setStatus}
          />
        ) : activeTab.kind === "sync" ? (
          <V2SyncCompare
            key={activeTab.id}
            leftPath={activeTab.leftPath}
            rightPath={activeTab.rightPath}
            onPathsChange={updateActivePaths}
            onRegisterCommands={setCommands}
            onStatusChange={setStatus}
          />
        ) : (
          <V2MediaCompare
            key={activeTab.id}
            leftPath={activeTab.leftPath}
            rightPath={activeTab.rightPath}
            onPathsChange={updateActivePaths}
            onRegisterCommands={setCommands}
            onStatusChange={setStatus}
          />
        )}
      </div>

      {/* 4. Clean 3-Zone Status Bar */}
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

      {/* Native Drag-and-Drop Overlay Indicator */}
      {dropOverlay !== null ? (
        <div className="pointer-events-none fixed inset-0 z-40 grid grid-cols-2 gap-3 bg-black/15 p-6 backdrop-blur-[1px]">
          <div
            className={`flex items-center justify-center rounded-xl border-2 border-dashed text-[14px] font-semibold transition ${
              dropOverlay === "left" || dropOverlay === "both"
                ? "border-[var(--v2-accent)] bg-[var(--v2-accent-soft)] text-[var(--v2-accent)]"
                : "border-[var(--v2-border)] bg-[var(--v2-surface)]/70 text-[var(--v2-fg-muted)]"
            }`}
          >
            Drop onto Left Pane
          </div>
          <div
            className={`flex items-center justify-center rounded-xl border-2 border-dashed text-[14px] font-semibold transition ${
              dropOverlay === "right" || dropOverlay === "both"
                ? "border-[var(--v2-accent)] bg-[var(--v2-accent-soft)] text-[var(--v2-accent)]"
                : "border-[var(--v2-border)] bg-[var(--v2-surface)]/70 text-[var(--v2-fg-muted)]"
            }`}
          >
            Drop onto Right Pane
          </div>
        </div>
      ) : null}

      {/* Centered New Session Modal */}
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
              <option value="merge">3-Way Text Merge</option>
              <option value="folderMerge">3-Way Folder Merge</option>
              <option value="sync">Folder Sync</option>
              <option value="media">Media Compare (Picture & Audio)</option>
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

      {/* Centered Preferences & Report Export Modals */}
      <V2SettingsModal
        open={settingsOpen}
        theme={theme}
        onThemeChange={setTheme}
        onClose={() => setSettingsOpen(false)}
      />
      <V2ReportExportModal
        open={exportOpen}
        tab={activeTab}
        onClose={() => setExportOpen(false)}
      />

      {/* Cmd+K Command Palette */}
      <V2CommandPalette
        open={paletteOpen}
        commands={paletteCommands}
        onClose={() => setPaletteOpen(false)}
      />
    </div>
  );
}
