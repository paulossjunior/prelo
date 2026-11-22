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
    expect(on.push).toEqual({ branches: ["main", "dev"] });
  });

  it("mantém permissões só de leitura", () => {
    expect(ci().permissions).toEqual({ contents: "read" });
  });

  it("mantém exatamente os checks obrigatórios verify (<sistema>) da proteção da main", () => {
    const jobs = ci().jobs;
    // `equivalence` (003, T070) roda junto, mas não é check obrigatório: o nome não é verify.
    expect(Object.keys(jobs)).toEqual(["verify", "equivalence"]);
    expect(String(jobs.equivalence.name)).not.toMatch(/^verify/);
    const verify = jobs.verify;
    expect(verify.name).toBe("verify (${{ matrix.os }})");
    // Gitflow: a matriz depende do destino; fora da dev (main, tag, workflow_call) são os 3.
    const full = JSON.parse(String(verify.strategy.matrix.os).match(/\|\| '(\[[^']*\])'\) \}\}$/)![1]!);
    const names = (full as string[]).map((os) => verify.name.replace("${{ matrix.os }}", os));
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

  it("confere no macOS que o binário é universal (x86_64 e arm64) antes do upload", () => {
    const st = steps("build");
    const build = st.findIndex((s) => /tauri build/.test(runOf(s)));
    const lipo = st.findIndex((s) => /lipo -archs/.test(runOf(s)));
    const upload = st.findIndex((s) => usesOf(s).startsWith("actions/upload-artifact@"));
    expect(lipo).toBeGreaterThan(build);
    expect(lipo).toBeLessThan(upload);
    expect(st[lipo].if).toBe("runner.os == 'macOS'");
    expect(runOf(st[lipo])).toContain("src-tauri/target/universal-apple-darwin/release/prelo");
    expect(runOf(st[lipo])).toMatch(/x86_64/);
    expect(runOf(st[lipo])).toMatch(/arm64/);
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
    expect(cleanup[0].if).toBe("(failure() || cancelled()) && steps.create.outputs.created == 'true'");
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

// ---------------------------------------------------------------------------------------
// US2: nada é gerado sem preflight e verify verdes (FR-001–FR-004, FR-016).

describe("release.yml: preflight e verify antes do build", () => {
  it("preflight só lê, em ubuntu-22.04, com histórico completo", () => {
    expect(job("preflight")["runs-on"]).toBe("ubuntu-22.04");
    expect(job("preflight").permissions).toEqual({ contents: "read" });
    const checkout = steps("preflight").find((s) => usesOf(s).startsWith("actions/checkout@"));
    expect(checkout?.with?.["fetch-depth"]).toBe(0);
  });

  it("preflight roda release-check.mjs com a tag vinda de env", () => {
    const step = steps("preflight").find((s) => /node scripts\/release-check\.mjs/.test(runOf(s)));
    expect(runOf(step)).toContain('node scripts/release-check.mjs "$TAG"');
    expect({ ...job("preflight").env, ...step?.env }.TAG).toBe("${{ github.ref_name }}");
  });

  it("verify reaproveita ci.yml, depois do preflight, só com leitura", () => {
    expect(job("verify").uses).toBe("./.github/workflows/ci.yml");
    expect(needsOf("verify")).toEqual(["preflight"]);
    expect(job("verify").permissions).toEqual({ contents: "read" });
  });

  it("build só roda depois do verify", () => {
    expect(needsOf("build")).toContain("verify");
  });

  it("ordem completa: preflight → verify → build → publish", () => {
    expect(Object.keys(release().jobs)).toEqual(["preflight", "verify", "build", "publish"]);
  });
});

// ---------------------------------------------------------------------------------------
// US3: checksums e notas do rascunho (FR-009, FR-014).

const NOTES = join(root, ".github", "release-notes.md");
const notes = () => {
  expect(existsSync(NOTES), `${NOTES} não existe`).toBe(true);
  return readFileSync(NOTES, "utf8");
};

describe("release.yml: checksums e notas no publish", () => {
  const index = (re: RegExp) => steps("publish").findIndex((s) => re.test(runOf(s)));
  const create = () => runOf(steps("publish").find((s) => s.id === "create"));

  it("gera SHA256SUMS.txt depois do collect e antes de criar, no diretório anexado", () => {
    const collect = index(/release-artifacts\.mjs collect downloads "\$\{TAG#v\}" release-files/);
    const sums = index(/node scripts\/release-artifacts\.mjs sums release-files/);
    const created = steps("publish").findIndex((s) => s.id === "create");
    expect(collect).toBeGreaterThanOrEqual(0);
    expect(sums).toBeGreaterThan(collect);
    expect(sums).toBeLessThan(created);
    expect(create()).toMatch(/ release-files\/\*$/m);
  });

  it("renderiza as notas com a versão e as passa por --notes-file", () => {
    const render = index(/\.github\/release-notes\.md/);
    const created = steps("publish").findIndex((s) => s.id === "create");
    expect(render).toBeGreaterThanOrEqual(0);
    expect(render).toBeLessThan(created);
    const run = runOf(steps("publish")[render]);
    expect(run).toContain("{{VERSION}}");
    expect(run).toContain('"$RUNNER_TEMP/notes.md"');
    expect(create()).toContain('--notes-file "$RUNNER_TEMP/notes.md"');
    expect(create()).not.toMatch(/--notes /);
  });
});

describe(".github/release-notes.md", () => {
  it("tem o marcador {{VERSION}}", () => {
    expect(notes()).toContain("{{VERSION}}");
  });

  it("avisa que os instaladores não são assinados", () => {
    expect(notes()).toMatch(/não são assinados/);
  });

  it("explica como abrir no macOS (Gatekeeper) e no Windows (SmartScreen)", () => {
    const text = notes();
    expect(text).toContain("Gatekeeper");
    expect(text).toContain("Abrir");
    expect(text).toContain("SmartScreen");
    expect(text).toContain("Mais informações");
    expect(text).toContain("Executar assim mesmo");
  });

  it("traz os comandos de conferência do checksum nos três sistemas", () => {
    const text = notes();
    expect(text).toContain("sha256sum -c SHA256SUMS.txt");
    expect(text).toContain("shasum -a 256 -c SHA256SUMS.txt");
    expect(text).toContain("Get-FileHash");
  });
});

// ---------------------------------------------------------------------------------------
// US4: segurança por construção (FR-010, FR-011, FR-012).

const allSteps = (wf: Yaml): Yaml[] => Object.values(wf.jobs ?? {}).flatMap((j: Yaml) => j.steps ?? []);

describe("release.yml: permissões e segredos", () => {
  it("escrita só no publish; os demais jobs com exatamente contents: read", () => {
    const jobs = release().jobs;
    expect(Object.keys(jobs).length).toBeGreaterThan(0);
    for (const [name, j] of Object.entries<Yaml>(jobs)) {
      const expected = name === "publish" ? { contents: "write" } : { contents: "read" };
      expect(j.permissions, name).toEqual(expected);
    }
  });

  it("nenhum segredo além do GITHUB_TOKEN automático, e nada de secrets: inherit", () => {
    const text = releaseText();
    const used = [...text.matchAll(/secrets\.([A-Za-z_][A-Za-z0-9_]*)/g)].map((m) => m[1]);
    expect(used.filter((s) => s !== "GITHUB_TOKEN")).toEqual([]);
    expect(job("verify").secrets).toBeUndefined();
  });

  it("nenhuma variável de assinatura ou notarização", () => {
    for (const re of [/APPLE_/, /TAURI_SIGNING_/, /WINDOWS_CERT/, /\bcodesign\b/, /notar/i]) {
      expect(releaseText()).not.toMatch(re);
    }
  });

  it("toda ação externa fixada por SHA de 40 caracteres, com a versão em comentário", () => {
    const lines = releaseText().split(/\r?\n/).filter((l) => /^\s*(-\s+)?uses:\s/.test(l));
    const external = lines.filter((l) => !/uses:\s+\.\//.test(l));
    expect(external.length).toBeGreaterThan(0);
    for (const l of external) {
      expect(l).toMatch(/uses:\s+[\w.-]+\/[\w./-]+@[0-9a-f]{40}\s+#\s+v\d+(\.\d+)*\s*$/);
    }
  });

  it.each([
    ["release.yml", release],
    ["ci.yml", ci],
  ])("%s: nenhum valor controlável de fora interpolado em run: (injeção de script)", (_name, wf) => {
    for (const step of allSteps(wf())) {
      expect(runOf(step)).not.toMatch(/\$\{\{\s*(github\.ref_name|github\.head_ref|github\.event\.|inputs\.)/);
    }
  });
});

describe("release.yml: self-test fora do instalador", () => {
  it("nenhum --features, self-test nem VITE_SELF_TEST no workflow de release", () => {
    const text = releaseText();
    expect(text).not.toContain("--features");
    expect(text).not.toContain("VITE_SELF_TEST");
    for (const s of steps("build").filter((st) => /tauri build/.test(runOf(st)))) {
      expect(runOf(s)).not.toContain("self-test");
      expect(s.env ?? {}).toEqual({});
    }
  });

  it("varre o binário do app depois do build e antes do upload", () => {
    const st = steps("build");
    const build = st.findIndex((s) => /tauri build/.test(runOf(s)));
    const scan = st.findIndex((s) => /node scripts\/release-artifacts\.mjs scan /.test(runOf(s)));
    const upload = st.findIndex((s) => usesOf(s).startsWith("actions/upload-artifact@"));
    expect(scan).toBeGreaterThan(build);
    expect(scan).toBeLessThan(upload);
    expect(runOf(st[scan])).toContain("src-tauri/target/${{ matrix.target }}/release/prelo");
  });
});

// ---------------------------------------------------------------------------------------
// Polish: documentação para contribuidores (FR-015).

describe("README: Publicar uma versão", () => {
  const section = () => {
    const text = readFileSync(join(root, "README.md"), "utf8");
    const start = text.indexOf("### Publicar uma versão");
    expect(start, "seção ausente").toBeGreaterThanOrEqual(0);
    const end = text.indexOf("\n## ", start);
    return text.slice(start, end < 0 ? undefined : end);
  };

  it("diz quais arquivos de versão atualizar e como criar a tag na main", () => {
    const s = section();
    for (const f of ["package.json", "src-tauri/tauri.conf.json", "src-tauri/Cargo.toml"]) expect(s).toContain(f);
    expect(s).toContain("git tag vX.Y.Z origin/main && git push origin vX.Y.Z");
  });

  it("explica revisar e publicar o rascunho, conferir checksums e o pré-requisito de licença", () => {
    const s = section();
    expect(s).toMatch(/rascunho/);
    expect(s).toContain("SHA256SUMS.txt");
    expect(s).toMatch(/licença/);
  });
});
