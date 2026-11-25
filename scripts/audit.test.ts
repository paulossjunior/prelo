// Testes da auditoria de dependências (T023, FR-008, FR-009).
// Campos: data-model.md "Exceção de vulnerabilidade" (id, ecosystem, reason, review_by) e
// "Lista de licenças permitidas".
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { parse as parseToml } from "smol-toml";
import { describe, expect, it } from "vitest";
import { readdirSync } from "node:fs";
import {
  denyAdvisoryIgnores,
  denyAllowList,
  embeddedDataViolations,
  exceptionIds,
  FONT_DIRS,
  fontLicenseViolations,
  licenseViolations,
  parseEmbeddedData,
  parseFontLicenses,
  parseAllowlist,
  parseExceptions,
  validateExceptions,
} from "./audit.mjs";

const root = fileURLToPath(new URL("..", import.meta.url));
const read = (rel: string) => readFileSync(`${root}/${rel}`, "utf8");
const TODAY = new Date("2026-11-18");

describe("lista única de licenças", () => {
  it("ignora comentários e linhas vazias", () => {
    expect(parseAllowlist("# comentário\nMIT\n\n  Apache-2.0  \n")).toEqual(["MIT", "Apache-2.0"]);
  });

  it("security/licenses-allow.txt e src-tauri/deny.toml têm exatamente a mesma lista", () => {
    const allow = parseAllowlist(read("security/licenses-allow.txt"));
    expect(allow.length).toBeGreaterThan(0);
    expect([...denyAllowList(read("src-tauri/deny.toml"))].sort()).toEqual([...allow].sort());
  });

  it("MPL-2.0 não está na lista geral (D-11)", () => {
    expect(parseAllowlist(read("security/licenses-allow.txt"))).not.toContain("MPL-2.0");
  });
});

describe("exceção de licença da crate typst-assets (D-16)", () => {
  type DenyException = { crate?: string; name?: string; allow?: string[] };
  const exceptions = (): DenyException[] =>
    ((parseToml(read("src-tauri/deny.toml")) as { licenses?: { exceptions?: DenyException[] } }).licenses
      ?.exceptions ?? []);
  const D16 = ["CC-BY-4.0", "W3C-20150513"];

  it("typst-assets tem exceção com exatamente CC-BY-4.0 e W3C-20150513", () => {
    const own = exceptions().filter((e) => (e.crate ?? e.name) === "typst-assets");
    expect(own).toHaveLength(1);
    expect([...(own[0]?.allow ?? [])].sort()).toEqual([...D16].sort());
  });

  it("a exceção cita D-16 em comentário", () => {
    const lines = read("src-tauri/deny.toml").split(/\r?\n/);
    const i = lines.findIndex((l) => l.includes('"typst-assets"'));
    expect(i).toBeGreaterThan(0);
    expect(lines.slice(Math.max(0, i - 3), i + 1).join("\n")).toContain("D-16");
  });

  it("nenhuma outra crate tem exceção com CC-BY-4.0 ou W3C-20150513", () => {
    const others = exceptions().filter(
      (e) => (e.crate ?? e.name) !== "typst-assets" && (e.allow ?? []).some((l) => D16.includes(l)),
    );
    expect(others).toEqual([]);
  });

  it("CC-BY-4.0 e W3C-20150513 não estão na lista geral nem em [licenses].allow", () => {
    const allow = parseAllowlist(read("security/licenses-allow.txt"));
    const denyAllow = denyAllowList(read("src-tauri/deny.toml"));
    for (const lic of D16) {
      expect(allow).not.toContain(lic);
      expect(denyAllow).not.toContain(lic);
    }
  });
});

describe("licenseViolations", () => {
  const allow = ["MIT", "Apache-2.0", "ISC"];

  it("aceita licença permitida e expressão OR com ao menos uma alternativa permitida", () => {
    expect(
      licenseViolations(
        [
          { name: "a@1.0.0", licenses: "MIT" },
          { name: "b@1.0.0", licenses: "(MIT OR GPL-3.0)" },
          { name: "c@1.0.0", licenses: "Apache-2.0 AND ISC" },
        ],
        allow,
      ),
    ).toEqual([]);
  });

  it("recusa licença fora da lista nomeando dependência e licença", () => {
    expect(licenseViolations([{ name: "gpl@2.0.0", licenses: "GPL-3.0" }], allow)).toEqual([
      { name: "gpl@2.0.0", licenses: "GPL-3.0" },
    ]);
  });

  it("recusa AND com uma parte fora da lista", () => {
    expect(licenseViolations([{ name: "x@1.0.0", licenses: "MIT AND GPL-3.0" }], allow)).toHaveLength(1);
  });

  it("dependência sem licença ou UNKNOWN é recusada", () => {
    expect(
      licenseViolations(
        [
          { name: "y@1.0.0", licenses: "" },
          { name: "z@1.0.0", licenses: "UNKNOWN" },
          { name: "w@1.0.0" },
        ],
        allow,
      ),
    ).toHaveLength(3);
  });
});

