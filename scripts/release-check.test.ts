// Pré-checagem da release (spec 002, contracts/release-scripts.md, data-model.md):
// formato da tag, commit na main, versões declaradas e arquivo de licença.
import { execFileSync, spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import { checkRelease, findLicense, isOnMain, parseTag, readVersions, versionMismatches } from "./release-check.mjs";

const SCRIPT = fileURLToPath(new URL("./release-check.mjs", import.meta.url));
const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const dirs: string[] = [];

// Dentro do hook pre-push o git exporta GIT_*; herdadas, apontariam para o repositório real.
const env = Object.fromEntries(Object.entries(process.env).filter(([k]) => !k.startsWith("GIT_")));

function git(cwd: string, ...args: string[]) {
  return execFileSync("git", args, { cwd, encoding: "utf8", stdio: "pipe", env }).trim();
}

function tempDir() {
  const d = mkdtempSync(join(tmpdir(), "prelo-release-check-"));
  dirs.push(d);
  return d;
}

interface Project {
  pkg?: string;
  conf?: string;
  cargo?: string;
  license?: Record<string, string>;
}

/** Projeto com os três arquivos de versão e, por padrão, LICENSE-MIT e LICENSE-APACHE. */
function project({ pkg = "1.2.3", conf = "1.2.3", cargo = "1.2.3", license }: Project = {}) {
  const root = tempDir();
  mkdirSync(join(root, "src-tauri"));
  writeFileSync(join(root, "package.json"), JSON.stringify({ name: "prelo", version: pkg }, null, 2));
  writeFileSync(join(root, "src-tauri", "tauri.conf.json"), JSON.stringify({ productName: "Prelo", version: conf }));
  writeFileSync(
    join(root, "src-tauri", "Cargo.toml"),
    `[package]\nname = "prelo"\nversion = "${cargo}"\nedition = "2021"\n\n[dependencies]\ntauri = { version = "2.12" }\n`,
  );
  for (const [name, text] of Object.entries(license ?? { "LICENSE-MIT": "MIT", "LICENSE-APACHE": "Apache" })) {
    writeFileSync(join(root, name), text);
  }
  return root;
}

/** Repositório git com main publicada em origin/main e uma branch com commit só dela. */
function repo(p: Project = {}) {
  const root = project(p);
  git(root, "init", "-q", "-b", "main");
  git(root, "config", "user.email", "teste@example.invalid");
  git(root, "config", "user.name", "Teste");
  git(root, "config", "commit.gpgsign", "false");
  git(root, "add", "-A");
  git(root, "commit", "-q", "-m", "inicial");
  const onMain = git(root, "rev-parse", "HEAD");
  git(root, "update-ref", "refs/remotes/origin/main", onMain);
  git(root, "checkout", "-q", "-b", "outra");
  git(root, "commit", "-q", "--allow-empty", "-m", "fora da main");
  const offMain = git(root, "rev-parse", "HEAD");
  git(root, "checkout", "-q", "main");
  return { root, onMain, offMain };
}

function cli(cwd: string, ...args: string[]) {
  return spawnSync(process.execPath, [SCRIPT, ...args], { cwd, encoding: "utf8", env });
}

afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true });
});

describe("parseTag", () => {
  it.each([
    ["v0.1.0", "0.1.0"],
    ["v10.20.30", "10.20.30"],
  ])("aceita %s", (tag, version) => {
    expect(parseTag(tag)).toEqual({ version });
  });

  it.each(["v1.2", "1.2.3", "release-1", "v1.2.3-rc.1", "v01.2.3", "v1.2.3 ", "v1.2.3\n", "", "V1.2.3"])(
    "recusa %j",
    (tag) => {
      expect(() => parseTag(tag)).toThrow(`tag "${tag}" fora do formato vX.Y.Z`);
    },
  );

  it("recusa tag com comando de shell embutido (segurança)", () => {
    expect(() => parseTag("v1.2.3;rm -rf")).toThrow("fora do formato vX.Y.Z");
    expect(() => parseTag("v1.2.3$(id)")).toThrow("fora do formato vX.Y.Z");
  });
});

describe("readVersions e versionMismatches", () => {
  it("lê a versão dos três arquivos", () => {
    expect(readVersions(project({ pkg: "1.0.0", conf: "1.0.1", cargo: "1.0.2" }))).toEqual([
      { file: "package.json", version: "1.0.0" },
      { file: "src-tauri/tauri.conf.json", version: "1.0.1" },
      { file: "src-tauri/Cargo.toml", version: "1.0.2" },
    ]);
  });

  it("no Cargo.toml lê só [package].version, não a versão de dependências", () => {
    const root = project();
    writeFileSync(
      join(root, "src-tauri", "Cargo.toml"),
      `[dependencies]\ntauri = "2"\nversion = "9.9.9"\n\n[package]\nname = "prelo"\nversion = "1.2.3"\n`,
    );
    expect(readVersions(root)[2]).toEqual({ file: "src-tauri/Cargo.toml", version: "1.2.3" });
  });

  it("nomeia cada arquivo divergente com o valor encontrado", () => {
    const versions = readVersions(project({ conf: "1.2.4", cargo: "0.9.0" }));
    expect(versionMismatches("1.2.3", versions)).toEqual([
      { file: "src-tauri/tauri.conf.json", version: "1.2.4" },
      { file: "src-tauri/Cargo.toml", version: "0.9.0" },
    ]);
  });

  it("versão ausente conta como divergente", () => {
    const root = project();
    writeFileSync(join(root, "src-tauri", "tauri.conf.json"), JSON.stringify({ productName: "Prelo" }));
    expect(versionMismatches("1.2.3", readVersions(root))).toEqual([
      { file: "src-tauri/tauri.conf.json", version: "(ausente)" },
    ]);
  });

  it("nenhuma divergência quando os três batem", () => {
    expect(versionMismatches("1.2.3", readVersions(project()))).toEqual([]);
  });
});

