import React, { lazy } from "react";
import {
  Binary,
  FileText,
  Folder,
  GitMerge,
  Image,
  RefreshCw,
  Table2,
} from "lucide-react";
import { registerComparator } from "./registry";
import type { ComparatorViewProps } from "./registry";
import { structuredDiffPlugin } from "./structuredDiffPlugin";
import { tomlXmlPlugin } from "./tomlXmlPlugin";

const V2TextCompare = lazy(() => import("../views/V2TextCompare"));
const V2FolderCompare = lazy(() => import("../views/V2FolderCompare"));
const V2TableCompare = lazy(() => import("../views/V2TableCompare"));
const V2HexCompare = lazy(() => import("../views/V2HexCompare"));
const V2MergeCompare = lazy(() => import("../views/V2MergeCompare"));
const V2FolderMerge = lazy(() => import("../views/V2FolderMerge"));
const V2SyncCompare = lazy(() => import("../views/V2SyncCompare"));
const V2MediaCompare = lazy(() => import("../views/V2MediaCompare"));

const TextAdapter: React.FC<ComparatorViewProps> = ({
  leftPath,
  rightPath,
  paneOrientation,
  onPathsChange,
  onDirtyChange,
  onRegisterCommands,
  onStatusChange,
}) => (
  <V2TextCompare
    leftPath={leftPath}
    rightPath={rightPath}
    paneOrientation={paneOrientation}
    onPathsChange={onPathsChange}
    onDirtyChange={onDirtyChange ?? (() => undefined)}
    onRegisterCommands={onRegisterCommands}
    onStatusChange={onStatusChange}
  />
);

const FolderAdapter: React.FC<ComparatorViewProps> = ({
  leftPath,
  rightPath,
  onPathsChange,
  onOpenSession,
  onRegisterCommands,
  onStatusChange,
}) => (
  <V2FolderCompare
    leftPath={leftPath}
    rightPath={rightPath}
    onPathsChange={onPathsChange}
    onOpenTextPair={(l, r, title) => onOpenSession?.("text", l, r, title)}
    onRegisterCommands={onRegisterCommands}
    onStatusChange={onStatusChange}
  />
);

const FolderMergeAdapter: React.FC<ComparatorViewProps> = ({
  leftPath,
  rightPath,
  onPathsChange,
  onOpenSession,
  onRegisterCommands,
  onStatusChange,
}) => (
  <V2FolderMerge
    leftPath={leftPath}
    rightPath={rightPath}
    onPathsChange={onPathsChange}
    onOpenTextMerge={(l, r, title) => onOpenSession?.("merge", l, r, title)}
    onRegisterCommands={onRegisterCommands}
    onStatusChange={onStatusChange}
  />
);

let initialized = false;

export function registerBuiltinComparators(): void {
  if (initialized) return;
  initialized = true;

  registerComparator({
    id: "text",
    title: "Text Compare",
    subtitle: "Side-by-side line and word diff",
    paletteLabel: "New Text Compare",
    shortcut: "⌘1",
    accent: "#2563eb",
    icon: FileText,
    keywords: ["text", "txt", "code"],
    component: TextAdapter,
  });

  registerComparator({
    id: "folder",
    title: "Folder Compare",
    subtitle: "Directories & archives",
    paletteLabel: "New Folder Compare",
    shortcut: "⌘2",
    accent: "#d97706",
    icon: Folder,
    keywords: ["folder"],
    component: FolderAdapter,
  });

  registerComparator({
    id: "table",
    title: "Table Compare",
    subtitle: "CSV and structured data grids",
    paletteLabel: "New Table Compare",
    shortcut: "⌘4",
    accent: "#059669",
    icon: Table2,
    extensions: [".csv", ".tsv"],
    keywords: ["table", "csv"],
    component: V2TableCompare,
  });

  registerComparator({
    id: "hex",
    title: "Hex Compare",
    subtitle: "Binary bytes and ASCII offsets",
    paletteLabel: "New Hex Compare",
    shortcut: "⌘3",
    accent: "#7c3aed",
    icon: Binary,
    extensions: [".bin", ".wasm", ".exe", ".dylib"],
    keywords: ["hex", "bin"],
    component: V2HexCompare,
  });

  registerComparator({
    id: "merge",
    title: "3-Way Text Merge",
    subtitle: "Resolve line conflicts with base",
    paletteLabel: "New 3-Way Text Merge",
    shortcut: "⌘7",
    accent: "#dc2626",
    icon: GitMerge,
    keywords: ["merge"],
    component: V2MergeCompare,
  });

  registerComparator({
    id: "folderMerge",
    title: "3-Way Folder Merge",
    subtitle: "Merge directory trees over base",
    paletteLabel: "New 3-Way Folder Merge",
    accent: "#ea580c",
    icon: Folder,
    keywords: ["folder merge"],
    component: FolderMergeAdapter,
  });

  registerComparator({
    id: "sync",
    title: "Folder Sync",
    subtitle: "Mirror and update directory trees",
    paletteLabel: "New Folder Sync",
    shortcut: "⌘6",
    accent: "#0891b2",
    icon: RefreshCw,
    keywords: ["sync"],
    component: V2SyncCompare,
  });

  registerComparator({
    id: "media",
    title: "Media Compare",
    subtitle: "Images, heatmaps, and audio",
    paletteLabel: "New Media Compare",
    shortcut: "⌘5",
    accent: "#db2777",
    icon: Image,
    extensions: [".png", ".jpg", ".jpeg", ".wav", ".mp3"],
    keywords: ["media", "image", "audio"],
    component: V2MediaCompare,
  });

  // Phase 2 Litmus-Test Plugin (#9)
  registerComparator(structuredDiffPlugin);
  // Phase 6 External SDK Sample Plugin (#10)
  registerComparator(tomlXmlPlugin);
}