describe("exceções de vulnerabilidade", () => {
  const valid = { id: "GHSA-xxxx-yyyy-zzzz", ecosystem: "npm", reason: "sem correção; não alcançável", review_by: "2027-01-31" };

  it("exceção completa e dentro do prazo é válida", () => {
    expect(validateExceptions([valid], TODAY)).toEqual([]);
  });

  it("exceção sem reason invalida a auditoria", () => {
    expect(validateExceptions([{ ...valid, reason: "" }], TODAY)).toHaveLength(1);
    expect(validateExceptions([{ ...valid, reason: undefined }], TODAY)).toHaveLength(1);
  });

  it("exceção com review_by vencido invalida a auditoria", () => {
    const errors = validateExceptions([{ ...valid, review_by: "2026-11-17" }], TODAY);
    expect(errors).toHaveLength(1);
    expect(errors[0]).toContain(valid.id);
  });

  it("exceção com review_by ausente ou inválido invalida a auditoria", () => {
    expect(validateExceptions([{ ...valid, review_by: undefined }], TODAY)).toHaveLength(1);
    expect(validateExceptions([{ ...valid, review_by: "amanhã" }], TODAY)).toHaveLength(1);
  });

  it("ecosystem fora de cargo/npm invalida a auditoria", () => {
    expect(validateExceptions([{ ...valid, ecosystem: "pip" }], TODAY)).toHaveLength(1);
  });

  it("id fora do formato de aviso (GHSA/RUSTSEC/CVE) invalida a auditoria (vai para linha de comando)", () => {
    expect(validateExceptions([{ ...valid, id: "GHSA-1; rm -rf ~" }], TODAY)).toHaveLength(1);
    expect(validateExceptions([{ ...valid, id: "qualquer" }], TODAY)).toHaveLength(1);
    expect(validateExceptions([{ ...valid, id: "RUSTSEC-2026-0001" }], TODAY)).toEqual([]);
    expect(validateExceptions([{ ...valid, id: "CVE-2026-12345" }], TODAY)).toEqual([]);
  });

  it("exceção sem id invalida a auditoria", () => {
    expect(validateExceptions([{ ...valid, id: "" }], TODAY)).toHaveLength(1);
  });

  it("lê o arquivo TOML e separa ids por ecossistema", () => {
    const list = parseExceptions(
      '[[exception]]\nid = "RUSTSEC-2026-0001"\necosystem = "cargo"\nreason = "r"\nreview_by = "2027-01-01"\n' +
        '[[exception]]\nid = "GHSA-a"\necosystem = "npm"\nreason = "r"\nreview_by = "2027-01-01"\n',
    );
    expect(exceptionIds(list, "cargo")).toEqual(["RUSTSEC-2026-0001"]);
    expect(exceptionIds(list, "npm")).toEqual(["GHSA-a"]);
  });

  it("security/vuln-exceptions.toml do repositório é válido", () => {
    expect(validateExceptions(parseExceptions(read("security/vuln-exceptions.toml")), new Date())).toEqual([]);
  });

  it("advisories.ignore do deny.toml corresponde às exceções cargo", () => {
    const cargo = exceptionIds(parseExceptions(read("security/vuln-exceptions.toml")), "cargo");
    expect([...denyAdvisoryIgnores(read("src-tauri/deny.toml"))].sort()).toEqual([...cargo].sort());
  });
});

