// Árvore de arquivos do projeto (T038; FR-004, cenário 1 de US1). Mostra o resultado de
// `list_tree` e abre no editor, com um clique ou Enter, os arquivos que ele edita (`.typ`,
// `.bib`, `.yml`/`.yaml`, D-14). Os demais aparecem, mas não são botões. Nomes entram como
// texto, nunca como marcação.
//
// "Novo arquivo" e "Nova pasta" (T087; FR-034, FR-035) pedem o nome num campo da própria
// árvore (Enter cria, Esc cancela) e criam na pasta escolhida com um clique, senão na pasta do
// arquivo aberto, senão na raiz. As regras de nome e de escopo são do núcleo; a recusa aparece
// em português no campo. Um arquivo novo que o editor abre vai para o editor.
import { toIpcError, type Ipc, type TreeNode } from "../ipc";
import "../workspace.css";

const OPENABLE = /\.(typ|bib|ya?ml)$/i;

/** O editor abre este caminho? */
export function isOpenable(path: string): boolean {
  return OPENABLE.test(path);
}

export interface TreeOptions {
  ipc: Ipc;
  /** Chamado com o caminho relativo do arquivo escolhido. */
  onOpen: (path: string) => void;
  /** Arquivo aberto no editor, marcado com `aria-current`. */
  current?: string;
}

export interface TreeHandle {
  readonly element: HTMLElement;
  /** Pede a árvore ao núcleo e redesenha. */
  refresh(): Promise<void>;
  setCurrent(path: string | null): void;
  destroy(): void;
}

export function mountTree(parent: HTMLElement, opts: TreeOptions): TreeHandle {
  const nav = document.createElement("nav");
  nav.className = "pt";
  nav.setAttribute("aria-label", "Arquivos do projeto");
  parent.append(nav);
  let current: string | null = opts.current ?? null;
  // Pastas recolhidas sobrevivem ao redesenho de `refresh`.
  const collapsed = new Set<string>();
  // Pasta escolhida com um clique: destino de "Novo arquivo"/"Nova pasta" (FR-034).
  let selectedDir: string | null = null;

  const actions = document.createElement("div");
  actions.className = "pt-actions";
  const newFileBtn = actionButton("new-file", "Novo arquivo");
  const newDirBtn = actionButton("new-dir", "Nova pasta");
  actions.append(newFileBtn, newDirBtn);
  const list = document.createElement("div");
  list.className = "pt-list";
  nav.append(actions, list);
  let form: HTMLElement | null = null;

  function actionButton(action: string, label: string): HTMLButtonElement {
    const b = document.createElement("button");
    b.type = "button";
    b.className = "pt-action";
    b.dataset.action = action;
    b.textContent = label;
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
        if (dir !== "") collapsed.delete(dir);
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

  function markCurrent() {
    for (const el of nav.querySelectorAll<HTMLElement>("[aria-current]")) el.removeAttribute("aria-current");
    if (current === null) return;
    for (const el of nav.querySelectorAll<HTMLElement>("button.pt-file")) {
      if (el.dataset.path === current) el.setAttribute("aria-current", "true");
    }
  }

  function renderList(nodes: TreeNode[]): HTMLUListElement {
    const ul = document.createElement("ul");
    for (const node of nodes) {
      const li = document.createElement("li");
      if (node.kind === "dir") {
        const btn = document.createElement("button");
        btn.type = "button";
        btn.className = "pt-item pt-dir";
        btn.dataset.path = node.path;
        btn.textContent = node.name;
        const children = renderList(node.children ?? []);
        const open = !collapsed.has(node.path);
        btn.setAttribute("aria-expanded", String(open));
        children.hidden = !open;
        btn.addEventListener("click", () => {
          selectedDir = node.path;
          const nowOpen = btn.getAttribute("aria-expanded") !== "true";
          btn.setAttribute("aria-expanded", String(nowOpen));
          children.hidden = !nowOpen;
          if (nowOpen) collapsed.delete(node.path);
          else collapsed.add(node.path);
        });
        li.append(btn, children);
      } else if (isOpenable(node.path)) {
        const btn = document.createElement("button");
        btn.type = "button";
        btn.className = "pt-item pt-file";
        btn.dataset.path = node.path;
        btn.textContent = node.name;
        btn.title = node.path;
        btn.addEventListener("click", () => {
          // O destino de "Novo arquivo" volta a ser a pasta do arquivo aberto.
          selectedDir = null;
          opts.onOpen(node.path);
        });
        li.append(btn);
      } else {
        const span = document.createElement("span");
        span.className = "pt-item pt-other";
        span.dataset.path = node.path;
        span.textContent = node.name;
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
    list.replaceChildren(renderList(Array.isArray(nodes) ? nodes : []));
    markCurrent();
  }

  return {
    element: nav,
    refresh,
    setCurrent(path) {
      current = path;
      markCurrent();
    },
    destroy() {
      nav.remove();
    },
  };
}
