// Job de equivalência do CI (T070; FR-031, SC-005; specs/003-editor-local/research.md R6).
// O `typst` CLI 0.15.1 entra só nesse job, do release oficial com SHA-256 fixado, nos três
// sistemas; o job `verify` (e portanto o app e a suíte normal) continua sem `typst` (FR-010).
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { parse } from "yaml";

const root = fileURLToPath(new URL("..", import.meta.url));
const read = (...p: string[]) => readFileSync(join(root, ...p), "utf8");
const ci = () => parse(read(".github", "workflows", "ci.yml"));

interface Step {
  name?: string;
  run?: string;
  uses?: string;
  env?: Record<string, string>;
}

const embedded = () => {
  const lock = read("src-tauri", "Cargo.lock");
  const m = lock.match(/name = "typst"\nversion = "([^"]+)"/);
  if (!m) throw new Error("typst ausente do Cargo.lock");
  return m[1];
};

describe("job equivalence", () => {
  it("existe e roda nos três sistemas", () => {
    const job = ci().jobs.equivalence;
    expect(job).toBeDefined();
    const oses = job.strategy.matrix.include.map((i: { os: string }) => i.os).sort();
    expect(oses).toEqual(["macos-latest", "ubuntu-22.04", "windows-latest"]);
    expect(job.strategy["fail-fast"]).toBe(false);
  });

  it("instala o typst da versão embutida, do release oficial, com SHA-256 fixado", () => {
    const job = ci().jobs.equivalence;
    expect(job.env.TYPST_VERSION).toBe(embedded());
    for (const entry of job.strategy.matrix.include) {
      expect(entry.asset).toMatch(/^typst-[\w-]+\.(tar\.xz|zip)$/);
      expect(entry.sha256).toMatch(/^[0-9a-f]{64}$/);
    }
    const install = (job.steps as Step[]).find((s) => s.run?.includes("releases/download"));
    expect(install?.run).toContain("https://github.com/typst/typst/releases/download/v$TYPST_VERSION/");
    expect(install?.run).toMatch(/sha256sum|shasum -a 256/);
    expect(install?.run).toContain("PRELO_TYPST_CLI=");
  });

  it("roda o teste com a feature e exige o CLI", () => {
    const steps = ci().jobs.equivalence.steps as Step[];
    const test = steps.find((s) => s.run?.includes("--features equivalence"));
    expect(test?.run).toContain("--test equivalence");
    expect(test?.env?.PRELO_EQUIVALENCE_REQUIRED).toBe("1");
  });

  it("o job verify não instala o typst", () => {
    const verify = JSON.stringify(ci().jobs.verify);
    expect(verify).not.toContain("typst/releases");
    expect(verify).not.toContain("PRELO_TYPST_CLI");
  });
});
