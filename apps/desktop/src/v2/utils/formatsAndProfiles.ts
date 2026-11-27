/**
 * Pure data model & persistence for Beyond-Compare-parity File Formats
 * (backed by Phase 3 `autocompare.formats.toml`) and VFS Profiles
 * (backed by Phase 3 `VfsRegistry`).
 */

export type ConversionKind = "none" | "canonical_json" | "normalize_eol" | "external";

export interface FileFormatEntry {
  readonly id: string;
  readonly name: string;
  readonly mask: string;
  readonly enabled: boolean;
  readonly conversion: ConversionKind;
  readonly externalProgram?: string;
  readonly externalArgs?: string;
  readonly disableEditing?: boolean;
}

export interface VfsProfileEntry {
  readonly id: string;
  readonly name: string;
  readonly anonymousEmail: string;
  readonly sshPrivateKeyPath: string;
  readonly sslClientCertPath: string;
  readonly asciiTypes: readonly string[];
  readonly timeoutSecs: number;
  readonly transferMode: "auto" | "binary" | "ascii";
  readonly preserveTimestamps: boolean;
}

const FORMATS_STORAGE_KEY = "autocompare.v2.fileFormats";
const PROFILES_STORAGE_KEY = "autocompare.v2.vfsProfiles";

export const DEFAULT_FILE_FORMATS: readonly FileFormatEntry[] = [
  { id: "bash", name: "Bash Scripts", mask: "*.sh;*.bash;*.zsh", enabled: true, conversion: "normalize_eol" },
  { id: "c-cpp", name: "C, C++, C#, Rust", mask: "*.c;*.cc;*.cpp;*.h;*.cs;*.rs", enabled: true, conversion: "none" },
  { id: "csv", name: "Comma Separated", mask: "*.csv;*.tsv", enabled: true, conversion: "none" },
  { id: "json", name: "JSON (Canonical)", mask: "*.json;*.arb", enabled: true, conversion: "canonical_json" },
  { id: "yaml-toml", name: "YAML & TOML", mask: "*.yaml;*.yml;*.toml", enabled: true, conversion: "normalize_eol" },
  { id: "html-xml", name: "HTML / XML", mask: "*.html;*.htm;*.xml;*.xsd;*.xsl", enabled: true, conversion: "none" },
  { id: "js-ts", name: "JavaScript / TypeScript", mask: "*.js;*.jsx;*.ts;*.tsx;*.mjs", enabled: true, conversion: "none" },
  { id: "python", name: "Python Source", mask: "*.py;*.pyi", enabled: true, conversion: "normalize_eol" },
  { id: "sql", name: "SQL", mask: "*.sql;*.dsql", enabled: true, conversion: "none" },
  { id: "media", name: "Media Files", mask: "*.wav;*.mp3;*.flac;*.m4a", enabled: true, conversion: "none", disableEditing: true },
  { id: "picture", name: "Picture Files", mask: "*.png;*.jpg;*.jpeg;*.bmp;*.gif;*.webp", enabled: true, conversion: "none", disableEditing: true },
  { id: "pdf", name: "PDF Documents", mask: "*.pdf", enabled: true, conversion: "external", externalProgram: "pdftotext", externalArgs: "- -", disableEditing: true },
  { id: "everything", name: "Everything Else", mask: "*.*", enabled: true, conversion: "none" },
];

export const DEFAULT_VFS_PROFILES: readonly VfsProfileEntry[] = [
  {
    id: "default",
    name: "<default>",
    anonymousEmail: "user@example.com",
    sshPrivateKeyPath: "",
    sslClientCertPath: "",
    asciiTypes: ["*.c", "*.cc", "*.conf", "*.cpp", "*.css", "*.h", "*.html", "*.json", "*.md", "*.py", "*.rs", "*.sh", "*.toml", "*.ts", "*.txt", "*.xml", "*.yaml"],
    timeoutSecs: 30,
    transferMode: "auto",
    preserveTimestamps: true,
  },
];

export function extractExtensionsFromMask(mask: string): string[] {
  return mask
    .split(/[;,\s]+/)
    .map((part) => part.trim().toLowerCase())
    .filter((part) => part.startsWith("*.") && part.length > 2 && part !== "*.*")
    .map((part) => part.slice(2));
}

