// Testes do zoom do editor (T094; FR-013c): controles no cabeçalho, degraus 70–200 %, atalhos só
// com o foco no editor, roda e pinça, tamanho da fonte do CodeMirror (código e números de linha
// juntos), cursor e linha do topo mantidos e memória por projeto.
import { EditorSelection } from "@codemirror/state";
import { afterEach, describe, expect, it, vi } from "vitest";
import { fakeCore } from "../testing/fake-core";
import { mountEditor, type EditorHandle } from "./editor";
import { BASE_FONT_PX, EDITOR_ZOOM_STEPS, mountEditorZoom, type EditorZoomHandle } from "./zoom";

let zoom: EditorZoomHandle | undefined;
let editors: EditorHandle[] = [];
afterEach(() => {
  zoom?.destroy();
  zoom = undefined;
  for (const e of editors) e.destroy();
  editors = [];
  document.body.innerHTML = "";
});

function memoryStorage() {
  const m = new Map<string, string>();
  return { getItem: (k: string) => m.get(k) ?? null, setItem: (k: string, v: string) => void m.set(k, v), map: m };
}

function newEditor(box: HTMLElement, text = Array.from({ length: 200 }, (_, i) => `linha ${i + 1}`).join("\n")) {
  const core = fakeCore({ edit_file: () => ({ version: 2 }) });
  const ed = mountEditor(box, { ipc: core.ipc, path: "main.typ", text, version: 1, onError: () => {}, onSaved: () => {}, tokenDelayMs: 0 });
  editors.push(ed);
  return ed;
}

