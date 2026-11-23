// Testes da prévia (T030; FR-013, FR-014; research R4): páginas como <img> com blob URL (o SVG
// nunca entra no DOM), só as visíveis ± 2 são pedidas, só as páginas de `changed` (ou de hash
// novo) são trocadas, `stale_page` é pedido de novo com o hash atual, e erro de compilação
// mantém as páginas e mostra a faixa de erro.
import { afterEach, describe, expect, it, vi } from "vitest";
import type { PageInfo, PreviewUpdate } from "../ipc";
import { deferred, fakeCore, settle } from "../testing/fake-core";
import { mountPreview, type PreviewHandle } from "./preview";

let handle: PreviewHandle | undefined;
afterEach(() => {
  handle?.destroy();
  handle = undefined;
  document.body.innerHTML = "";
});

function pages(n: number, hashOf: (i: number) => string = (i) => `h${i}`): PageInfo[] {
  return Array.from({ length: n }, (_, index) => ({
    index,
    hash: hashOf(index),
    widthPt: 595,
    heightPt: 842,
  }));
}

function upd(partial: Partial<PreviewUpdate> & { pages: PageInfo[] }): PreviewUpdate {
  return {
    compileId: 1,
    ok: true,
    changed: partial.pages.map((p) => p.index),
    diagnostics: [],
    durationMs: 12,
    ...partial,
  };
}

function setup(pageSvg?: (args: Record<string, unknown>) => unknown) {
  const core = fakeCore({
    page_svg: pageSvg ?? ((a) => `<svg data-page="${String(a.index)}" data-hash="${String(a.hash)}"></svg>`),
  });
  let n = 0;
  const blobs = new Map<string, Blob>();
  const urls = {
    create: vi.fn((b: Blob) => {
      const u = `blob:prelo/${++n}`;
      blobs.set(u, b);
      return u;
    }),
    revoke: vi.fn((_u: string) => {}),
  };
  const parent = document.createElement("div");
  document.body.append(parent);
  handle = mountPreview(parent, { ipc: core.ipc, urls });
  return { core, parent, urls, blobs, preview: handle };
}

const fetched = (core: ReturnType<typeof fakeCore>) =>
  core.calls("page_svg").map((c) => [c.args.index, c.args.hash]);

function imgOf(parent: HTMLElement, index: number) {
  return parent.querySelector<HTMLImageElement>(`[data-page-index="${index}"] img`);
}

describe("páginas como imagem", () => {
  it("mostra cada página como <img> com blob URL de SVG, sem SVG no DOM", async () => {
    const { parent, preview, urls, blobs } = setup();
    preview.update(upd({ pages: pages(2) }));
    await preview.idle();
    const img = imgOf(parent, 0);
    expect(img?.getAttribute("src")).toMatch(/^blob:/);
    expect(img?.alt).toBe("Página 1");
    expect(parent.querySelector("svg")).toBeNull();
    expect(parent.innerHTML).not.toContain("<svg");
    const blob = blobs.get(img!.getAttribute("src")!);
    expect(blob?.type).toBe("image/svg+xml");
    expect(await blob?.text()).toContain('data-page="0"');
    expect(urls.create).toHaveBeenCalledTimes(2);
  });

  it("reserva o espaço de cada página pela proporção", () => {
    const { parent, preview } = setup();
    preview.update(upd({ pages: [{ index: 0, hash: "a", widthPt: 400, heightPt: 600 }] }));
    const slot = parent.querySelector<HTMLElement>('[data-page-index="0"]');
    expect(slot?.style.aspectRatio).toBe("400 / 600");
  });
});

describe("virtualização (visíveis ± 2)", () => {
  it("de início pede só a primeira página e as duas seguintes", async () => {
    const { core, parent, preview } = setup();
    preview.update(upd({ pages: pages(10) }));
    await preview.idle();
    expect(fetched(core)).toEqual([
      [0, "h0"],
      [1, "h1"],
      [2, "h2"],
    ]);
    expect(parent.querySelectorAll("[data-page-index]")).toHaveLength(10);
    expect(imgOf(parent, 5)).toBeNull();
  });

  it("ao rolar pede as novas visíveis ± 2 e solta as que saíram", async () => {
    const { core, parent, preview, urls } = setup();
    preview.update(upd({ pages: pages(20) }));
    await preview.idle();
    const firstUrl = imgOf(parent, 0)!.getAttribute("src")!;
    preview.setVisible([10, 11]);
    await preview.idle();
    expect(fetched(core).slice(3)).toEqual([8, 9, 10, 11, 12, 13].map((i) => [i, `h${i}`]));
    expect(imgOf(parent, 0)).toBeNull();
    expect(urls.revoke).toHaveBeenCalledWith(firstUrl);
  });
});

