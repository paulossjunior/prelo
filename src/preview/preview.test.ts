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
