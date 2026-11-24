/**
 * Wave-176 real content e2e helper:
 * Sets up temporary copies of File 1.txt and File 2.txt outside the repo (in /tmp),
 * exposes Node fs read capability to the browser mock IPC via Playwright binding,
 * and declares difflib ground truth constants.
 */
import { copyFileSync, existsSync, readFileSync, statSync, unlinkSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import type { Page } from "@playwright/test";

export const GROUND_TRUTH_176 = {
  file1Lines: 170,
  file2Lines: 159,
  totalRows: 176,
  totalHunks: 10,
  equalRows: 147,
  changedRows: 6,
  leftOnlyRows: 17,
  rightOnlyRows: 6,
  hunkMap: [
    { start: 0, end: 2 },
    { start: 4, end: 8 },
    { start: 25, end: 27 },
    { start: 34, end: 37 },
    { start: 39, end: 41 },
    { start: 44, end: 44 },
    { start: 56, end: 56 },
    { start: 111, end: 113 },
    { start: 125, end: 125 },
    { start: 157, end: 161 },
  ],
} as const;

export interface RealContentFixture {
  readonly file1Path: string;
  readonly file2Path: string;
  readonly file1Content: string;
  readonly file2Content: string;
  readonly cleanup: () => void;
}

const DEFAULT_FILE1_PATH = "/Users/ektasaini/Documents/File 1.txt";
const DEFAULT_FILE2_PATH = "/Users/ektasaini/Documents/File 2.txt";

export function prepareRealContentFiles(): RealContentFixture {
  const tmpDir = tmpdir();
  const file1Path = join(tmpDir, `ac-176-file1-${Date.now()}.txt`);
  const file2Path = join(tmpDir, `ac-176-file2-${Date.now()}.txt`);

  let file1Content = "";
  let file2Content = "";

  if (existsSync(DEFAULT_FILE1_PATH) && existsSync(DEFAULT_FILE2_PATH)) {
    copyFileSync(DEFAULT_FILE1_PATH, file1Path);
    copyFileSync(DEFAULT_FILE2_PATH, file2Path);
    file1Content = readFileSync(file1Path, "utf8");
    file2Content = readFileSync(file2Path, "utf8");
  } else {
    throw new Error(`Default ground truth files not found at ${DEFAULT_FILE1_PATH} or ${DEFAULT_FILE2_PATH}`);
  }

  const cleanup = () => {
    try {
      if (existsSync(file1Path)) unlinkSync(file1Path);
    } catch {}
    try {
      if (existsSync(file2Path)) unlinkSync(file2Path);
    } catch {}
  };

  return {
    file1Path,
    file2Path,
    file1Content,
    file2Content,
    cleanup,
  };
}

export async function installRealContentMock(
  page: Page,
  extraFiles?: Record<string, string>,
): Promise<void> {
  // Expose binding to page for on-demand Node fs read
  try {
    await page.exposeFunction("__MOCK_READ_FILE__", async (filePath: string) => {
      try {
        if (existsSync(filePath)) {
          const stat = statSync(filePath);
          const content = readFileSync(filePath, "utf8");
          return {
            content,
            bytes: stat.size,
            mtimeUnix: Math.floor(stat.mtimeMs / 1000),
          };
        }
      } catch {}
      if (extraFiles && typeof extraFiles[filePath] === "string") {
        const content = extraFiles[filePath];
        return {
          content,
          bytes: Buffer.byteLength(content, "utf8"),
          mtimeUnix: Math.floor(Date.now() / 1000),
        };
      }
      return null;
    });
  } catch {
    // exposeFunction fails if already exposed on this page; that's fine
  }

  // Pre-populate window.__MOCK_FILES__ and localStorage in page context
  if (extraFiles) {
    await page.addInitScript((files) => {
      const w = window as unknown as Record<string, unknown>;
      w["__MOCK_FILES__"] = {
        ...((w["__MOCK_FILES__"] as Record<string, string>) || {}),
        ...files,
      };
      try {
        const existing = JSON.parse(localStorage.getItem("autocompare:mockFileStore:v1") || "{}");
        localStorage.setItem(
          "autocompare:mockFileStore:v1",
          JSON.stringify({ ...existing, ...files }),
        );
      } catch {}
    }, extraFiles);
  }
}
