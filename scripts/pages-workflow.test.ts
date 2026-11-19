// Forma do workflow que publica site/ no GitHub Pages pelo método oficial.
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { parse } from "yaml";

const root = fileURLToPath(new URL("..", import.meta.url));
const PAGES = join(root, ".github", "workflows", "pages.yml");

type Yaml = any;

function pages(): Yaml {
  expect(existsSync(PAGES), `${PAGES} não existe`).toBe(true);
  return parse(readFileSync(PAGES, "utf8")) ?? {};
}

const text = () => readFileSync(PAGES, "utf8");
const jobs = (): Record<string, Yaml> => pages().jobs ?? {};
const allSteps = (): Yaml[] => Object.values(jobs()).flatMap((j: Yaml) => j.steps ?? []);
const uses = (): string[] => allSteps().map((s) => String(s.uses ?? "")).filter(Boolean);
const stepUsing = (action: string): Yaml => allSteps().find((s) => String(s.uses ?? "").startsWith(`${action}@`));

describe("pages.yml: gatilho e concorrência", () => {
  it("roda em push na main que muda site/** e por workflow_dispatch", () => {
    const on = pages().on;
    expect(Object.keys(on).sort()).toEqual(["push", "workflow_dispatch"]);
    expect(on.push).toEqual({ branches: ["main"], paths: ["site/**"] });
  });

  it("usa o grupo de concorrência pages, sem cancelar publicação em andamento", () => {
    expect(pages().concurrency).toEqual({ group: "pages", "cancel-in-progress": false });
  });
});

describe("pages.yml: permissões mínimas", () => {
  it("não concede permissões no topo", () => {
    expect(pages().permissions).toEqual({});
  });

  it("tem um único job, deploy, com contents: read, pages: write e id-token: write", () => {
    expect(Object.keys(jobs())).toEqual(["deploy"]);
    expect(jobs().deploy.permissions).toEqual({ contents: "read", pages: "write", "id-token": "write" });
  });

  it("publica no ambiente github-pages com a URL da implantação", () => {
    const env = jobs().deploy.environment;
    expect(env.name).toBe("github-pages");
    expect(env.url).toBe("${{ steps.deployment.outputs.page_url }}");
  });
});

describe("pages.yml: método oficial e ações fixadas", () => {
  it("usa checkout, configure-pages, upload-pages-artifact e deploy-pages, nessa ordem", () => {
    const names = uses().map((u) => u.split("@")[0]);
    expect(names).toEqual([
      "actions/checkout",
      "actions/configure-pages",
      "actions/upload-pages-artifact",
      "actions/deploy-pages",
    ]);
  });

  it("fixa cada ação por SHA de 40 caracteres, com a tag em comentário", () => {
    for (const u of uses()) expect(u, u).toMatch(/^[\w.-]+\/[\w.-]+@[0-9a-f]{40}$/);
    const pinned = text().match(/uses: \S+@[0-9a-f]{40} # v\d+\.\d+\.\d+/g) ?? [];
    expect(pinned).toHaveLength(uses().length);
  });

  it("faz checkout sem guardar credenciais", () => {
    expect(stepUsing("actions/checkout").with?.["persist-credentials"]).toBe(false);
  });

  it("envia só a pasta site/", () => {
    expect(stepUsing("actions/upload-pages-artifact").with?.path).toBe("site");
  });

  it("dá ao passo de deploy o id deployment usado na URL do ambiente", () => {
    expect(stepUsing("actions/deploy-pages").id).toBe("deployment");
  });

  it("não roda build nem instala dependências: o site é estático", () => {
    const runs = allSteps().map((s) => String(s.run ?? "")).join("\n");
    expect(runs).not.toMatch(/npm|yarn|pnpm|cargo/);
  });
});
