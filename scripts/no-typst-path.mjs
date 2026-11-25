// SC-006 (T031): PATH sem as pastas que têm um executável `typst`, para o autoteste e o e2e
// provarem que prévia e diagnósticos vêm do compilador embutido.
import { existsSync } from "node:fs";
import { delimiter, join } from "node:path";

const NAMES = process.platform === "win32" ? ["typst.exe", "typst.cmd", "typst.bat"] : ["typst"];

/** @param {string | undefined} path */
export function pathWithoutTypst(path) {
  if (!path) return "";
  return path
    .split(delimiter)
    .filter((dir) => !NAMES.some((n) => dir && existsSync(join(dir, n))))
    .join(delimiter);
}
