// Fidelidade visual ao protótipo (T037/T038/T039): cores, tipografia e realce da área de
// trabalho vêm de prototype/index.html, a fonte de verdade do design. Estes testes travam os
// tokens e as fontes; a comparação visual fica nas capturas do PR.
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const read = (p: string) => readFileSync(resolve(process.cwd(), p), "utf8");
const proto = read("prototype/index.html");
const css = read("src/workspace.css");
const fontsCss = read("src/fonts/fonts.css");

/** Declarações `--nome: valor` do primeiro bloco que casa com `selector`. */
function tokens(text: string, selector: RegExp): Map<string, string> {
  const m = selector.exec(text);
  if (!m) throw new Error(`bloco ${selector} ausente`);
  const start = m.index + m[0].length;
  const body = text.slice(start, text.indexOf("}", start));
  const out = new Map<string, string>();
  for (const d of body.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)) out.set(d[1]!, d[2]!.replace(/\s+/g, "").toLowerCase());
  return out;
}

const LIGHT = /:root\s*\{/;
const DARK = /:root\[data-theme="dark"\]\s*\{/;

describe("tokens do protótipo", () => {
  for (const [name, sel] of [
    ["claro", LIGHT],
    ["escuro", DARK],
  ] as const) {
    it(`tema ${name}: todo token que o app compartilha com o protótipo tem o mesmo valor`, () => {
      const p = tokens(proto, sel);
      const a = tokens(css, sel);
      const shared = [...a.keys()].filter((k) => p.has(k));
      expect(shared.length).toBeGreaterThan(20);
      for (const k of shared) expect([k, a.get(k)]).toEqual([k, p.get(k)]);
    });
  }

  it("o app usa os tokens de cor, sombra e fonte do protótipo que a versão 1.0 precisa", () => {
    const a = tokens(css, LIGHT);
    for (const k of ["--ins", "--ins-bg", "--paper-fg", "--shadow", "--f-display", "--f-body", "--f-mono"]) {
      expect(a.has(k), k).toBe(true);
    }
  });

  it("realce: `#palavra-chave` e funções em roxo, como `#nome` no protótipo", () => {
    // O núcleo manda as classes do Typst sem o prefixo `typ-` (`key`, `func`, `label`, `ref`…).
    expect(css).toMatch(/\.tok-key\b[^{]*\{[^}]*color:\s*var\(--tok-fn\)/);
    expect(css).toMatch(/\.tok-func\b[^{]*\{[^}]*color:\s*var\(--tok-fn\)/);
    expect(css).toMatch(/\.tok-label\b[^{]*\{[^}]*color:\s*var\(--tok-lbl\)/);
    expect(css).toMatch(/\.tok-ref\b[^{]*\{[^}]*color:\s*var\(--tok-ref\)/);
    expect(css).toMatch(/\.tok-comment\b[^{]*\{[^}]*font-style:\s*italic/);
  });

  it("status de compilação ok em verde (--ins), como no protótipo", () => {
    expect(css).toMatch(/\.ws-compile\.is-ok\s*\{[^}]*color:\s*var\(--ins\)/);
  });
});

describe("fontes da interface", () => {
  it("cada família de --f-display, --f-body e --f-mono tem @font-face local (CSP: só 'self')", () => {
    const a = tokens(css, LIGHT);
    const first = (v: string) => v.split(",")[0]!.replace(/["']/g, "").trim();
    const families = ["--f-display", "--f-body", "--f-mono"].map((k) => first(a.get(k) ?? ""));
    // `tokens` tira os espaços: "Bricolage Grotesque" vira "bricolagegrotesque".
    expect(families).toEqual(["bricolagegrotesque", "ibmplexsans", "jetbrainsmono"]);
    const faces = fontsCss.replace(/\s+/g, "").toLowerCase();
    for (const fam of families) expect(faces).toContain(`font-family:"${fam}"`);
  });

  it("nenhuma fonte remota: só arquivos .woff2 da pasta src/fonts", () => {
    const urls = [...fontsCss.matchAll(/url\(([^)]+)\)/g)].map((m) => m[1]!.replace(/["']/g, ""));
    expect(urls.length).toBeGreaterThan(0);
    for (const u of urls) expect(u).toMatch(/^\.\/[\w-]+\.woff2$/);
    expect(css).not.toMatch(/fonts\.googleapis|fonts\.gstatic|https?:\/\//);
  });

  it("workspace.css importa as fontes", () => {
    expect(css).toMatch(/@import\s+["']\.\/fonts\/fonts\.css["']/);
  });
});
