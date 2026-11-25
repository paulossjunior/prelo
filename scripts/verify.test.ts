import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
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

// Casos adicionados pelo QA (validação da Phase 2): variações de skip/only previstas no
// contrato (contracts/dev-workflow.md), CRLF, falsos positivos e varredura em disco.
describe("findUnjustifiedSkips: variações do contrato", () => {
  it("acha test.only e it.only", () => {
    const ts = 'test.only("a", () => {});\n  it.only("b", () => {});\n';
    expect(findUnjustifiedSkips("d.test.ts", ts)).toEqual([
      { file: "d.test.ts", line: 1 },
      { file: "d.test.ts", line: 2 },
    ]);
  });

  it('acha #[ignore = "motivo"] e #[ignore] indentado', () => {
    const rust = '#[test]\n#[ignore = "lento"]\nfn a() {}\nmod m {\n    #[ignore]\n    fn b() {}\n}\n';
    expect(findUnjustifiedSkips("e.rs", rust)).toEqual([
      { file: "e.rs", line: 2 },
      { file: "e.rs", line: 5 },
    ]);
  });

  it("trata arquivos com CRLF (linha correta, justificativa na linha anterior vale)", () => {
    const ts = 'import x;\r\nit.skip("a", () => {});\r\n';
    const rust = "// justificativa: issue #99\r\n#[ignore]\r\nfn t() {}\r\n";
    expect(findUnjustifiedSkips("f.test.ts", ts)).toEqual([{ file: "f.test.ts", line: 2 }]);
    expect(findUnjustifiedSkips("f.rs", rust)).toEqual([]);
  });

  it("acha .skip/.only encadeados (each, concurrent) e o alias suite do Vitest", () => {
    const ts = [
      'it.skip.each([1])("a", () => {});',
      'test.concurrent.skip("b", () => {});',
      'describe.concurrent.only("c", () => {});',
      'suite.skip("d", () => {});',
    ].join("\n");
    expect(findUnjustifiedSkips("g.test.ts", ts).map((f) => f.line)).toEqual([1, 2, 3, 4]);
  });

  it("acha .skip quando a cadeia quebra linha", () => {
    const ts = 'it\n  .skip("a", () => {});\n';
    expect(findUnjustifiedSkips("h.test.ts", ts)).toHaveLength(1);
  });

  it("não gera falso positivo em comentário, string ou código de produção", () => {
    const ts = [
      "// it.skip(\"exemplo em comentário\")",
      "const s = 'it.only(\"x\")';",
      "list.skip(2).only(1);",
      "const ignore = true;",
    ].join("\n");
    const rust = '// #[ignore]\nlet s = "#[ignore]";\nfn ignore_me() {}\n';
    expect(findUnjustifiedSkips("src/x.ts", ts)).toEqual([]);
    expect(findUnjustifiedSkips("src/x.rs", rust)).toEqual([]);
  });
});

