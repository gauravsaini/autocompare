import React, { Suspense, lazy, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { CircleHelp, Command, Download, Home, Moon, Plus, Settings, Sun, X } from "lucide-react";
import type { PaletteCommand } from "./components/V2CommandPalette";
import V2MenuBar from "./components/V2MenuBar";
import V2Toolbar from "./components/V2Toolbar";
import { useTabs } from "./hooks/useTabs";
import { useV2Shortcuts } from "./hooks/useV2Shortcuts";
import V2Home from "./views/V2Home";
import { registerBuiltinComparators } from "./plugins/builtins";
import { getComparator, listComparators } from "./plugins/registry";
import {
  DEFAULT_FILE_FORMATS,
  DEFAULT_VFS_PROFILES,
  exportSettingsBundleJson,
  loadFileFormats,
  loadVfsProfiles,
  parseSettingsBundleJson,
  saveFileFormats,
  saveVfsProfiles,
} from "./utils/formatsAndProfiles";
import type { V2StatusInfo, V2ViewCommands } from "./types";

// Register all built-in + litmus comparator plugins into the Comparator Seam
registerBuiltinComparators();

const V2Modal = lazy(() => import("./components/V2Modal"));
const V2NewSessionModal = lazy(() => import("./components/V2NewSessionModal"));
const V2SessionManagerModal = lazy(() => import("./components/V2SessionManagerModal"));
const V2SettingsModal = lazy(() => import("./components/V2SettingsModal"));
const V2FileFormatsModal = lazy(() => import("./components/V2FileFormatsModal"));
const V2ProfilesModal = lazy(() => import("./components/V2ProfilesModal"));
const V2ReportExportModal = lazy(() => import("./components/V2ReportExportModal"));
const V2CommandPalette = lazy(() => import("./components/V2CommandPalette"));
const V2ShortcutHelpModal = lazy(() => import("./components/V2ShortcutHelpModal"));

interface ViewErrorBoundaryState {
  readonly error: string | null;
}

class V2ViewErrorBoundary extends React.Component<
  { readonly resetKey: string; readonly viewName: string; readonly onGoHome?: () => void; readonly children: React.ReactNode },
  ViewErrorBoundaryState
> {
  state: ViewErrorBoundaryState = { error: null };

  static getDerivedStateFromError(err: unknown): ViewErrorBoundaryState {
    return { error: err instanceof Error ? err.message : String(err) };
  }

  componentDidUpdate(prevProps: { readonly resetKey: string }): void {
    if (prevProps.resetKey !== this.props.resetKey && this.state.error !== null) {
      this.setState({ error: null });
    }
  }

  render(): React.ReactNode {
    if (this.state.error !== null) {
      return (
        <div
          data-testid="v2/error-banner"
          className="flex h-full flex-col items-center justify-center gap-2 bg-[var(--v2-surface)] p-6 text-center"
        >
          <p className="text-[13px] font-semibold text-[var(--v2-del-fg)]">
            {this.props.viewName} view crashed
          </p>
          <p className="v2-mono max-w-lg text-[11.5px] text-[var(--v2-fg-muted)]">
            {this.state.error}
          </p>
          <div className="mt-1 flex items-center gap-2">
            <button
              type="button"
              onClick={() => this.setState({ error: null })}
              className="v2-btn v2-btn-primary"
            >
              Reload View
            </button>
            {this.props.onGoHome ? (
              <button
                type="button"
                onClick={() => {
                  this.setState({ error: null });
                  this.props.onGoHome?.();
                }}
                className="v2-btn"
              >
                Return Home
              </button>
            ) : null}
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}

function ViewLoadingFallback(): React.JSX.Element {
  return (
    <div className="flex h-full items-center justify-center bg-[var(--v2-surface)] text-[12px] text-[var(--v2-fg-muted)]">
      Loading view…
    </div>
  );
}

export default function V2App(): React.JSX.Element {
  const [theme, setTheme] = useState<"light" | "dark">(() =>
    typeof window !== "undefined" && window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light",
  );
  const [layoutDensity, setLayoutDensity] = useState<"standard" | "compact">("standard");
  const [commands, setCommands] = useState<V2ViewCommands>({});
  const [status, setStatus] = useState<V2StatusInfo>({ left: "", center: "", right: "" });
  const [newSessionOpen, setNewSessionOpen] = useState(false);
  const [sessionManagerOpen, setSessionManagerOpen] = useState(false);
  const [sessionRefresh, setSessionRefresh] = useState(0);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [fileFormatsOpen, setFileFormatsOpen] = useState(false);
  const [profilesOpen, setProfilesOpen] = useState(false);
  const [showToolbar, setShowToolbar] = useState(true);
  const [exportOpen, setExportOpen] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const [pendingCloseTabId, setPendingCloseTabId] = useState<string | null>(null);
  const importInputRef = useRef<HTMLInputElement | null>(null);

  const resetCommands = useCallback(() => setCommands({}), []);

  const {
    tabs,
    setActiveId,
    activeTab,
    activeTabRef,
    saveToast,
    showToast,
    openSession,
    closeTab,
    updateActivePaths,
    updateActiveDirty,
    handleSaveWorkspaceSession,
    handleDroppedPaths,
  } = useTabs({ onResetCommands: resetCommands });

  const commandsRef = useRef<V2ViewCommands>(commands);
  commandsRef.current = commands;

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  useEffect(() => {
    document.documentElement.dataset.layout = layoutDensity;
  }, [layoutDensity]);

  const toggleTheme = useCallback(
    () => setTheme((t) => (t === "dark" ? "light" : "dark")),
    [],
  );

  const toggleLayout = useCallback(() => {
    setLayoutDensity((d) => {
      const next = d === "standard" ? "compact" : "standard";
      showToast(`Layout: ${next}`, 1800);
      return next;
    });
  }, [showToast]);

  const togglePalette = useCallback(() => setPaletteOpen((v) => !v), []);
  const toggleShortcuts = useCallback(() => setShortcutsOpen((v) => !v), []);
  const openNewSession = useCallback(() => setNewSessionOpen(true), []);
  const openSettings = useCallback(() => setSettingsOpen(true), []);
  const openFileFormats = useCallback(() => setFileFormatsOpen(true), []);
  const openProfiles = useCallback(() => setProfilesOpen(true), []);
  const toggleToolbar = useCallback(() => setShowToolbar((v) => !v), []);

  const handleExportSettings = useCallback(() => {
    const json = exportSettingsBundleJson(loadFileFormats(), loadVfsProfiles());
    const blob = new Blob([json], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "autocompare-settings.json";
    a.click();
    URL.revokeObjectURL(url);
    showToast("Exported autocompare-settings.json", 2200);
  }, [showToast]);

  const handleImportSettings = useCallback(() => {
    importInputRef.current?.click();
  }, []);

  const handleRestoreDefaults = useCallback(() => {
    if (!window.confirm("Restore factory default File Formats and VFS Profiles?")) return;
    saveFileFormats(DEFAULT_FILE_FORMATS);
    saveVfsProfiles(DEFAULT_VFS_PROFILES);
    showToast("Restored factory default formats & profiles", 2200);
  }, [showToast]);

  const requestCloseTab = useCallback(
    (id: string, e?: React.MouseEvent): void => {
      e?.stopPropagation();
      const target = tabs.find((t) => t.id === id);
      if (target?.dirty) {
        setPendingCloseTabId(id);
        return;
      }
      closeTab(id);
    },
    [tabs, closeTab],
  );

  const { dropOverlay, setDropOverlay } = useV2Shortcuts({
    activeTabRef,
    commandsRef,
    openSession,
    closeTab: requestCloseTab,
    onSaveSession: handleSaveWorkspaceSession,
    onDroppedPaths: handleDroppedPaths,
    onTogglePalette: togglePalette,
    onToggleShortcuts: toggleShortcuts,
    onOpenNewSession: openNewSession,
    onOpenSettings: openSettings,
    onToggleTheme: toggleTheme,
    onToggleLayout: toggleLayout,
  });

  const paletteCommands = useMemo<PaletteCommand[]>(() => {
    const list: PaletteCommand[] = [
      ...listComparators().map((plugin) => ({
        id: `new-${plugin.id}`,
        category: "Compare",
        label: plugin.paletteLabel ?? `New ${plugin.title}`,
        shortcut: plugin.shortcut,
        run: () => openSession(plugin.id),
      })),
      { id: "go-home", category: "Navigate", label: "Go to Home", run: () => setActiveId("home") },
      { id: "toggle-theme", category: "View", label: `Switch to ${theme === "dark" ? "Light" : "Dark"} Mode`, run: toggleTheme },
      { id: "open-shortcuts", category: "Help", label: "Keyboard Shortcuts Cheat Sheet", shortcut: "⌘/", run: toggleShortcuts },
      { id: "open-settings", category: "App", label: "Open Preferences", shortcut: "⌘,", run: openSettings },
      { id: "open-formats", category: "Tools", label: "Manage File Formats…", run: openFileFormats },
      { id: "open-profiles", category: "Tools", label: "Manage VFS Profiles…", run: openProfiles },
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
  }, [activeTab.kind, theme, openSession, setActiveId, toggleTheme, toggleShortcuts, openSettings, openFileFormats, openProfiles, handleSaveWorkspaceSession, commands]);

  return (
    <div
      data-testid="v2/shell"
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
      {/* 0. Plugin-Driven Application Menu Bar (BC Parity — Free & OSS) */}
      <V2MenuBar
        activeTab={activeTab}
        commands={commands}
        showToolbar={showToolbar}
        layoutDensity={layoutDensity}
        theme={theme}
        onNewSessionModal={openNewSession}
        onOpenSessionKind={openSession}
        onOpenSessionManager={() => setSessionManagerOpen(true)}
        onSaveSession={() => void handleSaveWorkspaceSession()}
        onCloseActiveTab={() => {
          if (activeTab.kind !== "home") requestCloseTab(activeTab.id);
        }}
        onOpenSettings={openSettings}
        onOpenFileFormats={openFileFormats}
        onOpenProfiles={openProfiles}
        onOpenExportReport={() => setExportOpen(true)}
        onOpenShortcuts={toggleShortcuts}
        onOpenPalette={togglePalette}
        onToggleToolbar={toggleToolbar}
        onToggleLayout={toggleLayout}
        onToggleTheme={toggleTheme}
        onExportSettings={handleExportSettings}
        onImportSettings={handleImportSettings}
        onRestoreDefaults={handleRestoreDefaults}
        onShowToast={(msg) => showToast(msg, 2400)}
      />
      <input
        ref={importInputRef}
        type="file"
        accept="application/json,.json"
        className="hidden"
        onChange={(e) => {
          const file = e.target.files?.[0];
          if (!file) return;
          void file.text().then((text) => {
            const parsed = parseSettingsBundleJson(text);
            if (!parsed) {
              showToast("Invalid settings bundle JSON", 2400);
              return;
            }
            saveFileFormats(parsed.fileFormats);
            saveVfsProfiles(parsed.vfsProfiles);
            showToast(
              `Imported ${parsed.fileFormats.length} formats & ${parsed.vfsProfiles.length} profiles`,
              2400,
            );
          });
          e.target.value = "";
        }}
      />

      {/* 1. Crisp macOS Tab Strip */}
      <header
        data-testid="v2/tabbar"
        className="flex h-[36px] shrink-0 items-center justify-between border-b border-[var(--v2-border)] bg-[var(--v2-chrome)] px-2"
      >
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
                    onClick={(e) => requestCloseTab(tab.id, e)}
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
            onClick={openNewSession}
            className="inline-flex h-[24px] w-[24px] shrink-0 items-center justify-center rounded-[5px] text-[var(--v2-fg-secondary)] hover:bg-[var(--v2-btn-hover)] hover:text-[var(--v2-fg)]"
          >
            <Plus size={14} strokeWidth={2} />
          </button>
        </div>

        <div className="ml-2 flex shrink-0 items-center gap-1.5">
          {saveToast ? (
            <span data-testid="v2/toast" className="text-[11px] font-medium text-[var(--v2-accent)]">
              {saveToast}
            </span>
          ) : null}
          {activeTab.kind !== "home" ? (
            <>
              <button
                type="button"
                data-testid="v2/export-report"
                title="Export comparison report"
                onClick={() => setExportOpen(true)}
                className="v2-btn h-[22px] text-[11px]"
              >
                <Download size={11} />
                Export
              </button>
              <button
                type="button"
                data-testid="v2/save-session"
                onClick={() => void handleSaveWorkspaceSession()}
                className="v2-btn h-[22px] text-[11px]"
              >
                Save Session
              </button>
            </>
          ) : null}
          <button
            type="button"
            data-testid="v2/open-palette"
            title="Command Palette (Cmd+K)"
            onClick={() => setPaletteOpen(true)}
            className="v2-btn h-[22px] gap-1 px-2 text-[11px] text-[var(--v2-fg-secondary)]"
          >
            <Command size={11} />
            <span>K</span>
          </button>
          <button
            type="button"
            data-testid="v2/open-shortcuts"
            title="Keyboard Shortcuts (? or Cmd+/)"
            aria-label="Keyboard Shortcuts"
            onClick={toggleShortcuts}
            className="inline-flex h-[24px] w-[24px] items-center justify-center rounded-[5px] text-[var(--v2-fg-secondary)] hover:bg-[var(--v2-btn-hover)] hover:text-[var(--v2-fg)]"
          >
            <CircleHelp size={14} />
          </button>
          <button
            type="button"
            data-testid="v2/open-settings"
            title="Preferences (Cmd+,)"
            aria-label="Preferences"
            onClick={openSettings}
            className="inline-flex h-[24px] w-[24px] items-center justify-center rounded-[5px] text-[var(--v2-fg-secondary)] hover:bg-[var(--v2-btn-hover)] hover:text-[var(--v2-fg)]"
          >
            <Settings size={14} />
          </button>
          <button
            type="button"
            data-testid="v2/toggle-theme"
            title={theme === "dark" ? "Switch to Light Mode" : "Switch to Dark Mode"}
            aria-label="Toggle theme"
            onClick={toggleTheme}
            className="inline-flex h-[24px] w-[24px] items-center justify-center rounded-[5px] text-[var(--v2-fg-secondary)] hover:bg-[var(--v2-btn-hover)] hover:text-[var(--v2-fg)]"
          >
            {theme === "dark" ? <Sun size={14} /> : <Moon size={14} />}
          </button>
        </div>
      </header>

      {/* 2. Context-Aware Unified Top Toolbar (Hidden on Home or when toggled off via View -> Toolbar) */}
      {activeTab.kind !== "home" && showToolbar ? (
        <V2Toolbar
          commands={commands}
          onGoHome={() => setActiveId("home")}
          onOpenSessions={openNewSession}
        />
      ) : null}

      {/* 3. Main Content Surface (Lazy-loaded Native v2 Views) */}
      <div className="min-h-0 flex-1 overflow-hidden">
        <V2ViewErrorBoundary resetKey={activeTab.id} viewName={activeTab.title} onGoHome={() => setActiveId("home")}>
          <Suspense fallback={<ViewLoadingFallback />}>
            {activeTab.kind === "home" ? (
              <V2Home
                onNewSession={openSession}
                onOpenSessionManager={() => setSessionManagerOpen(true)}
                refreshSignal={sessionRefresh}
              />
            ) : (() => {
              const plugin = getComparator(activeTab.kind);
              if (!plugin) {
                return (
                  <div className="flex h-full items-center justify-center text-[12px] text-[var(--v2-fg-muted)]">
                    Unknown comparator plugin: {activeTab.kind}
                  </div>
                );
              }
              const ActiveView = plugin.component;
              return (
                <ActiveView
                  key={activeTab.id}
                  leftPath={activeTab.leftPath}
                  rightPath={activeTab.rightPath}
                  onPathsChange={updateActivePaths}
                  onDirtyChange={updateActiveDirty}
                  onOpenSession={openSession}
                  onRegisterCommands={setCommands}
                  onStatusChange={setStatus}
                />
              );
            })()}
          </Suspense>
        </V2ViewErrorBoundary>
      </div>

      {/* 4. Clean 3-Zone Status Bar */}
      <footer
        data-testid="v2/statusbar"
        className="flex h-[24px] shrink-0 items-center justify-between border-t border-[var(--v2-border)] bg-[var(--v2-chrome)] px-3 text-[11px] text-[var(--v2-fg-muted)]"
      >
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

      {/* Lazy-Loaded Modals & Command Palette */}
      <Suspense fallback={null}>
        {newSessionOpen ? (
          <V2NewSessionModal
            open={newSessionOpen}
            onClose={() => setNewSessionOpen(false)}
            onCreate={openSession}
          />
        ) : null}
        {sessionManagerOpen ? (
          <V2SessionManagerModal
            open={sessionManagerOpen}
            onClose={() => setSessionManagerOpen(false)}
            onOpenSession={openSession}
            onSessionsChanged={() => setSessionRefresh((n) => n + 1)}
          />
        ) : null}
        {pendingCloseTabId !== null ? (
          <V2Modal
            open={true}
            title="Unsaved Changes"
            subtitle="This comparison tab has unsaved edits."
            widthClass="max-w-sm"
            onClose={() => setPendingCloseTabId(null)}
            footer={
              <>
                <button
                  type="button"
                  onClick={() => setPendingCloseTabId(null)}
                  className="v2-btn"
                >
                  Cancel
                </button>
                <button
                  type="button"
                  onClick={() => {
                    const id = pendingCloseTabId;
                    setPendingCloseTabId(null);
                    closeTab(id);
                  }}
                  className="v2-btn v2-btn-primary"
                >
                  Discard & Close
                </button>
              </>
            }
          >
            <p className="text-[12px] text-[var(--v2-fg-secondary)]">
              Closing this tab will discard unsaved text edits. Save the modified file side first if you want to keep your changes.
            </p>
          </V2Modal>
        ) : null}
        {settingsOpen ? (
          <V2SettingsModal
            open={settingsOpen}
            theme={theme}
            onThemeChange={setTheme}
            onClose={() => setSettingsOpen(false)}
          />
        ) : null}
        {fileFormatsOpen ? (
          <V2FileFormatsModal
            open={fileFormatsOpen}
            onClose={() => setFileFormatsOpen(false)}
            onSaved={(msg) => showToast(msg, 2200)}
          />
        ) : null}
        {profilesOpen ? (
          <V2ProfilesModal
            open={profilesOpen}
            onClose={() => setProfilesOpen(false)}
            onSaved={(msg) => showToast(msg, 2200)}
          />
        ) : null}
        {exportOpen ? (
          <V2ReportExportModal
            open={exportOpen}
            tab={activeTab}
            onClose={() => setExportOpen(false)}
          />
        ) : null}
        {paletteOpen ? (
          <V2CommandPalette
            open={paletteOpen}
            commands={paletteCommands}
            onClose={() => setPaletteOpen(false)}
          />
        ) : null}
        {shortcutsOpen ? (
          <V2ShortcutHelpModal
            open={shortcutsOpen}
            onClose={() => setShortcutsOpen(false)}
          />
        ) : null}
      </Suspense>
    </div>
  );
}