function setup(opts: { key?: string | null; storage?: ReturnType<typeof memoryStorage> } = {}) {
  const header = document.createElement("div");
  const box = document.createElement("div");
  document.body.append(header, box);
  zoom = mountEditorZoom({ header, target: box, key: opts.key ?? null, storage: opts.storage ?? memoryStorage() });
  const ed = newEditor(box);
  zoom.attach(ed.view);
  const btn = (label: string) => header.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`)!;
  return {
    header,
    box,
    ed,
    zoom,
    zoomIn: btn("Aumentar zoom do editor"),
    zoomOut: btn("Diminuir zoom do editor"),
    reset: btn("Zoom do editor em 100%"),
    pct: header.querySelector<HTMLElement>(".ed-zoom-pct")!,
    live: header.querySelector<HTMLElement>(".ed-live")!,
  };
}

const fontPx = (ed: EditorHandle) => parseFloat(ed.view.dom.style.fontSize);

describe("controles do zoom do editor", () => {
  it("'−', porcentagem, '+' e '100%' num grupo no estilo da barra da prévia", () => {
    const { header, zoomIn, zoomOut, reset, pct } = setup();
    const group = header.querySelector('[role="group"]')!;
    expect(group.getAttribute("aria-label")).toBe("Zoom do editor");
    expect(group.classList.contains("seg")).toBe(true);
    expect([zoomOut.textContent, pct.textContent, zoomIn.textContent, reset.textContent]).toEqual(["−", "100%", "+", "100%"]);
  });

  it("degraus de 70 % a 200 %", () => {
    expect(EDITOR_ZOOM_STEPS).toEqual([70, 80, 90, 100, 110, 125, 150, 175, 200]);
  });

  it("'+' aumenta a fonte do código e dos números de linha juntos", () => {
    const { ed, zoomIn, pct, live } = setup();
    expect(fontPx(ed) || BASE_FONT_PX).toBeCloseTo(BASE_FONT_PX);
    zoomIn.click();
    expect(zoom!.level()).toBe(110);
    expect(fontPx(ed)).toBeCloseTo(BASE_FONT_PX * 1.1);
    expect(pct.textContent).toBe("110%");
    expect(live.textContent).toBe("Zoom do editor 110%");
    // Os números de linha herdam o tamanho do editor (nenhum tamanho fixo no gutter).
    const gutter = ed.view.dom.querySelector<HTMLElement>(".cm-gutters");
    expect(gutter?.style.fontSize ?? "").toBe("");
  });

  it("'−' desativa em 70 %, '+' em 200 %, '100%' volta ao padrão", () => {
    const { zoomIn, zoomOut, reset } = setup();
    for (let i = 0; i < 10; i++) zoomOut.click();
    expect(zoom!.level()).toBe(70);
    expect(zoomOut.disabled).toBe(true);
    for (let i = 0; i < 20; i++) zoomIn.click();
    expect(zoom!.level()).toBe(200);
    expect(zoomIn.disabled).toBe(true);
    reset.click();
    expect(zoom!.level()).toBe(100);
    expect(reset.getAttribute("aria-pressed")).toBe("true");
  });

  it("mantém cursor e seleção", () => {
    const { ed, zoomIn } = setup();
    ed.view.dispatch({ selection: EditorSelection.range(40, 55) });
    ed.view.scrollDOM.scrollTop = 600;
    const spy = vi.spyOn(ed.view, "dispatch");
    zoomIn.click();
    expect(ed.view.state.selection.main.from).toBe(40);
    expect(ed.view.state.selection.main.to).toBe(55);
    // A linha do topo é levada de volta ao topo depois da nova medida.
    expect(spy).toHaveBeenCalled();
  });

  it("editor novo (outro arquivo) já abre no nível atual", () => {
    const { box, zoomIn, ed } = setup();
    zoomIn.click();
    ed.destroy();
    const other = newEditor(box, "outro");
    zoom!.attach(other.view);
    expect(fontPx(other)).toBeCloseTo(BASE_FONT_PX * 1.1);
  });
});

describe("atalhos e roda só no editor", () => {
  const key = (target: HTMLElement, k: string, mods: Partial<KeyboardEventInit> = { metaKey: true }) => {
    const ev = new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true, ...mods });
    target.dispatchEvent(ev);
    return ev;
  };

  it("⌘/Ctrl + '+', '-' e '0' com o foco no editor", () => {
    const { ed } = setup();
    const ev = key(ed.view.contentDOM, "=");
    expect(ev.defaultPrevented).toBe(true);
    expect(zoom!.level()).toBe(110);
    key(ed.view.contentDOM, "-", { ctrlKey: true });
    key(ed.view.contentDOM, "-", { ctrlKey: true });
    expect(zoom!.level()).toBe(90);
    key(ed.view.contentDOM, "0");
    expect(zoom!.level()).toBe(100);
  });

  it("fora do editor as teclas não mexem no zoom do editor", () => {
    setup();
    const other = document.createElement("div");
    document.body.append(other);
    const ev = key(other, "=");
    expect(ev.defaultPrevented).toBe(false);
    expect(zoom!.level()).toBe(100);
  });

  it("⌘/Ctrl + roda: um degrau por clique; deltas pequenos (pinça) contínuos dentro de 70–200 %", () => {
    const { ed } = setup();
    const wheel = (deltaY: number) => {
      const ev = new WheelEvent("wheel", { deltaY, ctrlKey: true, bubbles: true, cancelable: true });
      ed.view.scrollDOM.dispatchEvent(ev);
      return ev;
    };
    expect(wheel(-100).defaultPrevented).toBe(true);
    expect(zoom!.level()).toBe(110);
    wheel(-10);
    expect(zoom!.level()).toBeGreaterThan(110);
    expect(zoom!.level()).toBeLessThan(125);
    for (let i = 0; i < 200; i++) wheel(10);
    expect(zoom!.level()).toBe(70);
  });

  it("roda sem modificador só rola", () => {
    const { ed } = setup();
    const ev = new WheelEvent("wheel", { deltaY: -100, bubbles: true, cancelable: true });
    ed.view.scrollDOM.dispatchEvent(ev);
    expect(ev.defaultPrevented).toBe(false);
    expect(zoom!.level()).toBe(100);
  });
});

describe("zoom do editor lembrado por projeto", () => {
  it("grava por projeto, separado do zoom da prévia", () => {
    const storage = memoryStorage();
    const a = setup({ key: "artigo", storage });
    a.zoomIn.click();
    a.zoomIn.click();
    zoom!.destroy();
    zoom = undefined;
    document.body.innerHTML = "";
    const b = setup({ key: "artigo", storage });
    expect(zoom!.level()).toBe(125);
    expect(b.pct.textContent).toBe("125%");
    expect([...storage.map.keys()].every((k) => !k.endsWith(":artigo") || k.includes("editor"))).toBe(true);
    zoom!.setKey("outro");
    expect(zoom!.level()).toBe(100);
  });

  it("armazenamento quebrado não impede o zoom", () => {
    const broken = {
      getItem: () => {
        throw new Error("SecurityError");
      },
      setItem: () => {
        throw new Error("QuotaExceededError");
      },
    };
    const { zoomIn } = setup({ key: "artigo", storage: broken as unknown as ReturnType<typeof memoryStorage> });
    zoomIn.click();
    expect(zoom!.level()).toBe(110);
  });
});
