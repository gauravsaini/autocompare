import { lazy } from "react";
import { Braces } from "lucide-react";
import type { ComparatorPluginDescriptor } from "./registry";

const V2StructuredCompare = lazy(() => import("../views/V2StructuredCompare"));

/**
 * Phase 2 Litmus-Test Plugin (#9): JSON / YAML Structural AST Diff.
 *
 * Registered purely through the Comparator Plugin Registry without editing
 * V2App.tsx, V2Home.tsx, V2NewSessionModal.tsx, or useTabs.ts.
 */
export const structuredDiffPlugin: ComparatorPluginDescriptor = {
  id: "structured",
  title: "JSON / YAML Diff",
  subtitle: "Key-order-independent AST diff",
  paletteLabel: "New JSON / YAML Structural Diff",
  accent: "#0d9488",
  icon: Braces,
  extensions: [".json", ".jsonc", ".yaml", ".yml"],
  keywords: ["json", "yaml", "yml", "structured", "ast"],
  priority: 10,
  component: V2StructuredCompare,
};
