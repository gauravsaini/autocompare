import type React from "react";
import type { SessionKind, V2StatusInfo, V2ViewCommands } from "../types";

export interface ComparatorViewProps {
  readonly leftPath: string;
  readonly rightPath: string;
  readonly onPathsChange: (left: string, right: string) => void;
  readonly onDirtyChange?: (dirty: boolean) => void;
  readonly onOpenSession?: (kind: SessionKind, left?: string, right?: string, title?: string) => void;
  readonly onRegisterCommands: (cmds: V2ViewCommands) => void;
  readonly onStatusChange: (status: V2StatusInfo) => void;
}

export interface ComparatorPluginDescriptor {
  readonly id: Exclude<SessionKind, "home">;
  readonly title: string;
  readonly subtitle: string;
  readonly paletteLabel?: string;
  readonly shortcut?: string;
  readonly accent: string;
  readonly icon: React.ComponentType<{ size?: number; strokeWidth?: number }>;
  /** Lowercase file extensions (with leading dot, e.g. ".json") matched on drag-drop. */
  readonly extensions?: readonly string[];
  /** Keywords matched when inferring an icon badge for a saved session name. */
  readonly keywords?: readonly string[];
  /** Priority for path matching when multiple plugins match the same extension. */
  readonly priority?: number;
  readonly component: React.ComponentType<ComparatorViewProps>;
}

const registry = new Map<string, ComparatorPluginDescriptor>();

export function registerComparator(plugin: ComparatorPluginDescriptor): void {
  registry.set(plugin.id, plugin);
}

export function getComparator(id: string): ComparatorPluginDescriptor | undefined {
  return registry.get(id);
}

export function listComparators(): ComparatorPluginDescriptor[] {
  return Array.from(registry.values());
}

/**
 * Resolves a saved session type string (case-insensitive) to a registered
 * comparator id, falling back to "text".
 */
export function resolveComparatorId(rawKind: string | undefined | null): SessionKind {
  if (!rawKind) return "text";
  if (registry.has(rawKind)) return rawKind;
  const lower = rawKind.toLowerCase();
  for (const plugin of registry.values()) {
    if (plugin.id.toLowerCase() === lower) return plugin.id;
  }
  return "text";
}

/**
 * Infers the best registered comparator for dropped file/folder paths.
 * Falls back to "folder" for directories/extensionless paths and "text" otherwise.
 */
export function inferComparatorForPaths(paths: readonly string[]): SessionKind {
  const first = (paths[0] ?? "").toLowerCase();
  if (!first) return "text";

  let best: ComparatorPluginDescriptor | null = null;
  for (const plugin of registry.values()) {
    if (!plugin.extensions || plugin.extensions.length === 0) continue;
    if (plugin.extensions.some((ext) => first.endsWith(ext.toLowerCase()))) {
      if (!best || (plugin.priority ?? 0) > (best.priority ?? 0)) {
        best = plugin;
      }
    }
  }
  if (best) return best.id;

  const base = first.split(/[\\/]/).pop() ?? "";
  if (first.endsWith("/") || first.endsWith("\\") || (base !== "" && !base.includes("."))) {
    return "folder";
  }
  return "text";
}

export function resetComparatorRegistryForTests(): void {
  registry.clear();
}
