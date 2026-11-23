// Testes do botão "Compilar" e do atalho ⌘⇧B / Ctrl+Shift+B (T100; FR-014a, cenário 14 de US1;
// contracts/ipc.md `compile_now`). Núcleo falso: src/testing/fake-core.ts.
import { closeBracketsKeymap, completionKeymap } from "@codemirror/autocomplete";
import { defaultKeymap, historyKeymap, indentWithTab } from "@codemirror/commands";
import { lintKeymap } from "@codemirror/lint";
import { searchKeymap } from "@codemirror/search";
import type { KeyBinding } from "@codemirror/view";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ERROR_MESSAGES, type IpcError, type PreviewUpdate } from "../ipc";
import { deferred, fakeCore, settle } from "../testing/fake-core";
import { COMPILE_NOW_LABEL, mountCompileNow, type CompileNowHandle } from "./compile-now";

let handles: CompileNowHandle[] = [];
afterEach(() => {
  for (const h of handles) h.destroy();
  handles = [];
  document.body.innerHTML = "";
});

function update(compileId: number, over: Partial<PreviewUpdate> = {}): PreviewUpdate {
  return { compileId, ok: true, pages: [], changed: [], diagnostics: [], durationMs: 12, ...over };
}

function setup(compileNow: () => unknown = () => ({ compileId: 7 })) {
  const order: string[] = [];
  const core = fakeCore({
    compile_now: () => {
      order.push("compile_now");
      return compileNow();
    },
  });
  const flush = vi.fn(async () => {
    order.push("flush");
  });
  const errors: IpcError[] = [];
  const h = mountCompileNow({ ipc: core.ipc, flush, onError: (e) => errors.push(e) });
  handles.push(h);
  document.body.append(h.button);
  h.setAvailable(true);
  return { core, h, flush, order, errors };
}

const key = (over: KeyboardEventInit) =>
  new KeyboardEvent("keydown", { key: "B", code: "KeyB", shiftKey: true, bubbles: true, cancelable: true, ...over });

describe("botão Compilar", () => {
  it("tem nome visível, nome acessível e dica com o atalho", () => {
    const { h } = setup();
    expect(COMPILE_NOW_LABEL).toBe("Compilar agora (⌘⇧B / Ctrl+Shift+B)");
    expect(h.button.type).toBe("button");
    expect(h.button.textContent).toContain("Compilar");
    expect(h.button.getAttribute("aria-label")).toBe(COMPILE_NOW_LABEL);
    expect(h.button.title).toBe(COMPILE_NOW_LABEL);
    expect(h.button.disabled).toBe(false);
  });

  it("símbolo em SVG pelo DOM, em currentColor, sem estilo embutido nem data:", () => {
    const { h } = setup();
    const svg = h.button.querySelector("svg");
    expect(svg).not.toBeNull();
    expect(svg!.namespaceURI).toBe("http://www.w3.org/2000/svg");
    expect(svg!.getAttribute("aria-hidden")).toBe("true");
    const painted = [svg!, ...svg!.querySelectorAll("*")].map(
      (e) => `${e.getAttribute("fill") ?? ""} ${e.getAttribute("stroke") ?? ""}`,
    );
    expect(painted.join(" ")).toContain("currentColor");
    expect(h.button.outerHTML).not.toMatch(/style=|data:/);
  });

  it("clique envia as edições pendentes antes de pedir compile_now", async () => {
    const { core, h, order } = setup();
    h.button.click();
    await settle();
    expect(order).toEqual(["flush", "compile_now"]);
    expect(core.calls("compile_now")[0]?.args).toEqual({});
  });

  it("mostra Compilando… e fica desativado até a prévia da compilação pedida", async () => {
    const { h } = setup();
    h.button.click();
    await settle();
    expect(h.button.disabled).toBe(true);
    expect(h.button.textContent).toContain("Compilando…");
    expect(h.button.getAttribute("aria-busy")).toBe("true");
    expect(h.button.querySelector(".cn-spin")).not.toBeNull();
    // Prévia de uma compilação mais antiga não libera.
    h.update(update(6));
    expect(h.button.disabled).toBe(true);
    h.update(update(7, { ok: false }));
    expect(h.button.disabled).toBe(false);
    expect(h.button.textContent).toContain("Compilar");
    expect(h.button.textContent).not.toContain("Compilando");
    expect(h.button.getAttribute("aria-busy")).toBe("false");
  });

  it("uma edição durante a espera não prende o botão: prévia mais nova também libera", async () => {
    const { h } = setup();
    h.button.click();
    await settle();
    h.update(update(9));
    expect(h.button.disabled).toBe(false);
  });

  it("prévia que chega antes da resposta de compile_now também libera", async () => {
    const answer = deferred<unknown>();
    const { h } = setup(() => answer.promise);
    h.button.click();
    await settle();
    expect(h.button.disabled).toBe(true);
    h.update(update(7));
    answer.resolve({ compileId: 7 });
    await settle();
    expect(h.button.disabled).toBe(false);
  });

  it("recusa do núcleo reativa o botão e mostra a mensagem em português", async () => {
    const { h, errors } = setup(() => {
      throw "compile_suspended";
    });
    h.button.click();
    await settle();
    expect(h.button.disabled).toBe(false);
    expect(errors.map((e) => e.code)).toEqual(["compile_suspended"]);
    expect(errors[0]!.message).toBe(ERROR_MESSAGES.compile_suspended);
  });

  it("clique repetido durante a espera não pede de novo", async () => {
    const { core, h } = setup();
    h.button.click();
    await settle();
    h.button.click();
    await h.run();
    await settle();
    expect(core.calls("compile_now")).toHaveLength(1);
  });

  it("sem projeto (ou com versão diferente) fica desativado e não compila", async () => {
    const { core, h } = setup();
    h.setAvailable(false);
    expect(h.button.disabled).toBe(true);
    document.dispatchEvent(key({ metaKey: true }));
    await h.run();
    await settle();
    expect(core.calls("compile_now")).toHaveLength(0);
    h.setAvailable(true);
    expect(h.button.disabled).toBe(false);
  });
});

