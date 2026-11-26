export type SyntaxLang = "plain" | "code" | "json" | "yaml" | "xml" | "markdown" | "csv";

export type SyntaxTokenType =
  | "plain"
  | "keyword"
  | "string"
  | "comment"
  | "number"
  | "key"
  | "tag";

export interface SyntaxToken {
  readonly start: number;
  readonly end: number;
  readonly type: SyntaxTokenType;
}

const CODE_EXTS = new Set([
  "ts", "tsx", "js", "jsx", "mjs", "cjs", "rs", "py", "go", "java",
  "c", "cc", "cpp", "h", "hpp", "cs", "swift", "kt", "kts", "rb",
  "php", "sh", "bash", "zsh", "sql", "css", "scss", "toml", "ini",
]);

export function resolveSyntaxLang(formatName: string, filePath?: string): SyntaxLang {
  switch (formatName) {
    case "Plain Text":
      return "plain";
    case "Source Code":
      return "code";
    case "JSON":
      return "json";
    case "YAML":
      return "yaml";
    case "XML / HTML":
      return "xml";
    case "Markdown":
      return "markdown";
    case "CSV / TSV":
      return "csv";
    default:
      break;
  }
  if (!filePath) return "plain";
  const base = filePath.split(/[\\/]/).pop() ?? "";
  const dot = base.lastIndexOf(".");
  if (dot < 0) return "plain";
  const ext = base.slice(dot + 1).toLowerCase();
  if (ext === "json" || ext === "jsonc") return "json";
  if (ext === "yml" || ext === "yaml") return "yaml";
  if (ext === "xml" || ext === "html" || ext === "htm" || ext === "svg") return "xml";
  if (ext === "md" || ext === "markdown") return "markdown";
  if (ext === "csv" || ext === "tsv") return "csv";
  if (CODE_EXTS.has(ext)) return "code";
  return "plain";
}

const CODE_KEYWORDS = new Set([
  "const", "let", "var", "function", "return", "if", "else", "for", "while", "switch",
  "case", "break", "continue", "default", "import", "export", "from", "type", "interface",
  "class", "extends", "implements", "new", "async", "await", "try", "catch", "finally",
  "throw", "true", "false", "null", "undefined", "void", "readonly", "public", "private",
  "protected", "fn", "pub", "struct", "enum", "impl", "trait", "match", "use", "mod",
  "mut", "ref", "self", "Self", "where", "def", "lambda", "with", "as", "in", "not",
  "and", "or", "None", "True", "False", "package", "func", "defer", "select", "chan",
]);

const MAX_TOKENIZE_CHARS = 2000;

function pushPlain(out: SyntaxToken[], start: number, end: number): void {
  if (end > start) out.push({ start, end, type: "plain" });
}

/**
 * Lightweight, zero-dependency single-line tokenizer for diff panes.
 * Operates on UTF-16 code units (matching JS string indices) and caps
 * work on very long minified lines so 60fps scrolling never degrades.
 */
export function tokenizeLine(line: string, lang: SyntaxLang): SyntaxToken[] {
  if (lang === "plain" || line.length === 0) {
    return line.length === 0 ? [] : [{ start: 0, end: line.length, type: "plain" }];
  }
  if (line.length > MAX_TOKENIZE_CHARS) {
    return [{ start: 0, end: line.length, type: "plain" }];
  }

  const tokens: SyntaxToken[] = [];
  let cursor = 0;

  const scanRegex = (re: RegExp, classify: (m: RegExpExecArray) => SyntaxTokenType): void => {
    re.lastIndex = 0;
    let match: RegExpExecArray | null;
    while ((match = re.exec(line)) !== null) {
      const start = match.index;
      const end = start + match[0].length;
      if (end <= start) {
        re.lastIndex += 1;
        continue;
      }
      pushPlain(tokens, cursor, start);
      tokens.push({ start, end, type: classify(match) });
      cursor = end;
    }
    pushPlain(tokens, cursor, line.length);
  };

  if (lang === "json") {
    scanRegex(
      /("(?:\\.|[^"\\])*")(?=\s*:)|("(?:\\.|[^"\\])*")|\b(true|false|null)\b|-?\b\d+(?:\.\d+)?(?:[eE][+-]?\d+)?\b/g,
      (m) => {
        if (m[1] !== undefined) return "key";
        if (m[2] !== undefined) return "string";
        if (m[3] !== undefined) return "keyword";
        return "number";
      },
    );
    return tokens;
  }

  if (lang === "yaml") {
    scanRegex(
      /(#.*$)|^(\s*[\w.-]+)(?=\s*:)|("(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*')|\b(true|false|null|yes|no)\b|-?\b\d+(?:\.\d+)?\b/g,
      (m) => {
        if (m[1] !== undefined) return "comment";
        if (m[2] !== undefined) return "key";
        if (m[3] !== undefined) return "string";
        if (m[4] !== undefined) return "keyword";
        return "number";
      },
    );
    return tokens;
  }

  if (lang === "xml") {
    scanRegex(
      /(<!--.*?-->)|(<\/?[\w:.-]+|\/>|>)|("(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*')/g,
      (m) => {
        if (m[1] !== undefined) return "comment";
        if (m[2] !== undefined) return "tag";
        return "string";
      },
    );
    return tokens;
  }

  if (lang === "markdown") {
    if (/^\s*#{1,6}\s/.test(line)) {
      return [{ start: 0, end: line.length, type: "keyword" }];
    }
    scanRegex(/(`[^`]+`)|(\*\*[^*]+\*\*)|(\[[^\]]+\]\([^)]+\))/g, (m) => {
      if (m[1] !== undefined) return "string";
      if (m[2] !== undefined) return "keyword";
      return "tag";
    });
    return tokens;
  }

  if (lang === "csv") {
    scanRegex(/("(?:""|[^"])*")|-?\b\d+(?:\.\d+)?\b/g, (m) => {
      if (m[1] !== undefined) return "string";
      return "number";
    });
    return tokens;
  }

  // Default: "code"
  scanRegex(
    /(\/\/.*$|#.*$|\/\*.*?\*\/)|("(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|`(?:\\.|[^`\\])*`)|\b(0x[0-9a-fA-F]+|\d+(?:\.\d+)?)\b|\b([A-Za-z_]\w*)\b/g,
    (m) => {
      if (m[1] !== undefined) return "comment";
      if (m[2] !== undefined) return "string";
      if (m[3] !== undefined) return "number";
      if (m[4] !== undefined && CODE_KEYWORDS.has(m[4])) return "keyword";
      return "plain";
    },
  );
  return tokens;
}
