// Site público (GitHub Pages) em site/: HTML estático, sem terceiros, acessível e leve.
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { extname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { JSDOM } from "jsdom";
import { describe, expect, it } from "vitest";

const root = fileURLToPath(new URL("..", import.meta.url));
const SITE = join(root, "site");
const INDEX = join(SITE, "index.html");
const MAX_IMAGE_BYTES = 600 * 1024;
const IMAGE_EXT = new Set([".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg", ".avif"]);
const VOID = new Set(["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"]);

function html(): string {
  expect(existsSync(INDEX), "site/index.html não existe").toBe(true);
  return readFileSync(INDEX, "utf8");
}

const doc = () => new JSDOM(html()).window.document;

function files(dir: string): string[] {
  if (!existsSync(dir)) return [];
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? files(path) : [path];
  });
}

const isExternal = (url: string) => /^[a-z][a-z0-9+.-]*:/i.test(url) || url.startsWith("//");

/** Pilha de tags sobre o texto bruto: o parser do jsdom corrige erros e não os revela. */
function unbalancedTags(source: string): string[] {
  const text = source
    .replace(/<!--[\s\S]*?-->/g, "")
    .replace(/<(style|script)\b[^>]*>[\s\S]*?<\/\1>/gi, "")
    .replace(/<!doctype[^>]*>/i, "");
  const stack: string[] = [];
  const errors: string[] = [];
  for (const m of text.matchAll(/<(\/?)([a-zA-Z][a-zA-Z0-9-]*)\b[^>]*?(\/?)>/g)) {
    const [, closing, rawTag] = m;
    const tag = (rawTag ?? "").toLowerCase();
    if (VOID.has(tag)) continue;
    if (!closing) {
      stack.push(tag);
      continue;
    }
    const open = stack.pop();
    if (open !== tag) errors.push(`</${tag}> fecha <${open ?? "nada"}>`);
  }
  return [...errors, ...stack.map((t) => `<${t}> sem fechamento`)];
}

describe("site/index.html: estrutura", () => {
  it("tem todas as tags fechadas na ordem certa", () => {
    expect(unbalancedTags(html())).toEqual([]);
  });

  it("declara lang=pt-BR e tem um único h1", () => {
    const d = doc();
    expect(d.documentElement.getAttribute("lang")).toBe("pt-BR");
    expect(d.querySelectorAll("h1")).toHaveLength(1);
  });

  it("não repete ids", () => {
    const ids = [...doc().querySelectorAll("[id]")].map((e) => e.id);
    const repeated = ids.filter((id, i) => ids.indexOf(id) !== i);
    expect(repeated).toEqual([]);
  });

  it("tem os landmarks e o link para pular ao conteúdo", () => {
    const d = doc();
    for (const sel of ["header", "nav", "main", "footer"]) expect(d.querySelector(sel), sel).not.toBeNull();
    const skip = d.querySelector('a[href="#conteudo"]');
    expect(skip).not.toBeNull();
    expect(d.getElementById("conteudo")?.tagName).toBe("MAIN");
  });
});