describe("atualização incremental", () => {
  it("troca só as páginas de `changed` e revoga o blob antigo", async () => {
    const { core, parent, preview, urls } = setup();
    preview.update(upd({ pages: pages(3) }));
    await preview.idle();
    const img0 = imgOf(parent, 0)!;
    const src0 = img0.getAttribute("src");
    const old1 = imgOf(parent, 1)!.getAttribute("src")!;
    preview.update(
      upd({ compileId: 2, pages: pages(3, (i) => (i === 1 ? "h1b" : `h${i}`)), changed: [1] }),
    );
    await preview.idle();
    expect(fetched(core).slice(3)).toEqual([[1, "h1b"]]);
    expect(imgOf(parent, 0)).toBe(img0);
    expect(img0.getAttribute("src")).toBe(src0);
    expect(imgOf(parent, 1)!.getAttribute("src")).not.toBe(old1);
    expect(urls.revoke).toHaveBeenCalledWith(old1);
  });

  it("página que some do documento sai da prévia", async () => {
    const { parent, preview } = setup();
    preview.update(upd({ pages: pages(3) }));
    await preview.idle();
    preview.update(upd({ compileId: 2, pages: pages(2), changed: [] }));
    expect(parent.querySelectorAll("[data-page-index]")).toHaveLength(2);
  });

  it("ignora atualização de compilação mais antiga que a mostrada", async () => {
    const { core, preview } = setup();
    preview.update(upd({ compileId: 5, pages: pages(1) }));
    await preview.idle();
    preview.update(upd({ compileId: 4, pages: pages(1, () => "velho"), changed: [0] }));
    await preview.idle();
    expect(fetched(core)).toEqual([[0, "h0"]]);
  });

  it("stale_page: pede de novo com o hash da atualização mais nova", async () => {
    const gate = deferred<string>();
    let calls = 0;
    const { core, parent, preview } = setup((a) => {
      calls++;
      if (calls === 1) return gate.promise;
      return `<svg data-hash="${String(a.hash)}"></svg>`;
    });
    preview.update(upd({ pages: pages(1) }));
    await settle();
    // Chega uma compilação nova enquanto a página 0 ainda era pedida com o hash velho.
    preview.update(upd({ compileId: 2, pages: pages(1, () => "novo"), changed: [0] }));
    gate.reject("stale_page");
    await preview.idle();
    expect(fetched(core)).toEqual([
      [0, "h0"],
      [0, "novo"],
    ]);
    expect(imgOf(parent, 0)?.getAttribute("src")).toMatch(/^blob:/);
  });

  it("resposta atrasada de hash velho não substitui a página nova", async () => {
    const gate = deferred<string>();
    let calls = 0;
    const { parent, preview, blobs } = setup((a) => {
      calls++;
      if (calls === 1) return gate.promise;
      return `<svg data-hash="${String(a.hash)}"></svg>`;
    });
    preview.update(upd({ pages: pages(1) }));
    await settle();
    preview.update(upd({ compileId: 2, pages: pages(1, () => "novo"), changed: [0] }));
    gate.resolve('<svg data-hash="h0"></svg>');
    await preview.idle();
    const src = imgOf(parent, 0)!.getAttribute("src")!;
    expect(await blobs.get(src)?.text()).toContain('data-hash="novo"');
  });
});

