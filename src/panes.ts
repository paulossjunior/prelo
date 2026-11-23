// Painéis ajustáveis da área de trabalho (T093; FR-013b): árvore | editor | prévia com
// divisórias arrastáveis (ponteiro e teclado), botão de recolher em cada painel, trilho fino
// para reabrir o painel recolhido, modos Editor · Ambos · Prévia, atalhos e arranjo lembrado
// por projeto. Editor e prévia nunca ficam os dois recolhidos.
//
// Atalhos: ⌘ (macOS) ou Ctrl (outros sistemas) + 1 só editor, 2 só prévia, 3 ambos, `\` mostra
// ou esconde a árvore. O CodeMirror não usa Mod-1/2/3 nem Mod-\ (só Mod-Alt-\ e Shift-Mod-\,
// que ficam com ele: com Alt ou Shift a tecla segue adiante).

export type PaneId = "tree" | "editor" | "preview";
export type ViewMode = "editor" | "preview" | "both";

export interface PaneLayout {
  tree: boolean;
  editor: boolean;
  preview: boolean;
  /** Largura da árvore em px. */
  treeWidth: number;
  /** Fração da largura de editor + prévia que fica com o editor. */
  split: number;
}

/** Padrão do protótipo: 200 px | 1,25fr | 1fr. */
export const DEFAULT_LAYOUT: PaneLayout = {
  tree: true,
  editor: true,
  preview: true,
  treeWidth: 200,
  split: 1.25 / 2.25,
};
export const MIN_TREE = 140;
export const MAX_TREE = 420;
/** Largura mínima do editor e da prévia quando os dois estão abertos. */
export const MIN_PANE = 240;
export const RAIL = 28;
export const SPLITTER = 6;
const MIN_SPLIT = 0.15;
const MAX_SPLIT = 0.85;
const KEY_STEP = 16;
const KEY_STEP_BIG = 64;

export type PaneStorage = Pick<Storage, "getItem" | "setItem">;

export interface PanesOptions {
  body: HTMLElement;
  tree: HTMLElement;
  editor: HTMLElement;
  preview: HTMLElement;
  /** Cabeçalhos onde entram os botões de recolher da árvore e do editor. */
  treeHeader: HTMLElement;
  editorHeader: HTMLElement;
  /** Onde o arranjo é lembrado; padrão `localStorage`; `null` não lembra. */
  storage?: PaneStorage | null;
  /** Chave do projeto; sem chave, o arranjo não é lembrado. */
  key?: string | null;
  /** Onde escutar os atalhos; padrão `document`. */
  keyTarget?: Pick<Document, "addEventListener" | "removeEventListener">;
  /** Depois de cada mudança (ex.: pedir ao editor para medir de novo). */
  onChange?: (layout: PaneLayout) => void;
}

export interface PanesHandle {
  layout(): PaneLayout;
  setOpen(pane: PaneId, open: boolean): void;
  setMode(mode: ViewMode): void;
  toggleTree(): void;
  /** Troca de projeto: lê o arranjo guardado para a chave nova. */
  setKey(key: string | null): void;
  /** Botão de recolher da prévia; a área de trabalho o põe na barra da prévia. */
  readonly previewButton: HTMLButtonElement;
  /** Controle para a barra de cima: Editor · Ambos · Prévia e o botão da árvore. */
  readonly modeControl: HTMLElement;
  destroy(): void;
}

const PREFIX = "prelo.panes:";

const LABEL: Record<PaneId, { name: string; collapse: string; expand: string }> = {
  tree: { name: "Arquivos", collapse: "Recolher árvore de arquivos", expand: "Expandir árvore de arquivos" },
  editor: { name: "Editor", collapse: "Recolher editor", expand: "Expandir editor" },
  preview: { name: "Prévia", collapse: "Recolher prévia", expand: "Expandir prévia" },
};

/** Seta para a borda onde o painel se recolhe (‹ esquerda, › direita) e a oposta para abrir. */
const CHEVRON: Record<PaneId, { collapse: string; expand: string }> = {
  tree: { collapse: "‹", expand: "›" },
  editor: { collapse: "‹", expand: "›" },
  preview: { collapse: "›", expand: "‹" },
};

/** Elementos que rolam dentro de cada painel e têm a posição guardada ao esconder. */
const SCROLLERS = [".ws-side", ".cm-scroller", ".pv-pages"];

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

function defaultStorage(): PaneStorage | null {
  try {
    return globalThis.localStorage ?? null;
  } catch {
    return null;
  }
}