describe("site/index.html: links e arquivos", () => {
  it("toda âncora # aponta para um id existente", () => {
    const d = doc();
    const missing = [...d.querySelectorAll("a[href^='#']")]
      .map((a) => a.getAttribute("href")!.slice(1))
      .filter((id) => !id || !d.getElementById(decodeURIComponent(id)));
    expect(missing).toEqual([]);
  });

  it("todo src e href local existe em site/", () => {
    const d = doc();
    const refs = [
      ...[...d.querySelectorAll("[src]")].map((e) => e.getAttribute("src")!),
      ...[...d.querySelectorAll("[href]")].map((e) => e.getAttribute("href")!),
    ].filter((u) => !u.startsWith("#") && !isExternal(u));
    const missing = refs.filter((u) => !existsSync(join(SITE, decodeURIComponent(u.split(/[?#]/)[0] ?? ""))));
    expect(refs.length).toBeGreaterThan(0);
    expect(missing).toEqual([]);
  });

  it("não carrega script nem folha de estilo de fora", () => {
    const d = doc();
    const scripts = [...d.querySelectorAll("script[src]")].map((s) => s.getAttribute("src")!).filter(isExternal);
    const styles = [...d.querySelectorAll("link")]
      .filter((l) => /stylesheet|preload|modulepreload/i.test(l.getAttribute("rel") ?? ""))
      .map((l) => l.getAttribute("href")!)
      .filter(isExternal);
    expect(scripts).toEqual([]);
    expect(styles).toEqual([]);
    expect(html()).not.toMatch(/@import\s+url\(\s*["']?https?:/i);
  });

  it("abre links externos com rel=noopener", () => {
    const bad = [...doc().querySelectorAll("a[href]")]
      .filter((a) => /^https?:/i.test(a.getAttribute("href")!))
      .filter((a) => !(a.getAttribute("rel") ?? "").split(/\s+/).includes("noopener"))
      .map((a) => a.getAttribute("href"));
    expect(bad).toEqual([]);
  });
});

describe("site/: imagens", () => {
  it("toda <img> tem alt não vazio, width e height", () => {
    const imgs = [...doc().querySelectorAll("img")];
    expect(imgs.length).toBeGreaterThan(0);
    const bad = imgs
      .filter((i) => !i.getAttribute("alt")?.trim() || !/^\d+$/.test(i.getAttribute("width") ?? "") || !/^\d+$/.test(i.getAttribute("height") ?? ""))
      .map((i) => i.getAttribute("src"));
    expect(bad).toEqual([]);
  });

  it("nenhuma imagem passa de 600 KB", () => {
    const heavy = files(SITE)
      .filter((f) => IMAGE_EXT.has(extname(f).toLowerCase()))
      .filter((f) => statSync(f).size > MAX_IMAGE_BYTES)
      .map((f) => `${relative(SITE, f)} (${Math.round(statSync(f).size / 1024)} KB)`);
    expect(heavy).toEqual([]);
  });
});

describe("site/index.html: honestidade e licença", () => {
  it("avisa no topo que o projeto está em desenvolvimento", () => {
    const status = doc().querySelector(".status")?.textContent ?? "";
    expect(status).toMatch(/em desenvolvimento/i);
  });

  it("não é mais a proposta de revisão", () => {
    expect(html()).not.toMatch(/Proposta de site para revisão/);
  });

  it("mostra a licença MIT OR Apache-2.0 no rodapé, com links para os arquivos", () => {
    const footer = doc().querySelector("footer")!;
    expect(footer.textContent).toMatch(/MIT OR Apache-2\.0/);
    expect(footer.textContent).not.toMatch(/ainda não escolhida/i);
    const hrefs = [...footer.querySelectorAll("a")].map((a) => a.getAttribute("href") ?? "");
    expect(hrefs.some((h) => h.endsWith("LICENSE-MIT"))).toBe(true);
    expect(hrefs.some((h) => h.endsWith("LICENSE-APACHE"))).toBe(true);
  });

  it("oferece a v0.1.0 publicada, avisando que é só o esqueleto e não é assinada", () => {
    const d = doc();
    const cta = [...d.querySelectorAll("a")].find((a) => /Baixar a v0\.1\.0/.test(a.textContent ?? ""));
    expect(cta?.getAttribute("href")).toBe("https://github.com/paulossjunior/prelo/releases/latest");
    expect(html()).not.toMatch(/Downloads: em breve/);
    const note = d.getElementById("nota-download")?.textContent ?? "";
    expect(note).toMatch(/esqueleto/);
    expect(note).toMatch(/não é assinado/);
    const notes = d.querySelector("#nota-download a[href$='/releases/tag/v0.1.0']");
    expect(notes).not.toBeNull();
  });
});