describe("faixa de erro (FR-014)", () => {
  it("compilação com erro mantém as páginas e mostra a faixa; sucesso a esconde", async () => {
    const { core, parent, preview } = setup();
    preview.update(upd({ pages: pages(2) }));
    await preview.idle();
    const before = imgOf(parent, 0)!.getAttribute("src");
    preview.update({
      compileId: 2,
      ok: false,
      pages: [],
      changed: [],
      diagnostics: [
        { severity: "error", path: "main.typ", line: 3, message: "unclosed delimiter", hints: [], kind: "typst" },
      ],
      durationMs: 3,
    });
    await preview.idle();
    const band = parent.querySelector<HTMLElement>(".pv-error");
    expect(band?.hidden).toBe(false);
    expect(band?.getAttribute("role")).toBe("status");
    expect(band?.textContent).toContain("última versão que compilou");
    expect(band?.textContent).toContain("1 erro");
    expect(parent.querySelectorAll("[data-page-index]")).toHaveLength(2);
    expect(imgOf(parent, 0)?.getAttribute("src")).toBe(before);
    expect(core.calls("page_svg")).toHaveLength(2);

    preview.update(upd({ compileId: 3, pages: pages(2), changed: [] }));
    expect(band?.hidden).toBe(true);
  });

  it("faixa de erro com onShowErrors tem botão 'Ver os erros' (FR-006a)", async () => {
    const parent = document.createElement("div");
    document.body.append(parent);
    let shown = 0;
    const p = mountPreview(parent, {
      ipc: fakeCore().ipc,
      urls: { create: () => "blob:x", revoke: () => {} },
      onShowErrors: () => shown++,
    });
    p.update({
      compileId: 1,
      ok: false,
      pages: [],
      changed: [],
      diagnostics: [{ severity: "error", path: "main.typ", line: 1, message: "x", hints: [], kind: "typst" }],
      durationMs: 1,
    });
    const band = parent.querySelector<HTMLElement>(".pv-error")!;
    expect(band.textContent).toContain("1 erro");
    const b = band.querySelector<HTMLButtonElement>("button")!;
    expect(b.textContent).toBe("Ver os erros");
    b.click();
    expect(shown).toBe(1);
    p.destroy();
  });

  it("falha ao buscar a página (não stale) não quebra a prévia", async () => {
    const { parent, preview } = setup(() => {
      throw "invalid_arguments";
    });
    preview.update(upd({ pages: pages(1) }));
    await preview.idle();
    expect(parent.querySelector('[data-page-index="0"]')).not.toBeNull();
    expect(imgOf(parent, 0)).toBeNull();
  });
});

// ---------------------------------------------------------------- zoom (T090; FR-013a)

const PAD = 18;
const GAP = 10;

/**
 * O jsdom não calcula layout: as medidas das páginas vêm da largura que a prévia põe em
 * `style.width` (zoom fixo) ou da largura do painel (ajustar à largura), na proporção da página.
 */
function fakeLayout(parent: HTMLElement, pane = { width: 500, height: 600 }) {
  const pagesEl = parent.querySelector<HTMLElement>(".pv-pages")!;
  Object.defineProperty(pagesEl, "clientWidth", { configurable: true, get: () => pane.width });
  Object.defineProperty(pagesEl, "clientHeight", { configurable: true, get: () => pane.height });
  const widthOf = (el: HTMLElement) => {
    const w = parseFloat(el.style.width);
    return Number.isFinite(w) ? w : pane.width - 2 * PAD;
  };
  const heightOf = (el: HTMLElement) => {
    const [w = 1, h = 1] = el.style.aspectRatio.split("/").map(Number);
    return (widthOf(el) * h) / w;
  };
  const all = () => [...pagesEl.querySelectorAll<HTMLElement>("[data-page-index]")];
  const topOf = (el: HTMLElement) => {
    let y = PAD;
    for (const p of all()) {
      if (p === el) return y;
      y += heightOf(p) + GAP;
    }
    return y;
  };
  for (const el of all()) {
    Object.defineProperty(el, "offsetWidth", { configurable: true, get: () => widthOf(el) });
    Object.defineProperty(el, "offsetHeight", { configurable: true, get: () => heightOf(el) });
    Object.defineProperty(el, "offsetTop", { configurable: true, get: () => topOf(el) });
    Object.defineProperty(el, "offsetLeft", { configurable: true, get: () => PAD });
  }
  return { pagesEl, widthOf, heightOf, topOf, page: (i: number) => all()[i]! };
}