/**
 * Generates an `autocompare.formats.toml` document directly compatible with
 * `crates/compare/src/format_converter.rs` (`FormatConverterRegistry::from_toml`).
 */
export function exportFormatsToToml(formats: readonly FileFormatEntry[]): string {
  const blocks: string[] = [
    "# AutoCompare declarative format converters (Phase 3 FormatConverterRegistry)",
  ];
  for (const fmt of formats) {
    if (!fmt.enabled || fmt.conversion === "none") continue;
    const exts = extractExtensionsFromMask(fmt.mask);
    if (exts.length === 0) continue;
    const extList = exts.map((e) => `"${e}"`).join(", ");
    const lines = [
      "",
      "[[format]]",
      `name = "${fmt.name.replace(/"/g, "")}"`,
      `extensions = [${extList}]`,
      `kind = "${fmt.conversion}"`,
    ];
    if (fmt.conversion === "canonical_json") {
      lines.push("sort_keys = true");
    } else if (fmt.conversion === "normalize_eol") {
      lines.push("strip_trailing_whitespace = false");
    } else if (fmt.conversion === "external" && fmt.externalProgram?.trim()) {
      lines.push(`program = "${fmt.externalProgram.trim()}"`);
      const args = (fmt.externalArgs ?? "")
        .split(/\s+/)
        .filter(Boolean)
        .map((a) => `"${a}"`)
        .join(", ");
      lines.push(`args = [${args}]`);
    }
    blocks.push(lines.join("\n"));
  }
  return `${blocks.join("\n")}\n`;
}

export function loadFileFormats(): FileFormatEntry[] {
  try {
    const raw = globalThis.localStorage?.getItem(FORMATS_STORAGE_KEY);
    if (!raw) return [...DEFAULT_FILE_FORMATS];
    const parsed = JSON.parse(raw) as unknown;
    return Array.isArray(parsed) && parsed.length > 0
      ? (parsed as FileFormatEntry[])
      : [...DEFAULT_FILE_FORMATS];
  } catch {
    return [...DEFAULT_FILE_FORMATS];
  }
}

export function saveFileFormats(formats: readonly FileFormatEntry[]): void {
  try {
    globalThis.localStorage?.setItem(FORMATS_STORAGE_KEY, JSON.stringify(formats));
  } catch {
    // Ignore storage quota errors
  }
}

export function loadVfsProfiles(): VfsProfileEntry[] {
  try {
    const raw = globalThis.localStorage?.getItem(PROFILES_STORAGE_KEY);
    if (!raw) return [...DEFAULT_VFS_PROFILES];
    const parsed = JSON.parse(raw) as unknown;
    return Array.isArray(parsed) && parsed.length > 0
      ? (parsed as VfsProfileEntry[])
      : [...DEFAULT_VFS_PROFILES];
  } catch {
    return [...DEFAULT_VFS_PROFILES];
  }
}

export function saveVfsProfiles(profiles: readonly VfsProfileEntry[]): void {
  try {
    globalThis.localStorage?.setItem(PROFILES_STORAGE_KEY, JSON.stringify(profiles));
  } catch {
    // Ignore storage quota errors
  }
}

export function exportSettingsBundleJson(
  formats: readonly FileFormatEntry[],
  profiles: readonly VfsProfileEntry[],
): string {
  return JSON.stringify(
    {
      schema: "autocompare-settings-bundle-v1",
      exportedAt: new Date().toISOString(),
      fileFormats: formats,
      vfsProfiles: profiles,
    },
    null,
    2,
  );
}

export function parseSettingsBundleJson(jsonText: string): {
  fileFormats: FileFormatEntry[];
  vfsProfiles: VfsProfileEntry[];
} | null {
  try {
    const parsed = JSON.parse(jsonText) as Record<string, unknown>;
    if (parsed.schema !== "autocompare-settings-bundle-v1") return null;
    const fileFormats = Array.isArray(parsed.fileFormats)
      ? (parsed.fileFormats as FileFormatEntry[])
      : [...DEFAULT_FILE_FORMATS];
    const vfsProfiles = Array.isArray(parsed.vfsProfiles)
      ? (parsed.vfsProfiles as VfsProfileEntry[])
      : [...DEFAULT_VFS_PROFILES];
    return { fileFormats, vfsProfiles };
  } catch {
    return null;
  }
}
