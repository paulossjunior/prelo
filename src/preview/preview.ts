// Prévia ao vivo (T038; FR-013, FR-014; research R4). Cada página é um <img> com blob URL do
// SVG de `page_svg`: o SVG nunca entra no DOM como marcação, então não executa script nem
// carrega recurso externo. Só as páginas visíveis ± 2 são pedidas (virtualização); as que saem
// dessa janela soltam a imagem. Uma página só é pedida de novo quando o hash dela muda (o
// núcleo lista essas em `PreviewUpdate.changed`). Compilação com erro mantém as páginas e mostra
// a faixa de erro.
import { toIpcError, type Ipc, type PageInfo, type PreviewUpdate } from "../ipc";
import "../workspace.css";

export interface BlobUrls {
  create(blob: Blob): string;
  revoke(url: string): void;
}

export interface PreviewOptions {
  ipc: Ipc;
  /** Criação e revogação de blob URLs; padrão `URL.createObjectURL`/`revokeObjectURL`. */
  urls?: BlobUrls;
}

export interface PreviewHandle {
  readonly element: HTMLElement;
  /** Aplica um `PreviewUpdate` recebido por `subscribe_preview`. */
  update(u: PreviewUpdate): void;
  /** Índices das páginas visíveis (o observador de rolagem chama isto). */
  setVisible(indices: number[]): void;
  /** Espera os pedidos de página em curso terminarem. */
  idle(): Promise<void>;
  destroy(): void;
}

/** Páginas pedidas além das visíveis, para cada lado. */
export const MARGIN = 2;

interface Slot {
  info: PageInfo;
  el: HTMLElement;
  img: HTMLImageElement | null;
  url: string | null;
  shownHash: string | null;
  loadingHash: string | null;
  /** Hash cujo pedido falhou; só é pedido de novo com hash novo ou ao voltar à janela. */
  failedHash: string | null;
  removed: boolean;
}

const defaultUrls: BlobUrls = {
  create: (b) => URL.createObjectURL(b),
  revoke: (u) => URL.revokeObjectURL(u),
};

function errorText(u: PreviewUpdate): string {
  const n = u.diagnostics.filter((d) => d.severity === "error").length;
  const what = n === 0 ? "A compilação falhou" : `Há ${n} ${n === 1 ? "erro" : "erros"} no documento`;
  return `${what}; a prévia mostra a última versão que compilou.`;
}

export function mountPreview(parent: HTMLElement, opts: PreviewOptions): PreviewHandle {
  const { ipc } = opts;
  const urls = opts.urls ?? defaultUrls;

  const root = document.createElement("section");
  root.className = "pv";
  root.setAttribute("aria-label", "Prévia");
  const band = document.createElement("div");
  band.className = "pv-error";
  band.setAttribute("role", "status");
  band.hidden = true;
  const pagesEl = document.createElement("div");
  pagesEl.className = "pv-pages";
  root.append(band, pagesEl);
  parent.append(root);

  const slots: Slot[] = [];
  const inflight = new Set<Promise<void>>();
  let lastCompileId = -Infinity;
  let visible: number[] = [0];
  let destroyed = false;

  const visibleSet = new Set<number>();
  const observer =
    typeof IntersectionObserver === "function"
      ? new IntersectionObserver(
          (entries) => {
            for (const e of entries) {
              const i = Number((e.target as HTMLElement).dataset.pageIndex);
              if (e.isIntersecting) visibleSet.add(i);
              else visibleSet.delete(i);
            }
            if (visibleSet.size > 0) setVisible([...visibleSet]);
          },
          { root: pagesEl },
        )
      : null;

  function release(slot: Slot) {
    slot.img?.remove();
    slot.img = null;
    if (slot.url) urls.revoke(slot.url);
    slot.url = null;
    slot.shownHash = null;
    slot.failedHash = null;
  }

  function show(slot: Slot, hash: string, svg: string) {
    const url = urls.create(new Blob([svg], { type: "image/svg+xml" }));
    if (!slot.img) {
      const img = document.createElement("img");
      img.alt = `Página ${slot.info.index + 1}`;
      img.decoding = "async";
      img.draggable = false;
      slot.el.append(img);
      slot.img = img;
    }
    const old = slot.url;
    slot.img.src = url;
    slot.url = url;
    slot.shownHash = hash;
    if (old) urls.revoke(old);
  }

  function inWindow(index: number): boolean {
    const lo = Math.min(...visible) - MARGIN;
    const hi = Math.max(...visible) + MARGIN;
    return index >= lo && index <= hi;
  }

  function fetchPage(slot: Slot) {
    const { index, hash } = slot.info;
    slot.loadingHash = hash;
    const p = ipc
      .pageSvg(index, hash)
      .then(
        (svg) => {
          // Resposta de hash velho, página removida ou fora da janela: descarta.
          if (destroyed || slot.removed || slot.info.hash !== hash || !inWindow(index)) return;
          if (typeof svg === "string") show(slot, hash, svg);
        },
        (err: unknown) => {
          // `stale_page`: chegou compilação nova; `sync` abaixo pede com o hash atual (ou o
          // próximo `PreviewUpdate` traz o hash novo). Outras falhas deixam o espaço da página
          // vazio, sem derrubar a prévia e sem repetir o pedido em laço.
          if (!slot.removed) slot.failedHash = hash;
          if (toIpcError(err).code !== "stale_page") console.warn("prévia: página", index, err);
        },
      )
      .finally(() => {
        inflight.delete(p);
        if (slot.loadingHash === hash) slot.loadingHash = null;
        if (!destroyed) sync();
      });
    inflight.add(p);
  }

  /** Pede as páginas da janela que estão com hash desatualizado e solta as de fora. */
  function sync() {
    for (const slot of slots) {
      const { index, hash } = slot.info;
      if (!inWindow(index)) {
        release(slot);
        continue;
      }
      if (slot.shownHash === hash || slot.loadingHash !== null || slot.failedHash === hash) continue;
      fetchPage(slot);
    }
  }

  function setVisible(indices: number[]) {
    visible = indices.length > 0 ? indices : [0];
    sync();
  }

  function update(u: PreviewUpdate) {
    if (destroyed || u.compileId <= lastCompileId) return;
    lastCompileId = u.compileId;
    if (!u.ok) {
      band.textContent = errorText(u);
      band.hidden = false;
      return;
    }
    band.hidden = true;
    band.textContent = "";

    while (slots.length > u.pages.length) {
      const slot = slots.pop()!;
      slot.removed = true;
      release(slot);
      observer?.unobserve(slot.el);
      slot.el.remove();
    }
    u.pages.forEach((info, i) => {
      let slot = slots[i];
      if (!slot) {
        const el = document.createElement("div");
        el.className = "pv-page";
        pagesEl.append(el);
        slot = { info, el, img: null, url: null, shownHash: null, loadingHash: null, failedHash: null, removed: false };
        slots.push(slot);
        observer?.observe(el);
      }
      slot.info = info;
      slot.el.dataset.pageIndex = String(info.index);
      slot.el.style.aspectRatio = `${info.widthPt} / ${info.heightPt}`;
      if (slot.img) slot.img.alt = `Página ${info.index + 1}`;
    });
    sync();
  }

  async function idle() {
    while (inflight.size > 0) await Promise.allSettled([...inflight]);
  }

  return {
    element: root,
    update,
    setVisible,
    idle,
    destroy() {
      destroyed = true;
      observer?.disconnect();
      for (const slot of slots) release(slot);
      slots.length = 0;
      root.remove();
    },
  };
}
