// Árvore de arquivos do projeto (T038; FR-004, cenário 1 de US1). Mostra o resultado de
// `list_tree` e abre no editor, com um clique ou Enter, os arquivos que ele edita (`.typ`,
// `.bib`, `.yml`/`.yaml`, D-14). Os demais aparecem, mas não são botões. Nomes entram como
// texto, nunca como marcação.
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
        btn.addEventListener("click", () => opts.onOpen(node.path));
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
      nav.replaceChildren(msg);
      return;
    }
    nav.replaceChildren(renderList(Array.isArray(nodes) ? nodes : []));
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
