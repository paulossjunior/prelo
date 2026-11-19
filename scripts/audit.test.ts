// Testes da auditoria de dependências (T023, FR-008, FR-009).
// Campos: data-model.md "Exceção de vulnerabilidade" (id, ecosystem, reason, review_by) e
// "Lista de licenças permitidas".
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { parse as parseToml } from "smol-toml";
import { describe, expect, it } from "vitest";
import {
  denyAdvisoryIgnores,
  denyAllowList,
  exceptionIds,
  licenseViolations,
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
