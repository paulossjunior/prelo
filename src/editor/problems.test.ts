// Testes do painel "Erros" (T098; FR-006a, cenário 13 de US1, emenda de 2026-11-23): lista de
// todos os diagnósticos com ícone, arquivo:linha:coluna, explicação em português, "Como
// corrigir", mensagem original e dicas; itens são botões com nome acessível; região viva
// anuncia a contagem; recolhível.
import { afterEach, describe, expect, it } from "vitest";
import type { Diagnostic } from "../ipc";
import { location, mountProblems, type ProblemsHandle } from "./problems";

const MAIN = "= Título\n\n#let x = \nFim #foo.\n";

function diag(d: Partial<Diagnostic>): Diagnostic {
  return { severity: "error", message: "erro", hints: [], kind: "typst", path: "main.typ", ...d };
}

let handle: ProblemsHandle | undefined;
afterEach(() => {
  handle?.destroy();
  handle = undefined;
  document.body.innerHTML = "";
});

function setup(texts: Record<string, string> = { "main.typ": MAIN }) {
  const parent = document.createElement("div");
  document.body.append(parent);
  const selected: Diagnostic[] = [];
  handle = mountProblems(parent, {
    onSelect: (d) => selected.push(d),
    textOf: (p) => texts[p] ?? null,
  });
  return { parent, handle, selected };
}

const items = (p: ParentNode) => [...p.querySelectorAll<HTMLElement>(".pb-item")];

describe("location", () => {
  it("linha e coluna (1-based, UTF-16) a partir do intervalo quando há texto", () => {
    // "Fim #foo." começa na linha 4; `#foo` na coluna 5.
    const from = MAIN.indexOf("#foo");
    expect(location(diag({ range: { from, to: from + 4 } }), MAIN)).toEqual({ line: 4, column: 5 });
  });

  it("sem texto, usa a linha do núcleo e fica sem coluna", () => {
    expect(location(diag({ line: 3, range: { from: 10, to: 14 } }), null)).toEqual({ line: 3 });
  });

  it("sem posição nenhuma → null", () => {
    expect(location(diag({}), MAIN)).toBeNull();
  });
});

