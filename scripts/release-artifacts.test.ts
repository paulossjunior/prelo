// Instaladores, checksums e marca de autoteste da release
// (specs/002-release-pipeline/contracts/release-scripts.md, data-model.md).
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import {
  expectedInstallers,
  formatSums,
  matchInstallers,
  sha256File,
  verifySums,
} from "./release-artifacts.mjs";

const SCRIPT = fileURLToPath(new URL("./release-artifacts.mjs", import.meta.url));
const V = "1.2.3";
const FULL = ["Prelo_1.2.3_universal.dmg", "Prelo_1.2.3_x64-setup.exe", "Prelo_1.2.3_amd64.AppImage", "Prelo_1.2.3_amd64.deb"];
const dirs: string[] = [];

function tempDir() {
  const d = mkdtempSync(join(tmpdir(), "prelo-release-"));
  dirs.push(d);
  return d;
}

function cli(...args: string[]) {
  return spawnSync(process.execPath, [SCRIPT, ...args], { encoding: "utf8" });
}

/** Diretório como o do download-artifact: um subdiretório por sistema. */
function downloads(names: string[]) {
  const dir = tempDir();
  names.forEach((name, i) => {
    const sub = join(dir, `installers-${i}`);
    mkdirSync(sub, { recursive: true });
    writeFileSync(join(sub, name), `conteúdo de ${name}`);
  });
  return dir;
}

afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true });
});

describe("expectedInstallers", () => {
  it("lista os quatro instaladores da versão (dmg universal, exe NSIS x64, AppImage e deb amd64)", () => {
    expect(expectedInstallers(V)).toEqual(FULL);
  });
});

describe("matchInstallers", () => {
  it("conjunto completo: nada falta nem sobra", () => {
    expect(matchInstallers([...FULL].reverse(), V)).toEqual({ found: FULL, missing: [], extra: [] });
  });

  it("sem o .exe: acusa o que falta", () => {
    const r = matchInstallers(FULL.filter((f) => !f.endsWith(".exe")), V);
    expect(r.missing).toEqual(["Prelo_1.2.3_x64-setup.exe"]);
    expect(r.extra).toEqual([]);
  });

  it(".msi ou .rpm a mais: acusa o que sobra", () => {
    const r = matchInstallers([...FULL, "Prelo_1.2.3_x64_en-US.msi", "Prelo-1.2.3-1.x86_64.rpm"], V);
    expect(r.missing).toEqual([]);
    expect(r.extra).toEqual(["Prelo-1.2.3-1.x86_64.rpm", "Prelo_1.2.3_x64_en-US.msi"]);
  });

  it("instalador de outra versão conta como sobra e o da versão certa como falta", () => {
    const files = FULL.map((f) => (f.endsWith(".deb") ? "Prelo_1.2.2_amd64.deb" : f));
    const r = matchInstallers(files, V);
    expect(r.missing).toEqual(["Prelo_1.2.3_amd64.deb"]);
    expect(r.extra).toEqual(["Prelo_1.2.2_amd64.deb"]);
  });
});

describe("CLI collect", () => {
  it("copia os quatro instaladores para a saída e sai com 0", () => {
    const out = join(tempDir(), "saida");
    const r = cli("collect", downloads(FULL), V, out);
    expect(r.status, r.stderr).toBe(0);
    expect(readdirSync(out).sort()).toEqual([...FULL].sort());
  });

  it("sai com 1 e nomeia o instalador que falta", () => {
    const out = join(tempDir(), "saida");
    const r = cli("collect", downloads(FULL.slice(1)), V, out);
    expect(r.status).toBe(1);
    expect(r.stderr).toContain("release: falta instalador Prelo_1.2.3_universal.dmg");
    expect(existsSync(out)).toBe(false);
  });

  it("sai com 1 e nomeia o instalador que sobra", () => {
    const r = cli("collect", downloads([...FULL, "Prelo_1.2.3_x64_en-US.msi"]), V, join(tempDir(), "saida"));
    expect(r.status).toBe(1);
    expect(r.stderr).toContain("release: instalador inesperado Prelo_1.2.3_x64_en-US.msi");
  });

  it("sai com 1 se dois sistemas entregarem o mesmo nome", () => {
    const r = cli("collect", downloads([...FULL, FULL[0]!]), V, join(tempDir(), "saida"));
    expect(r.status).toBe(1);
    expect(r.stderr).toContain("release: instalador repetido Prelo_1.2.3_universal.dmg");
  });
});

