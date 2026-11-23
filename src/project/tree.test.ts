// Testes da árvore do projeto (T030; FR-004, cenário 1 de US1): lista os arquivos de
// `list_tree` e abre no editor `.typ`, `.bib` e `.yml` com um clique.
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ERROR_MESSAGES, type TreeNode } from "../ipc";
import { fakeCore, settle, type Handler } from "../testing/fake-core";
import { isOpenable, mountTree, type TreeHandle } from "./tree";

const TREE: TreeNode[] = [
  {
    name: "capitulos",
    path: "capitulos",
    kind: "dir",
    children: [{ name: "intro.typ", path: "capitulos/intro.typ", kind: "file" }],
  },
  { name: "figura.png", path: "figura.png", kind: "file" },
  { name: "main.typ", path: "main.typ", kind: "file" },
  { name: "refs.bib", path: "refs.bib", kind: "file" },
  { name: "refs.yml", path: "refs.yml", kind: "file" },
];

let handle: TreeHandle | undefined;
afterEach(() => {
  handle?.destroy();
  handle = undefined;
  document.body.innerHTML = "";
});

async function setup(tree: unknown = TREE, current?: string) {
  const core = fakeCore({
    list_tree: () => {
      if (tree instanceof Error) throw tree.message;
      return tree;
    },
  });
  const onOpen = vi.fn();
  const parent = document.createElement("div");
  document.body.append(parent);
  handle = mountTree(parent, { ipc: core.ipc, onOpen, ...(current ? { current } : {}) });
  await handle.refresh();
  return { core, parent, onOpen, tree: handle };
}

function item(parent: HTMLElement, path: string) {
  return parent.querySelector<HTMLElement>(`[data-path="${path}"]`);
}

describe("árvore do projeto", () => {
  it("lista pastas e arquivos de list_tree, com aninhamento", async () => {
    const { parent, core } = await setup();
    expect(core.calls("list_tree")).toHaveLength(1);
    const nav = parent.querySelector("nav");
    expect(nav?.getAttribute("aria-label")).toBe("Arquivos do projeto");
    for (const p of ["capitulos", "capitulos/intro.typ", "figura.png", "main.typ", "refs.bib", "refs.yml"]) {
      expect(item(parent, p), p).not.toBeNull();
    }
    expect(item(parent, "capitulos")?.closest("li")?.querySelector('[data-path="capitulos/intro.typ"]')).not.toBeNull();
  });

  it("clique em .typ, .bib ou .yml abre o arquivo no editor", async () => {
    const { parent, onOpen } = await setup();
    for (const p of ["main.typ", "refs.bib", "refs.yml", "capitulos/intro.typ"]) {
      item(parent, p)?.click();
    }
    expect(onOpen.mock.calls.map((c) => c[0])).toEqual([
      "main.typ",
      "refs.bib",
      "refs.yml",
      "capitulos/intro.typ",
    ]);
  });

  it("arquivo que o editor não abre aparece, mas não é botão nem abre", async () => {
    const { parent, onOpen } = await setup();
    const png = item(parent, "figura.png");
    expect(png?.tagName).not.toBe("BUTTON");
    png?.click();
    expect(onOpen).not.toHaveBeenCalled();
    expect(item(parent, "main.typ")?.tagName).toBe("BUTTON");
  });

  it("marca o arquivo aberto com aria-current", async () => {
    const { parent, tree } = await setup(TREE, "main.typ");
    expect(item(parent, "main.typ")?.getAttribute("aria-current")).toBe("true");
    tree.setCurrent("refs.bib");
    expect(item(parent, "main.typ")?.hasAttribute("aria-current")).toBe(false);
    expect(item(parent, "refs.bib")?.getAttribute("aria-current")).toBe("true");
  });

  it("pasta recolhe e expande pelo botão, com aria-expanded", async () => {
    const { parent } = await setup();
    const dir = item(parent, "capitulos") as HTMLButtonElement;
    expect(dir.tagName).toBe("BUTTON");
    expect(dir.getAttribute("aria-expanded")).toBe("true");
    dir.click();
    expect(dir.getAttribute("aria-expanded")).toBe("false");
    expect(dir.closest("li")?.querySelector("ul")?.hidden).toBe(true);
  });

  it("falha do núcleo vira mensagem em português", async () => {
    const { parent } = await setup(new Error("no_project"));
    expect(parent.querySelector('[role="alert"]')?.textContent).toBe(
      "Nenhum projeto aberto. Abra ou crie um projeto.",
    );
  });

  it("nome de arquivo é texto, nunca marcação", async () => {
    const { parent } = await setup([{ name: "<img src=x onerror=1>.typ", path: "x.typ", kind: "file" }]);
    expect(parent.querySelector("img")).toBeNull();
    expect(item(parent, "x.typ")?.textContent).toBe("<img src=x onerror=1>.typ");
  });
});

