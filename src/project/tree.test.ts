// Testes da árvore do projeto (T030; FR-004, cenário 1 de US1): lista os arquivos de
// `list_tree` e abre no editor `.typ`, `.bib` e `.yml` com um clique.
import { afterEach, describe, expect, it, vi } from "vitest";
import type { TreeNode } from "../ipc";
import { fakeCore } from "../testing/fake-core";
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

describe("isOpenable", () => {
  it("aceita .typ, .bib, .yml e .yaml; recusa o resto", () => {
    expect(["a.typ", "b.bib", "c.yml", "d.yaml", "E.TYP"].map(isOpenable)).toEqual([true, true, true, true, true]);
    expect(["a.png", "typ", "a.typ.bak", "Makefile"].map(isOpenable)).toEqual([false, false, false, false]);
  });
});