// ---------------------------------------------------------------------------------------
// US3: SHA256SUMS.txt no formato do sha256sum (FR-009, data-model "Arquivo de checksums").

const ABC = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
const EMPTY = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

describe("sha256File", () => {
  it("calcula o hex minúsculo de vetores conhecidos", async () => {
    const d = tempDir();
    writeFileSync(join(d, "abc"), "abc");
    writeFileSync(join(d, "vazio"), "");
    expect(await sha256File(join(d, "abc"))).toBe(ABC);
    expect(await sha256File(join(d, "vazio"))).toBe(EMPTY);
  });
});

describe("formatSums", () => {
  it("uma linha <hex>  <nome> por arquivo, em ordem alfabética, com \\n final", () => {
    expect(
      formatSums([
        { file: "b.deb", sha256: EMPTY },
        { file: "a.dmg", sha256: ABC },
      ]),
    ).toBe(`${ABC}  a.dmg\n${EMPTY}  b.deb\n`);
  });
});

describe("verifySums", () => {
  function signed() {
    const d = tempDir();
    writeFileSync(join(d, "a.dmg"), "abc");
    writeFileSync(join(d, "b.deb"), "");
    return { d, text: `${ABC}  a.dmg\n${EMPTY}  b.deb\n` };
  }

  it("aceita arquivos íntegros", async () => {
    const { d, text } = signed();
    expect(await verifySums(text, d)).toEqual([]);
  });

  it("acusa arquivo adulterado depois da geração", async () => {
    const { d, text } = signed();
    writeFileSync(join(d, "a.dmg"), "abd");
    expect(await verifySums(text, d)).toEqual(["a.dmg"]);
  });

  it("acusa arquivo listado que não existe", async () => {
    const { d, text } = signed();
    rmSync(join(d, "b.deb"));
    expect(await verifySums(text, d)).toEqual(["b.deb"]);
  });

  it("não segue nome com caminho para fora do diretório (segurança)", async () => {
    const { d } = signed();
    expect(await verifySums(`${ABC}  ../a.dmg\n`, d)).toEqual(["../a.dmg"]);
  });
});

describe("CLI sums", () => {
  it("grava SHA256SUMS.txt com uma linha por instalador, conferível por verifySums", async () => {
    const d = tempDir();
    for (const f of FULL) writeFileSync(join(d, f), `conteúdo de ${f}`);
    const r = cli("sums", d);
    expect(r.status, r.stderr).toBe(0);
    const text = readFileSync(join(d, "SHA256SUMS.txt"), "utf8");
    const lines = text.trimEnd().split("\n");
    expect(lines.map((l) => l.split("  ")[1])).toEqual([...FULL].sort());
    for (const l of lines) expect(l).toMatch(/^[0-9a-f]{64}  \S+$/);
    expect(await verifySums(text, d)).toEqual([]);
  });

  it("não inclui o próprio SHA256SUMS.txt ao regerar", () => {
    const d = tempDir();
    writeFileSync(join(d, FULL[0]!), "x");
    expect(cli("sums", d).status).toBe(0);
    expect(cli("sums", d).status).toBe(0);
    expect(readFileSync(join(d, "SHA256SUMS.txt"), "utf8").split("\n").filter(Boolean)).toHaveLength(1);
  });

  it("falha com diretório vazio", () => {
    const d = tempDir();
    const r = cli("sums", d);
    expect(r.status).toBe(1);
    expect(r.stderr).toContain("release:");
    expect(existsSync(join(d, "SHA256SUMS.txt"))).toBe(false);
  });
});
