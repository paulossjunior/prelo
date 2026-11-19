// Testes da trava local de push (T027, FR-010, contracts/dev-workflow.md).
// Usa um `git push` real para um remoto temporário, com `npm run verify` simulado.
import { execFileSync, spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it } from "vitest";

const root = fileURLToPath(new URL("..", import.meta.url));
const dirs: string[] = [];

// Dentro do hook pre-push o git exporta variáveis GIT_*; herdadas, fariam estes comandos
// agirem no repositório real em vez dos temporários.
const env = Object.fromEntries(Object.entries(process.env).filter(([k]) => !k.startsWith("GIT_")));

function git(cwd: string, ...args: string[]) {
  return execFileSync("git", args, { cwd, encoding: "utf8", stdio: "pipe", env });
}

/** Repositório com a trava do Prelo e um `verify` que sai com `code`. */
function repoWithVerify(code: number) {
  const base = mkdtempSync(join(tmpdir(), "prelo-hook-"));
  dirs.push(base);
  const remote = join(base, "remote.git");
  const work = join(base, "work");
  git(base, "init", "-q", "--bare", remote);
  git(base, "init", "-q", "-b", "main", work);
  git(work, "config", "user.email", "teste@example.invalid");
  git(work, "config", "user.name", "Teste");
  git(work, "config", "commit.gpgsign", "false");
  git(work, "remote", "add", "origin", remote);
  mkdirSync(join(work, ".githooks"));
  copyFileSync(join(root, ".githooks", "pre-push"), join(work, ".githooks", "pre-push"));
  writeFileSync(
    join(work, "package.json"),
    JSON.stringify({ name: "t", private: true, scripts: { verify: `node -e "process.exit(${code})"` } }),
  );
  git(work, "config", "core.hooksPath", ".githooks");
  git(work, "add", "-A");
  git(work, "commit", "-q", "-m", "inicial");
  return { work, remote };
}

function push(work: string) {
  return spawnSync("git", ["push", "-q", "origin", "main"], { cwd: work, encoding: "utf8", env });
}

afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true });
});

// Cada caso cria repositórios e faz `git push` real; em máquina carregada passa de 5 s.
describe("pre-push", { timeout: 30_000 }, () => {
  it("bloqueia o push quando npm run verify falha", () => {
    const { work, remote } = repoWithVerify(1);
    const r = push(work);
    expect(r.status).not.toBe(0);
    expect(() => git(remote, "rev-parse", "main")).toThrow();
  });

  it("deixa o push passar quando npm run verify passa", () => {
    const { work, remote } = repoWithVerify(0);
    const r = push(work);
    expect(r.status, r.stderr).toBe(0);
    expect(git(remote, "rev-parse", "main").trim()).toBe(git(work, "rev-parse", "HEAD").trim());
  });
});

describe("prepare", { timeout: 30_000 }, () => {
  it("ativa .githooks como core.hooksPath", () => {
    const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
    const base = mkdtempSync(join(tmpdir(), "prelo-prepare-"));
    dirs.push(base);
    git(base, "init", "-q");
    execFileSync(pkg.scripts.prepare, { cwd: base, shell: true, env });
    expect(git(base, "config", "core.hooksPath").trim()).toBe(".githooks");
  });
});