// Criar arquivo e pasta pela árvore (T085; FR-034, FR-035, cenário 10 de US1). O nome é
// digitado num campo da própria árvore: `window.prompt` é bloqueado no app (plugin de diálogo).
describe("criar arquivo e pasta", () => {
  function action(parent: HTMLElement, name: "new-file" | "new-dir") {
    return parent.querySelector<HTMLButtonElement>(`button[data-action="${name}"]`);
  }

  function nameInput(parent: HTMLElement) {
    return parent.querySelector<HTMLInputElement>("input.pt-new-name");
  }

  function key(input: HTMLInputElement, k: string) {
    input.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true }));
  }

  async function type(parent: HTMLElement, name: string, k = "Enter") {
    const input = nameInput(parent);
    expect(input, "campo de nome").not.toBeNull();
    input!.value = name;
    input!.dispatchEvent(new Event("input", { bubbles: true }));
    key(input!, k);
    await settle();
  }

  async function setupCreate(handlers: Record<string, Handler> = {}, current?: string) {
    const ctx = await setup(TREE, current);
    ctx.core.on("create_file", (a) => ({ path: a.path }));
    ctx.core.on("create_dir", (a) => ({ path: a.path }));
    for (const [cmd, h] of Object.entries(handlers)) ctx.core.on(cmd, h);
    return ctx;
  }

  it("mostra os botões Novo arquivo e Nova pasta", async () => {
    const { parent } = await setupCreate();
    expect(action(parent, "new-file")?.textContent).toBe("Novo arquivo");
    expect(action(parent, "new-dir")?.textContent).toBe("Nova pasta");
    expect(nameInput(parent)).toBeNull();
  });

  it("Novo arquivo abre um campo de nome na página, focado, sem window.prompt", async () => {
    const prompt = vi.spyOn(window, "prompt").mockImplementation(() => null);
    const { parent } = await setupCreate();
    action(parent, "new-file")!.click();
    const input = nameInput(parent);
    expect(input).not.toBeNull();
    expect(input!.getAttribute("aria-label")).toBe("Nome do novo arquivo");
    expect(document.activeElement).toBe(input);
    expect(prompt).not.toHaveBeenCalled();
    prompt.mockRestore();
  });

  it("Enter cria o arquivo na raiz, atualiza a árvore e abre o .typ no editor", async () => {
    const { parent, core, onOpen } = await setupCreate();
    action(parent, "new-file")!.click();
    await type(parent, "novo.typ");
    expect(core.calls("create_file").map((c) => c.args)).toEqual([{ path: "novo.typ" }]);
    expect(core.calls("list_tree")).toHaveLength(2);
    expect(onOpen).toHaveBeenCalledWith("novo.typ");
    expect(nameInput(parent)).toBeNull();
  });

  it("cria dentro da pasta escolhida na árvore", async () => {
    const { parent, core } = await setupCreate();
    item(parent, "capitulos")!.click();
    action(parent, "new-file")!.click();
    expect(parent.querySelector(".pt-new")?.textContent).toContain("capitulos/");
    await type(parent, "metodo.typ");
    expect(core.calls("create_file").map((c) => c.args)).toEqual([{ path: "capitulos/metodo.typ" }]);
  });

  it("sem pasta escolhida, cria na pasta do arquivo aberto", async () => {
    const { parent, core } = await setupCreate({}, "capitulos/intro.typ");
    action(parent, "new-dir")!.click();
    await type(parent, "anexos");
    expect(core.calls("create_dir").map((c) => c.args)).toEqual([{ path: "capitulos/anexos" }]);
  });

  it("Nova pasta chama create_dir e não abre nada no editor", async () => {
    const { parent, core, onOpen } = await setupCreate();
    action(parent, "new-dir")!.click();
    expect(nameInput(parent)!.getAttribute("aria-label")).toBe("Nome da nova pasta");
    await type(parent, "figuras");
    expect(core.calls("create_dir").map((c) => c.args)).toEqual([{ path: "figuras" }]);
    expect(core.calls("create_file")).toHaveLength(0);
    expect(onOpen).not.toHaveBeenCalled();
    expect(core.calls("list_tree")).toHaveLength(2);
  });

  it("arquivo que o editor não abre é criado, mas não abre", async () => {
    const { parent, onOpen } = await setupCreate();
    action(parent, "new-file")!.click();
    await type(parent, "dados.csv");
    expect(onOpen).not.toHaveBeenCalled();
  });

  it("Esc cancela sem chamar o núcleo", async () => {
    const { parent, core } = await setupCreate();
    action(parent, "new-file")!.click();
    await type(parent, "novo.typ", "Escape");
    expect(nameInput(parent)).toBeNull();
    expect(core.calls("create_file")).toHaveLength(0);
  });

  it("nome vazio não chama o núcleo", async () => {
    const { parent, core } = await setupCreate();
    action(parent, "new-file")!.click();
    await type(parent, "   ");
    expect(core.calls("create_file")).toHaveLength(0);
    expect(nameInput(parent)).not.toBeNull();
  });

  it.each([
    ["already_exists", "Já existe um arquivo com esse nome."],
    ["not_found", "A pasta de destino não existe."],
    ["invalid_name", ERROR_MESSAGES.invalid_name],
    ["path_outside_project", "Acesso recusado: o caminho fica fora da pasta do projeto."],
  ])("recusa %s vira mensagem em português e mantém o campo", async (code, message) => {
    const { parent, onOpen } = await setupCreate({
      create_file: () => {
        throw code;
      },
    });
    action(parent, "new-file")!.click();
    await type(parent, "main.typ");
    expect(parent.querySelector(".pt-new [role=alert]")?.textContent).toBe(message);
    expect(nameInput(parent)?.value).toBe("main.typ");
    expect(onOpen).not.toHaveBeenCalled();
  });

  it("already_exists com uma pasta de mesmo nome diz que é pasta", async () => {
    const { parent } = await setupCreate({
      create_file: () => {
        throw "already_exists";
      },
    });
    action(parent, "new-file")!.click();
    await type(parent, "capitulos");
    expect(parent.querySelector(".pt-new [role=alert]")?.textContent).toBe("Já existe uma pasta com esse nome.");
  });

  // Pasta de destino (bug do dono, 2026-11-23: "Novo arquivo em plugin/" num projeto chamado
  // `plugin` sem subpasta `plugin` → `not_found`). O caminho é sempre relativo à raiz.
  it("sem nada escolhido, o destino é a raiz", async () => {
    const { parent, core } = await setupCreate();
    action(parent, "new-file")!.click();
    expect(parent.querySelector(".pt-new-label")?.textContent).toBe("Novo arquivo na raiz do projeto");
    await type(parent, "main2.typ");
    expect(core.calls("create_file").map((c) => c.args)).toEqual([{ path: "main2.typ" }]);
  });

  it("pasta aninhada escolhida é o destino", async () => {
    const { parent, core } = await setupCreate();
    core.on("list_tree", () => [
      { name: "a", path: "a", kind: "dir", children: [{ name: "b", path: "a/b", kind: "dir", children: [] }] },
    ]);
    await handle!.refresh();
    item(parent, "a/b")!.click();
    action(parent, "new-file")!.click();
    expect(parent.querySelector(".pt-new-label")?.textContent).toBe("Novo arquivo em a/b/");
    await type(parent, "x.typ");
    expect(core.calls("create_file").map((c) => c.args)).toEqual([{ path: "a/b/x.typ" }]);
  });

  it("arquivo escolhido: o destino é a pasta dele; da raiz, a raiz", async () => {
    const { parent, core } = await setupCreate();
    item(parent, "capitulos")!.click();
    item(parent, "capitulos/intro.typ")!.click();
    action(parent, "new-file")!.click();
    await type(parent, "b.typ");
    item(parent, "capitulos")!.click();
    item(parent, "main.typ")!.click();
    action(parent, "new-file")!.click();
    expect(parent.querySelector(".pt-new-label")?.textContent).toBe("Novo arquivo na raiz do projeto");
    await type(parent, "c.typ");
    expect(core.calls("create_file").map((c) => c.args)).toEqual([{ path: "capitulos/b.typ" }, { path: "c.typ" }]);
  });

  it("pasta escolhida que deixou de existir não vira destino", async () => {
    const { parent, core } = await setupCreate();
    item(parent, "capitulos")!.click();
    core.on("list_tree", () => [{ name: "main.typ", path: "main.typ", kind: "file" }]);
    await handle!.refresh();
    action(parent, "new-file")!.click();
    await type(parent, "x.typ");
    expect(core.calls("create_file").map((c) => c.args)).toEqual([{ path: "x.typ" }]);
  });

  it("outro projeto começa limpo: sem campo, sem erro, destino na raiz, pastas do projeto novo", async () => {
    // Projeto A tem a pasta `plugin`; o B se chama `plugin` e não tem essa subpasta.
    const storage = {
      data: new Map<string, string>([["prelo.tree:plugin", JSON.stringify(["docs"])]]),
      getItem(k: string) {
        return this.data.get(k) ?? null;
      },
      setItem(k: string, v: string) {
        this.data.set(k, v);
      },
    };
    const core = fakeCore({
      list_tree: () => [{ name: "plugin", path: "plugin", kind: "dir", children: [{ name: "p.typ", path: "plugin/p.typ", kind: "file" }] }],
      create_file: () => {
        throw "not_found";
      },
    });
    const parent = document.createElement("div");
    document.body.append(parent);
    handle = mountTree(parent, { ipc: core.ipc, onOpen: vi.fn(), storage, key: "artigo", current: "plugin/p.typ" });
    await handle.refresh();
    item(parent, "plugin")!.click();
    action(parent, "new-file")!.click();
    await type(parent, "main.typ");
    expect(parent.querySelector(".pt-new [role=alert]")?.hidden).toBe(false);

    core.on("list_tree", () => [
      { name: "docs", path: "docs", kind: "dir", children: [{ name: "d.typ", path: "docs/d.typ", kind: "file" }] },
      { name: "main.typ", path: "main.typ", kind: "file" },
    ]);
    core.on("create_file", (a) => ({ path: a.path }));
    handle.resetForProject("plugin");
    await handle.refresh();
    expect(parent.querySelector(".pt-new")).toBeNull();
    expect(parent.querySelector("[role=alert]")).toBeNull();
    expect(parent.querySelector("[aria-current]")).toBeNull();
    expect(item(parent, "docs")?.getAttribute("aria-expanded")).toBe("false");
    action(parent, "new-file")!.click();
    expect(parent.querySelector(".pt-new-label")?.textContent).toBe("Novo arquivo na raiz do projeto");
    expect(nameInput(parent)?.value).toBe("");
    await type(parent, "main2.typ");
    expect(core.calls("create_file").at(-1)?.args).toEqual({ path: "main2.typ" });
  });

  it("nome digitado é texto, nunca marcação", async () => {
    const { parent } = await setupCreate({
      create_file: () => {
        throw "invalid_name";
      },
    });
    action(parent, "new-file")!.click();
    await type(parent, "<img src=x onerror=1>");
    expect(parent.querySelector("img")).toBeNull();
  });
});

