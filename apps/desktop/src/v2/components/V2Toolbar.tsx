import React from "react";
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
  RefreshCw,
  Search,
  SlidersHorizontal,
  Sparkles,
  Type,
} from "lucide-react";
import type { V2ViewCommands } from "../types";

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
      data-testid={`v2/toolbar-${label.toLowerCase()}`}
      disabled={disabled}
      onClick={onClick}
      className={`inline-flex h-[38px] min-w-[46px] shrink-0 flex-col items-center justify-center gap-0.5 rounded-lg px-2 text-[10.5px] font-medium transition-colors select-none ${
        active
          ? "bg-[var(--v2-accent-soft)] text-[var(--v2-accent)]"
          : "text-[var(--v2-fg-secondary)] hover:bg-[var(--v2-surface-subtle)] hover:text-[var(--v2-fg)]"
      } disabled:cursor-not-allowed disabled:opacity-35 disabled:hover:bg-transparent`}
    >
      <Icon size={15} strokeWidth={1.8} />
      <span className="leading-none">{label}</span>
    </button>
  );
}

function BarSep(): React.JSX.Element {
  return <span aria-hidden="true" className="mx-1 h-6 w-px shrink-0 bg-[var(--v2-border)]" />;
}

export interface V2ToolbarProps {
  readonly commands: V2ViewCommands;
  readonly onGoHome: () => void;
  readonly onOpenSessions: () => void;
}

export default function V2Toolbar({
  commands,
  onGoHome,
  onOpenSessions,
}: V2ToolbarProps): React.JSX.Element {
  return (
    <div
      role="toolbar"
      data-testid="v2/toolbar"
      aria-label="Comparison actions"
      className="flex h-[48px] shrink-0 items-center gap-0.5 overflow-x-auto border-b border-[var(--v2-border)] bg-[var(--v2-surface)] px-3"
    >
      <TopBarBtn icon={Home} label="Home" onClick={onGoHome} />
      <TopBarBtn icon={FolderOpen} label="Sessions" onClick={onOpenSessions} />
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
  );
}
