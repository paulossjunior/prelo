// Ícones de gravidade (erro, aviso, informação) desenhados em SVG pelo DOM (FR-006a; regressão
// do marcador da margem). Sem `data:` (a CSP `img-src` não libera) e sem atributo `style` (a
// CSP `style-src` não libera 'unsafe-inline'): a forma vem de atributos SVG e a cor de
// workspace.css (`.prelo-icon.is-error { color: var(--err) }`, `fill: currentColor`).
export type Severity = "error" | "warning" | "info";

const NS = "http://www.w3.org/2000/svg";

function node(tag: string, attrs: Record<string, string | number>): SVGElement {
  const e = document.createElementNS(NS, tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, String(v));
  return e;
}

/** Fundo (na cor da gravidade) e símbolo (na cor da superfície) de cada ícone, num quadro 16×16. */
function shapes(sev: Severity): SVGElement[] {
  if (sev === "warning") {
    return [
      node("path", { class: "pi-bg", d: "M8 1.6 15 14.4H1Z", "stroke-linejoin": "round" }),
      node("rect", { class: "pi-fg", x: 7.1, y: 5.6, width: 1.8, height: 4.8, rx: 0.9 }),
      node("circle", { class: "pi-fg", cx: 8, cy: 12.3, r: 1 }),
    ];
  }
  if (sev === "info") {
    return [
      node("circle", { class: "pi-bg", cx: 8, cy: 8, r: 7 }),
      node("circle", { class: "pi-fg", cx: 8, cy: 4.6, r: 1.1 }),
      node("rect", { class: "pi-fg", x: 7.1, y: 6.6, width: 1.8, height: 5.6, rx: 0.9 }),
    ];
  }
  return [
    node("circle", { class: "pi-bg", cx: 8, cy: 8, r: 7 }),
    node("rect", { class: "pi-fg", x: 7.1, y: 3.6, width: 1.8, height: 5.6, rx: 0.9 }),
    node("circle", { class: "pi-fg", cx: 8, cy: 11.6, r: 1.1 }),
  ];
}

/**
 * `<span class="prelo-icon is-{sev}">` com o SVG. Sem `title`, é decorativo (`aria-hidden`); com
 * `title`, o texto vai no atributo `title` (dica ao passar o ponteiro).
 */
export function severityIcon(sev: Severity, title?: string): HTMLSpanElement {
  const span = document.createElement("span");
  span.className = `prelo-icon is-${sev}`;
  span.setAttribute("aria-hidden", "true");
  if (title) span.title = title;
  const svg = node("svg", { viewBox: "0 0 16 16", width: 16, height: 16, focusable: "false" });
  svg.append(...shapes(sev));
  span.append(svg);
  return span;
}
