// Instaladores, checksums e marca de autoteste da release
// (specs/002-release-pipeline/contracts/release-scripts.md, data-model.md).
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import { expectedInstallers, matchInstallers } from "./release-artifacts.mjs";

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
