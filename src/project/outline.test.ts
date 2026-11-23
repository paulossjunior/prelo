// Testes do painel de estrutura (T062; FR-005, US4 cenário 4; contracts/ipc.md `outline`;
// data-model.md `OutlineItem`): títulos em ordem e hierarquia, como texto; clicar numa seção
// leva o editor à linha do título (inclusive em `.typ` incluído) e a prévia até ela.
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import type { OutlineItem, PreviewUpdate, ProjectInfo } from "../ipc";
import { deferred, fakeCore, settle } from "../testing/fake-core";
import { stubRangeLayout } from "../testing/layout";
import { mountWorkspace, type WorkspaceHandle } from "../workspace";
import { mountOutline, type OutlineHandle } from "./outline";

beforeAll(stubRangeLayout);

let cleanup: (() => void)[] = [];
afterEach(() => {
  for (const c of cleanup) c();
  cleanup = [];
  document.body.innerHTML = "";
});

const ITEMS: OutlineItem[] = [
  { level: 1, title: "1 Introdução", path: "main.typ", line: 9, page: 0, yPt: 120 },
  { level: 1, title: "2 Método", path: "capitulos/metodo.typ", line: 3, page: 1, yPt: 80 },
  { level: 2, title: "2.1 Coleta de dados", path: "capitulos/metodo.typ", line: 7, page: 1, yPt: 300 },
  { level: 1, title: "3 <b>Conclusão</b>", path: "main.typ", line: 17, page: 2, yPt: 500 },
];

function setup(items: () => unknown = () => ITEMS) {
  const core = fakeCore({ outline: items });
  const parent = document.createElement("div");
  document.body.append(parent);
  const onSelect = vi.fn();
  const handle: OutlineHandle = mountOutline(parent, { ipc: core.ipc, onSelect });
  cleanup.push(() => handle.destroy());
  return { core, parent, handle, onSelect };
}

const entries = (parent: ParentNode) => [...parent.querySelectorAll<HTMLButtonElement>("button.ol-item")];

describe("painel de estrutura", () => {
  it("lista os títulos em ordem, com o nível, como texto", async () => {
    const { parent, handle } = setup();
    await handle.refresh();
    expect(parent.querySelector("h4")?.textContent).toBe("Estrutura");
    const items = entries(parent);
    expect(items.map((b) => b.textContent)).toEqual(ITEMS.map((i) => i.title));
    expect(items.map((b) => b.dataset.level)).toEqual(["1", "1", "2", "1"]);
    // Nível mais fundo, mais recuo.
    expect(parseFloat(items[2]!.style.paddingLeft)).toBeGreaterThan(parseFloat(items[1]!.style.paddingLeft));
    // Título vira texto, nunca marcação.
    expect(parent.querySelector("b")).toBeNull();
    expect(items[2]!.title).toContain("capitulos/metodo.typ");
  });

  it("clicar numa seção entrega o item", async () => {
    const { parent, handle, onSelect } = setup();
    await handle.refresh();
    entries(parent)[2]!.click();
    expect(onSelect).toHaveBeenCalledWith(ITEMS[2]);
    expect(entries(parent)[2]!.getAttribute("aria-current")).toBe("true");
  });

  it("sem títulos: diz que não há seções", async () => {
    const { parent, handle } = setup(() => []);
    await handle.refresh();
    expect(entries(parent)).toHaveLength(0);
    expect(parent.textContent).toMatch(/Sem títulos de seção/);
  });

  it("recusa do núcleo não derruba o painel", async () => {
    const { parent, handle } = setup(() => {
      throw "not_implemented";
    });
    await handle.refresh();
    expect(entries(parent)).toHaveLength(0);
    expect(parent.querySelector("h4")).not.toBeNull();
  });

  it("resposta atrasada de um pedido antigo não sobrescreve a nova", async () => {
    const slow = deferred<OutlineItem[]>();
    let n = 0;
    const { parent, handle } = setup(() => (++n === 1 ? slow.promise : [ITEMS[0]]));
    const first = handle.refresh();
    await handle.refresh();
    slow.resolve(ITEMS);
    await first;
    expect(entries(parent).map((b) => b.textContent)).toEqual([ITEMS[0]!.title]);
  });
});