function zoomSetup(opts: { zoomKey?: string | null; storage?: Pick<Storage, "getItem" | "setItem"> } = {}) {
  const core = fakeCore({ page_svg: (a) => `<svg data-page="${String(a.index)}"></svg>` });
  let n = 0;
  const urls = { create: vi.fn(() => `blob:prelo/${++n}`), revoke: vi.fn() };
  const parent = document.createElement("div");
  document.body.append(parent);
  handle = mountPreview(parent, { ipc: core.ipc, urls, ...opts });
  const q = <T extends HTMLElement>(sel: string) => parent.querySelector<T>(sel)!;
  return {
    core,
    parent,
    preview: handle,
    zoomIn: q<HTMLButtonElement>('button[aria-label="Aumentar zoom"]'),
    zoomOut: q<HTMLButtonElement>('button[aria-label="Diminuir zoom"]'),
    fit: q<HTMLButtonElement>(".pv-zoom-fit"),
    pct: q<HTMLElement>(".pv-zoom-pct"),
    live: q<HTMLElement>(".pv-live"),
    pagesEl: q<HTMLElement>(".pv-pages"),
  };
}

function memoryStorage(init: Record<string, string> = {}) {
  const m = new Map(Object.entries(init));
  return { getItem: (k: string) => m.get(k) ?? null, setItem: (k: string, v: string) => void m.set(k, v), map: m };
}

describe("barra de zoom", () => {
  it("tem '−', '+', porcentagem e 'Ajustar à largura', que começa ligado", () => {
    const { zoomIn, zoomOut, fit, pct, parent } = zoomSetup();
    expect(zoomIn.textContent).toBe("+");
    expect(zoomOut.textContent).toBe("−");
    expect(fit.textContent).toBe("Ajustar à largura");
    expect(fit.getAttribute("aria-pressed")).toBe("true");
    expect(pct).not.toBeNull();
    const group = parent.querySelector('.pvbar [role="group"]');
    expect(group?.getAttribute("aria-label")).toBe("Zoom da prévia");
    expect(group?.classList.contains("seg")).toBe(true);
  });

  it("'+' a partir do ajuste à largura vai ao degrau acima do nível efetivo", () => {
    const { preview, parent, zoomIn, fit, pct } = zoomSetup();
    preview.update(upd({ pages: pages(3) }));
    fakeLayout(parent); // 464 px de 793,3 px (A4 a 100 %) ≈ 58 %
    zoomIn.click();
    expect(preview.zoom()).toBe(67);
    expect(fit.getAttribute("aria-pressed")).toBe("false");
    expect(pct.textContent).toBe("67%");
  });

  it("muda a largura das páginas pelo nível: 100 % = tamanho real (1 pt = 1/72 pol.)", () => {
    const { preview, parent } = zoomSetup();
    preview.update(upd({ pages: [{ index: 0, hash: "a", widthPt: 600, heightPt: 800 }] }));
    preview.setZoom(100);
    const page = parent.querySelector<HTMLElement>('[data-page-index="0"]')!;
    expect(parseFloat(page.style.width)).toBeCloseTo(800);
    preview.setZoom(200);
    expect(parseFloat(page.style.width)).toBeCloseTo(1600);
    preview.setZoom("fit");
    expect(page.style.width).toBe("");
  });

  it("páginas que chegam depois do zoom já vêm no nível atual", () => {
    const { preview, parent } = zoomSetup();
    preview.setZoom(150);
    preview.update(upd({ pages: [{ index: 0, hash: "a", widthPt: 600, heightPt: 800 }] }));
    const page = parent.querySelector<HTMLElement>('[data-page-index="0"]')!;
    expect(parseFloat(page.style.width)).toBeCloseTo(1200);
  });

  it("'−' desativa em 50 % e '+' em 300 %", () => {
    const { preview, zoomIn, zoomOut } = zoomSetup();
    preview.setZoom(67);
    zoomOut.click();
    expect(preview.zoom()).toBe(50);
    expect(zoomOut.disabled).toBe(true);
    expect(zoomIn.disabled).toBe(false);
    preview.setZoom(250);
    zoomIn.click();
    expect(preview.zoom()).toBe(300);
    expect(zoomIn.disabled).toBe(true);
    expect(zoomOut.disabled).toBe(false);
  });

  it("'Ajustar à largura' volta ao padrão", () => {
    const { preview, fit } = zoomSetup();
    preview.setZoom(200);
    fit.click();
    expect(preview.zoom()).toBe("fit");
    expect(fit.getAttribute("aria-pressed")).toBe("true");
  });

  it("anuncia a porcentagem numa região viva", () => {
    const { preview, live, zoomIn } = zoomSetup();
    expect(live.getAttribute("aria-live")).toBe("polite");
    preview.setZoom(100);
    zoomIn.click();
    expect(live.textContent).toBe("Zoom 110%");
  });

  it("mostra a página atual e o total", () => {
    const { preview, parent } = zoomSetup();
    preview.update(upd({ pages: pages(7) }));
    preview.setVisible([2, 3]);
    expect(parent.querySelector(".pv-label")?.textContent).toBe("Prévia · pág. 3 de 7");
  });
});

