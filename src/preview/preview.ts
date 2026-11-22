// Prévia ao vivo (T038; FR-013, FR-014; research R4). Cada página é um <img> com blob URL do
// SVG de `page_svg`: o SVG nunca entra no DOM como marcação, então não executa script nem
// carrega recurso externo. Só as páginas visíveis ± 2 são pedidas (virtualização); as que saem
// dessa janela soltam a imagem. Uma página só é pedida de novo quando o hash dela muda (o
// núcleo lista essas em `PreviewUpdate.changed`). Compilação com erro mantém as páginas e mostra
// a faixa de erro.
//
// Zoom (T091; FR-013a): a barra da prévia tem "−", "+", a porcentagem e "Ajustar à largura"
// (padrão). O zoom só muda a largura CSS das páginas; como cada página é um SVG, ela continua
// nítida e nada é pedido de novo ao núcleo. Ao mudar o nível, o ponto da página sob o centro (ou
// sob o ponteiro, na roda e na pinça) volta ao mesmo lugar da tela.
import { toIpcError, type Ipc, type PageInfo, type PreviewUpdate } from "../ipc";
import {
  MAX_ZOOM,
  MIN_ZOOM,
  PX_PER_PT,
  anchorScroll,
  clampZoom,
  loadZoom,
  saveZoom,
  stepIn,
  stepOut,
  zoomShortcut,
  type Box,
  type ZoomLevel,
  type ZoomStorage,
} from "./zoom";
import "../workspace.css";

export interface BlobUrls {
  create(blob: Blob): string;
  revoke(url: string): void;
}

export interface PreviewOptions {
  ipc: Ipc;
  /** Criação e revogação de blob URLs; padrão `URL.createObjectURL`/`revokeObjectURL`. */
  urls?: BlobUrls;
  /** Chave do projeto para lembrar o zoom; sem chave, o zoom não é lembrado. */
  zoomKey?: string | null;
  /** Onde o zoom é lembrado; padrão `localStorage`. */
  storage?: ZoomStorage | null;
}

export interface PreviewHandle {
  readonly element: HTMLElement;
  /** Aplica um `PreviewUpdate` recebido por `subscribe_preview`. */
  update(u: PreviewUpdate): void;
  /** Índices das páginas visíveis (o observador de rolagem chama isto). */
  setVisible(indices: number[]): void;
  /** Espera os pedidos de página em curso terminarem. */
  idle(): Promise<void>;
  /** Nível de zoom atual: "fit" (ajustar à largura) ou porcentagem. */
  zoom(): ZoomLevel;
  /** Muda o zoom mantendo o trecho sob o centro da prévia. */
  setZoom(level: ZoomLevel): void;
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

  // Barra da prévia como a do protótipo (.pvbar + .seg).
  const bar = document.createElement("div");
  bar.className = "pvbar";
  const label = document.createElement("span");
  label.className = "pv-label";
  label.textContent = "Prévia";
  const seg = document.createElement("div");
  seg.className = "seg";
  seg.setAttribute("role", "group");
  seg.setAttribute("aria-label", "Zoom da prévia");
  const mkButton = (text: string, aria: string | null, cls: string) => {
    const b = document.createElement("button");
    b.type = "button";
    b.className = cls;
    b.textContent = text;
    if (aria) b.setAttribute("aria-label", aria);
    return b;
  };
  const outBtn = mkButton("−", "Diminuir zoom", "pv-zoom-out");
  outBtn.title = "Diminuir zoom (⌘− / Ctrl+−)";
  const pct = document.createElement("span");
  pct.className = "pv-zoom-pct";
  const inBtn = mkButton("+", "Aumentar zoom", "pv-zoom-in");
  inBtn.title = "Aumentar zoom (⌘+ / Ctrl++)";
  const fitBtn = mkButton("Ajustar à largura", null, "pv-zoom-fit");
  fitBtn.title = "Ajustar à largura (⌘0 / Ctrl+0)";
  seg.append(outBtn, pct, inBtn, fitBtn);
  const live = document.createElement("span");
  live.className = "pv-live";
  live.setAttribute("role", "status");
  live.setAttribute("aria-live", "polite");
  bar.append(label, seg, live);

  const band = document.createElement("div");
  band.className = "pv-error";
  band.setAttribute("role", "status");
  band.hidden = true;
  const pagesEl = document.createElement("div");
  pagesEl.className = "pv-pages pv-fit";
  // Focável para rolar pelo teclado e para os atalhos de zoom valerem só aqui.
  pagesEl.tabIndex = 0;
  pagesEl.setAttribute("aria-label", "Páginas da prévia");
  root.append(bar, band, pagesEl);
  parent.append(root);