// ---------------------------------------------------------------- área de trabalho

const INFO: ProjectInfo = {
  name: "artigo-basico",
  mainFile: "main.typ",
  compiler: { embedded: "0.15.1", registered: "0.15.1" },
  warnings: [],
};
const FILES: Record<string, string> = {
  "main.typ": "#set heading(numbering: \"1.1\")\n\n= Introdução\n\nTexto.\n\n#include \"capitulos/metodo.typ\"\n\n\n= Conclusão\n",
  "capitulos/metodo.typ": "#import \"../utils.typ\": x\n\n= Método\n\nTexto.\n\n== Coleta de dados\n",
};

function upd(compileId: number): PreviewUpdate {
  const pages = [0, 1, 2].map((index) => ({ index, hash: `h${index}`, widthPt: 595, heightPt: 842 }));
  return { compileId, ok: true, pages, changed: [0, 1, 2], diagnostics: [], durationMs: 9 };
}

async function workspace() {
  const core = fakeCore({
    get_app_info: () => ({ version: "0.1.0", os: "macos", arch: "aarch64" }),
    subscribe_preview: () => null,
    open_project_dialog: () => INFO,
    list_tree: () => [],
    open_file: (a) => ({ text: FILES[a.path as string] ?? "", version: 1 }),
    edit_file: () => ({ version: 2 }),
    page_svg: () => "<svg/>",
    outline: () => ITEMS,
  });
  const root = document.createElement("main");
  document.body.append(root);
  const ws: WorkspaceHandle = mountWorkspace(root, {
    ipc: core.ipc,
    appInfo: (cmd) => core.invoke(cmd),
    previewUrls: { create: () => "blob:x", revoke: () => {} },
    tokenDelayMs: 0,
  });
  cleanup.push(() => ws.destroy());
  await ws.ready;
  await ws.showProject(INFO);
  core.push(upd(1));
  await settle(10);
  return { core, root, ws };
}

describe("na área de trabalho", () => {
  it("a estrutura fica abaixo dos arquivos e acompanha cada compilação boa", async () => {
    const { core, root } = await workspace();
    const panel = root.querySelector(".ws-side [aria-label='Estrutura do artigo']");
    expect(panel).not.toBeNull();
    expect(entries(root)).toHaveLength(ITEMS.length);
    const before = core.calls("outline").length;
    core.push({ ...upd(2), ok: false, changed: [] });
    await settle(10);
    expect(core.calls("outline")).toHaveLength(before);
    core.push(upd(3));
    await settle(10);
    expect(core.calls("outline").length).toBeGreaterThan(before);
  });

  it("clicar numa seção de .typ incluído abre o arquivo na linha e rola a prévia até ela", async () => {
    const { root, ws } = await workspace();
    const pageEl = root.querySelector<HTMLElement>('[data-page-index="1"]')!;
    Object.defineProperty(pageEl, "offsetTop", { configurable: true, value: 900 });
    Object.defineProperty(pageEl, "offsetHeight", { configurable: true, value: 842 });
    const scroller = root.querySelector<HTMLElement>(".pv-pages")!;
    Object.defineProperty(scroller, "clientHeight", { configurable: true, value: 400 });
    entries(root).find((b) => b.textContent === "2.1 Coleta de dados")!.click();
    await settle(10);
    const ed = ws.editor()!;
    expect(ed.path).toBe("capitulos/metodo.typ");
    expect(ed.view.state.doc.lineAt(ed.view.state.selection.main.head).number).toBe(7);
    // y = 300 pt numa página de 842 pt desenhada com 842 px.
    expect(scroller.scrollTop).toBeGreaterThan(900 + 300 - 400);
    expect(scroller.scrollTop).toBeLessThan(900 + 300);
    expect(pageEl.querySelector(".pv-mark")).not.toBeNull();
  });
});