describe("atalhos de zoom", () => {
  const press = (target: HTMLElement, key: string, mods: Partial<KeyboardEventInit> = {}) => {
    const ev = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...mods });
    target.dispatchEvent(ev);
    return ev;
  };

  it("⌘/Ctrl + '+', '-' e '0' com o foco na prévia", () => {
    const { preview, pagesEl } = zoomSetup();
    expect(pagesEl.tabIndex).toBe(0);
    preview.setZoom(100);
    const ev = press(pagesEl, "+", { metaKey: true });
    expect(ev.defaultPrevented).toBe(true);
    expect(preview.zoom()).toBe(110);
    press(pagesEl, "-", { ctrlKey: true });
    press(pagesEl, "-", { ctrlKey: true });
    expect(preview.zoom()).toBe(90);
    press(pagesEl, "0", { metaKey: true });
    expect(preview.zoom()).toBe("fit");
  });

  it("fora da prévia (editor) as teclas não mudam o zoom nem são consumidas", () => {
    const { preview } = zoomSetup();
    const editor = document.createElement("div");
    editor.tabIndex = 0;
    document.body.append(editor);
    preview.setZoom(100);
    const ev = press(editor, "+", { metaKey: true });
    expect(ev.defaultPrevented).toBe(false);
    expect(preview.zoom()).toBe(100);
  });
});

describe("roda e pinça", () => {
  it("⌘/Ctrl + roda muda o zoom e não rola a página", () => {
    const { preview, pagesEl } = zoomSetup();
    preview.setZoom(100);
    const ev = new WheelEvent("wheel", { deltaY: -100, ctrlKey: true, bubbles: true, cancelable: true });
    pagesEl.dispatchEvent(ev);
    expect(ev.defaultPrevented).toBe(true);
    expect(preview.zoom()).toBe(110);
    pagesEl.dispatchEvent(new WheelEvent("wheel", { deltaY: 100, metaKey: true, bubbles: true, cancelable: true }));
    expect(preview.zoom()).toBe(100);
  });

  it("pinça do trackpad (roda com Ctrl e deltas pequenos) dá zoom contínuo dentro dos limites", () => {
    const { preview, pagesEl } = zoomSetup();
    preview.setZoom(100);
    pagesEl.dispatchEvent(new WheelEvent("wheel", { deltaY: -10, ctrlKey: true, bubbles: true, cancelable: true }));
    const z = preview.zoom() as number;
    expect(z).toBeGreaterThan(100);
    expect(z).toBeLessThan(125);
    for (let i = 0; i < 200; i++) {
      pagesEl.dispatchEvent(new WheelEvent("wheel", { deltaY: -10, ctrlKey: true, bubbles: true, cancelable: true }));
    }
    expect(preview.zoom()).toBe(300);
  });

  it("pinça do WebKit (gesturechange) muda o zoom pela escala do gesto", () => {
    const { preview, pagesEl } = zoomSetup();
    preview.setZoom(100);
    const gesture = (type: string, scale: number) => {
      const ev = new Event(type, { bubbles: true, cancelable: true }) as Event & { scale: number };
      ev.scale = scale;
      pagesEl.dispatchEvent(ev);
      return ev;
    };
    gesture("gesturestart", 1);
    const ev = gesture("gesturechange", 1.5);
    expect(ev.defaultPrevented).toBe(true);
    expect(preview.zoom()).toBeCloseTo(150);
    gesture("gesturechange", 0.1);
    expect(preview.zoom()).toBe(50);
  });

  it("roda sem modificador só rola (não muda o zoom)", () => {
    const { preview, pagesEl } = zoomSetup();
    preview.setZoom(100);
    const ev = new WheelEvent("wheel", { deltaY: -100, bubbles: true, cancelable: true });
    pagesEl.dispatchEvent(ev);
    expect(ev.defaultPrevented).toBe(false);
    expect(preview.zoom()).toBe(100);
  });
});

