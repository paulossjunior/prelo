// Árvore de arquivos do projeto (T038; FR-004, cenário 1 de US1). Mostra o resultado de
// `list_tree` e abre no editor, com um clique ou Enter, os arquivos que ele edita (`.typ`,
// `.bib`, `.yml`/`.yaml`, D-14). Os demais aparecem, mas não abrem. Nomes entram como
// texto, nunca como marcação.
//
// "Novo arquivo" e "Nova pasta" (T087; FR-034, FR-035) pedem o nome num campo da própria
// árvore (Enter cria, Esc cancela) e criam na pasta escolhida com um clique, senão na pasta do
// arquivo aberto, senão na raiz. As regras de nome e de escopo são do núcleo; a recusa aparece
// em português no campo. Um arquivo novo que o editor abre vai para o editor.
//
// Pastas recolhíveis (T097; FR-004a): seta ▾/▸ e a linha inteira alternando, padrão de árvore
// do WAI-ARIA (tree/treeitem/group, tabindex móvel, Enter/Espaço/←/→/↑/↓/Home/End),
// "Recolher tudo"/"Expandir tudo", estado que sobrevive a `refresh` e é lembrado por projeto,
// e as pastas do arquivo aberto sempre expandidas. Ícones SVG feitos pelo DOM, sem `style`
// (CSP `style-src 'self'`): tamanho e cor vêm de `src/workspace.css`.
import { toIpcError, type Ipc, type TreeNode } from "../ipc";
import "../workspace.css";

const OPENABLE = /\.(typ|bib|ya?ml)$/i;

/** O editor abre este caminho? */
export function isOpenable(path: string): boolean {
  return OPENABLE.test(path);
}

export type TreeStorage = Pick<Storage, "getItem" | "setItem">;

export interface TreeOptions {
  ipc: Ipc;
  /** Chamado com o caminho relativo do arquivo escolhido. */
  onOpen: (path: string) => void;
  /** Arquivo aberto no editor, marcado com `aria-current`. */
  current?: string;
  /** Onde as pastas recolhidas são lembradas; padrão `localStorage`; `null` não lembra. */
  storage?: TreeStorage | null;
  /** Chave do projeto; sem chave, o estado vale só nesta sessão. */
  key?: string | null;
  /** Cabeçalho do painel onde entram "Recolher tudo"/"Expandir tudo"; sem ele, a barra da árvore. */
  header?: HTMLElement;
}

export interface TreeHandle {
  readonly element: HTMLElement;
  /** Pede a árvore ao núcleo e redesenha. */
  refresh(): Promise<void>;
  /** Marca o arquivo aberto e expande as pastas que o contêm (FR-004a). */
  setCurrent(path: string | null): void;
  /** Troca de projeto: lê as pastas recolhidas guardadas para a chave nova. */
  setKey(key: string | null): void;
  destroy(): void;
}

const PREFIX = "prelo.tree:";
const SVG = "http://www.w3.org/2000/svg";

/** Traços dos ícones, numa grade de 16 × 16. */
const ICONS = {
  "new-file": ["M9 1.75H4.25a1 1 0 0 0-1 1v10.5a1 1 0 0 0 1 1h7.5a1 1 0 0 0 1-1V5.5z", "M9 1.75V5.5h3.75", "M8 7.75v4.5M5.75 10h4.5"],
  "new-dir": ["M1.75 4a1 1 0 0 1 1-1h3.5l1.5 1.5h5.5a1 1 0 0 1 1 1v6.75a1 1 0 0 1-1 1H2.75a1 1 0 0 1-1-1z", "M8 6.75v4.5M5.75 9h4.5"],
  "collapse-all": ["M5 2.5l3 3 3-3", "M5 13.5l3-3 3 3", "M2.5 8h11"],
  "expand-all": ["M5 5.5l3-3 3 3", "M5 10.5l3 3 3-3", "M2.5 8h11"],
} as const;

type IconName = keyof typeof ICONS;

function icon(name: IconName): SVGSVGElement {
  const svg = document.createElementNS(SVG, "svg");
  svg.setAttribute("class", "pt-icon");
  svg.setAttribute("viewBox", "0 0 16 16");
  svg.setAttribute("aria-hidden", "true");
  svg.setAttribute("focusable", "false");
  for (const d of ICONS[name]) {
    const path = document.createElementNS(SVG, "path");
    path.setAttribute("d", d);
    svg.append(path);
  }
  return svg;
}

function defaultStorage(): TreeStorage | null {
  try {
    return globalThis.localStorage ?? null;
  } catch {
    return null;
  }
}