  const slots: Slot[] = [];
  const inflight = new Set<Promise<void>>();
  let lastCompileId = -Infinity;
  let visible: number[] = [0];
  let destroyed = false;
  let level: ZoomLevel = loadZoom(opts.zoomKey, opts.storage);

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
    renderLabel();
    sync();
  }

  // ---------------------------------------------------------------- zoom

  function renderLabel() {
    label.textContent =
      slots.length === 0 ? "Prévia" : `Prévia · pág. ${Math.min(Math.min(...visible) + 1, slots.length)} de ${slots.length}`;
  }

  function sizePage(slot: Slot) {
    slot.el.style.width = level === "fit" ? "" : `${(slot.info.widthPt * PX_PER_PT * level) / 100}px`;
  }

  function padX(): number {
    const cs = getComputedStyle(pagesEl);
    const l = parseFloat(cs.paddingLeft);
    const r = parseFloat(cs.paddingRight);
    return (Number.isFinite(l) ? l : 18) + (Number.isFinite(r) ? r : 18);
  }

  /** Porcentagem em vigor; em "ajustar à largura", a calculada pela largura do painel. */
  function effective(): number | null {
    if (level !== "fit") return level;
    const widest = Math.max(0, ...slots.map((s) => s.info.widthPt * PX_PER_PT));
    const avail = pagesEl.clientWidth - padX();
    return widest > 0 && avail > 0 ? (avail / widest) * 100 : null;
  }

  function renderZoom() {
    const eff = effective();
    pct.textContent = eff === null ? "—" : `${Math.round(eff)}%`;
    fitBtn.setAttribute("aria-pressed", String(level === "fit"));
    const cur = eff ?? 100;
    outBtn.disabled = cur <= MIN_ZOOM + 0.5;
    inBtn.disabled = cur >= MAX_ZOOM - 0.5;
  }

  function boxOf(el: HTMLElement): Box {
    return { top: el.offsetTop, left: el.offsetLeft, width: el.offsetWidth, height: el.offsetHeight };
  }

  /** Página sob o ponto `at` da tela da prévia (ou a mais próxima acima). */
  function slotAt(at: { x: number; y: number }): Slot | null {
    const y = pagesEl.scrollTop + at.y;
    let best: Slot | null = null;
    for (const s of slots) {
      if (s.el.offsetTop <= y) best = s;
      else break;
    }
    return best;
  }

  function applyZoom(next: ZoomLevel, at?: { x: number; y: number }) {
    next = next === "fit" ? "fit" : clampZoom(next);
    const point = at ?? { x: pagesEl.clientWidth / 2, y: pagesEl.clientHeight / 2 };
    const anchor = slotAt(point);
    const before = anchor ? boxOf(anchor.el) : null;
    const scroll = { top: pagesEl.scrollTop, left: pagesEl.scrollLeft };
    level = next;
    pagesEl.classList.toggle("pv-fit", level === "fit");
    for (const s of slots) sizePage(s);
    if (anchor && before) {
      const s = anchorScroll(before, boxOf(anchor.el), point, scroll);
      pagesEl.scrollTop = s.top;
      pagesEl.scrollLeft = s.left;
    }
    saveZoom(opts.zoomKey, level, opts.storage);
    renderZoom();
  }

  function announce() {
    const eff = effective();
    live.textContent =
      level === "fit" ? "Zoom: ajustar à largura" : `Zoom ${Math.round(eff ?? level)}%`;
  }

  function zoomBy(dir: "in" | "out" | "fit", at?: { x: number; y: number }) {
    const cur = effective() ?? 100;
    applyZoom(dir === "fit" ? "fit" : dir === "in" ? stepIn(cur) : stepOut(cur), at);
    announce();
  }

  function pointerIn(e: { clientX?: number; clientY?: number }): { x: number; y: number } | undefined {
    if (typeof e.clientX !== "number" || typeof e.clientY !== "number") return undefined;
    const r = pagesEl.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  }

  inBtn.addEventListener("click", () => zoomBy("in"));
  outBtn.addEventListener("click", () => zoomBy("out"));
  fitBtn.addEventListener("click", () => zoomBy("fit"));

  // Atalhos só com o foco dentro da prévia: o editor fica com as teclas dele.
  root.addEventListener("keydown", (e) => {
    const dir = zoomShortcut(e);
    if (!dir) return;
    e.preventDefault();
    zoomBy(dir);
  });

  // Pinça do WebKit (WKWebView do macOS): eventos de gesto com `scale` relativo ao início.
  let gestureBase: number | null = null;
  pagesEl.addEventListener("gesturestart", (e) => {
    e.preventDefault();
    gestureBase = effective() ?? 100;
  });
  pagesEl.addEventListener("gesturechange", (e) => {
    e.preventDefault();
    const scale = (e as Event & { scale?: number }).scale ?? 1;
    applyZoom(clampZoom((gestureBase ?? effective() ?? 100) * scale), pointerIn(e as unknown as MouseEvent));
  });
  pagesEl.addEventListener("gestureend", (e) => {
    e.preventDefault();
    gestureBase = null;
    announce();
  });

  // ⌘/Ctrl + roda: degrau por "clique" do mouse; deltas pequenos (pinça no Chromium/WebView2,
  // que chega como roda com Ctrl) dão zoom contínuo.
  pagesEl.addEventListener(
    "wheel",
    (e) => {
      if (!(e.ctrlKey || e.metaKey)) return;
      e.preventDefault();
      if (gestureBase !== null) return; // o gesto do WebKit já está cuidando
      const at = pointerIn(e);
      if (e.deltaMode !== 0 || Math.abs(e.deltaY) >= 50) {
        if (e.deltaY !== 0) zoomBy(e.deltaY < 0 ? "in" : "out", at);
        return;
      }
      applyZoom(clampZoom((effective() ?? 100) * Math.exp(-e.deltaY / 100)), at);
      announce();
    },
    { passive: false },
  );

  const resizer =
    typeof ResizeObserver === "function" ? new ResizeObserver(() => level === "fit" && renderZoom()) : null;
  resizer?.observe(pagesEl);
  applyZoom(level);

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
      sizePage(slot);
    });
    renderLabel();
    renderZoom();
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
    zoom: () => level,
    setZoom: (next: ZoomLevel) => applyZoom(next),
    destroy() {
      destroyed = true;
      observer?.disconnect();
      resizer?.disconnect();
      for (const slot of slots) release(slot);
      slots.length = 0;
      root.remove();
    },
  };
}