describe("posição de leitura", () => {
  /** Página e fração (vertical) do documento sob o ponto y da tela da prévia. */
  function under(l: ReturnType<typeof fakeLayout>, n: number, y: number) {
    const docY = l.pagesEl.scrollTop + y;
    for (let i = 0; i < n; i++) {
      const el = l.page(i);
      const top = l.topOf(el);
      if (docY >= top && docY <= top + l.heightOf(el)) return { i, f: (docY - top) / l.heightOf(el) };
    }
    return null;
  }

  it("mantém o trecho sob o centro ao mudar o zoom pelos botões", () => {
    const { preview, parent, zoomIn } = zoomSetup();
    preview.update(upd({ pages: pages(10) }));
    const l = fakeLayout(parent);
    preview.setZoom(100);
    l.pagesEl.scrollTop = 1200;
    const before = under(l, 10, 300)!;
    expect(before.i).toBe(1);
    zoomIn.click();
    zoomIn.click();
    const after = under(l, 10, 300)!;
    expect(after.i).toBe(before.i);
    expect(after.f).toBeCloseTo(before.f, 3);
    expect(l.pagesEl.scrollTop).not.toBe(1200);
  });

  it("na roda, mantém o trecho sob o ponteiro", () => {
    const { preview, parent } = zoomSetup();
    preview.update(upd({ pages: pages(10) }));
    const l = fakeLayout(parent);
    preview.setZoom(100);
    l.pagesEl.scrollTop = 3000;
    l.pagesEl.getBoundingClientRect = () =>
      ({ left: 0, top: 50, right: 500, bottom: 650, width: 500, height: 600, x: 0, y: 50 }) as DOMRect;
    const before = under(l, 10, 100)!;
    l.pagesEl.dispatchEvent(
      new WheelEvent("wheel", { deltaY: -100, ctrlKey: true, clientX: 200, clientY: 150, bubbles: true, cancelable: true }),
    );
    const after = under(l, 10, 100)!;
    expect(after.i).toBe(before.i);
    expect(after.f).toBeCloseTo(before.f, 3);
  });
});

describe("zoom sem novo pedido ao núcleo", () => {
  it("mudar o zoom não pede página de novo nem troca as imagens", async () => {
    const { core, preview, parent } = zoomSetup();
    preview.update(upd({ pages: pages(5) }));
    await preview.idle();
    const src = parent.querySelector('[data-page-index="0"] img')!.getAttribute("src");
    const n = core.calls("page_svg").length;
    for (const z of [50, 100, 200, 300, "fit"] as const) preview.setZoom(z);
    await preview.idle();
    expect(core.calls("page_svg")).toHaveLength(n);
    expect(parent.querySelector('[data-page-index="0"] img')!.getAttribute("src")).toBe(src);
  });

  it("visíveis ± 2 continua valendo em qualquer zoom", async () => {
    const { core, preview } = zoomSetup();
    preview.update(upd({ pages: pages(20) }));
    preview.setZoom(300);
    await preview.idle();
    preview.setVisible([10]);
    await preview.idle();
    expect(core.calls("page_svg").map((c) => c.args.index)).toEqual([0, 1, 2, 8, 9, 10, 11, 12]);
  });
});

describe("zoom lembrado por projeto", () => {
  it("grava a escolha e a recupera ao montar a prévia do mesmo projeto", () => {
    const storage = memoryStorage();
    const a = zoomSetup({ zoomKey: "artigo", storage });
    a.preview.setZoom(175);
    a.preview.destroy();
    const b = zoomSetup({ zoomKey: "artigo", storage });
    expect(b.preview.zoom()).toBe(175);
    expect(b.pct.textContent).toBe("175%");
    b.preview.destroy();
    const c = zoomSetup({ zoomKey: "outro", storage });
    expect(c.preview.zoom()).toBe("fit");
  });

  it("armazenamento quebrado não impede o zoom", () => {
    const storage = {
      getItem: () => {
        throw new Error("SecurityError");
      },
      setItem: () => {
        throw new Error("QuotaExceededError");
      },
    };
    const { preview, zoomIn } = zoomSetup({ zoomKey: "artigo", storage });
    preview.setZoom(100);
    zoomIn.click();
    expect(preview.zoom()).toBe(110);
  });
});

