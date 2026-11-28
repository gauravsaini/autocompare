export const MIN_SIDEBAR_WIDTH = 220;
export const MAX_SIDEBAR_WIDTH = 420;
export const DEFAULT_SIDEBAR_WIDTH = 272;

const HOME_SIDEBAR_STORAGE_KEY = "autocompare.v2.home.sidebar.v1";

export interface HomeSidebarPrefs {
  readonly open: boolean;
  readonly width: number;
}

export function clampSidebarWidth(width: number): number {
  if (!Number.isFinite(width)) return DEFAULT_SIDEBAR_WIDTH;
  return Math.round(Math.min(MAX_SIDEBAR_WIDTH, Math.max(MIN_SIDEBAR_WIDTH, width)));
}

function getDefaultStorage(): Storage | null {
  try {
    return typeof window !== "undefined" ? window.localStorage : null;
  } catch {
    return null;
  }
}

export function loadHomeSidebarPrefs(
  storage: Pick<Storage, "getItem"> | null = getDefaultStorage(),
): HomeSidebarPrefs {
  const fallback: HomeSidebarPrefs = { open: true, width: DEFAULT_SIDEBAR_WIDTH };
  if (!storage) return fallback;
  try {
    const raw = storage.getItem(HOME_SIDEBAR_STORAGE_KEY);
    if (!raw) return fallback;
    const parsed = JSON.parse(raw) as Partial<HomeSidebarPrefs> | null;
    if (!parsed || typeof parsed !== "object") return fallback;
    return {
      open: typeof parsed.open === "boolean" ? parsed.open : true,
      width: typeof parsed.width === "number" ? clampSidebarWidth(parsed.width) : DEFAULT_SIDEBAR_WIDTH,
    };
  } catch {
    return fallback;
  }
}

export function saveHomeSidebarPrefs(
  prefs: HomeSidebarPrefs,
  storage: Pick<Storage, "setItem"> | null = getDefaultStorage(),
): void {
  if (!storage) return;
  try {
    const safe: HomeSidebarPrefs = {
      open: Boolean(prefs.open),
      width: clampSidebarWidth(prefs.width),
    };
    storage.setItem(HOME_SIDEBAR_STORAGE_KEY, JSON.stringify(safe));
  } catch {
    // Ignore quota or private browsing storage failures
  }
}