/** Pastas que contêm `path` (`a/b/c.typ` → `a`, `a/b`). */
function ancestors(path: string): string[] {
  const parts = path.split("/");
  const out: string[] = [];
  for (let i = 1; i < parts.length; i++) out.push(parts.slice(0, i).join("/"));
  return out;
}

let groupSeq = 0;

export function mountTree(parent: HTMLElement, opts: TreeOptions): TreeHandle {
  const nav = document.createElement("nav");
  nav.className = "pt";
  nav.setAttribute("aria-label", "Arquivos do projeto");
  parent.append(nav);
  let current: string | null = opts.current ?? null;
  const storage = opts.storage === undefined ? defaultStorage() : opts.storage;
  let key: string | null = opts.key ?? null;
  // Pastas recolhidas: sobrevivem ao redesenho de `refresh` e ficam guardadas por projeto.
  let collapsed = load();
  // Pasta escolhida com um clique: destino de "Novo arquivo"/"Nova pasta" (FR-034).
  let selectedDir: string | null = null;
  // Item com o tabindex 0 (tabindex móvel do padrão de árvore).
  let focusPath: string | null = null;

  const actions = document.createElement("div");
  actions.className = "pt-actions";
  const newFileBtn = actionButton("new-file", "Novo arquivo");
  const newDirBtn = actionButton("new-dir", "Nova pasta");
  actions.append(newFileBtn, newDirBtn);
  const fold = document.createElement("div");
  fold.className = "pt-fold";
  const collapseAllBtn = foldButton("collapse-all", "Recolher tudo");
  const expandAllBtn = foldButton("expand-all", "Expandir tudo");
  fold.append(collapseAllBtn, expandAllBtn);
  if (opts.header) {
    // Antes do botão de recolher o painel (FR-013b), que fica na ponta.
    opts.header.insertBefore(fold, opts.header.querySelector(".ws-collapse"));
  } else {
    actions.append(fold);
  }
  const list = document.createElement("div");
  list.className = "pt-list";
  nav.append(actions, list);
  let form: HTMLElement | null = null;

  function load(): Set<string> {
    if (!key || !storage) return new Set();
    try {
      const raw: unknown = JSON.parse(storage.getItem(PREFIX + key) ?? "[]");
      return new Set(Array.isArray(raw) ? raw.filter((p): p is string => typeof p === "string") : []);
    } catch {
      return new Set();
    }
  }

  function save() {
    if (!key || !storage) return;
    try {
      storage.setItem(PREFIX + key, JSON.stringify([...collapsed].sort()));
    } catch {
      // Sem armazenamento, o estado vale só nesta sessão.
    }
  }

  function actionButton(action: IconName, label: string): HTMLButtonElement {
    const b = document.createElement("button");
    b.type = "button";
    b.className = "pt-action";
    b.dataset.action = action;
    b.title = label;
    const text = document.createElement("span");
    text.className = "pt-action-label";
    text.textContent = label;
    b.append(icon(action), text);
    return b;
  }

  function foldButton(action: IconName, label: string): HTMLButtonElement {
    const b = document.createElement("button");
    b.type = "button";
    b.className = "pt-fold-btn";
    b.dataset.action = action;
    b.title = label;
    b.setAttribute("aria-label", label);
    b.append(icon(action));
    return b;
  }

  /** Pasta onde o item novo nasce: a escolhida, senão a do arquivo aberto, senão a raiz. */
  function targetDir(): string {
    if (selectedDir !== null) return selectedDir;
    if (current !== null && current.includes("/")) return current.slice(0, current.lastIndexOf("/"));
    return "";
  }

  function closeForm() {
    form?.remove();
    form = null;
  }

  // Campo de nome na própria árvore: `window.prompt` é bloqueado no app (plugin de diálogo).
  function openForm(kind: "file" | "dir") {
    closeForm();
    const dir = targetDir();
    const box = document.createElement("div");
    box.className = "pt-new";
    const label = document.createElement("span");
    label.className = "pt-new-label";
    const what = kind === "file" ? "Novo arquivo" : "Nova pasta";
    label.textContent = dir === "" ? `${what} na raiz do projeto` : `${what} em ${dir}/`;
    const input = document.createElement("input");
    input.type = "text";
    input.className = "pt-new-name";
    input.setAttribute("aria-label", kind === "file" ? "Nome do novo arquivo" : "Nome da nova pasta");
    input.placeholder = kind === "file" ? "capitulo.typ" : "figuras";
    input.autocomplete = "off";
    input.spellcheck = false;
    const error = document.createElement("p");
    error.className = "pt-new-error";
    error.setAttribute("role", "alert");
    error.hidden = true;
    box.append(label, input, error);
    actions.after(box);
    form = box;
    input.focus();

    let busy = false;
    async function submit() {
      const name = input.value.trim();
      if (busy || name === "") return;
      busy = true;
      const requested = dir === "" ? name : `${dir}/${name}`;
      try {
        const created = kind === "file" ? await opts.ipc.createFile(requested) : await opts.ipc.createDir(requested);
        const path = typeof created?.path === "string" ? created.path : requested;
        if (form === box) closeForm();
        // O item novo aparece: as pastas que o contêm se abrem.
        expandAncestors(path);
        await refresh();
        if (kind === "file" && isOpenable(path)) opts.onOpen(path);
      } catch (err) {
        error.textContent = toIpcError(err).message;
        error.hidden = false;
        input.focus();
      } finally {
        busy = false;
      }
    }

    input.addEventListener("keydown", (e) => {
      if (e.key === "Enter") {
        e.preventDefault();
        void submit();
      } else if (e.key === "Escape") {
        e.preventDefault();
        closeForm();
      }
    });
  }

  newFileBtn.addEventListener("click", () => openForm("file"));
  newDirBtn.addEventListener("click", () => openForm("dir"));

  // ------------------------------------------------------------ pastas
  function dirButton(path: string): HTMLElement | null {
    for (const el of list.querySelectorAll<HTMLElement>(".pt-dir")) if (el.dataset.path === path) return el;
    return null;
  }

  /** Mostra o estado de uma pasta já desenhada: seta, `aria-expanded` e o grupo de filhos. */
  function paint(btn: HTMLElement, open: boolean) {
    btn.setAttribute("aria-expanded", String(open));
    const chev = btn.querySelector(".pt-chev");
    if (chev) chev.textContent = open ? "▾" : "▸";
    const group = btn.parentElement?.querySelector<HTMLElement>(":scope > ul");
    if (group) group.hidden = !open;
  }

  function setOpen(path: string, open: boolean) {
    if (open) collapsed.delete(path);
    else collapsed.add(path);
    const btn = dirButton(path);
    if (btn) paint(btn, open);
  }

  function toggle(btn: HTMLElement) {
    const path = btn.dataset.path ?? "";
    setOpen(path, btn.getAttribute("aria-expanded") !== "true");
    save();
    syncTabStop();
  }

  function setAll(open: boolean) {
    for (const btn of list.querySelectorAll<HTMLElement>(".pt-dir")) setOpen(btn.dataset.path ?? "", open);
    save();
    syncTabStop();
  }

  /** Abre as pastas que contêm `path`; devolve se alguma mudou. */
  function expandAncestors(path: string): boolean {
    let changed = false;
    for (const dir of ancestors(path)) {
      if (collapsed.has(dir)) {
        setOpen(dir, true);
        changed = true;
      }
    }
    if (changed) save();
    return changed;
  }

  collapseAllBtn.addEventListener("click", () => setAll(false));
  expandAllBtn.addEventListener("click", () => setAll(true));

  // ------------------------------------------------------------ teclado (padrão de árvore)
  function visibleItems(): HTMLElement[] {
    return [...list.querySelectorAll<HTMLElement>('[role="treeitem"]')].filter((el) => !el.closest("ul[hidden]"));
  }

  /** Deixa um só item na ordem de tabulação: o último focado, senão o aberto, senão o primeiro. */
  function syncTabStop() {
    const items = visibleItems();
    for (const el of list.querySelectorAll<HTMLElement>('[role="treeitem"]')) el.tabIndex = -1;
    const pick =
      items.find((el) => el.dataset.path === focusPath) ?? items.find((el) => el.dataset.path === current) ?? items[0];
    if (pick) pick.tabIndex = 0;
  }

  function focusItem(el: HTMLElement | undefined | null) {
    if (!el) return;
    focusPath = el.dataset.path ?? null;
    syncTabStop();
    el.focus();
  }

  function parentItem(el: HTMLElement): HTMLElement | null {
    const li = el.parentElement?.parentElement?.closest("li");
    return li?.querySelector<HTMLElement>(':scope > [role="treeitem"]') ?? null;
  }

  list.addEventListener("focusin", (e) => {
    const el = (e.target as HTMLElement).closest<HTMLElement>('[role="treeitem"]');
    if (!el) return;
    focusPath = el.dataset.path ?? null;
    syncTabStop();
  });

  list.addEventListener("keydown", (e) => {
    const el = (e.target as HTMLElement).closest<HTMLElement>('[role="treeitem"]');
    if (!el || e.altKey || e.ctrlKey || e.metaKey) return;
    const isDir = el.classList.contains("pt-dir");
    const open = el.getAttribute("aria-expanded") === "true";
    const items = visibleItems();
    const at = items.indexOf(el);
    switch (e.key) {
      case "Enter":
      case " ":
        if (isDir) toggle(el);
        else if (el.classList.contains("pt-file")) el.click();
        else return;
        break;
      case "ArrowRight":
        if (!isDir) return;
        if (!open) toggle(el);
        else focusItem(el.parentElement?.querySelector<HTMLElement>(':scope > ul [role="treeitem"]'));
        break;
      case "ArrowLeft":
        if (isDir && open) toggle(el);
        else focusItem(parentItem(el));
        break;
      case "ArrowDown":
        focusItem(items[at + 1]);
        break;
      case "ArrowUp":
        focusItem(items[at - 1]);
        break;
      case "Home":
        focusItem(items[0]);
        break;
      case "End":
        focusItem(items[items.length - 1]);
        break;
      default:
        return;
    }
    e.preventDefault();
  });

  // ------------------------------------------------------------ desenho
  function markCurrent() {
    for (const el of nav.querySelectorAll<HTMLElement>("[aria-current]")) el.removeAttribute("aria-current");
    if (current === null) return;
    for (const el of nav.querySelectorAll<HTMLElement>(".pt-file")) {
      if (el.dataset.path === current) el.setAttribute("aria-current", "true");
    }
  }

  function row(tag: "button" | "span", node: TreeNode, kind: string, chevron: string): HTMLElement {
    const el = document.createElement(tag);
    if (el instanceof HTMLButtonElement) el.type = "button";
    el.className = `pt-item ${kind}`;
    el.dataset.path = node.path;
    el.setAttribute("role", "treeitem");
    el.tabIndex = -1;
    const chev = document.createElement("span");
    chev.className = "pt-chev";
    chev.setAttribute("aria-hidden", "true");
    chev.textContent = chevron;
    const name = document.createElement("span");
    name.className = "pt-name";
    name.textContent = node.name;
    el.append(chev, name);
    return el;
  }

  function renderList(nodes: TreeNode[], role: "tree" | "group", dirs: Set<string>): HTMLUListElement {
    const ul = document.createElement("ul");
    ul.setAttribute("role", role);
    for (const node of nodes) {
      const li = document.createElement("li");
      li.setAttribute("role", "none");
      if (node.kind === "dir") {
        dirs.add(node.path);
        const btn = row("button", node, "pt-dir", "");
        btn.title = node.path;
        const children = renderList(node.children ?? [], "group", dirs);
        children.id = `pt-g-${++groupSeq}`;
        btn.setAttribute("aria-owns", children.id);
        btn.addEventListener("click", () => {
          selectedDir = node.path;
          toggle(btn);
        });
        li.append(btn, children);
        paint(btn, !collapsed.has(node.path));
      } else if (isOpenable(node.path)) {
        const btn = row("button", node, "pt-file", "");
        btn.title = node.path;
        btn.addEventListener("click", () => {
          // O destino de "Novo arquivo" volta a ser a pasta do arquivo aberto.
          selectedDir = null;
          opts.onOpen(node.path);
        });
        li.append(btn);
      } else {
        const span = row("span", node, "pt-other", "");
        span.setAttribute("aria-disabled", "true");
        span.title = `${node.path} (não abre no editor)`;
        li.append(span);
      }
      ul.append(li);
    }
    return ul;
  }

  async function refresh() {
    let nodes: TreeNode[];
    try {
      nodes = await opts.ipc.listTree();
    } catch (err) {
      const msg = document.createElement("p");
      msg.className = "pt-error";
      msg.setAttribute("role", "alert");
      msg.textContent = toIpcError(err).message;
      list.replaceChildren(msg);
      return;
    }
    const dirs = new Set<string>();
    const tree = renderList(Array.isArray(nodes) ? nodes : [], "tree", dirs);
    tree.setAttribute("aria-label", "Arquivos");
    list.replaceChildren(tree);
    // Pasta que sumiu ou foi renomeada: o estado guardado dela é esquecido.
    let pruned = false;
    for (const p of collapsed) {
      if (!dirs.has(p)) {
        collapsed.delete(p);
        pruned = true;
      }
    }
    if (pruned) save();
    markCurrent();
    syncTabStop();
  }

  if (current !== null) expandAncestors(current);

  return {
    element: nav,
    refresh,
    setCurrent(path) {
      current = path;
      if (path !== null) expandAncestors(path);
      markCurrent();
      syncTabStop();
    },
    setKey(next) {
      key = next;
      collapsed = load();
    },
    destroy() {
      fold.remove();
      nav.remove();
    },
  };
}
