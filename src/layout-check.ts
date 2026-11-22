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
