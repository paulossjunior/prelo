// Testes dos painéis ajustáveis da área de trabalho (T092; FR-013b): divisórias arrastáveis e
// por teclado, recolher/expandir cada painel com trilho para reabrir, modos só editor / só
// prévia / ambos, atalhos, posição de rolagem preservada e layout lembrado por projeto.
import { afterEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_LAYOUT, MIN_PANE, MIN_TREE, MAX_TREE, RAIL, mountPanes, type PanesHandle } from "./panes";

let handle: PanesHandle | undefined;
afterEach(() => {
  handle?.destroy();
  handle = undefined;
  document.body.innerHTML = "";
});

function memoryStorage() {
  const m = new Map<string, string>();
  return { getItem: (k: string) => m.get(k) ?? null, setItem: (k: string, v: string) => void m.set(k, v), map: m };
}

type Rect = { left: number; width: number };
function rect(el: Element, r: Rect) {
  el.getBoundingClientRect = () =>
    ({ left: r.left, width: r.width, right: r.left + r.width, top: 0, bottom: 600, height: 600, x: r.left, y: 0 }) as DOMRect;
}

function setup(opts: { key?: string | null; storage?: ReturnType<typeof memoryStorage> | null; onChange?: () => void } = {}) {
  const body = document.createElement("div");
  body.className = "ws-body";
  const mk = (cls: string) => {
    const col = document.createElement("div");
    col.className = cls;
    const header = document.createElement("div");
    header.className = "ws-pane-h";
    col.append(header);
    return { col, header };
  };
  const tree = mk("ws-col-tree");
  const editor = mk("ws-col-editor");
  const preview = mk("ws-col-preview");
  const scroller = document.createElement("div");
  scroller.className = "cm-scroller";
  editor.col.append(scroller);
  const pages = document.createElement("div");
  pages.className = "pv-pages";
  preview.col.append(pages);
  body.append(tree.col, editor.col, preview.col);
  const top = document.createElement("header");
  document.body.append(top, body);
  handle = mountPanes({
    body,
    tree: tree.col,
    editor: editor.col,
    preview: preview.col,
    treeHeader: tree.header,
    editorHeader: editor.header,
    storage: opts.storage === undefined ? memoryStorage() : opts.storage,
    key: opts.key ?? null,
    onChange: opts.onChange,
  });
  top.append(handle.modeControl);
  preview.header.append(handle.previewButton);
  const btn = (label: string) => document.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`);
  return { body, tree, editor, preview, scroller, pages, panes: handle, btn };
}

const cols = (body: HTMLElement) => body.style.gridTemplateColumns;

describe("recolher e expandir cada painel", () => {
  it("cada painel tem o seu botão de recolher, com rótulo e aria-expanded", () => {
    const { btn } = setup();
    for (const label of ["Recolher árvore de arquivos", "Recolher editor", "Recolher prévia"]) {
      const b = btn(label);
      expect(b, label).not.toBeNull();
      expect(b!.getAttribute("aria-expanded")).toBe("true");
    }
    expect(btn("Recolher árvore de arquivos")!.textContent).toBe("‹");
    expect(btn("Recolher prévia")!.textContent).toBe("›");
  });

  it("painel recolhido vira um trilho com botão de expandir e o nome do painel", () => {
    const { btn, tree, body, panes } = setup();
    btn("Recolher árvore de arquivos")!.click();
    expect(panes.layout().tree).toBe(false);
    expect(tree.col.hidden).toBe(true);
    const rail = btn("Expandir árvore de arquivos")!;
    expect(rail).not.toBeNull();
    expect(rail.hidden).toBe(false);
    expect(rail.getAttribute("aria-expanded")).toBe("false");
    expect(rail.textContent).toContain("Arquivos");
    expect(cols(body).startsWith(`${RAIL}px`)).toBe(true);
    rail.click();
    expect(panes.layout().tree).toBe(true);
    expect(tree.col.hidden).toBe(false);
    expect(rail.hidden).toBe(true);
  });

  it("recolher a prévia deixa o editor ocupar o espaço; o trilho da prévia fica à direita", () => {
    const { btn, preview, body } = setup();
    btn("Recolher prévia")!.click();
    expect(preview.col.hidden).toBe(true);
    expect(cols(body)).toMatch(/minmax\(0(px)?, 1fr\) 28px$/);
    expect(btn("Expandir prévia")!.textContent).toContain("Prévia");
  });

  it("pelo menos um entre editor e prévia fica aberto: o último botão fica desativado", () => {
    const { btn, panes } = setup();
    btn("Recolher editor")!.click();
    expect(panes.layout().editor).toBe(false);
    expect(btn("Recolher prévia")!.disabled).toBe(true);
    btn("Recolher prévia")!.click();
    expect(panes.layout().preview).toBe(true);
    btn("Expandir editor")!.click();
    expect(btn("Recolher prévia")!.disabled).toBe(false);
  });
});

describe("modos e atalhos", () => {
  it("controle segmentado Editor · Ambos · Prévia e botão da árvore", () => {
    const { panes } = setup();
    const seg = panes.modeControl.querySelector('[role="group"]')!;
    expect(seg.getAttribute("aria-label")).toBe("Painéis visíveis");
    const [ed, both, pv] = [...seg.querySelectorAll("button")];
    expect([ed!.textContent, both!.textContent, pv!.textContent]).toEqual(["Editor", "Ambos", "Prévia"]);
    expect(both!.getAttribute("aria-pressed")).toBe("true");
    pv!.click();
    expect(panes.layout()).toMatchObject({ editor: false, preview: true });
    expect(pv!.getAttribute("aria-pressed")).toBe("true");
    ed!.click();
    expect(panes.layout()).toMatchObject({ editor: true, preview: false });
    const treeBtn = panes.modeControl.querySelector<HTMLButtonElement>(".ws-tree-toggle")!;
    expect(treeBtn.getAttribute("aria-pressed")).toBe("true");
    treeBtn.click();
    expect(panes.layout().tree).toBe(false);
    expect(treeBtn.getAttribute("aria-pressed")).toBe("false");
  });

  const press = (key: string, mods: Partial<KeyboardEventInit> = { metaKey: true }) => {
    const target = document.activeElement ?? document.body;
    const ev = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...mods });
    target.dispatchEvent(ev);
    return ev;
  };

  it("⌘/Ctrl+1 só editor, +2 só prévia, +3 ambos, +\\ mostra ou esconde a árvore", () => {
    const { panes } = setup();
    expect(press("2").defaultPrevented).toBe(true);
    expect(panes.layout()).toMatchObject({ editor: false, preview: true });
    press("1", { ctrlKey: true });
    expect(panes.layout()).toMatchObject({ editor: true, preview: false });
    press("3");
    expect(panes.layout()).toMatchObject({ editor: true, preview: true });
    press("\\");
    expect(panes.layout().tree).toBe(false);
    press("\\", { ctrlKey: true });
    expect(panes.layout().tree).toBe(true);
  });

  it("sem modificador, ou com Alt/Shift, as teclas seguem para quem está com o foco", () => {
    const { panes } = setup();
    expect(press("2", {}).defaultPrevented).toBe(false);
    expect(press("2", { metaKey: true, altKey: true }).defaultPrevented).toBe(false);
    expect(press("\\", { metaKey: true, shiftKey: true }).defaultPrevented).toBe(false);
    expect(panes.layout()).toMatchObject({ tree: true, editor: true, preview: true });
  });
});

describe("divisórias", () => {
  it("são separadores focáveis com valor atual", () => {
    setup();
    const seps = [...document.querySelectorAll<HTMLElement>('[role="separator"]')];
    expect(seps).toHaveLength(2);
    for (const s of seps) {
      expect(s.tabIndex).toBe(0);
      expect(s.getAttribute("aria-orientation")).toBe("vertical");
      expect(s.getAttribute("aria-label")).toBeTruthy();
    }
    expect(seps[0]!.getAttribute("aria-valuenow")).toBe("200");
    expect(seps[1]!.getAttribute("aria-valuenow")).toBe(String(Math.round(DEFAULT_LAYOUT.split * 100)));
  });

  it("setas movem a divisória da árvore dentro dos limites", () => {
    const { panes, body } = setup();
    const sep = document.querySelector<HTMLElement>('[role="separator"]')!;
    const key = (k: string, shift = false) =>
      sep.dispatchEvent(new KeyboardEvent("keydown", { key: k, shiftKey: shift, bubbles: true, cancelable: true }));
    key("ArrowRight");
    expect(panes.layout().treeWidth).toBe(216);
    expect(cols(body).startsWith("216px")).toBe(true);
    expect(sep.getAttribute("aria-valuenow")).toBe("216");
    for (let i = 0; i < 40; i++) key("ArrowRight", true);
    expect(panes.layout().treeWidth).toBe(MAX_TREE);
    for (let i = 0; i < 40; i++) key("ArrowLeft", true);
    expect(panes.layout().treeWidth).toBe(MIN_TREE);
  });

  it("arrastar a divisória editor|prévia muda a proporção e respeita a largura mínima", () => {
    const { panes, editor, preview } = setup();
    rect(editor.col, { left: 206, width: 500 });
    rect(preview.col, { left: 712, width: 400 });
    const sep = document.querySelectorAll<HTMLElement>('[role="separator"]')[1]!;
    const ev = (type: string, x: number) =>
      sep.dispatchEvent(new MouseEvent(type, { clientX: x, bubbles: true, cancelable: true, button: 0 }));
    ev("pointerdown", 709);
    ev("pointermove", 506);
    ev("pointerup", 506);
    // Editor com 300 px de 900 px.
    expect(panes.layout().split).toBeCloseTo(300 / 900, 2);
    ev("pointerdown", 506);
    ev("pointermove", 1100);
    ev("pointerup", 1100);
    expect(panes.layout().split).toBeCloseTo((900 - MIN_PANE) / 900, 2);
  });

  it("clique duplo volta ao padrão do protótipo (200 px | 1,25fr | 1fr)", () => {
    const { panes } = setup();
    const [s1, s2] = [...document.querySelectorAll<HTMLElement>('[role="separator"]')];
    s1!.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }));
    s2!.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowLeft", bubbles: true }));
    s1!.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
    s2!.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
    expect(panes.layout().treeWidth).toBe(200);
    expect(panes.layout().split).toBeCloseTo(1.25 / 2.25);
  });

  it("divisória ao lado de painel recolhido some", () => {
    const { btn } = setup();
    btn("Recolher prévia")!.click();
    const seps = [...document.querySelectorAll<HTMLElement>('[role="separator"]')];
    expect(seps.filter((s) => !s.hidden)).toHaveLength(1);
  });
});

describe("rolagem e prévia ao esconder e mostrar", () => {
  it("o editor e a prévia voltam à mesma posição de rolagem", () => {
    const onChange = vi.fn();
    const { btn, scroller, pages } = setup({ onChange });
    scroller.scrollTop = 480;
    pages.scrollTop = 900;
    pages.scrollLeft = 30;
    btn("Recolher prévia")!.click();
    pages.scrollTop = 0; // a webview zera a rolagem de quem sai do layout
    pages.scrollLeft = 0;
    btn("Expandir prévia")!.click();
    expect(pages.scrollTop).toBe(900);
    expect(pages.scrollLeft).toBe(30);
    btn("Recolher editor")!.click();
    scroller.scrollTop = 0;
    btn("Expandir editor")!.click();
    expect(scroller.scrollTop).toBe(480);
    expect(onChange).toHaveBeenCalled();
  });
});

describe("layout lembrado por projeto", () => {
  it("grava ao mudar e recupera ao abrir o mesmo projeto", () => {
    const storage = memoryStorage();
    const a = setup({ key: "artigo", storage });
    a.btn("Recolher árvore de arquivos")!.click();
    a.btn("Recolher prévia")!.click();
    a.panes.destroy();
    document.body.innerHTML = "";
    const b = setup({ key: "artigo", storage });
    expect(b.panes.layout()).toMatchObject({ tree: false, editor: true, preview: false });
    b.panes.setKey("outro");
    expect(b.panes.layout()).toMatchObject({ tree: true, editor: true, preview: true });
  });

  it("valor guardado inválido ou armazenamento quebrado: usa o padrão sem quebrar", () => {
    const storage = memoryStorage();
    storage.setItem("prelo.panes:artigo", "{não é json");
    expect(setup({ key: "artigo", storage }).panes.layout()).toEqual(DEFAULT_LAYOUT);
    handle?.destroy();
    document.body.innerHTML = "";
    storage.setItem("prelo.panes:artigo", JSON.stringify({ tree: false, editor: false, preview: false, treeWidth: 9999, split: 7 }));
    const l = setup({ key: "artigo", storage }).panes.layout();
    expect(l.editor || l.preview).toBe(true);
    expect(l.treeWidth).toBe(MAX_TREE);
    expect(l.split).toBeLessThan(1);
    handle?.destroy();
    document.body.innerHTML = "";
    const broken = {
      getItem: () => {
        throw new Error("SecurityError");
      },
      setItem: () => {
        throw new Error("QuotaExceededError");
      },
    };
    const c = setup({ key: "artigo", storage: broken as unknown as ReturnType<typeof memoryStorage> });
    c.btn("Recolher prévia")!.click();
    expect(c.panes.layout().preview).toBe(false);
  });
});
