// Job de desempenho do CI (T075; SC-001, SC-002; research R13 "Desempenho"): o teste
// src-tauri/tests/perf_preview.rs só compila com a feature `perf` e só faz sentido em release,
// então fica fora do `npm run verify` e roda num job próprio, nos três sistemas.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { parse } from "yaml";
import { STEPS } from "./verify.mjs";

const root = fileURLToPath(new URL("..", import.meta.url));
const read = (...p: string[]) => readFileSync(join(root, ...p), "utf8");
const ci = () => parse(read(".github", "workflows", "ci.yml"));

interface Step {
  run?: string;
  env?: Record<string, string>;
}

describe("job perf", () => {
  it("roda nos três sistemas sem parar no primeiro que falhar", () => {
    const job = ci().jobs.perf;
    expect(job).toBeDefined();
    expect([...job.strategy.matrix.os].sort()).toEqual(["macos-latest", "ubuntu-22.04", "windows-latest"]);
    expect(job.strategy["fail-fast"]).toBe(false);
  });

  it("roda o teste em release, com a feature, e limite de 250 ms", () => {
    const steps = ci().jobs.perf.steps as Step[];
    const test = steps.find((s) => s.run?.includes("--features perf"));
    expect(test?.run).toContain("--release");
    expect(test?.run).toContain("--test perf_preview");
    expect(test?.env?.PRELO_PERF_LIMIT_MS).toBe("250");
  });

  it("não barra a release (tag)", () => {
    expect(String(ci().jobs.perf.if)).toContain("refs/tags/");
  });

  it("o teste só compila com a feature e o limite padrão é 250 ms", () => {
    const test = read("src-tauri", "tests", "perf_preview.rs");
    expect(test).toContain('#![cfg(feature = "perf")]');
    expect(test).toMatch(/const CI_LIMIT_MS: u64 = 250;/);
    expect(read("src-tauri", "Cargo.toml")).toMatch(/^perf = \[\]$/m);
  });

  it("o verify não liga a feature perf", () => {
    for (const step of STEPS) expect(step.cmd).not.toContain("perf");
  });
});
