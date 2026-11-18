import { describe, expect, it } from "vitest";
import {
  exitCode,
  findUnjustifiedSkips,
  runSteps,
  summarize,
  type StepResult,
} from "./verify.mjs";

describe("runSteps", () => {
  it("roda todas as etapas mesmo depois de uma falha", async () => {
    const ran: string[] = [];
    const results = await runSteps(
      [
        { name: "a", cmd: "a" },
        { name: "b", cmd: "b" },
        { name: "c", cmd: "c" },
      ],
      async (step) => {
        ran.push(step.name);
        return step.name === "b" ? 1 : 0;
      },
    );
    expect(ran).toEqual(["a", "b", "c"]);
    expect(results.map((r) => r.ok)).toEqual([true, false, true]);
  });

  it("conta como falha a etapa cujo comando não existe ou lança erro", async () => {
    const results = await runSteps([{ name: "ausente", cmd: "nao-existe" }], async () => {
      throw new Error("spawn nao-existe ENOENT");
    });
    expect(results[0]?.ok).toBe(false);
    expect(results[0]?.error).toContain("ENOENT");
  });
});

describe("exitCode e summarize", () => {
  const ok: StepResult = { name: "fmt", ok: true };
  const bad: StepResult = { name: "clippy", ok: false };

  it("sai com 0 só quando tudo passa", () => {
    expect(exitCode([ok])).toBe(0);
    expect(exitCode([ok, bad])).not.toBe(0);
  });

  it("lista as etapas que falharam no resumo", () => {
    const text = summarize([ok, bad]);
    expect(text).toContain("clippy");
    expect(text).toMatch(/falh/i);
  });

  it("lista vazia de etapas é falha (nada foi verificado)", () => {
    expect(exitCode([])).not.toBe(0);
  });
});

describe("findUnjustifiedSkips", () => {
  it("acha #[ignore], .skip e .only sem justificativa", () => {
    const rust = "#[test]\n#[ignore]\nfn t() {}\n";
    const ts = 'it.skip("x", () => {});\ndescribe.only("y", () => {});\n';
    expect(findUnjustifiedSkips("a.rs", rust)).toEqual([{ file: "a.rs", line: 2 }]);
    expect(findUnjustifiedSkips("b.test.ts", ts)).toEqual([
      { file: "b.test.ts", line: 1 },
      { file: "b.test.ts", line: 2 },
    ]);
  });

  it("aceita quando há // justificativa: na mesma linha ou na anterior", () => {
    const rust = "// justificativa: depende de rede, issue #99\n#[ignore]\nfn t() {}\n";
    const ts = 'it.skip("x", () => {}); // justificativa: flaky upstream #98\n';
    expect(findUnjustifiedSkips("a.rs", rust)).toEqual([]);
    expect(findUnjustifiedSkips("b.test.ts", ts)).toEqual([]);
  });

  it("não confunde palavras parecidas", () => {
    const ts = "const skipped = items.skipWhile(x);\nconst only = 1;\n";
    expect(findUnjustifiedSkips("c.ts", ts)).toEqual([]);
  });
});
