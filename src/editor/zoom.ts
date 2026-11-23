// Zoom do editor (T095; FR-013c): muda o tamanho da fonte do CodeMirror, e com ela os números de
// linha (que herdam o tamanho e têm largura em em) e a entrelinha (proporcional). Controles no
// cabeçalho do editor, no estilo da barra da prévia; ⌘/Ctrl + `+`/`-`/`0` só com o foco no
// editor (o mapa de teclas do CodeMirror não usa Mod-=, Mod-- nem Mod-0); ⌘/Ctrl + roda e pinça
// sobre o editor. A linha que estava no topo volta ao topo depois da nova medida; cursor e
// seleção ficam no estado do editor e não mudam. Nível lembrado por projeto com o mesmo
// armazenamento do zoom da prévia.
import { EditorView } from "@codemirror/view";
import { loadLevel, saveLevel, zoomShortcut, type ZoomStorage } from "../preview/zoom";

export const EDITOR_ZOOM_STEPS = [70, 80, 90, 100, 110, 125, 150, 175, 200] as const;
export const EDITOR_MIN = 70;
export const EDITOR_MAX = 200;
/** Tamanho da fonte do editor a 100 % (protótipo: mono 12,5 px). */
export const BASE_FONT_PX = 12.5;
const PREFIX = "prelo.editor.zoom:";
const TOL = 0.5;

export function clampEditorZoom(pct: number): number {
  if (!Number.isFinite(pct)) return 100;
  return Math.min(EDITOR_MAX, Math.max(EDITOR_MIN, pct));
}
const up = (c: number) => EDITOR_ZOOM_STEPS.find((s) => s > c + TOL) ?? EDITOR_MAX;
const down = (c: number) => [...EDITOR_ZOOM_STEPS].reverse().find((s) => s < c - TOL) ?? EDITOR_MIN;

export interface EditorZoomOptions {
  /** Cabeçalho do editor, onde entra o grupo de controles. */
  header: HTMLElement;
  /** Área do editor: atalhos, roda e pinça só valem dentro dela. */
  target: HTMLElement;
  key?: string | null;
  storage?: ZoomStorage | null;
}

export interface EditorZoomHandle {
  level(): number;
  set(level: number): void;
  /** Aplica o nível a um editor recém-montado (cada arquivo aberto é um editor novo). */
  attach(view: EditorView): void;
  setKey(key: string | null): void;
  destroy(): void;
}

function load(key: string | null | undefined, storage: ZoomStorage | null | undefined): number {
  const v = loadLevel(PREFIX, key, storage);
  return typeof v === "number" ? clampEditorZoom(v) : 100;
}

export function mountEditorZoom(opts: EditorZoomOptions): EditorZoomHandle {
  const { target } = opts;
  let key = opts.key ?? null;
  let level = load(key, opts.storage);
  let view: EditorView | null = null;

  const seg = document.createElement("div");
  seg.className = "seg ed-zoom";
  seg.setAttribute("role", "group");
  seg.setAttribute("aria-label", "Zoom do editor");
  const mk = (text: string, aria: string, title: string) => {
    const b = document.createElement("button");
    b.type = "button";
    b.textContent = text;
    b.setAttribute("aria-label", aria);
    b.title = title;
    return b;
  };
  const outBtn = mk("−", "Diminuir zoom do editor", "Diminuir zoom do editor (⌘− / Ctrl+−)");
  const pct = document.createElement("span");
  pct.className = "ed-zoom-pct";
  const inBtn = mk("+", "Aumentar zoom do editor", "Aumentar zoom do editor (⌘+ / Ctrl++)");
  const resetBtn = mk("100%", "Zoom do editor em 100%", "Zoom do editor em 100% (⌘0 / Ctrl+0)");
  seg.append(outBtn, pct, inBtn, resetBtn);
  const live = document.createElement("span");
  live.className = "ed-live";
  live.setAttribute("role", "status");
  live.setAttribute("aria-live", "polite");
  opts.header.append(seg, live);

  function render() {
    pct.textContent = `${Math.round(level)}%`;
    outBtn.disabled = level <= EDITOR_MIN + TOL;
    inBtn.disabled = level >= EDITOR_MAX - TOL;
    resetBtn.setAttribute("aria-pressed", String(Math.abs(level - 100) < TOL));
  }

  /** Muda a fonte do editor; a linha do topo volta ao topo depois da nova medida. */
  function applyTo(v: EditorView, keepTop: boolean) {
    const top = keepTop ? v.lineBlockAtHeight(v.scrollDOM.scrollTop).from : null;
    // CSSOM (não atributo `style`): permitido pela CSP `style-src 'self'`.
    v.dom.style.fontSize = `${(BASE_FONT_PX * level) / 100}px`;
    v.requestMeasure();
    if (top !== null && top > 0) v.dispatch({ effects: EditorView.scrollIntoView(top, { y: "start" }) });
  }

  function set(next: number, announce = true) {
    level = clampEditorZoom(next);
    if (view) applyTo(view, true);
    saveLevel(PREFIX, key, Math.round(level * 10) / 10, opts.storage);
    render();
    if (announce) live.textContent = `Zoom do editor ${Math.round(level)}%`;
  }

  outBtn.addEventListener("click", () => set(down(level)));
  inBtn.addEventListener("click", () => set(up(level)));
  resetBtn.addEventListener("click", () => set(100));

  const onKey = (e: KeyboardEvent) => {
    const dir = zoomShortcut(e);
    if (!dir) return;
    e.preventDefault();
    set(dir === "fit" ? 100 : dir === "in" ? up(level) : down(level));
  };
  // Na roda e na pinça só a área de código conta (o cabeçalho tem os próprios botões).
  let gestureBase: number | null = null;
  const onWheel = (e: WheelEvent) => {
    if (!(e.ctrlKey || e.metaKey)) return;
    e.preventDefault();
    if (gestureBase !== null) return;
    if (e.deltaMode !== 0 || Math.abs(e.deltaY) >= 50) {
      if (e.deltaY !== 0) set(e.deltaY < 0 ? up(level) : down(level));
      return;
    }
    set(level * Math.exp(-e.deltaY / 100));
  };
  const onGestureStart = (e: Event) => {
    e.preventDefault();
    gestureBase = level;
  };
  const onGestureChange = (e: Event) => {
    e.preventDefault();
    set((gestureBase ?? level) * ((e as Event & { scale?: number }).scale ?? 1), false);
  };
  const onGestureEnd = (e: Event) => {
    e.preventDefault();
    gestureBase = null;
    live.textContent = `Zoom do editor ${Math.round(level)}%`;
  };
  target.addEventListener("keydown", onKey);
  target.addEventListener("wheel", onWheel, { passive: false });
  target.addEventListener("gesturestart", onGestureStart);
  target.addEventListener("gesturechange", onGestureChange);
  target.addEventListener("gestureend", onGestureEnd);

  render();

  return {
    level: () => level,
    set: (n) => set(n),
    attach(v) {
      view = v;
      applyTo(v, false);
    },
    setKey(next) {
      key = next;
      level = load(key, opts.storage);
      if (view) applyTo(view, false);
      render();
    },
    destroy() {
      target.removeEventListener("keydown", onKey);
      target.removeEventListener("wheel", onWheel);
      target.removeEventListener("gesturestart", onGestureStart);
      target.removeEventListener("gesturechange", onGestureChange);
      target.removeEventListener("gestureend", onGestureEnd);
      seg.remove();
      live.remove();
      view = null;
    },
  };
}
