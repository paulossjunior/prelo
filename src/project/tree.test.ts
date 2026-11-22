// Testes da árvore do projeto (T030; FR-004, cenário 1 de US1): lista os arquivos de
// `list_tree` e abre no editor `.typ`, `.bib` e `.yml` com um clique.
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
    ["already_exists", "Já existe um arquivo ou pasta com esse nome."],
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
