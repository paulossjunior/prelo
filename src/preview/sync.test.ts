// Testes da sincronização prévia ↔ código na interface (T062; FR-020, US4 cenários 1, 2, 3 e
// 5; contracts/ipc.md `jump_from_click`, `jump_from_cursor`). O clique vira pontos pela caixa
// que a página ocupa na tela (vale em qualquer zoom); `null` avisa e não mexe no cursor;
// "Localizar na prévia" rola até o ponto e o destaca.
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import type { PageInfo, PreviewUpdate, ProjectInfo } from "../ipc";
import { fakeCore, settle } from "../testing/fake-core";
import { stubRangeLayout } from "../testing/layout";
import { mountWorkspace, type WorkspaceHandle } from "../workspace";
import { mountPreview, type PreviewHandle } from "./preview";
import { clickToPoint, mountSync, offsetOf, type SyncHandle } from "./sync";

beforeAll(stubRangeLayout);

let cleanup: (() => void)[] = [];
afterEach(() => {
  for (const c of cleanup) c();
  cleanup = [];
  vi.useRealTimers();
  document.body.innerHTML = "";
});

const A4 = { widthPt: 595, heightPt: 842 };

function pages(n: number): PageInfo[] {
  return Array.from({ length: n }, (_, index) => ({ index, hash: `h${index}`, ...A4 }));
}

function upd(n = 2, over: Partial<PreviewUpdate> = {}): PreviewUpdate {
  const ps = pages(n);
  return { compileId: 1, ok: true, pages: ps, changed: ps.map((p) => p.index), diagnostics: [], durationMs: 9, ...over };
}

/** Caixa da página `index` na tela (jsdom não diagramam: o teste diz onde ela está). */
function placePage(root: ParentNode, index: number, box: { left: number; top: number; width: number; height: number }) {
  const el = root.querySelector<HTMLElement>(`[data-page-index="${index}"]`)!;
  el.getBoundingClientRect = () =>
    ({ ...box, x: box.left, y: box.top, right: box.left + box.width, bottom: box.top + box.height, toJSON() {} }) as DOMRect;
  Object.defineProperty(el, "offsetTop", { configurable: true, value: 18 + index * (box.height + 10) });
  Object.defineProperty(el, "offsetHeight", { configurable: true, value: box.height });
  Object.defineProperty(el, "offsetWidth", { configurable: true, value: box.width });
  return el;
}

function click(el: HTMLElement, clientX: number, clientY: number) {
  el.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true, clientX, clientY }));
}

describe("conversões", () => {
  it("clique converte pixels da tela em pontos da página, em qualquer zoom", () => {
    // Página A4 a 50 %: 297,5 px de largura.
    const half = { left: 10, top: 20, width: 297.5, height: 421 };
    expect(clickToPoint(half, A4, 10 + 148.75, 20 + 210.5)).toEqual({ xPt: 297.5, yPt: 421 });
    // A 200 %: 1190 px; o mesmo ponto da página.
    const double = { left: -100, top: -300, width: 1190, height: 1684 };
    const p = clickToPoint(double, A4, -100 + 595, -300 + 842)!;
    expect(p.xPt).toBeCloseTo(297.5);
    expect(p.yPt).toBeCloseTo(421);
    // Fora da página ou página sem tamanho: sem ponto.
    expect(clickToPoint(half, A4, 5, 30)).toBeNull();
    expect(clickToPoint({ left: 0, top: 0, width: 0, height: 0 }, A4, 0, 0)).toBeNull();
  });

  it("linha e coluna (a partir de 1, UTF-16) viram deslocamento no texto", () => {
    const text = "= Título\n\nÉ um parágrafo 🙂 aqui.\nfim";
    expect(offsetOf(text, 1, 1)).toBe(0);
    expect(offsetOf(text, 3, 1)).toBe(text.indexOf("É"));
    expect(offsetOf(text, 3, 18)).toBe(text.indexOf(" aqui"));
    // Além do fim da linha ou do texto: para no fim.
    expect(offsetOf(text, 1, 99)).toBe(text.indexOf("\n"));
    expect(offsetOf(text, 99, 1)).toBe(text.length);
  });
});