describe("atalho ⌘⇧B / Ctrl+Shift+B", () => {
  it.each([
    ["⌘⇧B", { metaKey: true }],
    ["Ctrl+Shift+B", { ctrlKey: true }],
    ["minúscula", { ctrlKey: true, key: "b" }],
  ])("%s compila e evita o padrão do navegador", async (_name, over) => {
    const { core, order } = setup();
    const e = key(over as KeyboardEventInit);
    document.body.dispatchEvent(e);
    await settle();
    expect(e.defaultPrevented).toBe(true);
    expect(order).toEqual(["flush", "compile_now"]);
    expect(core.calls("compile_now")).toHaveLength(1);
  });

  it.each([
    ["sem Shift", { metaKey: true, shiftKey: false }],
    ["sem ⌘/Ctrl", {}],
    ["com Alt", { metaKey: true, altKey: true }],
    ["outra tecla", { metaKey: true, key: "N", code: "KeyN" }],
  ])("%s não faz nada", async (_name, over) => {
    const { core } = setup();
    const e = key(over as KeyboardEventInit);
    document.body.dispatchEvent(e);
    await settle();
    expect(e.defaultPrevented).toBe(false);
    expect(core.calls("compile_now")).toHaveLength(0);
  });

  it("vale com o foco dentro do editor", async () => {
    const { core } = setup();
    const content = document.createElement("div");
    content.className = "cm-content";
    content.contentEditable = "true";
    document.body.append(content);
    content.dispatchEvent(key({ ctrlKey: true }));
    await settle();
    expect(core.calls("compile_now")).toHaveLength(1);
  });

  it("destroy remove o atalho", async () => {
    const { core, h } = setup();
    h.destroy();
    document.body.dispatchEvent(key({ metaKey: true }));
    await settle();
    expect(core.calls("compile_now")).toHaveLength(0);
  });

  it("nenhum mapa de teclas do editor usa Mod-Shift-b", () => {
    const normalize = (k: string) =>
      k
        .split("-")
        .map((p) => p.toLowerCase())
        .sort()
        .join("-");
    const taken = new Set(["mod-shift-b", "cmd-shift-b", "ctrl-shift-b", "meta-shift-b"].map(normalize));
    const maps: readonly KeyBinding[] = [
      ...closeBracketsKeymap,
      ...defaultKeymap,
      ...searchKeymap,
      ...historyKeymap,
      ...completionKeymap,
      ...lintKeymap,
      indentWithTab,
    ];
    const clashes = maps.flatMap((b) =>
      [b.key, b.mac, b.win, b.linux].filter((k): k is string => !!k && taken.has(normalize(k))),
    );
    expect(clashes).toEqual([]);
  });
});