describe("scanSkips em disco", () => {
  it("varre src, src-tauri/src, src-tauri/tests, scripts e e2e", async () => {
    const { mkdtempSync, mkdirSync, writeFileSync, rmSync } = await import("node:fs");
    const { tmpdir } = await import("node:os");
    const { join } = await import("node:path");
    const { scanSkips } = await import("./verify.mjs");
    const root = mkdtempSync(join(tmpdir(), "prelo-scan-"));
    try {
      const put = (rel: string, text: string) => {
        mkdirSync(join(root, rel, ".."), { recursive: true });
        writeFileSync(join(root, rel), text);
      };
      put("src/a.test.ts", 'it.only("a", () => {});\n');
      put("src-tauri/src/b.rs", "#[ignore]\nfn b() {}\n");
      put("src-tauri/tests/c.rs", "#[ignore]\nfn c() {}\n");
      put("scripts/d.test.ts", 'describe.skip("d", () => {});\n');
      put("e2e/e.e2e.ts", 'it.skip("e", () => {});\n');
      put("node_modules/z/z.test.ts", 'it.skip("z", () => {});\n');
      const files = scanSkips(root).map((f) => f.file.replaceAll("\\", "/")).sort();
      expect(files).toEqual([
        "e2e/e.e2e.ts",
        "scripts/d.test.ts",
        "src-tauri/src/b.rs",
        "src-tauri/tests/c.rs",
        "src/a.test.ts",
      ]);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });
});

// Regressão do achado alto do QA: com o repositório acessado por link simbólico, o guarda
// de entrada não reconhecia o script como principal e `verify` saía com 0 sem rodar nada.
describe("isMain", () => {
  it("reconhece o script principal mesmo por caminho com link simbólico", async () => {
    const { mkdtempSync, symlinkSync, writeFileSync, rmSync } = await import("node:fs");
    const { tmpdir } = await import("node:os");
    const { join } = await import("node:path");
    const { pathToFileURL } = await import("node:url");
    const { isMain } = await import("./verify.mjs");
    const real = mkdtempSync(join(tmpdir(), "prelo-real-"));
    const link = `${real}-link`;
    try {
      writeFileSync(join(real, "v.mjs"), "");
      symlinkSync(real, link, "junction");
      const moduleUrl = pathToFileURL(join(real, "v.mjs")).href;
      expect(isMain(moduleUrl, join(link, "v.mjs"))).toBe(true);
      expect(isMain(moduleUrl, join(real, "v.mjs"))).toBe(true);
      expect(isMain(moduleUrl, join(real, "outro.mjs"))).toBe(false);
      expect(isMain(moduleUrl, undefined)).toBe(false);
    } finally {
      rmSync(link, { force: true, recursive: true });
      rmSync(real, { recursive: true, force: true });
    }
  });
});

// Achado médio do QA (US1): código sob a feature self-test nunca era testado nem passava
// pelo clippy dentro do verify.
describe("STEPS", () => {
  it("roda testes e clippy também com a feature self-test", async () => {
    const { STEPS } = await import("./verify.mjs");
    const cmds = STEPS.map((s) => s.cmd);
    expect(cmds.some((c) => c.includes("clippy") && c.includes("--features self-test"))).toBe(true);
    expect(cmds.some((c) => c.includes("cargo test") && c.includes("--features self-test"))).toBe(true);
  });
});

describe("modo rápido (gitflow: push em feature e dev)", () => {
  it("o modo rápido tira só o build ponta a ponta", async () => {
    const { STEPS, stepsFor } = await import("./verify.mjs");
    const quick = stepsFor("quick").map((s) => s.name);
    expect(quick).not.toContain("test:e2e");
    expect(quick).toEqual(STEPS.map((s) => s.name).filter((n) => n !== "test:e2e"));
    expect(quick).toEqual(expect.arrayContaining(["fmt", "clippy", "tsc", "test:rust", "test:ui", "audit"]));
  });

  it("o modo completo mantém todas as etapas", async () => {
    const { STEPS, stepsFor } = await import("./verify.mjs");
    expect(stepsFor("full")).toEqual(STEPS);
  });

  it("lê o modo dos argumentos", async () => {
    const { modeFromArgs } = await import("./verify.mjs");
    expect(modeFromArgs(["node", "verify.mjs", "--quick"])).toBe("quick");
    expect(modeFromArgs(["node", "verify.mjs"])).toBe("full");
  });
});

// Windows no CI do #232: o `cargo test` parava no primeiro binário com falha e escondia as
// falhas dos binários seguintes, uma rodada de CI por falha.
describe("testes Rust mostram todas as falhas", () => {
  it("todo cargo test do verify usa --no-fail-fast", async () => {
    const { STEPS } = await import("./verify.mjs");
    const pkg = JSON.parse(readFileSync("package.json", "utf8"));
    const cmds = STEPS.map((s) => s.cmd.replace("npm run test:rust", pkg.scripts["test:rust"]));
    const cargoTests = cmds.filter((c) => c.includes("cargo test"));
    expect(cargoTests.length).toBeGreaterThanOrEqual(2);
    for (const cmd of cargoTests) expect(cmd).toContain("--no-fail-fast");
  });
});

// O Git no Windows (core.autocrlf) gravava as fixtures em CRLF e os testes viam arquivos
// diferentes dos de macOS e Linux.
describe(".gitattributes", () => {
  it("força LF no checkout de todo arquivo de texto", () => {
    const attrs = readFileSync(".gitattributes", "utf8");
    expect(attrs).toMatch(/^\*\s+text=auto\s+eol=lf\s*$/m);
    for (const ext of ["png", "otf", "ttf", "woff2", "wasm", "pdf", "ico", "icns"]) {
      expect(attrs).toMatch(new RegExp(`^\\*\\.${ext}\\s+binary\\s*$`, "m"));
    }
  });
});