describe("findLicense", () => {
  it("sem arquivo de licença: null", () => {
    expect(findLicense(project({ license: {} }))).toBeNull();
  });

  it("arquivo de licença vazio conta como ausente", () => {
    expect(findLicense(project({ license: { LICENSE: "  \n" } }))).toBeNull();
  });

  it.each(["LICENSE", "LICENSE.md", "LICENSE.txt", "COPYING", "LICENSE-MIT", "LICENSE-APACHE"])(
    "aceita %s",
    (name) => {
      expect(findLicense(project({ license: { [name]: "texto da licença" } }))).toBe(name);
    },
  );

  it("o repositório do Prelo tem licença", () => {
    expect(findLicense(repoRoot)).not.toBeNull();
  });
});

describe("isOnMain", () => {
  it("commit da main: true", () => {
    const { root, onMain } = repo();
    expect(isOnMain(onMain, root)).toBe(true);
  });

  it("commit só de outra branch: false", () => {
    const { root, offMain } = repo();
    expect(isOnMain(offMain, root)).toBe(false);
  });

  it("entrada que não é SHA não chega ao git como opção (segurança)", () => {
    const { root } = repo();
    expect(isOnMain("--help", root)).toBe(false);
    expect(isOnMain("main", root)).toBe(false);
  });
});

describe("checkRelease", () => {
  it("tudo certo: nenhuma mensagem", () => {
    const { root, onMain } = repo();
    expect(checkRelease({ tag: "v1.2.3", root, commit: onMain })).toEqual([]);
  });

  it("tag fora do formato: só essa mensagem", () => {
    const { root, onMain } = repo();
    expect(checkRelease({ tag: "v1.2", root, commit: onMain })).toEqual([
      'release: tag "v1.2" fora do formato vX.Y.Z',
    ]);
  });

  it("commit fora da main", () => {
    const { root, offMain } = repo();
    expect(checkRelease({ tag: "v1.2.3", root, commit: offMain })).toEqual([
      `release: o commit ${offMain} da tag não pertence à main`,
    ]);
  });

  it("versões divergentes: uma mensagem por arquivo", () => {
    const { root, onMain } = repo({ pkg: "1.2.2", conf: "1.2.2", cargo: "1.2.2" });
    expect(checkRelease({ tag: "v1.2.3", root, commit: onMain })).toEqual([
      "release: package.json declara versão 1.2.2, mas a tag é v1.2.3",
      "release: src-tauri/tauri.conf.json declara versão 1.2.2, mas a tag é v1.2.3",
      "release: src-tauri/Cargo.toml declara versão 1.2.2, mas a tag é v1.2.3",
    ]);
  });

  it("sem licença", () => {
    const { root, onMain } = repo({ license: {} });
    expect(checkRelease({ tag: "v1.2.3", root, commit: onMain })).toEqual([
      "release: falta arquivo de licença na raiz (LICENSE); escolha a licença antes de publicar",
    ]);
  });
});

describe("CLI", { timeout: 30_000 }, () => {
  it("sai com 0 na tag válida, no HEAD da main", () => {
    const { root } = repo();
    const r = cli(root, "v1.2.3");
    expect(r.status, r.stderr).toBe(0);
  });

  it.each([
    ["formato", "v1.2.3-rc.1", {}, "fora do formato"],
    ["versão", "v1.2.4", {}, "package.json declara versão 1.2.3, mas a tag é v1.2.4"],
    ["licença", "v1.2.3", { license: {} }, "falta arquivo de licença"],
  ])("sai com 1 na falha de %s", (_n, tag, p, message) => {
    const { root } = repo(p as Project);
    const r = cli(root, tag);
    expect(r.status).toBe(1);
    expect(r.stderr).toContain("release: ");
    expect(r.stderr).toContain(message);
  });

  it("sai com 1 quando o HEAD não está na main", () => {
    const { root } = repo();
    git(root, "checkout", "-q", "outra");
    const r = cli(root, "v1.2.3");
    expect(r.status).toBe(1);
    expect(r.stderr).toContain("não pertence à main");
  });

  it("sai com 1 sem argumento", () => {
    const r = cli(repo().root);
    expect(r.status).toBe(1);
    expect(r.stderr).toContain("release:");
  });
});