// Imagens das páginas (T058; RF-51, research R4): o SVG de `page_svg` cita cada imagem por
// `prelo-asset://<sha256>`. Um SVG mostrado como <img> não carrega recurso de fora, nem do
// protocolo próprio; por isso a prévia pede cada conteúdo uma vez (`preview_asset`), guarda a
// `data:` URL por hash e a devolve ao SVG antes de criar o blob.
describe("imagens por hash", () => {
  const A = "a".repeat(64);
  const B = "b".repeat(64);
  const dataUrl = (sha: string) => `data:image/png;base64,${sha.slice(0, 4)}`;
  const svgWith = (...shas: string[]) =>
    `<svg>${shas.map((s) => `<image xlink:href="prelo-asset://${s}" width="1" height="1"/>`).join("")}</svg>`;

  function setupAssets(svgOf: (a: Record<string, unknown>) => string) {
    const s = setup(svgOf);
    s.core.on("preview_asset", (a) => dataUrl(String(a.sha256)));
    return s;
  }

  it("troca a referência pela data: URL antes de criar o blob", async () => {
    const { core, parent, preview, blobs } = setupAssets(() => svgWith(A));
    preview.update(upd({ pages: pages(1) }));
    await preview.idle();
    const text = await blobs.get(imgOf(parent, 0)!.getAttribute("src")!)!.text();
    expect(text).toContain(`xlink:href="${dataUrl(A)}"`);
    expect(text).not.toContain("prelo-asset:");
    expect(core.calls("preview_asset").map((c) => c.args)).toEqual([{ sha256: A }]);
  });

  it("pede cada hash uma vez, mesmo em várias páginas e em edições seguintes", async () => {
    const { core, preview } = setupAssets((a) => (a.index === 1 ? svgWith(A, B) : svgWith(A)));
    preview.update(upd({ pages: pages(3) }));
    await preview.idle();
    // Edição de texto na página 0: SVG novo, mesma imagem.
    preview.update(upd({ compileId: 2, pages: pages(3, (i) => (i === 0 ? "novo" : `h${i}`)), changed: [0] }));
    await preview.idle();
    expect(core.calls("page_svg")).toHaveLength(4);
    expect(
      core
        .calls("preview_asset")
        .map((c) => c.args.sha256)
        .sort(),
    ).toEqual([A, B]);
  });

  it("solta a data: URL quando nenhuma página mostrada usa o hash", async () => {
    const { core, preview } = setupAssets((a) => (a.index === 0 ? svgWith(A) : "<svg></svg>"));
    preview.update(upd({ pages: pages(10) }));
    await preview.idle();
    preview.setVisible([9]);
    await preview.idle();
    preview.setVisible([0]);
    await preview.idle();
    expect(core.calls("preview_asset")).toHaveLength(2);
  });

  it("hash recusado pelo núcleo deixa a página vazia sem derrubar a prévia", async () => {
    const { core, parent, preview } = setup((a) => (a.index === 0 ? svgWith(A) : "<svg></svg>"));
    core.on("preview_asset", () => {
      throw "not_found";
    });
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    preview.update(upd({ pages: pages(2) }));
    await preview.idle();
    expect(imgOf(parent, 0)).toBeNull();
    expect(imgOf(parent, 1)?.getAttribute("src")).toMatch(/^blob:/);
    warn.mockRestore();
  });

  it("ignora texto que só parece referência (hash malformado)", async () => {
    const { core, parent, preview, blobs } = setupAssets(() => `<svg><text>prelo-asset://xyz</text></svg>`);
    preview.update(upd({ pages: pages(1) }));
    await preview.idle();
    expect(core.calls("preview_asset")).toHaveLength(0);
    expect(await blobs.get(imgOf(parent, 0)!.getAttribute("src")!)!.text()).toContain("prelo-asset://xyz");
  });
});
