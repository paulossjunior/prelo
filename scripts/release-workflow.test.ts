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

// ---------------------------------------------------------------------------------------
// US1: build em matriz e publicação em rascunho, nunca parcial (FR-005–FR-008, FR-013).

const releaseText = () => readFileSync(RELEASE, "utf8");
const job = (name: string): Yaml => release().jobs?.[name] ?? {};
const steps = (name: string): Yaml[] => job(name).steps ?? [];
const runOf = (step: Yaml): string => String(step?.run ?? "");
const usesOf = (step: Yaml): string => String(step?.uses ?? "");
const needsOf = (name: string): string[] => [job(name).needs ?? []].flat();

describe("release.yml: job build", () => {
  it("usa a matriz do contrato (dmg universal, NSIS x64, AppImage e deb x64) com fail-fast", () => {
    const strategy = job("build").strategy ?? {};
    expect(strategy["fail-fast"]).toBe(true);
    expect(strategy.matrix?.include).toEqual([
      { os: "macos-latest", target: "universal-apple-darwin", bundles: "dmg" },
      { os: "windows-latest", target: "x86_64-pc-windows-msvc", bundles: "nsis" },
      { os: "ubuntu-22.04", target: "x86_64-unknown-linux-gnu", bundles: "appimage,deb" },
    ]);
    expect(job("build")["runs-on"]).toBe("${{ matrix.os }}");
  });

  it("gera com tauri build pelo alvo e pelos bundles da matriz", () => {
    const build = steps("build").filter((s) => /tauri build/.test(runOf(s)));
    expect(build).toHaveLength(1);
    expect(runOf(build[0])).toContain("npx --no-install tauri build --target ${{ matrix.target }} --bundles ${{ matrix.bundles }}");
  });

  it("adiciona os dois alvos Apple antes do build universal", () => {
    const i = steps("build").findIndex((s) => /rustup target add aarch64-apple-darwin x86_64-apple-darwin/.test(runOf(s)));
    const b = steps("build").findIndex((s) => /tauri build/.test(runOf(s)));
    expect(i).toBeGreaterThanOrEqual(0);
    expect(i).toBeLessThan(b);
  });

  it("envia os instaladores como artefato do workflow installers-<os>", () => {
    const up = steps("build").filter((s) => usesOf(s).startsWith("actions/upload-artifact@"));
    expect(up).toHaveLength(1);
    expect(up[0].with?.name).toBe("installers-${{ matrix.os }}");
    expect(up[0].with?.["if-no-files-found"]).toBe("error");
  });
});

describe("release.yml: job publish", () => {
  const createIndex = () => steps("publish").findIndex((s) => s.id === "create");

  it("depende de preflight, verify e build", () => {
    expect(needsOf("publish")).toEqual(expect.arrayContaining(["preflight", "verify", "build"]));
  });

  it("baixa os artefatos dos três sistemas e confere o conjunto com collect antes de criar", () => {
    const st = steps("publish");
    const dl = st.findIndex((s) => usesOf(s).startsWith("actions/download-artifact@"));
    const collect = st.findIndex((s) => /node scripts\/release-artifacts\.mjs collect /.test(runOf(s)));
    expect(dl).toBeGreaterThanOrEqual(0);
    expect(st[dl].with?.pattern).toBe("installers-*");
    expect(dl).toBeLessThan(collect);
    expect(collect).toBeLessThan(createIndex());
  });

  it("exige que a release ainda não exista antes de criar", () => {
    const st = steps("publish");
    const check = st.findIndex((s) => /gh release view "\$TAG"/.test(runOf(s)) && /exit 1/.test(runOf(s)));
    expect(check).toBeGreaterThanOrEqual(0);
    expect(check).toBeLessThan(createIndex());
  });

  it("cria o rascunho num só comando, com --draft e --verify-tag", () => {
    const create = steps("publish")[createIndex()];
    const run = runOf(create);
    expect(run.match(/gh release create/g)).toHaveLength(1);
    expect(run).toMatch(/gh release create "\$TAG" --draft --verify-tag --title "\$TAG"/);
  });

  it("marca created=true antes de chamar gh release create (falha no upload também limpa)", () => {
    const run = runOf(steps("publish")[createIndex()]);
    const mark = run.indexOf('echo "created=true" >> "$GITHUB_OUTPUT"');
    expect(mark).toBeGreaterThanOrEqual(0);
    expect(mark).toBeLessThan(run.indexOf("gh release create"));
  });

  it("confere os assets anexados depois de criar", () => {
    const st = steps("publish");
    const check = st.findIndex((s, i) => i > createIndex() && /gh release view "\$TAG" --json assets/.test(runOf(s)));
    expect(check).toBeGreaterThan(createIndex());
  });

  it("limpa só o rascunho criado nesta execução, sem apagar a tag", () => {
    const cleanup = steps("publish").filter((s) => /gh release delete/.test(runOf(s)));
    expect(cleanup).toHaveLength(1);
    expect(cleanup[0].if).toBe("failure() && steps.create.outputs.created == 'true'");
    expect(runOf(cleanup[0])).toContain('gh release delete "$TAG" --yes');
    expect(runOf(cleanup[0])).not.toContain("--cleanup-tag");
  });

  it("nunca publica, marca como latest, sobrescreve nem anexa a release existente", () => {
    const text = releaseText();
    for (const forbidden of ["--draft=false", "--latest", "--clobber", "gh release upload", "gh release edit"]) {
      expect(text, forbidden).not.toContain(forbidden);
    }
  });
});
