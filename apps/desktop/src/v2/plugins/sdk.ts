import type React from "react";
import { Puzzle } from "lucide-react";
import { invoke } from "../../ipc/client";
import {
  registerComparator,
  type ComparatorPluginDescriptor,
  type ComparatorViewProps,
} from "./registry";
import {
  createPluginSandbox as createSandboxWithInvoker,
  isMutatingCommand,
  validatePluginManifest,
  type ExternalPluginManifest,
  type PluginCapability,
  type PluginCapabilitySandbox,
} from "./sdkCore";

export {
  isMutatingCommand,
  validatePluginManifest,
  type ExternalPluginManifest,
  type PluginCapability,
  type PluginCapabilitySandbox,
};

const externalManifests = new Map<string, ExternalPluginManifest>();

export function createPluginSandbox(
  manifest: ExternalPluginManifest,
  invoker: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T> = invoke,
): PluginCapabilitySandbox {
  return createSandboxWithInvoker(manifest, invoker);
}

export function defineExternalComparatorPlugin(
  manifest: ExternalPluginManifest,
  component: React.ComponentType<ComparatorViewProps>,
  icon: React.ComponentType<{ size?: number; strokeWidth?: number }> = Puzzle,
): ComparatorPluginDescriptor {
  const check = validatePluginManifest(manifest);
  if (!check.ok) {
    throw new Error(`Invalid external plugin manifest: ${check.errors.join(" ")}`);
  }
  return {
    id: manifest.id as ComparatorPluginDescriptor["id"],
    title: manifest.name,
    subtitle: manifest.description,
    paletteLabel: `New ${manifest.name}`,
    accent: manifest.accent ?? "#0d9488",
    icon,
    extensions: manifest.extensions,
    keywords: manifest.keywords ?? [manifest.id],
    priority: manifest.priority ?? 10,
    component,
  };
}

export function registerExternalPlugin(
  manifest: ExternalPluginManifest,
  component: React.ComponentType<ComparatorViewProps>,
  icon?: React.ComponentType<{ size?: number; strokeWidth?: number }>,
): ComparatorPluginDescriptor {
  const descriptor = defineExternalComparatorPlugin(manifest, component, icon);
  externalManifests.set(manifest.id, manifest);
  registerComparator(descriptor);
  return descriptor;
}

export function listExternalPlugins(): ExternalPluginManifest[] {
  return Array.from(externalManifests.values());
}

export function resetExternalPluginsForTests(): void {
  externalManifests.clear();
}
