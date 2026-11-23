// Testes de diagnósticos no editor (T030; FR-006, cenário 4 de US1): o erro aparece no trecho
// e na linha certos, com a mensagem e as dicas.
import { forEachDiagnostic } from "@codemirror/lint";
import { Text } from "@codemirror/state";
import { afterEach, describe, expect, it } from "vitest";
import type { Diagnostic } from "../ipc";
import { fakeCore } from "../testing/fake-core";
import { toLintDiagnostics } from "./diagnostics";
import { mountEditor, type EditorHandle } from "./editor";

const DOC = Text.of(["= Título", "", "#let x = ", "Fim."]);

function diag(d: Partial<Diagnostic>): Diagnostic {
  return { severity: "error", message: "erro", hints: [], kind: "typst", path: "main.typ", ...d };
}

describe("toLintDiagnostics", () => {
  it("usa o intervalo do diagnóstico e a severidade", () => {
    const out = toLintDiagnostics(
      [diag({ range: { from: 10, to: 14 }, message: "expected expression", hints: [] })],
      "main.typ",
      DOC,
    );
    expect(out).toEqual([
      expect.objectContaining({ from: 10, to: 14, severity: "error", message: "expected expression" }),
    ]);
  });

  it("inclui as dicas na mensagem", () => {
    const [d] = toLintDiagnostics(
      [diag({ range: { from: 0, to: 1 }, message: "unknown variable", hints: ["try `x`", "see docs"] })],
      "main.typ",
      DOC,
    );
    expect(d?.message).toBe("unknown variable\nDica: try `x`\nDica: see docs");
  });

  it("aviso vira warning", () => {
    const [d] = toLintDiagnostics(
      [diag({ severity: "warning", range: { from: 0, to: 1 } })],
      "main.typ",
      DOC,
    );
    expect(d?.severity).toBe("warning");
  });

  it("só com linha (1-based) marca a linha inteira", () => {
    const [d] = toLintDiagnostics([diag({ line: 3 })], "main.typ", DOC);
    expect(d).toMatchObject({ from: DOC.line(3).from, to: DOC.line(3).to });
  });

  it("ignora diagnósticos de outro arquivo ou sem posição", () => {
    const out = toLintDiagnostics(
      [
        diag({ path: "cap/a.typ", range: { from: 0, to: 1 } }),
        diag({ path: "main.typ" }),
        diag({ path: undefined, range: { from: 0, to: 1 } }),
      ],
      "main.typ",
      DOC,
    );
    expect(out).toEqual([]);
  });

  it("tooltip do sublinhado mostra a explicação em português, como corrigir, original e dicas (FR-006a)", () => {
    const [d] = toLintDiagnostics(
      [diag({ range: { from: 0, to: 1 }, message: "unknown variable: foo", hints: ["did you mean `for`?"] })],
      "main.typ",
      DOC,
    );
    expect(d?.renderMessage).toBeTypeOf("function");
    const node = d!.renderMessage!({} as never) as HTMLElement;
    expect(node.querySelector(".pb-reason")?.textContent).toMatch(/`foo` não existe/);
    expect(node.querySelector(".pb-fix")?.textContent).toMatch(/^Como corrigir:/);
    expect(node.querySelector(".pb-orig")?.textContent).toContain("unknown variable: foo");
    expect(node.querySelector(".pb-hint")?.textContent).toContain("did you mean `for`?");
  });

  it("tooltip de mensagem desconhecida mostra a original e as dicas", () => {
    const [d] = toLintDiagnostics(
      [diag({ range: { from: 0, to: 1 }, message: "brand new error", hints: ["h1"] })],
      "main.typ",
      DOC,
    );
    const node = d!.renderMessage!({} as never) as HTMLElement;
    expect(node.querySelector(".pb-reason")?.textContent).toBe("brand new error");
    expect(node.querySelector(".pb-fix")).toBeNull();
    expect(node.querySelector(".pb-hint")?.textContent).toContain("h1");
  });

  it("intervalo fora do texto é limitado ao fim do texto", () => {
    const [d] = toLintDiagnostics([diag({ range: { from: 20, to: 900 } })], "main.typ", DOC);
    expect(d).toMatchObject({ from: 20, to: DOC.length });
    const [e] = toLintDiagnostics([diag({ range: { from: 900, to: 950 } })], "main.typ", DOC);
    expect(e).toMatchObject({ from: DOC.length, to: DOC.length });
  });
});

describe("diagnósticos no editor", () => {
  let editor: EditorHandle | undefined;
  afterEach(() => {
    editor?.destroy();
    document.body.innerHTML = "";
  });

  function mount() {
    const parent = document.createElement("div");
    document.body.append(parent);
    editor = mountEditor(parent, {
      ipc: fakeCore().ipc,
      path: "main.typ",
      text: DOC.toString(),
      version: 1,
      tokenDelayMs: 0,
    });
    return { parent, editor };
  }

  it("marca o trecho com o erro no texto", () => {
    const { parent, editor } = mount();
    editor.setDiagnostics([diag({ range: { from: 10, to: 14 }, message: "expected expression" })]);
    const seen: { from: number; to: number; message: string }[] = [];
    forEachDiagnostic(editor.view.state, (d, from, to) => seen.push({ from, to, message: d.message }));
    expect(seen).toEqual([{ from: 10, to: 14, message: "expected expression" }]);
    const mark = parent.querySelector(".cm-lintRange-error");
    expect(mark?.textContent).toBe("#let");
  });

  it("margem com ícone desenhado em SVG por severidade, sem o marcador `data:` do lint (regressão)", () => {
    const { parent, editor } = mount();
    editor.setDiagnostics([
      diag({ range: { from: 10, to: 14 }, message: "expected expression" }),
      diag({ severity: "warning", range: { from: 20, to: 24 }, message: "aviso" }),
    ]);
    // O marcador do @codemirror/lint usa `content: url(data:…)`, bloqueado pela CSP: no WebKit
    // vira um quadrado vazio. Não deve existir.
    expect(parent.querySelector(".cm-lint-marker")).toBeNull();
    const err = parent.querySelector<HTMLElement>(".prelo-lint-gutter .prelo-icon.is-error");
    const warn = parent.querySelector<HTMLElement>(".prelo-lint-gutter .prelo-icon.is-warning");
    expect(err?.querySelector("svg")).not.toBeNull();
    expect(warn?.querySelector("svg")).not.toBeNull();
    expect(err?.getAttribute("title")).toBe("expected expression");
    // Sem atributo `style` (CSP `style-src` sem 'unsafe-inline').
    expect(err?.querySelector("[style]")).toBeNull();
    editor.setDiagnostics([]);
    expect(parent.querySelector(".prelo-lint-gutter .prelo-icon")).toBeNull();
  });

  it("lista nova substitui a anterior; lista vazia limpa", () => {
    const { parent, editor } = mount();
    editor.setDiagnostics([diag({ range: { from: 0, to: 1 } })]);
    editor.setDiagnostics([diag({ severity: "warning", range: { from: 20, to: 24 } })]);
    expect(parent.querySelector(".cm-lintRange-error")).toBeNull();
    expect(parent.querySelector(".cm-lintRange-warning")?.textContent).toBe("Fim.");
    editor.setDiagnostics([]);
    expect(parent.querySelector(".cm-lintRange-warning")).toBeNull();
  });
});