describe("licenças de arquivos embutidos (D-15, D-16)", () => {
  const allow = ["MIT", "Apache-2.0", "CC0-1.0", "BSD-3-Clause"];
  const entry = (file: string, license = "OFL-1.1") => ({
    file,
    family: "Teste",
    license,
    copyright: "Copyright © Teste",
  });

  it("lê src-tauri/fonts/licenses.toml como lista de FontLicense", () => {
    const list = parseFontLicenses(
      '[[font]]\nfile = "A.otf"\nfamily = "A"\nlicense = "OFL-1.1"\ncopyright = "c"\n',
    );
    expect(list).toEqual([{ file: "A.otf", family: "A", license: "OFL-1.1", copyright: "c" }]);
  });

  it("arquivo de fonte sem entrada em licenses.toml falha", () => {
    const errors = fontLicenseViolations(["A.otf", "B.ttf"], [entry("A.otf")], allow);
    expect(errors).toHaveLength(1);
    expect(errors[0]).toContain("B.ttf");
  });

  it("todas as extensões de fonte são conferidas; outros arquivos não", () => {
    const files = ["a.ttf", "b.otf", "c.ttc", "d.otc", "OFL.txt", "licenses.toml"];
    expect(fontLicenseViolations(files, [], allow)).toHaveLength(4);
  });

  it("licença fora da lista + OFL-1.1 falha", () => {
    const errors = fontLicenseViolations(["A.otf"], [entry("A.otf", "GPL-3.0")], allow);
    expect(errors).toHaveLength(1);
    expect(errors[0]).toContain("GPL-3.0");
  });

  it("OFL-1.1 e licenças da lista são aceitas para arquivo de fonte", () => {
    expect(fontLicenseViolations(["A.otf", "B.otf"], [entry("A.otf"), entry("B.otf", "MIT")], allow)).toEqual([]);
  });

  it("entrada sem copyright falha (OFL exige o aviso)", () => {
    expect(fontLicenseViolations(["A.otf"], [{ ...entry("A.otf"), copyright: "" }], allow)).toHaveLength(1);
  });

  it("licenças das fontes do typst CLI valem só para arquivo de fonte", () => {
    const repoAllow = parseAllowlist(read("security/licenses-allow.txt"));
    for (const license of [
      "LicenseRef-GUST-Font-License",
      "LicenseRef-NewCM-GPL-3.0-or-later-FE-DE",
      "Bitstream-Vera",
    ]) {
      expect(fontLicenseViolations(["A.otf"], [entry("A.otf", license)], repoAllow)).toEqual([]);
      expect(licenseViolations([{ name: "dep@1.0.0", licenses: license }], repoAllow)).toHaveLength(1);
    }
  });

  it("OFL-1.1 em dependência (não fonte) continua proibida", () => {
    const repoAllow = parseAllowlist(read("security/licenses-allow.txt"));
    expect(repoAllow).not.toContain("OFL-1.1");
    expect(denyAllowList(read("src-tauri/deny.toml"))).not.toContain("OFL-1.1");
    expect(licenseViolations([{ name: "fonte-npm@1.0.0", licenses: "OFL-1.1" }], repoAllow)).toHaveLength(1);
  });

  it("as fontes de src-tauri/fonts/ do repositório passam na verificação", () => {
    const files = readdirSync(`${root}/src-tauri/fonts`);
    const entries = parseFontLicenses(read("src-tauri/fonts/licenses.toml"));
    expect(files.some((f) => f.endsWith(".otf"))).toBe(true);
    expect(fontLicenseViolations(files, entries, parseAllowlist(read("security/licenses-allow.txt")))).toEqual([]);
  });

  it("fontes web (.woff2, .woff) também são conferidas", () => {
    expect(fontLicenseViolations(["a.woff2", "b.woff", "fonts.css"], [], allow)).toHaveLength(2);
  });

  it("a auditoria confere as fontes do compilador e as da interface", () => {
    expect(FONT_DIRS).toEqual(["src-tauri/fonts", "src/fonts"]);
  });

  it("as fontes da interface em src/fonts/ passam na verificação", () => {
    const files = readdirSync(`${root}/src/fonts`);
    const entries = parseFontLicenses(read("src/fonts/licenses.toml"));
    expect(files.some((f) => f.endsWith(".woff2"))).toBe(true);
    expect(fontLicenseViolations(files, entries, parseAllowlist(read("security/licenses-allow.txt")))).toEqual([]);
  });

  it("dados embutidos do typst-assets (ICC, HTML, MathML, Foxit) estão listados com licenças (D-16)", () => {
    const data = parseEmbeddedData(read("security/embedded-data.toml"));
    const byName = Object.fromEntries(data.map((d) => [d.name, d]));
    for (const name of ["icc", "html", "mathml", "foxit"]) {
      expect(byName[name]?.crate).toBe("typst-assets");
      expect(byName[name]?.licenses.length).toBeGreaterThan(0);
    }
    expect(byName.html?.licenses).toContain("CC-BY-4.0");
    expect(byName.mathml?.licenses).toContain("W3C-20150513");
  });

  it("licença de dado embutido fora da lista e fora da exceção da própria crate falha", () => {
    const exceptions = [{ crate: "typst-assets", allow: ["CC-BY-4.0", "W3C-20150513"] }];
    const ok = [{ name: "html", crate: "typst-assets", path: "files/html", licenses: ["CC-BY-4.0"] }];
    expect(embeddedDataViolations(ok, allow, exceptions)).toEqual([]);
    const outra = [{ name: "x", crate: "outra-crate", path: "x", licenses: ["CC-BY-4.0"] }];
    expect(embeddedDataViolations(outra, allow, exceptions)).toHaveLength(1);
    const gpl = [{ name: "y", crate: "typst-assets", path: "y", licenses: ["GPL-3.0"] }];
    expect(embeddedDataViolations(gpl, allow, exceptions)).toHaveLength(1);
  });

  it("security/embedded-data.toml do repositório passa contra deny.toml", () => {
    const deny = parseToml(read("src-tauri/deny.toml")) as {
      licenses?: { exceptions?: { crate: string; allow: string[] }[] };
    };
    expect(
      embeddedDataViolations(
        parseEmbeddedData(read("security/embedded-data.toml")),
        parseAllowlist(read("security/licenses-allow.txt")),
        deny.licenses?.exceptions ?? [],
      ),
    ).toEqual([]);
  });
});

