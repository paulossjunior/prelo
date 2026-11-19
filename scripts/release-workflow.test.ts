// Forma do workflow de release e do CI chamado por ele
// (specs/002-release-pipeline/contracts/release-workflow.md).
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { parse } from "yaml";

const root = fileURLToPath(new URL("..", import.meta.url));
const RELEASE = join(root, ".github", "workflows", "release.yml");
const CI = join(root, ".github", "workflows", "ci.yml");

type Yaml = any;

function load(path: string): Yaml {
  expect(existsSync(path), `${path} não existe`).toBe(true);
  return parse(readFileSync(path, "utf8")) ?? {};
}

const release = () => load(RELEASE);
const ci = () => load(CI);

describe("release.yml: gatilho e concorrência", () => {
  it("dispara só por push de tag no padrão vX.Y.Z", () => {
    const on = release().on;
    expect(Object.keys(on)).toEqual(["push"]);
    expect(Object.keys(on.push)).toEqual(["tags"]);
    expect(on.push.tags).toEqual(["v[0-9]+.[0-9]+.[0-9]+"]);
  });

  it("usa grupo de concorrência próprio, sem cancelar release em andamento", () => {
    const group = release().concurrency?.group;
    expect(group).toMatch(/^release-/);
    expect(group).not.toBe(ci().concurrency?.group);
    expect(release().concurrency["cancel-in-progress"]).toBe(false);
  });

  it("não concede permissões no topo", () => {
    expect(release().permissions).toEqual({});
  });
});

describe("ci.yml: chamável pela release sem mudar os checks da main", () => {
  it("aceita workflow_call e mantém pull_request e push na main", () => {
    const on = ci().on;
    expect(on).toHaveProperty("workflow_call");
    expect(on).toHaveProperty("pull_request");
    expect(on.push).toEqual({ branches: ["main"] });
  });

  it("mantém permissões só de leitura", () => {
    expect(ci().permissions).toEqual({ contents: "read" });
  });

  it("mantém exatamente os checks obrigatórios verify (<sistema>) da proteção da main", () => {
    const jobs = ci().jobs;
    expect(Object.keys(jobs)).toEqual(["verify"]);
    const verify = jobs.verify;
    expect(verify.name).toBe("verify (${{ matrix.os }})");
    const names = (verify.strategy.matrix.os as string[]).map((os) =>
      verify.name.replace("${{ matrix.os }}", os),
    );
    expect(names).toEqual(["verify (ubuntu-22.04)", "verify (windows-latest)", "verify (macos-latest)"]);
    expect(verify.strategy.matrix.include).toBeUndefined();
  });
});
