// Painel de estrutura do artigo (T064; FR-005, US4 cenário 4; contracts/ipc.md `outline`;
// data-model.md `OutlineItem`). Lista os títulos de seção do último documento que compilou, em
// ordem e com recuo por nível, como no protótipo (`.side h4` + `.outline`). Títulos entram como
// texto, nunca como marcação. Clicar numa seção entrega o item a quem montou, que leva editor
// e prévia até ela. A contagem de pendências por seção fica para a fase de revisão (FR-005).
import type { Ipc, OutlineItem } from "../ipc";
import "../workspace.css";

export interface OutlineOptions {
  ipc: Ipc;
  onSelect: (item: OutlineItem) => void;
}

export interface OutlineHandle {
  readonly element: HTMLElement;
  /** Pede a estrutura ao núcleo e redesenha; pedido antigo que chega depois é descartado. */
  refresh(): Promise<void>;
  /** Esvazia (projeto novo, antes da primeira compilação). */
  clear(): void;
  destroy(): void;
}

/** Recuo por nível, em px (o mesmo das subpastas da árvore). */
const INDENT = 12;

export function mountOutline(parent: HTMLElement, opts: OutlineOptions): OutlineHandle {
  const nav = document.createElement("nav");
  nav.className = "ol";
  nav.setAttribute("aria-label", "Estrutura do artigo");
  const heading = document.createElement("h4");
  heading.textContent = "Estrutura";
  const list = document.createElement("ul");
  list.className = "ol-list";
  const empty = document.createElement("p");
  empty.className = "ol-empty";
  empty.hidden = true;
  nav.append(heading, list, empty);
  parent.append(nav);

  let ticket = 0;
  let current: OutlineItem | null = null;
  let destroyed = false;

  const same = (a: OutlineItem | null, b: OutlineItem) =>
    a !== null && a.path === b.path && a.line === b.line && a.title === b.title;

  function render(items: OutlineItem[]) {
    const rows = items.map((item) => {
      const li = document.createElement("li");
      const b = document.createElement("button");
      b.type = "button";
      b.className = "ol-item";
      b.textContent = item.title;
      b.title = `${item.path}, linha ${item.line}`;
      b.dataset.level = String(item.level);
      b.style.paddingLeft = `${6 + Math.max(0, item.level - 1) * INDENT}px`;
      if (same(current, item)) b.setAttribute("aria-current", "true");
      b.addEventListener("click", () => {
        current = item;
        for (const other of list.querySelectorAll("[aria-current]")) other.removeAttribute("aria-current");
        b.setAttribute("aria-current", "true");
        opts.onSelect(item);
      });
      li.append(b);
      return li;
    });
    list.replaceChildren(...rows);
    empty.textContent = "Sem títulos de seção.";
    empty.hidden = rows.length > 0;
  }

  async function refresh() {
    const mine = ++ticket;
    let items: OutlineItem[];
    try {
      items = await opts.ipc.outline();
    } catch (err) {
      // Sem estrutura (núcleo sem o recurso, sem projeto): o painel fica vazio, sem alarde.
      if (!destroyed && mine === ticket) {
        list.replaceChildren();
        empty.hidden = true;
      }
      console.warn("estrutura indisponível", err);
      return;
    }
    if (destroyed || mine !== ticket) return;
    render(Array.isArray(items) ? items : []);
  }

  return {
    element: nav,
    refresh,
    clear() {
      ticket++;
      current = null;
      list.replaceChildren();
      empty.hidden = true;
    },
    destroy() {
      destroyed = true;
      nav.remove();
    },
  };
}
