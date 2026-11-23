// Sincronização prévia ↔ código na interface (T064; FR-020, US4 cenários 1, 2, 3 e 5; research
// R5; contracts/ipc.md `jump_from_click`, `jump_from_cursor`).
//
// Clique na prévia: o ponto vira pontos tipográficos pela caixa que a página ocupa na tela
// (`getBoundingClientRect`), então vale em qualquer zoom sem conhecer o nível; o núcleo devolve
// `{ path, line, column }` e quem monta abre o arquivo ali. `null` (sem origem no projeto:
// numeração automática, pacote, margem) só avisa, sem mexer no cursor.
//
// "Localizar na prévia": o núcleo devolve os pontos do trecho sob o cursor; a prévia rola até o
// primeiro e desenha um destaque temporário. O destaque é posicionado em porcentagem da página,
// então acompanha o zoom enquanto estiver na tela.
//
// Convenções: `page` a partir de 0 (o índice de `PreviewUpdate.pages`); `line` e `column` a
// partir de 1, `column` em unidades UTF-16 (as do CodeMirror).
import { toIpcError, type Ipc, type PagePosition, type SourcePosition } from "../ipc";
import type { PreviewHandle } from "./preview";

export interface Rect {
  left: number;
  top: number;
  width: number;
  height: number;
}

export interface PageSize {
  widthPt: number;
  heightPt: number;
}

/** Quanto tempo o destaque fica na prévia (ms). */
export const MARK_MS = 1800;

export const NO_ORIGIN = "Este trecho da prévia não tem origem no projeto (vem de numeração automática ou de pacote).";
export const NO_POINT = "Não há trecho correspondente na prévia: ponha o cursor sobre texto do documento.";

/** Ponto da página (pt) sob `(clientX, clientY)`, ou `null` fora dela. */
export function clickToPoint(
  rect: Rect,
  page: PageSize,
  clientX: number,
  clientY: number,
): { xPt: number; yPt: number } | null {
  if (!(rect.width > 0 && rect.height > 0)) return null;
  const fx = (clientX - rect.left) / rect.width;
  const fy = (clientY - rect.top) / rect.height;
  if (fx < 0 || fx > 1 || fy < 0 || fy > 1) return null;
  return { xPt: fx * page.widthPt, yPt: fy * page.heightPt };
}

/** Deslocamento UTF-16 de `(line, column)` (a partir de 1) em `text`, limitado ao texto. */
export function offsetOf(text: string, line: number, column: number): number {
  let start = 0;
  for (let n = 1; n < line; n++) {
    const nl = text.indexOf("\n", start);
    if (nl < 0) return text.length;
    start = nl + 1;
  }
  let end = text.indexOf("\n", start);
  if (end < 0) end = text.length;
  return Math.min(start + Math.max(0, column - 1), end);
}

export interface SyncOptions {
  ipc: Ipc;
  preview: PreviewHandle;
  /** Abre `pos.path` no editor com o cursor em `pos.line`/`pos.column`. */
  openAt(pos: SourcePosition): void | Promise<void>;
  /** Arquivo e deslocamento (UTF-16) do cursor do editor; `null` sem arquivo aberto. */
  cursor(): { path: string; offset: number } | null;
  /** Antes de perguntar ao núcleo, envia as edições pendentes do editor. */
  flush?(): Promise<void>;
  /** Aviso curto para a pessoa (sem origem, sem ponto, recusa). */
  notify(message: string): void;
}

export interface SyncHandle {
  /** "Localizar na prévia" a partir do cursor; `true` se achou e destacou. */
  locate(): Promise<boolean>;
  /** Rola a prévia até `pos` e o destaca; sem `xPt`, destaca a largura da página. */
  reveal(pos: { page: number; yPt: number; xPt?: number }): boolean;
  destroy(): void;
}

export function mountSync(opts: SyncOptions): SyncHandle {
  const { ipc, preview } = opts;
  const scroller = preview.pagesElement;
  let destroyed = false;
  let mark: HTMLElement | null = null;
  let markTimer: ReturnType<typeof setTimeout> | undefined;
  // Cliques seguidos: só a resposta do último move o editor.
  let clickTicket = 0;

  const pageEl = (index: number) => scroller.querySelector<HTMLElement>(`.pv-page[data-page-index="${index}"]`);

  function clearMark() {
    clearTimeout(markTimer);
    mark?.remove();
    mark = null;
  }

  function reveal(pos: { page: number; yPt: number; xPt?: number }): boolean {
    const el = pageEl(pos.page);
    const info = preview.pageInfo(pos.page);
    if (!el || !info || !(info.heightPt > 0 && info.widthPt > 0)) return false;
    const fy = Math.min(1, Math.max(0, pos.yPt / info.heightPt));
    // Linha do ponto a um terço da altura visível: dá para ler o que vem antes e depois.
    const y = el.offsetTop + fy * el.offsetHeight;
    scroller.scrollTop = Math.max(0, y - scroller.clientHeight / 3);

    clearMark();
    const m = document.createElement("div");
    m.className = "pv-mark";
    m.setAttribute("aria-hidden", "true");
    // O ponto do Typst fica na linha de base: o destaque cobre a linha acima dela.
    const linePt = 14;
    m.style.top = `${Math.max(0, ((pos.yPt - linePt * 0.8) / info.heightPt) * 100)}%`;
    m.style.height = `${(linePt / info.heightPt) * 100}%`;
    if (pos.xPt === undefined) {
      m.style.left = "0%";
      m.style.width = "100%";
    } else {
      m.style.left = `${Math.max(0, (pos.xPt / info.widthPt) * 100)}%`;
      m.style.width = `${(40 / info.widthPt) * 100}%`;
    }
    el.append(m);
    mark = m;
    markTimer = setTimeout(clearMark, MARK_MS);
    return true;
  }

  async function onClick(e: MouseEvent) {
    if (e.button !== 0 || e.defaultPrevented) return;
    const el = (e.target as Element | null)?.closest?.<HTMLElement>(".pv-page[data-page-index]");
    if (!el || !scroller.contains(el)) return;
    const index = Number(el.dataset.pageIndex);
    const info = preview.pageInfo(index);
    if (!info) return;
    const point = clickToPoint(el.getBoundingClientRect(), info, e.clientX, e.clientY);
    if (!point) return;
    const ticket = ++clickTicket;
    let pos: SourcePosition | null;
    try {
      pos = await ipc.jumpFromClick(index, point.xPt, point.yPt);
    } catch (err) {
      if (!destroyed && ticket === clickTicket) opts.notify(toIpcError(err).message);
      return;
    }
    if (destroyed || ticket !== clickTicket) return;
    if (!pos) {
      opts.notify(NO_ORIGIN);
      return;
    }
    await opts.openAt(pos);
  }

  async function locate(): Promise<boolean> {
    const at = opts.cursor();
    if (!at) return false;
    let targets: PagePosition[];
    try {
      await opts.flush?.();
      targets = await ipc.jumpFromCursor(at.path, at.offset);
    } catch (err) {
      if (!destroyed) opts.notify(toIpcError(err).message);
      return false;
    }
    if (destroyed) return false;
    const first = targets[0];
    if (!first || !reveal(first)) {
      opts.notify(NO_POINT);
      return false;
    }
    return true;
  }

  const listener = (e: MouseEvent) => void onClick(e);
  scroller.addEventListener("click", listener);

  return {
    locate,
    reveal,
    destroy() {
      destroyed = true;
      clearMark();
      scroller.removeEventListener("click", listener);
    },
  };
}