describe("painel Erros", () => {
  it("lista cada diagnóstico com ícone, local, explicação, como corrigir, original e dicas", () => {
    const { parent, handle } = setup();
    const from = MAIN.indexOf("#foo") + 1;
    handle.update([
      diag({ range: { from, to: from + 3 }, line: 4, message: "unknown variable: foo", hints: ["did you mean `for`?"] }),
      diag({ severity: "warning", path: "refs.bib", line: 2, message: "something new", hints: ["check it"] }),
    ]);
    const [a, b] = items(parent);
    expect(items(parent)).toHaveLength(2);
    expect(a!.querySelector(".prelo-icon.is-error svg")).not.toBeNull();
    expect(a!.classList.contains("is-error")).toBe(true);
    expect(a!.querySelector(".pb-loc")?.textContent).toBe("main.typ:4:6");
    expect(a!.querySelector(".pb-reason")?.textContent).toMatch(/`foo` não existe/);
    expect(a!.querySelector(".pb-fix")?.textContent).toMatch(/^Como corrigir:/);
    expect(a!.querySelector(".pb-orig")?.textContent).toContain("unknown variable: foo");
    expect(a!.querySelector(".pb-hint")?.textContent).toContain("did you mean `for`?");
    // Mensagem desconhecida: a original vira o texto principal, com as dicas; sem "Como corrigir".
    expect(b!.classList.contains("is-warning")).toBe(true);
    expect(b!.querySelector(".prelo-icon.is-warning svg")).not.toBeNull();
    expect(b!.querySelector(".pb-loc")?.textContent).toBe("refs.bib:2");
    expect(b!.querySelector(".pb-reason")?.textContent).toBe("something new");
    expect(b!.querySelector(".pb-fix")).toBeNull();
    expect(b!.querySelector(".pb-orig")).toBeNull();
    expect(b!.querySelector(".pb-hint")?.textContent).toContain("check it");
  });

  it("itens com arquivo são botões com aria-label; clique chama onSelect com o diagnóstico", () => {
    const { parent, handle, selected } = setup();
    const d = diag({ range: { from: 10, to: 14 }, line: 3, message: "expected expression" });
    handle.update([d]);
    const [a] = items(parent);
    expect(a!.tagName).toBe("BUTTON");
    expect(a!.getAttribute("aria-label")).toMatch(/^Erro em main\.typ, linha 3, coluna 1: /);
    a!.click();
    expect(selected).toEqual([d]);
  });

  it("diagnóstico sem arquivo aparece sem botão de navegação", () => {
    const { parent, handle } = setup();
    handle.update([diag({ path: undefined, kind: "project", message: "pasta do projeto não encontrada" })]);
    const [a] = items(parent);
    expect(a!.tagName).not.toBe("BUTTON");
    expect(a!.textContent).toContain("pasta do projeto não encontrada");
  });

  it("não interpreta HTML das mensagens", () => {
    const { parent, handle } = setup();
    handle.update([diag({ message: "<img src=x onerror=alert(1)>", hints: ["<b>x</b>"] })]);
    expect(parent.querySelector("img")).toBeNull();
    expect(parent.querySelector("b")).toBeNull();
  });

  it("cabeçalho com a contagem; recolher e abrir pelo botão com aria-expanded", () => {
    const { parent, handle } = setup();
    handle.update([diag({ line: 1 }), diag({ line: 2 }), diag({ severity: "warning", line: 3 })]);
    const toggle = parent.querySelector<HTMLButtonElement>(".pb-toggle")!;
    expect(toggle.textContent).toMatch(/Erros/);
    expect(parent.querySelector(".pb-count")?.textContent).toBe("3");
    const list = parent.querySelector<HTMLElement>(".pb-list")!;
    expect(toggle.getAttribute("aria-controls")).toBe(list.id);
    // Começa recolhido.
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    expect(list.hidden).toBe(true);
    toggle.click();
    expect(toggle.getAttribute("aria-expanded")).toBe("true");
    expect(list.hidden).toBe(false);
    expect(handle.isOpen()).toBe(true);
    toggle.click();
    expect(list.hidden).toBe(true);
  });

  it("open() abre o painel e põe o foco no primeiro item", () => {
    const { parent, handle } = setup();
    handle.update([diag({ line: 1 })]);
    handle.open();
    expect(parent.querySelector<HTMLElement>(".pb-list")!.hidden).toBe(false);
    expect(document.activeElement).toBe(items(parent)[0]);
  });

  it("sem diagnósticos mostra 'Nenhum erro.'", () => {
    const { parent, handle } = setup();
    handle.update([]);
    handle.open();
    expect(items(parent)).toHaveLength(0);
    expect(parent.querySelector(".pb-empty")?.textContent).toBe("Nenhum erro.");
  });

  it("região viva anuncia a contagem só quando ela muda", () => {
    const { parent, handle } = setup();
    const live = parent.querySelector<HTMLElement>(".pb-live")!;
    expect(live.getAttribute("role")).toBe("status");
    expect(live.getAttribute("aria-live")).toBe("polite");
    handle.update([diag({ line: 1 }), diag({ line: 2 })]);
    expect(live.textContent).toBe("2 erros no documento.");
    live.textContent = "";
    handle.update([diag({ line: 1, message: "outro" }), diag({ line: 5 })]);
    expect(live.textContent).toBe("");
    handle.update([diag({ line: 1 }), diag({ severity: "warning", line: 2 })]);
    expect(live.textContent).toBe("1 erro e 1 aviso no documento.");
    handle.update([]);
    expect(live.textContent).toBe("Nenhum erro no documento.");
  });
});
