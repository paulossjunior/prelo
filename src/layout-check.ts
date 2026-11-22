// Conferência de layout no app real (T037/T039): o jsdom não calcula layout nem aplica a CSP,
// então o autoteste mede na webview de verdade o que a pessoa vê. Pega a regressão em que a CSP
// bloqueou o tema base do CodeMirror: números de linha empilhados acima do código e editor sem
// rolagem (PR #176 só tinha rodado em jsdom).

/** Violações da CSP vistas desde o início da página (`securitypolicyviolation`). */
export function watchCspViolations(target: Pick<Document, "addEventListener"> = document): () => string[] {
  const seen: string[] = [];
  target.addEventListener("securitypolicyviolation", (ev) => {
    const e = ev as SecurityPolicyViolationEvent;
    seen.push(`CSP bloqueou ${e.effectiveDirective || e.violatedDirective}: ${e.blockedURI || "inline"}`);
  });
  return () => [...seen];
}

const scrolls = (el: Element) => /^(auto|scroll)$/.test(getComputedStyle(el).overflowY);
const TOL = 2;

/**
 * Problemas de layout da área de trabalho com um arquivo aberto no editor. Vazio quando:
 * os números de linha são uma coluna à esquerda do código, alinhados com as linhas; o editor
 * ocupa só a sua célula e rola por dentro; árvore e prévia rolam cada uma no seu painel; a
 * página inteira não rola.
 */
export function layoutProblems(doc: Document = document): string[] {
  const problems: string[] = [];
  const q = (sel: string) => doc.querySelector<HTMLElement>(sel);
  const editorBox = q(".ws-editor");
  const cm = q(".ws-editor .cm-editor");
  const scroller = q(".ws-editor .cm-scroller");
  const gutters = q(".ws-editor .cm-gutters");
  const content = q(".ws-editor .cm-content");
  const firstLine = q(".ws-editor .cm-line");
  const firstNumber = [...doc.querySelectorAll<HTMLElement>(".ws-editor .cm-lineNumbers .cm-gutterElement")].find(
    (e) => e.textContent?.trim() === "1",
  );
  if (!editorBox || !cm || !scroller || !gutters || !content || !firstLine || !firstNumber) {
    return ["editor incompleto: falta .cm-editor, .cm-scroller, .cm-gutters, .cm-content ou a linha 1"];
  }

  const g = gutters.getBoundingClientRect();
  const c = content.getBoundingClientRect();
  if (g.right > c.left + TOL || g.width <= 0) {
    problems.push(`números de linha não estão numa coluna à esquerda do código (gutter ${g.left}–${g.right}, código a partir de ${c.left})`);
  }
  const n = firstNumber.getBoundingClientRect();
  const l = firstLine.getBoundingClientRect();
  if (Math.abs(n.top - l.top) > TOL) {
    problems.push(`número da linha 1 (top ${n.top}) desalinhado da linha 1 (top ${l.top})`);
  }

  const box = editorBox.getBoundingClientRect();
  const ed = cm.getBoundingClientRect();
  if (ed.bottom > box.bottom + TOL || ed.height < box.height - TOL) {
    problems.push(`editor (altura ${ed.height}) não ocupa só a sua célula (altura ${box.height})`);
  }
  if (!scrolls(scroller)) problems.push("o editor não rola: .cm-scroller sem overflow auto");
  if (scroller.scrollHeight > scroller.clientHeight + TOL) {
    const before = scroller.scrollTop;
    scroller.scrollTop = before + 40;
    const moved = scroller.scrollTop !== before;
    scroller.scrollTop = before;
    if (!moved) problems.push("o editor tem texto além da altura visível e não rola");
  }
  if (scroller.scrollWidth > scroller.clientWidth + TOL) {
    problems.push("linha longa passa da largura do editor (a quebra de linha não funcionou)");
  }

  for (const [sel, name] of [
    [".ws-side", "árvore"],
    [".pv-pages", "prévia"],
  ] as const) {
    const pane = q(sel);
    if (!pane) {
      problems.push(`${name} ausente (${sel})`);
      continue;
    }
    if (!scrolls(pane)) problems.push(`${name} não rola no próprio painel (${sel} sem overflow auto)`);
    const cell = (pane.closest(".ws-body > *") as HTMLElement | null) ?? pane;
    if (pane.getBoundingClientRect().bottom > cell.getBoundingClientRect().bottom + TOL) {
      problems.push(`${name} passa do fim do seu painel`);
    }
  }

  const page = doc.scrollingElement ?? doc.documentElement;
  if (page.scrollHeight > page.clientHeight + TOL) {
    problems.push(`a página inteira rola (altura ${page.scrollHeight} > ${page.clientHeight})`);
  }
  return problems;
}

const pctOf = (el: Element) => parseInt(el.textContent ?? "", 10);

/**
 * Zoom da prévia no app real (T091; FR-013a): leva a prévia a 200 % pelo botão "+", confere que
 * a página ficou mais larga que o painel, que o painel rola na horizontal e que a janela não
 * rola, e devolve o zoom ao nível de antes. Vazio quando está tudo certo.
 */