describe("isOpenable", () => {
  it("aceita .typ, .bib, .yml e .yaml; recusa o resto", () => {
    expect(["a.typ", "b.bib", "c.yml", "d.yaml", "E.TYP"].map(isOpenable)).toEqual([true, true, true, true, true]);
    expect(["a.png", "typ", "a.typ.bak", "Makefile"].map(isOpenable)).toEqual([false, false, false, false]);
  });
});

// Pastas recolhíveis (T096; FR-004a, cenário 12 de US1, emenda de 2026-11-23): "No projeto,
// quando existir uma pasta, permita encolher a pasta e não mostrar os arquivos." O teste antigo
// só olhava `ul.hidden`; no app, `.pt ul { display: grid }` vencia o atributo `hidden` e os
// filhos continuavam na tela. Aqui o CSS real de `src/workspace.css` entra no documento.
describe("pastas recolhíveis", () => {
  const NESTED: TreeNode[] = [
    {
      name: "capitulos",
      path: "capitulos",
      kind: "dir",
      children: [
        {
          name: "partes",
          path: "capitulos/partes",
          kind: "dir",
          children: [{ name: "a.typ", path: "capitulos/partes/a.typ", kind: "file" }],
        },
        { name: "intro.typ", path: "capitulos/intro.typ", kind: "file" },
      ],
    },
    { name: "figuras", path: "figuras", kind: "dir", children: [{ name: "f.png", path: "figuras/f.png", kind: "file" }] },
    { name: "main.typ", path: "main.typ", kind: "file" },
  ];

  function memoryStorage(initial: Record<string, string> = {}) {
    const data = new Map(Object.entries(initial));
    return {
      getItem: (k: string) => data.get(k) ?? null,
      setItem: (k: string, v: string) => void data.set(k, v),
    };
  }

  type MemStorage = ReturnType<typeof memoryStorage>;

  async function mount(
    o: { tree?: TreeNode[]; key?: string | null; storage?: MemStorage | null; header?: boolean; current?: string } = {},
  ) {
    const core = fakeCore({ list_tree: () => o.tree ?? NESTED });
    core.on("create_file", (a) => ({ path: a.path }));
    const onOpen = vi.fn();
    const parent = document.createElement("div");
    const header = document.createElement("div");
    document.body.append(header, parent);
    handle = mountTree(parent, {
      ipc: core.ipc,
      onOpen,
      storage: o.storage === undefined ? memoryStorage() : o.storage,
      key: o.key ?? null,
      ...(o.header ? { header } : {}),
      ...(o.current ? { current: o.current } : {}),
    });
    await handle.refresh();
    return { core, parent, header, onOpen, tree: handle };
  }

  function group(parent: HTMLElement, path: string) {
    return item(parent, path)?.closest("li")?.querySelector<HTMLElement>(":scope > ul") ?? null;
  }

  function visible(el: Element | null): boolean {
    for (let n = el; n; n = n.parentElement) if (getComputedStyle(n).display === "none") return false;
    return el !== null;
  }

  function press(el: Element, key: string) {
    el.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
  }

  function withCss() {
    const css = readFileSync(resolve(process.cwd(), "src/workspace.css"), "utf8").replace(/@import[^;]+;/g, "");
    const style = document.createElement("style");
    style.textContent = css;
    document.head.append(style);
    return () => style.remove();
  }

  it("com o CSS do app, pasta recolhida esconde de fato arquivos e subpastas", async () => {
    const drop = withCss();
    try {
      const { parent } = await mount();
      expect(visible(item(parent, "capitulos/intro.typ"))).toBe(true);
      item(parent, "capitulos")!.click();
      expect(visible(item(parent, "capitulos/intro.typ"))).toBe(false);
      expect(visible(item(parent, "capitulos/partes"))).toBe(false);
      expect(visible(item(parent, "main.typ"))).toBe(true);
      item(parent, "capitulos")!.click();
      expect(visible(item(parent, "capitulos/intro.typ"))).toBe(true);
    } finally {
      drop();
    }
  });

  it("árvore com role tree, itens treeitem e grupos group; pasta com aria-expanded", async () => {
    const { parent } = await mount();
    expect(parent.querySelector('[role="tree"]')).not.toBeNull();
    const dir = item(parent, "capitulos")!;
    expect(dir.getAttribute("role")).toBe("treeitem");
    expect(dir.getAttribute("aria-expanded")).toBe("true");
    expect(item(parent, "main.typ")?.getAttribute("role")).toBe("treeitem");
    expect(item(parent, "figuras/f.png")?.getAttribute("role")).toBe("treeitem");
    expect(item(parent, "main.typ")?.hasAttribute("aria-expanded")).toBe(false);
    const g = group(parent, "capitulos")!;
    expect(g.getAttribute("role")).toBe("group");
    expect(g.id).not.toBe("");
    expect(dir.getAttribute("aria-owns")).toBe(g.id);
    // Um só item na ordem de tabulação (tabindex móvel).
    expect(parent.querySelectorAll('[role="treeitem"][tabindex="0"]')).toHaveLength(1);
  });

  it("seta ▾ na pasta aberta e ▸ na recolhida; a linha inteira alterna", async () => {
    const { parent } = await mount();
    const dir = item(parent, "capitulos")!;
    const chev = () => dir.querySelector(".pt-chev");
    expect(chev()?.textContent).toBe("▾");
    expect(chev()?.getAttribute("aria-hidden")).toBe("true");
    // O clique no nome (não só na seta) recolhe.
    (dir.querySelector(".pt-name") as HTMLElement).click();
    expect(dir.getAttribute("aria-expanded")).toBe("false");
    expect(chev()?.textContent).toBe("▸");
    expect(group(parent, "capitulos")?.hidden).toBe(true);
  });

  it("Enter e Espaço alternam a pasta; Enter num arquivo o abre", async () => {
    const { parent, onOpen } = await mount();
    const dir = item(parent, "capitulos")!;
    press(dir, "Enter");
    expect(dir.getAttribute("aria-expanded")).toBe("false");
    press(dir, " ");
    expect(dir.getAttribute("aria-expanded")).toBe("true");
    press(item(parent, "main.typ")!, "Enter");
    expect(onOpen).toHaveBeenCalledWith("main.typ");
  });

  it("← recolhe ou vai à pasta-mãe; → expande ou vai ao primeiro filho", async () => {
    const { parent } = await mount();
    const dir = item(parent, "capitulos")!;
    dir.focus();
    press(dir, "ArrowLeft");
    expect(dir.getAttribute("aria-expanded")).toBe("false");
    press(dir, "ArrowRight");
    expect(dir.getAttribute("aria-expanded")).toBe("true");
    press(dir, "ArrowRight");
    expect(document.activeElement).toBe(item(parent, "capitulos/partes"));
    const intro = item(parent, "capitulos/intro.typ")!;
    intro.focus();
    press(intro, "ArrowLeft");
    expect(document.activeElement).toBe(dir);
    expect(dir.getAttribute("aria-expanded")).toBe("true");
  });

  it("↑/↓ movem entre itens visíveis, pulando os de pasta recolhida", async () => {
    const { parent } = await mount();
    item(parent, "capitulos/partes")!.click();
    const partes = item(parent, "capitulos/partes")!;
    partes.focus();
    press(partes, "ArrowDown");
    expect(document.activeElement).toBe(item(parent, "capitulos/intro.typ"));
    press(document.activeElement!, "ArrowUp");
    press(document.activeElement!, "ArrowUp");
    expect(document.activeElement).toBe(item(parent, "capitulos"));
    expect(item(parent, "capitulos")!.getAttribute("tabindex")).toBe("0");
  });

  it("Recolher tudo e Expandir tudo no cabeçalho da árvore, só com ícone e nome acessível", async () => {
    const { parent, header } = await mount({ header: true });
    const collapseAll = header.querySelector<HTMLButtonElement>('button[data-action="collapse-all"]')!;
    const expandAll = header.querySelector<HTMLButtonElement>('button[data-action="expand-all"]')!;
    expect(collapseAll.getAttribute("aria-label")).toBe("Recolher tudo");
    expect(collapseAll.title).toBe("Recolher tudo");
    expect(expandAll.getAttribute("aria-label")).toBe("Expandir tudo");
    expect(expandAll.title).toBe("Expandir tudo");
    expect(collapseAll.querySelector("svg")).not.toBeNull();
    expect(expandAll.querySelector("svg")).not.toBeNull();
    collapseAll.click();
    for (const p of ["capitulos", "capitulos/partes", "figuras"]) {
      expect(item(parent, p)?.getAttribute("aria-expanded"), p).toBe("false");
    }
    expandAll.click();
    for (const p of ["capitulos", "capitulos/partes", "figuras"]) {
      expect(item(parent, p)?.getAttribute("aria-expanded"), p).toBe("true");
    }
  });

  it("sem cabeçalho, Recolher tudo e Expandir tudo ficam na barra da própria árvore", async () => {
    const { parent } = await mount();
    expect(parent.querySelector('button[data-action="collapse-all"]')).not.toBeNull();
    expect(parent.querySelector('button[data-action="expand-all"]')).not.toBeNull();
  });

  it("Novo arquivo e Nova pasta têm ícone, nome visível e o mesmo nome acessível", async () => {
    const { parent } = await mount();
    for (const [action, name] of [
      ["new-file", "Novo arquivo"],
      ["new-dir", "Nova pasta"],
    ] as const) {
      const b = parent.querySelector<HTMLButtonElement>(`button[data-action="${action}"]`)!;
      const svg = b.querySelector("svg");
      expect(svg, action).not.toBeNull();
      expect(svg?.getAttribute("aria-hidden")).toBe("true");
      expect(svg?.hasAttribute("style")).toBe(false);
      expect(b.textContent).toBe(name);
      expect(b.title).toBe(name);
    }
  });

  it("estado recolhido sobrevive a refresh e à criação de arquivo", async () => {
    const { parent, tree, core } = await mount();
    item(parent, "figuras")!.click();
    await tree.refresh();
    expect(item(parent, "figuras")?.getAttribute("aria-expanded")).toBe("false");
    // Abrir um arquivo da raiz tira a pasta da escolha: o novo nasce na raiz, não em figuras/.
    item(parent, "main.typ")!.click();
    parent.querySelector<HTMLButtonElement>('button[data-action="new-file"]')!.click();
    const input = parent.querySelector<HTMLInputElement>("input.pt-new-name")!;
    input.value = "novo.typ";
    press(input, "Enter");
    await settle();
    expect(core.calls("create_file").map((c) => c.args)).toEqual([{ path: "novo.typ" }]);
    expect(core.calls("list_tree")).toHaveLength(3);
    expect(item(parent, "figuras")?.getAttribute("aria-expanded")).toBe("false");
  });

  it("lembra as pastas recolhidas por projeto; outra chave começa expandida", async () => {
    const storage = memoryStorage();
    const a = await mount({ key: "artigo", storage });
    item(a.parent, "capitulos/partes")!.click();
    item(a.parent, "figuras")!.click();
    a.tree.destroy();
    const b = await mount({ key: "artigo", storage });
    expect(item(b.parent, "figuras")?.getAttribute("aria-expanded")).toBe("false");
    expect(item(b.parent, "capitulos/partes")?.getAttribute("aria-expanded")).toBe("false");
    expect(item(b.parent, "capitulos")?.getAttribute("aria-expanded")).toBe("true");
    b.tree.resetForProject("outro");
    await b.tree.refresh();
    expect(item(b.parent, "figuras")?.getAttribute("aria-expanded")).toBe("true");
  });

  it("armazenamento quebrado ou ilegível não quebra a árvore", async () => {
    const broken = {
      getItem: (): string | null => {
        throw new Error("negado");
      },
      setItem: () => {
        throw new Error("cota");
      },
    };
    const { parent, tree } = await mount({ key: "artigo", storage: broken });
    item(parent, "figuras")!.click();
    expect(item(parent, "figuras")?.getAttribute("aria-expanded")).toBe("false");
    tree.destroy();
    const junk = memoryStorage({ "prelo.tree:artigo": "{não é json" });
    const again = await mount({ key: "artigo", storage: junk });
    expect(item(again.parent, "figuras")?.getAttribute("aria-expanded")).toBe("true");
  });

  it("abrir um arquivo expande as pastas que o contêm", async () => {
    const { parent, tree } = await mount();
    item(parent, "capitulos/partes")!.click();
    item(parent, "capitulos")!.click();
    tree.setCurrent("capitulos/partes/a.typ");
    expect(item(parent, "capitulos")?.getAttribute("aria-expanded")).toBe("true");
    expect(item(parent, "capitulos/partes")?.getAttribute("aria-expanded")).toBe("true");
    expect(group(parent, "capitulos/partes")?.hidden).toBe(false);
    expect(item(parent, "capitulos/partes/a.typ")?.getAttribute("aria-current")).toBe("true");
  });

  it("arquivo aberto numa pasta lembrada como recolhida aparece ao montar", async () => {
    const storage = memoryStorage({ "prelo.tree:artigo": JSON.stringify(["capitulos", "figuras"]) });
    const { parent } = await mount({ key: "artigo", storage, current: "capitulos/intro.typ" });
    expect(item(parent, "capitulos")?.getAttribute("aria-expanded")).toBe("true");
    expect(item(parent, "figuras")?.getAttribute("aria-expanded")).toBe("false");
  });
});