/** Valida o que veio do armazenamento; qualquer coisa estranha volta ao padrão. */
export function normalizeLayout(raw: unknown): PaneLayout {
  if (!raw || typeof raw !== "object") return { ...DEFAULT_LAYOUT };
  const r = raw as Record<string, unknown>;
  const bool = (v: unknown, d: boolean) => (typeof v === "boolean" ? v : d);
  const num = (v: unknown, d: number) => (typeof v === "number" && Number.isFinite(v) ? v : d);
  const l: PaneLayout = {
    tree: bool(r.tree, true),
    editor: bool(r.editor, true),
    preview: bool(r.preview, true),
    treeWidth: clamp(num(r.treeWidth, DEFAULT_LAYOUT.treeWidth), MIN_TREE, MAX_TREE),
    split: clamp(num(r.split, DEFAULT_LAYOUT.split), MIN_SPLIT, MAX_SPLIT),
  };
  if (!l.editor && !l.preview) l.editor = l.preview = true;
  return l;
}

function load(key: string | null, storage: PaneStorage | null): PaneLayout {
  if (!key || !storage) return { ...DEFAULT_LAYOUT };
  try {
    const v = storage.getItem(PREFIX + key);
    return v === null ? { ...DEFAULT_LAYOUT } : normalizeLayout(JSON.parse(v));
  } catch {
    return { ...DEFAULT_LAYOUT };
  }
}

function save(key: string | null, storage: PaneStorage | null, l: PaneLayout) {
  if (!key || !storage) return;
  try {
    storage.setItem(PREFIX + key, JSON.stringify(l));
  } catch {
    // O arranjo vale nesta sessão; só não é lembrado.
  }
}

function el<K extends keyof HTMLElementTagNameMap>(tag: K, cls: string, text?: string): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  e.className = cls;
  if (text !== undefined) e.textContent = text;
  return e;
}

function btn(cls: string, text: string): HTMLButtonElement {
  const b = el("button", cls, text);
  b.type = "button";
  return b;
}