export function previewZoomProblems(doc: Document = document): string[] {
  const q = <T extends HTMLElement = HTMLElement>(sel: string) => doc.querySelector<T>(sel);
  const pane = q(".pv-pages");
  const plus = q<HTMLButtonElement>(".pv-zoom-in");
  const minus = q<HTMLButtonElement>(".pv-zoom-out");
  const fit = q<HTMLButtonElement>(".pv-zoom-fit");
  const pct = q(".pv-zoom-pct");
  if (!pane || !plus || !minus || !fit || !pct) return ["barra de zoom da prévia ausente (.pv-zoom-in/out/fit/pct)"];
  if (!pane.querySelector(".pv-page")) return ["prévia sem páginas para conferir o zoom"];

  const wasFit = fit.getAttribute("aria-pressed") === "true";
  const problems: string[] = [];
  let clicks = 0;
  while (!(pctOf(pct) >= 200) && !plus.disabled && clicks < 20) {
    plus.click();
    clicks++;
  }
  if (pctOf(pct) !== 200) problems.push(`o "+" não chegou a 200 % (parou em ${pct.textContent})`);

  const page0 = pane.querySelector<HTMLElement>(".pv-page")!;
  if (pane.scrollWidth <= pane.clientWidth + TOL) {
    problems.push(
      `a 200 % a página não ficou mais larga que o painel (${pane.scrollWidth} ≤ ${pane.clientWidth}; página ${page0.style.width || "100%"})`,
    );
  }
  if (!/^(auto|scroll)$/.test(getComputedStyle(pane).overflowX)) {
    problems.push("a 200 % a prévia não rola na horizontal (.pv-pages sem overflow-x auto)");
  }
  const page = doc.scrollingElement ?? doc.documentElement;
  if (page.scrollWidth > page.clientWidth + TOL || page.scrollHeight > page.clientHeight + TOL) {
    problems.push(
      `a 200 % a janela rola (${page.scrollWidth}×${page.scrollHeight} > ${page.clientWidth}×${page.clientHeight})`,
    );
  }

  if (wasFit) fit.click();
  else for (let i = 0; i < clicks; i++) minus.click();
  return problems;
}

type Pane = "tree" | "editor" | "preview";
const PANE_NAME: Record<Pane, string> = { tree: "árvore", editor: "editor", preview: "prévia" };
const COLLAPSE: Record<Pane, string> = {
  tree: "Recolher árvore de arquivos",
  editor: "Recolher editor",
  preview: "Recolher prévia",
};
/** Painel que precisa continuar ocupando o espaço e rolando quando o outro é recolhido. */
const REMAINS: Record<Pane, { col: string; scroller: string }> = {
  tree: { col: ".ws-col-editor", scroller: ".cm-scroller" },
  editor: { col: ".ws-col-preview", scroller: ".pv-pages" },
  preview: { col: ".ws-col-editor", scroller: ".cm-scroller" },
};

/**
 * Painéis ajustáveis no app real (T093; FR-013b): recolhe cada painel pelo próprio botão,
 * confere o trilho (visível, fino), o painel que sobra (largura, rolagem própria, dentro da
 * área) e a janela (não rola), expande de novo e devolve o arranjo de antes.
 */
export function paneLayoutProblems(doc: Document = document): string[] {
  const q = <T extends HTMLElement = HTMLElement>(sel: string) => doc.querySelector<T>(sel);
  const collapse = (p: Pane) => q<HTMLButtonElement>(`button[aria-label="${COLLAPSE[p]}"]`);
  const rail = (p: Pane) => q<HTMLButtonElement>(`.ws-rail-${p}`);
  const col = (p: Pane) => q(`.ws-col-${p}`);
  const body = q(".ws-body");
  const panes: Pane[] = ["tree", "editor", "preview"];
  if (!body || panes.some((p) => !collapse(p) || !rail(p) || !col(p))) {
    return ["botões de recolher ou trilhos dos painéis ausentes (.ws-col-*, .ws-rail-*)"];
  }
  const closedBefore = panes.filter((p) => col(p)!.hidden);
  for (const p of closedBefore) rail(p)!.click();

  const problems: string[] = [];
  const page = doc.scrollingElement ?? doc.documentElement;
  for (const p of ["preview", "editor", "tree"] as const) {
    const name = PANE_NAME[p];
    collapse(p)!.click();
    if (!col(p)!.hidden) problems.push(`o botão de recolher a ${name} não a recolheu`);
    const r = rail(p)!;
    const rw = r.getBoundingClientRect().width;
    if (r.hidden || rw <= 0 || rw > 40) problems.push(`trilho da ${name} recolhida com ${rw} px (esperado de 1 a 40 px)`);
    const rest = q(REMAINS[p].col);
    const scroller = rest?.querySelector<HTMLElement>(REMAINS[p].scroller);
    const rr = rest?.getBoundingClientRect();
    if (!rest || !rr || rr.width <= 0) {
      problems.push(`com a ${name} recolhida, ${REMAINS[p].col} sumiu`);
    } else if (rr.right > body.getBoundingClientRect().right + TOL) {
      problems.push(`com a ${name} recolhida, ${REMAINS[p].col} passa da área de trabalho`);
    }
    if (scroller && !scrolls(scroller)) problems.push(`com a ${name} recolhida, ${REMAINS[p].scroller} não rola no próprio painel`);
    if (page.scrollWidth > page.clientWidth + TOL || page.scrollHeight > page.clientHeight + TOL) {
      problems.push(`com a ${name} recolhida, a janela rola (${page.scrollWidth}×${page.scrollHeight})`);
    }
    r.click();
    if (col(p)!.hidden || col(p)!.getBoundingClientRect().width <= 0) problems.push(`o trilho da ${name} não a reabriu`);
  }

  for (const p of closedBefore) collapse(p)!.click();
  return problems;
}
