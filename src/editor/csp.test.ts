// Estilos do CodeMirror sob a CSP `style-src 'self'` (T037; contracts/ipc.md, "Permissões da
// janela"). O CodeMirror injeta o tema base num <style>; sem nonce a webview real o bloqueia e o
// editor fica sem layout (números de linha empilhados acima do código, sem rolagem). O Tauri
// põe um nonce aleatório, a cada carga da página, em todo <style> de index.html e na CSP servida;
// o editor reaproveita esse nonce (`EditorView.cspNonce`).
import { readFileSync } from "node:fs";
import { afterEach, describe, expect, it } from "vitest";
import { fakeCore } from "../testing/fake-core";
import { mountEditor, type EditorHandle } from "./editor";
import { STYLE_NONCE_TOKEN, styleNonce } from "./csp";

let handles: EditorHandle[] = [];
afterEach(() => {
  for (const h of handles) h.destroy();
  handles = [];
  document.body.innerHTML = "";
  document.head.querySelectorAll("style[data-csp-nonce]").forEach((e) => e.remove());
});

function nonceCarrier(nonce: string) {
  const el = document.createElement("style");
  el.setAttribute("data-csp-nonce", "");
  el.setAttribute("nonce", nonce);
  document.head.append(el);
  return el;
}

describe("nonce de estilo", () => {
  it("index.html tem o <style> vazio onde o Tauri põe o nonce", () => {
    const html = readFileSync(new URL("../../index.html", import.meta.url), "utf8");
    expect(html).toMatch(/<style data-csp-nonce><\/style>/);
  });

  it("lê o nonce que o Tauri pôs no <style> marcado", () => {
    nonceCarrier("1234567890");
    expect(styleNonce(document)).toBe("1234567890");
  });

  it("sem nonce (ou com o marcador não substituído) não inventa um", () => {
    expect(styleNonce(document)).toBeUndefined();
    nonceCarrier(STYLE_NONCE_TOKEN);
    expect(styleNonce(document)).toBeUndefined();
  });

  it("o <style> que o CodeMirror injeta leva o nonce, e o tema base está nele", () => {
    nonceCarrier("987654321");
    const parent = document.createElement("div");
    document.body.append(parent);
    handles.push(
      mountEditor(parent, {
        ipc: fakeCore({}).ipc,
        path: "main.typ",
        text: "= Título\n\nOlá.",
        version: 1,
        tokenDelayMs: 0,
      }),
    );
    const injected = [...document.head.querySelectorAll("style")].filter(
      (s) => !s.hasAttribute("data-csp-nonce") && /\.cm-gutters/.test(s.textContent ?? ""),
    );
    expect(injected).toHaveLength(1);
    expect(injected[0].getAttribute("nonce")).toBe("987654321");
  });
});