export function mountPanes(opts: PanesOptions): PanesHandle {
  const { body } = opts;
  const storage = opts.storage === undefined ? defaultStorage() : opts.storage;
  let key = opts.key ?? null;
  let layout = load(key, storage);
  const cols: Record<PaneId, HTMLElement> = { tree: opts.tree, editor: opts.editor, preview: opts.preview };

  // ---------------------------------------------------------------- botões e trilhos
  const collapse = {} as Record<PaneId, HTMLButtonElement>;
  const rails = {} as Record<PaneId, HTMLButtonElement>;
  for (const id of ["tree", "editor", "preview"] as const) {
    const c = btn("ws-collapse", CHEVRON[id].collapse);
    c.setAttribute("aria-label", LABEL[id].collapse);
    c.title = LABEL[id].collapse;
    c.addEventListener("click", () => setOpen(id, false));
    collapse[id] = c;

    const r = btn(`ws-rail ws-rail-${id}`, "");
    r.dataset.pane = id;
    r.setAttribute("aria-label", LABEL[id].expand);
    r.setAttribute("aria-expanded", "false");
    r.title = LABEL[id].expand;
    r.append(el("span", "ws-rail-icon", CHEVRON[id].expand), el("span", "ws-rail-label", LABEL[id].name));
    r.addEventListener("click", () => setOpen(id, true));
    rails[id] = r;
  }
  opts.treeHeader.append(collapse.tree);
  opts.editorHeader.append(collapse.editor);

  // ---------------------------------------------------------------- divisórias
  function splitter(label: string): HTMLElement {
    const s = el("div", "ws-split");
    s.setAttribute("role", "separator");
    s.setAttribute("aria-orientation", "vertical");
    s.setAttribute("aria-label", label);
    s.tabIndex = 0;
    return s;
  }
  const splitTree = splitter("Largura da árvore de arquivos");
  splitTree.setAttribute("aria-valuemin", String(MIN_TREE));
  splitTree.setAttribute("aria-valuemax", String(MAX_TREE));
  const splitMain = splitter("Divisão entre editor e prévia (% do editor)");
  splitMain.setAttribute("aria-valuemin", String(Math.round(MIN_SPLIT * 100)));
  splitMain.setAttribute("aria-valuemax", String(Math.round(MAX_SPLIT * 100)));

  // Ordem no grid: árvore, trilho, divisória, editor, trilho, divisória, prévia, trilho.
  cols.tree.after(rails.tree, splitTree);
  cols.editor.after(rails.editor, splitMain);
  cols.preview.after(rails.preview);

  /** Largura útil de editor + prévia (sem a divisória), medida no layout. */
  function mainWidth(): number {
    const a = cols.editor.getBoundingClientRect();
    const b = cols.preview.getBoundingClientRect();
    return Math.max(0, b.right - a.left - SPLITTER);
  }

  function setSplitFromPx(editorPx: number, total: number) {
    if (total <= 0) return;
    const lo = Math.max(MIN_SPLIT, Math.min(0.5, MIN_PANE / total));
    const hi = Math.min(MAX_SPLIT, Math.max(0.5, 1 - MIN_PANE / total));
    layout.split = clamp(editorPx / total, lo, hi);
  }

  // ---------------------------------------------------------------- arrastar
  let drag: { which: "tree" | "main"; left: number; total: number } | null = null;
  function onDown(which: "tree" | "main", s: HTMLElement) {
    return (e: Event) => {
      const ev = e as PointerEvent;
      if (typeof ev.button === "number" && ev.button !== 0) return;
      e.preventDefault();
      const left = which === "tree" ? cols.tree.getBoundingClientRect().left : cols.editor.getBoundingClientRect().left;
      drag = { which, left, total: which === "main" ? mainWidth() : 0 };
      try {
        if (typeof ev.pointerId === "number") s.setPointerCapture?.(ev.pointerId);
      } catch {
        // sem captura de ponteiro: o arrasto segue pelo próprio separador
      }
      body.classList.add("is-resizing");
    };
  }
  function onMove(e: Event) {
    if (!drag) return;
    const x = (e as MouseEvent).clientX;
    if (drag.which === "tree") layout.treeWidth = clamp(x - drag.left, MIN_TREE, MAX_TREE);
    else setSplitFromPx(x - drag.left, drag.total);
    apply(false);
  }
  function onUp() {
    if (!drag) return;
    drag = null;
    body.classList.remove("is-resizing");
    commit();
  }
  for (const [which, s] of [
    ["tree", splitTree],
    ["main", splitMain],
  ] as const) {
    s.addEventListener("pointerdown", onDown(which, s));
    s.addEventListener("pointermove", onMove);
    s.addEventListener("pointerup", onUp);
    s.addEventListener("pointercancel", onUp);
    s.addEventListener("dblclick", () => {
      if (which === "tree") layout.treeWidth = DEFAULT_LAYOUT.treeWidth;
      else layout.split = DEFAULT_LAYOUT.split;
      commit();
    });
    s.addEventListener("keydown", (e) => {
      const dir = e.key === "ArrowLeft" ? -1 : e.key === "ArrowRight" ? 1 : 0;
      if (dir === 0) return;
      e.preventDefault();
      const px = (e.shiftKey ? KEY_STEP_BIG : KEY_STEP) * dir;
      if (which === "tree") {
        layout.treeWidth = clamp(layout.treeWidth + px, MIN_TREE, MAX_TREE);
      } else {
        const total = mainWidth();
        if (total > 0) setSplitFromPx(layout.split * total + px, total);
        else layout.split = clamp(layout.split + px / 800, MIN_SPLIT, MAX_SPLIT);
      }
      commit();
    });
  }

  // ---------------------------------------------------------------- modos (barra de cima)
  const modeControl = el("div", "ws-panes");
  const seg = el("div", "seg");
  seg.setAttribute("role", "group");
  seg.setAttribute("aria-label", "Painéis visíveis");
  const modeBtns: Record<ViewMode, HTMLButtonElement> = {
    editor: btn("", "Editor"),
    both: btn("", "Ambos"),
    preview: btn("", "Prévia"),
  };
  modeBtns.editor.title = "Só o editor (⌘1 / Ctrl+1)";
  modeBtns.both.title = "Editor e prévia (⌘3 / Ctrl+3)";
  modeBtns.preview.title = "Só a prévia (⌘2 / Ctrl+2)";
  for (const m of ["editor", "both", "preview"] as const) {
    modeBtns[m].addEventListener("click", () => setMode(m));
    seg.append(modeBtns[m]);
  }
  const treeToggle = btn("btn ws-tree-toggle", "Arquivos");
  treeToggle.title = "Mostrar ou esconder a árvore de arquivos (⌘\\ / Ctrl+\\)";
  treeToggle.addEventListener("click", () => toggleTree());
  modeControl.append(treeToggle, seg);

  // ---------------------------------------------------------------- rolagem guardada
  const saved = new Map<HTMLElement, { top: number; left: number }>();
  function remember(id: PaneId) {
    for (const sel of SCROLLERS) {
      for (const s of cols[id].querySelectorAll<HTMLElement>(sel)) saved.set(s, { top: s.scrollTop, left: s.scrollLeft });
    }
  }
  function restore(id: PaneId) {
    for (const [s, pos] of saved) {
      if (!cols[id].contains(s)) continue;
      s.scrollTop = pos.top;
      s.scrollLeft = pos.left;
      saved.delete(s);
    }
  }

  // ---------------------------------------------------------------- aplicar
  function template(): string {
    const parts: string[] = [];
    parts.push(layout.tree ? `${layout.treeWidth}px ${SPLITTER}px` : `${RAIL}px`);
    const both = layout.editor && layout.preview;
    if (layout.editor) parts.push(both ? `minmax(0, ${layout.split.toFixed(4)}fr)` : "minmax(0, 1fr)");
    else parts.push(`${RAIL}px`);
    if (both) parts.push(`${SPLITTER}px`);
    if (layout.preview) parts.push(both ? `minmax(0, ${(1 - layout.split).toFixed(4)}fr)` : "minmax(0, 1fr)");
    else parts.push(`${RAIL}px`);
    return parts.join(" ");
  }

  function apply(notify = true) {
    for (const id of ["tree", "editor", "preview"] as const) {
      const open = layout[id];
      cols[id].hidden = !open;
      rails[id].hidden = open;
      collapse[id].setAttribute("aria-expanded", String(open));
    }
    splitTree.hidden = !layout.tree;
    splitMain.hidden = !(layout.editor && layout.preview);
    collapse.editor.disabled = !layout.preview;
    collapse.preview.disabled = !layout.editor;
    splitTree.setAttribute("aria-valuenow", String(Math.round(layout.treeWidth)));
    splitMain.setAttribute("aria-valuenow", String(Math.round(layout.split * 100)));
    body.style.gridTemplateColumns = template();
    const mode: ViewMode = layout.editor && layout.preview ? "both" : layout.editor ? "editor" : "preview";
    for (const m of ["editor", "both", "preview"] as const) modeBtns[m].setAttribute("aria-pressed", String(m === mode));
    treeToggle.setAttribute("aria-pressed", String(layout.tree));
    if (notify) opts.onChange?.({ ...layout });
  }

  function commit() {
    apply();
    save(key, storage, layout);
  }

  function change(next: Partial<Pick<PaneLayout, PaneId>>) {
    const target = { ...layout, ...next };
    if (!target.editor && !target.preview) return; // um dos dois fica aberto
    for (const id of ["tree", "editor", "preview"] as const) {
      if (layout[id] && !target[id]) remember(id);
    }
    const opened = (["tree", "editor", "preview"] as const).filter((id) => !layout[id] && target[id]);
    Object.assign(layout, next);
    commit();
    for (const id of opened) restore(id);
  }

  function setOpen(id: PaneId, open: boolean) {
    change({ [id]: open });
    if (!open) {
      // O foco não fica num botão que sumiu: vai para o trilho que o substitui.
      if (document.activeElement === null || cols[id].contains(document.activeElement) || document.activeElement === collapse[id]) {
        rails[id].focus();
      }
    } else if (document.activeElement === rails[id]) {
      collapse[id].focus();
    }
  }

  function setMode(mode: ViewMode) {
    change({ editor: mode !== "preview", preview: mode !== "editor" });
  }

  function toggleTree() {
    change({ tree: !layout.tree });
  }

  // ---------------------------------------------------------------- atalhos
  const keyTarget = opts.keyTarget ?? document;
  const onKey = (e: Event) => {
    const k = e as KeyboardEvent;
    if (!(k.metaKey || k.ctrlKey) || k.altKey || k.shiftKey) return;
    const action =
      k.key === "1" ? () => setMode("editor")
      : k.key === "2" ? () => setMode("preview")
      : k.key === "3" ? () => setMode("both")
      : k.key === "\\" ? () => toggleTree()
      : null;
    if (!action) return;
    k.preventDefault();
    action();
  };
  keyTarget.addEventListener("keydown", onKey);

  apply(false);

  return {
    layout: () => ({ ...layout }),
    setOpen,
    setMode,
    toggleTree,
    setKey(next) {
      key = next;
      layout = load(key, storage);
      apply();
    },
    previewButton: collapse.preview,
    modeControl,
    destroy() {
      keyTarget.removeEventListener("keydown", onKey);
      for (const id of ["tree", "editor", "preview"] as const) rails[id].remove();
      splitTree.remove();
      splitMain.remove();
      modeControl.remove();
      collapse.preview.remove();
    },
  };
}
