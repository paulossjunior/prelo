// SC-006: o autoteste e o e2e rodam o app sem `typst` no PATH (T031).
import { chmodSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { delimiter, join } from "node:path";
import { describe, expect, it } from "vitest";
import { pathWithoutTypst } from "./no-typst-path.mjs";

describe("pathWithoutTypst", () => {
  it("tira só as pastas que têm o executável typst", () => {
    const withTypst = mkdtempSync(join(tmpdir(), "prelo-typst-"));
    const exe = join(withTypst, process.platform === "win32" ? "typst.exe" : "typst");
    writeFileSync(exe, "");
    chmodSync(exe, 0o755);
    const other = mkdtempSync(join(tmpdir(), "prelo-other-"));
    const out = pathWithoutTypst([other, withTypst, other].join(delimiter));
    expect(out.split(delimiter)).toEqual([other, other]);
  });

  it("PATH vazio ou ausente continua vazio", () => {
    expect(pathWithoutTypst("")).toBe("");
    expect(pathWithoutTypst(undefined)).toBe("");
  });
});
