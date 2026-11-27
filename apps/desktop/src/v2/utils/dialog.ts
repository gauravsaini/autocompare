/**
 * Thin wrapper over @tauri-apps/plugin-dialog for AutoCompare v2.
 * Returns the selected absolute path string, or null when cancelled or
 * running outside the native Tauri desktop shell.
 */
export async function browsePath(directory: boolean): Promise<string | null> {
  try {
    const mod = await import("@tauri-apps/plugin-dialog");
    const picked = await mod.open({ multiple: false, directory });
    return typeof picked === "string" && picked.length > 0 ? picked : null;
  } catch {
    return null;
  }
}
