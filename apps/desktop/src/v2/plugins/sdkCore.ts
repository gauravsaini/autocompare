/**
 * Pure, runtime-agnostic External Plugin SDK primitives:
 * manifest validation and capability-enforced IPC sandboxing.
 *
 * Kept free of React/Tauri imports so it can be imported directly in Node tests
 * and headless plugin tooling.
 */

export type PluginCapability = "read_only" | "write_files" | "external_converter";

export interface ExternalPluginManifest {
  readonly id: string;
  readonly name: string;
  readonly version: string;
  readonly description: string;
  readonly extensions: readonly string[];
  readonly capabilities: readonly PluginCapability[];
  readonly accent?: string;
  readonly priority?: number;
  readonly keywords?: readonly string[];
}

export interface PluginCapabilitySandbox {
  readonly pluginId: string;
  readonly capabilities: ReadonlySet<PluginCapability>;
  readonly canWriteFiles: boolean;
  invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T>;
}

const MUTATING_CMD_PREFIXES = [
  "save_",
  "write_",
  "delete_",
  "remove_",
  "apply_",
  "copy_",
  "sync_apply",
  "fs_delete",
  "fs_copy",
  "fs_write",
  "merge_write",
] as const;

const SLUG_RE = /^[a-z][a-z0-9_-]{1,31}$/;

export function isMutatingCommand(cmd: string): boolean {
  const lower = cmd.trim().toLowerCase();
  return MUTATING_CMD_PREFIXES.some((prefix) => lower.startsWith(prefix));
}

export function validatePluginManifest(
  manifest: ExternalPluginManifest,
): { ok: true } | { ok: false; errors: string[] } {
  const errors: string[] = [];
  if (!manifest.id || !SLUG_RE.test(manifest.id) || manifest.id === "home") {
    errors.push(
      `Invalid plugin id "${manifest.id}": must match /^[a-z][a-z0-9_-]{1,31}$/ and cannot be "home".`,
    );
  }
  if (!manifest.name || manifest.name.trim().length < 2) {
    errors.push("Plugin name must be at least 2 characters.");
  }
  if (!manifest.version || !/^\d+\.\d+\.\d+/.test(manifest.version)) {
    errors.push(`Invalid plugin version "${manifest.version}": expected semver (e.g. 1.0.0).`);
  }
  if (!Array.isArray(manifest.extensions) || manifest.extensions.length === 0) {
    errors.push("Plugin must declare at least one file extension (e.g. '.toml').");
  } else {
    for (const ext of manifest.extensions) {
      if (!ext.startsWith(".") || ext.length < 2) {
        errors.push(`Invalid extension "${ext}": must start with "." (e.g. ".toml").`);
      }
    }
  }
  if (!Array.isArray(manifest.capabilities) || manifest.capabilities.length === 0) {
    errors.push("Plugin must declare explicit capabilities (e.g. ['read_only']).");
  }
  return errors.length === 0 ? { ok: true } : { ok: false, errors };
}

export function createPluginSandbox(
  manifest: ExternalPluginManifest,
  invoker: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>,
): PluginCapabilitySandbox {
  const capSet = new Set<PluginCapability>(manifest.capabilities);
  const canWriteFiles = capSet.has("write_files");
  return {
    pluginId: manifest.id,
    capabilities: capSet,
    canWriteFiles,
    async invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
      if (isMutatingCommand(cmd) && !canWriteFiles) {
        throw new Error(
          `Plugin "${manifest.id}" denied capability "write_files" for command "${cmd}".`,
        );
      }
      return invoker<T>(cmd, args);
    },
  };
}