function setupSync(handlers: Parameters<typeof fakeCore>[0] = {}) {
  const core = fakeCore({ page_svg: () => "<svg/>", ...handlers });
  const parent = document.createElement("div");
  document.body.append(parent);
  const preview: PreviewHandle = mountPreview(parent, { ipc: core.ipc, urls: { create: () => "blob:x", revoke: () => {} } });
  const openAt = vi.fn();
  const notify = vi.fn();
  let cursor: { path: string; offset: number } | null = { path: "main.typ", offset: 42 };
  const sync: SyncHandle = mountSync({
    ipc: core.ipc,
    preview,
    openAt,
    notify,
    cursor: () => cursor,
  });
  cleanup.push(() => sync.destroy(), () => preview.destroy());
  preview.update(upd(2));
  return { core, parent, preview, sync, openAt, notify, setCursor: (c: typeof cursor) => (cursor = c) };
}

describe("clique na prévia (cenários 1, 2 e 5)", () => {
  it("manda página e pontos ao núcleo e abre o arquivo na linha devolvida", async () => {
    const { core, parent, openAt, notify } = setupSync({
      jump_from_click: () => ({ path: "capitulos/metodo.typ", line: 5, column: 3 }),
    });
    const page = placePage(parent, 1, { left: 50, top: 100, width: 297.5, height: 421 });
    click(page, 50 + 100, 100 + 50);
    await settle();
    const args = core.calls("jump_from_click")[0]!.args;
    expect(args.page).toBe(1);
    expect(args.xPt).toBeCloseTo(200);
    expect(args.yPt).toBeCloseTo(100);
    expect(openAt).toHaveBeenCalledWith({ path: "capitulos/metodo.typ", line: 5, column: 3 });
    expect(notify).not.toHaveBeenCalled();
  });

  it("sem origem no projeto (null): avisa e não move o cursor", async () => {
    const { parent, openAt, notify } = setupSync({ jump_from_click: () => null });
    const page = placePage(parent, 0, { left: 0, top: 0, width: 595, height: 842 });
    click(page, 300, 820);
    await settle();
    expect(openAt).not.toHaveBeenCalled();
    expect(notify).toHaveBeenCalledWith(expect.stringMatching(/sem origem no projeto|não tem origem no projeto/i));
  });

  it("clique fora das páginas não chama o núcleo", async () => {
    const { core, preview } = setupSync({ jump_from_click: () => null });
    click(preview.element.querySelector<HTMLElement>(".pv-pages")!, 1, 1);
    await settle();
    expect(core.calls("jump_from_click")).toHaveLength(0);
  });

  it("recusa do núcleo vira aviso, sem mover o cursor", async () => {
    const { parent, openAt, notify } = setupSync({
      jump_from_click: () => {
        throw "no_project";
      },
    });
    click(placePage(parent, 0, { left: 0, top: 0, width: 595, height: 842 }), 100, 100);
    await settle();
    expect(openAt).not.toHaveBeenCalled();
    expect(notify).toHaveBeenCalled();
  });
});

