// Testes da conferência de layout do autoteste (T037/T039). O jsdom não faz layout: as medidas
// são fixadas à mão, reproduzindo o app real com e sem o tema base do CodeMirror.
import { afterEach, describe, expect, it } from "vitest";
import { layoutProblems, watchCspViolations } from "./layout-check";

afterEach(() => {
  document.body.innerHTML = "";
});

type Rect = { left: number; top: number; width: number; height: number };
function rect(el: Element, r: Rect) {
  el.getBoundingClientRect = () =>
    ({ ...r, x: r.left, y: r.top, right: r.left + r.width, bottom: r.top + r.height, toJSON: () => r }) as DOMRect;
}
function scrollable(el: HTMLElement, s: { scrollHeight: number; clientHeight: number; scrollWidth?: number; clientWidth?: number; stuck?: boolean }) {
  let top = 0;
  Object.defineProperty(el, "scrollHeight", { configurable: true, value: s.scrollHeight });
  Object.defineProperty(el, "clientHeight", { configurable: true, value: s.clientHeight });
  Object.defineProperty(el, "scrollWidth", { configurable: true, value: s.scrollWidth ?? 100 });
  Object.defineProperty(el, "clientWidth", { configurable: true, value: s.clientWidth ?? 100 });
  Object.defineProperty(el, "scrollTop", {
    configurable: true,
    get: () => top,
    set: (v: number) => {
      if (!s.stuck) top = Math.max(0, Math.min(v, s.scrollHeight - s.clientHeight));
    },
  });
}

/** Área de trabalho do app com o tema base aplicado: gutter à esquerda, editor rolando. */
function workspace() {
  document.body.innerHTML = `
    <div class="ws-body">
      <aside class="ws-side" style="overflow-y: auto"></aside>
      <div class="ws-editor"><div class="cm-editor"><div class="cm-scroller" style="overflow-y: auto">
        <div class="cm-gutters"><div class="cm-gutter cm-lineNumbers"><div class="cm-gutterElement">1</div></div></div>
        <div class="cm-content"><div class="cm-line">= Título</div></div>
      </div></div></div>
      <div class="ws-preview"><div class="pv"><div class="pv-pages" style="overflow-y: auto"></div></div></div>
    </div>`;
  const $ = (s: string) => document.querySelector<HTMLElement>(s)!;
  rect($(".ws-side"), { left: 0, top: 40, width: 210, height: 600 });
  rect($(".ws-editor"), { left: 210, top: 40, width: 420, height: 600 });
  rect($(".cm-editor"), { left: 210, top: 40, width: 420, height: 600 });
  rect($(".cm-gutters"), { left: 210, top: 40, width: 30, height: 600 });
  rect($(".cm-content"), { left: 240, top: 40, width: 390, height: 2000 });
  rect($(".cm-gutterElement"), { left: 210, top: 44, width: 30, height: 21 });
  rect($(".cm-line"), { left: 240, top: 44, width: 390, height: 21 });
  rect($(".ws-preview"), { left: 630, top: 40, width: 370, height: 600 });
  rect($(".pv-pages"), { left: 630, top: 40, width: 370, height: 600 });
  scrollable($(".cm-scroller"), { scrollHeight: 2000, clientHeight: 600 });
  scrollable(document.documentElement, { scrollHeight: 700, clientHeight: 700 });
  return $;
}

describe("layoutProblems", () => {
  it("aceita o layout do app com o tema base do CodeMirror", () => {
    workspace();
    expect(layoutProblems(document)).toEqual([]);
  });

  it("recusa números de linha empilhados acima do código (tema base bloqueado pela CSP)", () => {
    const $ = workspace();
    rect($(".cm-gutters"), { left: 210, top: 40, width: 420, height: 400 });
    rect($(".cm-gutterElement"), { left: 210, top: 40, width: 420, height: 21 });
    rect($(".cm-content"), { left: 210, top: 440, width: 420, height: 400 });
    rect($(".cm-line"), { left: 210, top: 444, width: 420, height: 21 });
    const p = layoutProblems(document);
    expect(p.some((x) => x.includes("coluna à esquerda"))).toBe(true);
    expect(p.some((x) => x.includes("desalinhado"))).toBe(true);
  });

  it("recusa editor que não rola", () => {
    const $ = workspace();
    $(".cm-scroller").style.overflowY = "visible";
    scrollable($(".cm-scroller"), { scrollHeight: 2000, clientHeight: 600, stuck: true });
    const p = layoutProblems(document);
    expect(p.some((x) => x.includes("overflow auto"))).toBe(true);
    expect(p.some((x) => x.includes("não rola"))).toBe(true);
  });

  it("recusa editor mais alto que a sua célula e página que rola inteira", () => {
    const $ = workspace();
    rect($(".cm-editor"), { left: 210, top: 40, width: 420, height: 2000 });
    scrollable(document.documentElement, { scrollHeight: 2100, clientHeight: 700 });
    const p = layoutProblems(document);
    expect(p.some((x) => x.includes("sua célula"))).toBe(true);
    expect(p.some((x) => x.includes("página inteira"))).toBe(true);
  });

  it("recusa linha longa que estoura a largura (sem quebra)", () => {
    const $ = workspace();
    scrollable($(".cm-scroller"), { scrollHeight: 2000, clientHeight: 600, scrollWidth: 3000, clientWidth: 420 });
    expect(layoutProblems(document).some((x) => x.includes("linha longa"))).toBe(true);
  });

  it("recusa árvore ou prévia que não rolam no próprio painel", () => {
    const $ = workspace();
    $(".ws-side").style.overflowY = "hidden";
    rect($(".pv-pages"), { left: 630, top: 40, width: 370, height: 1400 });
    const p = layoutProblems(document);
    expect(p.some((x) => x.startsWith("árvore não rola"))).toBe(true);
    expect(p.some((x) => x.startsWith("prévia passa do fim"))).toBe(true);
  });

  it("recusa editor sem as partes do CodeMirror", () => {
    document.body.innerHTML = `<div class="ws-editor"></div>`;
    expect(layoutProblems(document)[0]).toMatch(/editor incompleto/);
  });
});

describe("watchCspViolations", () => {
  it("guarda cada violação da CSP vista na página", () => {
    const target = new EventTarget();
    const seen = watchCspViolations(target as unknown as Document);
    expect(seen()).toEqual([]);
    const ev = new Event("securitypolicyviolation");
    Object.assign(ev, { effectiveDirective: "style-src-elem", violatedDirective: "style-src", blockedURI: "inline" });
    target.dispatchEvent(ev);
    expect(seen()).toEqual(["CSP bloqueou style-src-elem: inline"]);
  });
});