describe("localizar na prévia (cenário 3)", () => {
  it("pede o ponto do cursor, rola até ele e o destaca", async () => {
    const { core, parent, preview, sync, notify } = setupSync({
      jump_from_cursor: () => [{ page: 1, xPt: 72, yPt: 421 }],
    });
    placePage(parent, 0, { left: 0, top: 0, width: 297.5, height: 421 });
    placePage(parent, 1, { left: 0, top: 431, width: 297.5, height: 421 });
    const scroller = preview.element.querySelector<HTMLElement>(".pv-pages")!;
    Object.defineProperty(scroller, "clientHeight", { configurable: true, value: 300 });
    expect(await sync.locate()).toBe(true);
    expect(core.calls("jump_from_cursor")[0]!.args).toEqual({ path: "main.typ", offset: 42 });
    // Página 2 começa em 18 + 421 + 10 = 449; o ponto está na metade dela (210,5 px).
    const y = 449 + 210.5;
    expect(scroller.scrollTop).toBeGreaterThan(y - 300);
    expect(scroller.scrollTop).toBeLessThan(y);
    const mark = parent.querySelector<HTMLElement>('[data-page-index="1"] .pv-mark');
    expect(mark).not.toBeNull();
    // Posição em porcentagem da página: acompanha o zoom.
    expect(mark!.style.top).toMatch(/%$/);
    expect(parseFloat(mark!.style.left)).toBeCloseTo((72 / 595) * 100, 1);
    expect(notify).not.toHaveBeenCalled();
  });

  it("o destaque some sozinho e um novo substitui o anterior", async () => {
    const { parent, sync } = setupSync({ jump_from_cursor: () => [{ page: 0, xPt: 72, yPt: 100 }] });
    placePage(parent, 0, { left: 0, top: 0, width: 595, height: 842 });
    vi.useFakeTimers();
    await sync.locate();
    await sync.locate();
    expect(parent.querySelectorAll(".pv-mark")).toHaveLength(1);
    vi.advanceTimersByTime(5000);
    expect(parent.querySelectorAll(".pv-mark")).toHaveLength(0);
  });

  it("sem ponto na prévia: avisa", async () => {
    const { sync, notify } = setupSync({ jump_from_cursor: () => [] });
    expect(await sync.locate()).toBe(false);
    expect(notify).toHaveBeenCalledWith(expect.stringMatching(/prévia/));
  });

  it("sem arquivo aberto: não chama o núcleo", async () => {
    const { core, sync, setCursor } = setupSync({ jump_from_cursor: () => [] });
    setCursor(null);
    expect(await sync.locate()).toBe(false);
    expect(core.calls("jump_from_cursor")).toHaveLength(0);
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
  "main.typ": "= Introdução\n\nPrimeiro parágrafo.\n\n#include \"capitulos/metodo.typ\"\n",
  "capitulos/metodo.typ": "= Método\n\nLinha dois.\n\nParágrafo do capítulo incluído.\n",
};

async function workspace(extra: Parameters<typeof fakeCore>[0] = {}) {
  let version = 1;
  const core = fakeCore({
    get_app_info: () => ({ version: "0.1.0", os: "macos", arch: "aarch64" }),
    subscribe_preview: () => null,
    open_project_dialog: () => INFO,
    list_tree: () => [],
    open_file: (a) => ({ text: FILES[a.path as string] ?? "", version: 1 }),
    edit_file: () => ({ version: ++version }),
    page_svg: () => "<svg/>",
    outline: () => [],
    ...extra,
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
  core.push(upd(2));
  await settle();
  return { core, root, ws };
}

describe("na área de trabalho", () => {
  it("clique num trecho de .typ incluído abre o incluído com o cursor na linha", async () => {
    const { root, ws } = await workspace({
      jump_from_click: () => ({ path: "capitulos/metodo.typ", line: 5, column: 4 }),
    });
    click(placePage(root, 1, { left: 0, top: 0, width: 595, height: 842 }), 100, 100);
    await settle(10);
    const ed = ws.editor()!;
    expect(ed.path).toBe("capitulos/metodo.typ");
    const head = ed.view.state.selection.main.head;
    const line = ed.view.state.doc.lineAt(head);
    expect(line.number).toBe(5);
    expect(head - line.from).toBe(3);
  });

  it("clique sem origem mostra o aviso e deixa o cursor onde estava", async () => {
    const { root, ws } = await workspace({ jump_from_click: () => null });
    const ed = ws.editor()!;
    ed.view.dispatch({ selection: { anchor: 5 } });
    click(placePage(root, 0, { left: 0, top: 0, width: 595, height: 842 }), 300, 830);
    await settle(10);
    expect(ws.editor()).toBe(ed);
    expect(ed.view.state.selection.main.head).toBe(5);
    expect(root.querySelector(".ws-message")?.textContent).toMatch(/origem no projeto/);
  });

  it("botão Localizar na prévia usa o cursor do editor", async () => {
    const { core, root, ws } = await workspace({ jump_from_cursor: () => [{ page: 0, xPt: 72, yPt: 100 }] });
    placePage(root, 0, { left: 0, top: 0, width: 595, height: 842 });
    ws.editor()!.view.dispatch({ selection: { anchor: 16 } });
    const btn = [...root.querySelectorAll<HTMLButtonElement>("button")].find((b) => /Localizar na prévia/.test(b.textContent ?? ""));
    expect(btn).toBeDefined();
    btn!.click();
    await settle(10);
    expect(core.calls("jump_from_cursor").at(-1)!.args).toEqual({ path: "main.typ", offset: 16 });
    expect(root.querySelector('[data-page-index="0"] .pv-mark')).not.toBeNull();
  });
});
