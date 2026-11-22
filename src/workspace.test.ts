// Testes da área de trabalho montada em src/main.ts (T039; US1 cenários 1, 2, 4, 6).
// Núcleo falso: contracts/ipc.md.
import { diagnosticCount } from "@codemirror/lint";
import { afterEach, describe, expect, it } from "vitest";
import type { PreviewUpdate, ProjectInfo } from "./ipc";
import { fakeCore, settle } from "./testing/fake-core";
import { mountWorkspace, type WorkspaceHandle } from "./workspace";

let handles: WorkspaceHandle[] = [];
afterEach(() => {
  for (const h of handles) h.destroy();
  handles = [];
  document.body.innerHTML = "";
});

const INFO: ProjectInfo = {
  name: "artigo-basico",
  mainFile: "main.typ",
  compiler: { embedded: "0.15.1", registered: "0.15.1" },
  warnings: [],
};

const TREE = [
  { name: "capitulos", path: "capitulos", kind: "dir", children: [{ name: "metodo.typ", path: "capitulos/metodo.typ", kind: "file" }] },
  { name: "main.typ", path: "main.typ", kind: "file" },
];

const FILES: Record<string, string> = {
  "main.typ": "= Introdução\n\nOlá.\n",
  "capitulos/metodo.typ": "= Método\n",
};

function update(over: Partial<PreviewUpdate> = {}): PreviewUpdate {
  return {
    compileId: 1,
    ok: true,
    pages: [{ index: 0, hash: "h1", widthPt: 595, heightPt: 842 }],
    changed: [0],
    diagnostics: [],
    durationMs: 41,
    ...over,
  };
}

function setup(extra: Parameters<typeof fakeCore>[0] = {}) {
  let version = 1;
  const core = fakeCore({
    get_app_info: () => ({ version: "0.1.0", os: "macos", arch: "aarch64" }),
    subscribe_preview: () => null,
    open_project_dialog: () => INFO,
    list_tree: () => TREE,
    open_file: (a) => ({ text: FILES[a.path as string] ?? "", version: 1 }),
    edit_file: () => ({ version: ++version }),
    save_file: () => ({ savedAt: "2026-11-22T10:15:00Z" }),
    page_svg: () => "<svg xmlns='http://www.w3.org/2000/svg'/>",
    ...extra,
  });
  const root = document.createElement("main");
  document.body.append(root);
  const ws = mountWorkspace(root, {
    ipc: core.ipc,
    appInfo: (cmd) => core.invoke(cmd),
    previewUrls: { create: () => "blob:x", revoke: () => {} },
    tokenDelayMs: 0,
  });
  handles.push(ws);
  return { core, root, ws };
}

const button = (root: ParentNode, name: RegExp) =>
  [...root.querySelectorAll<HTMLButtonElement>("button")].find((b) => name.test(b.textContent ?? ""));

describe("layout", () => {
  it("barra com abrir projeto, salvar e sobre; árvore | editor | prévia; versão do app", async () => {
    const { root, ws } = setup();
    await ws.ready;
    expect(button(root, /Abrir projeto/)).toBeDefined();
    expect(button(root, /Salvar/)).toBeDefined();
    expect(button(root, /Sobre/)).toBeDefined();
    expect(root.querySelector("[aria-label='Arquivos do projeto']")).not.toBeNull();
    expect(root.querySelector(".ws-editor")).not.toBeNull();
    expect(root.querySelector("[aria-label='Prévia']")).not.toBeNull();
    expect(ws.appInfoElement.textContent).toBe("Prelo 0.1.0 · macos");
  });

  it("sem projeto: convida a abrir um e Salvar fica desativado", async () => {
    const { root, ws } = setup();
    await ws.ready;
    expect(root.querySelector(".ws-editor")?.textContent).toContain("Nenhum projeto aberto");
    expect(button(root, /Salvar/)!.disabled).toBe(true);
  });

  it("assina a prévia uma vez ao montar", async () => {
    const { core, ws } = setup();
    await ws.ready;
    expect(core.calls("subscribe_preview")).toHaveLength(1);
  });
});

describe("abrir projeto (cenário 1)", () => {
  it("diálogo do núcleo → nome, árvore, arquivo principal no editor e marcado na árvore", async () => {
    const { core, root, ws } = setup();
    await ws.ready;
    button(root, /Abrir projeto/)!.click();
    await settle();
    expect(core.calls("open_project_dialog")).toHaveLength(1);
    expect(root.querySelector(".ws-project")?.textContent).toContain("artigo-basico");
    expect(root.querySelector(".ws-project")?.textContent).toContain("main.typ");
    expect(core.calls("open_file")[0]?.args).toEqual({ path: "main.typ" });
    expect(ws.editor()?.view.state.doc.toString()).toBe(FILES["main.typ"]);
    expect(root.querySelector("button.pt-file[data-path='main.typ']")?.getAttribute("aria-current")).toBe("true");
    expect(button(root, /Salvar/)!.disabled).toBe(false);
    expect(root.querySelector(".ws-status")?.textContent).toContain("Typst 0.15.1");
  });

  it("cancelar o diálogo não muda nada", async () => {
    const { core, root, ws } = setup({ open_project_dialog: () => null });
    await ws.ready;
    button(root, /Abrir projeto/)!.click();
    await settle();
    expect(core.calls("list_tree")).toHaveLength(0);
    expect(ws.editor()).toBeNull();
  });

  it("núcleo sem o comando: mensagem em português, sem quebrar", async () => {
    const { core, root, ws } = setup();
    core.on("open_project_dialog", () => {
      throw "not_implemented";
    });
    await ws.ready;
    button(root, /Abrir projeto/)!.click();
    await settle();
    expect(root.querySelector(".ws-message")?.textContent).toContain("Recurso ainda não disponível");
  });

  it("clicar noutro arquivo da árvore abre no editor e move a marcação", async () => {
    const { core, root, ws } = setup();
    await ws.ready;
    await ws.showProject(INFO);
    root.querySelector<HTMLButtonElement>("button.pt-file[data-path='capitulos/metodo.typ']")!.click();
    await settle();
    expect(core.calls("open_file").at(-1)?.args).toEqual({ path: "capitulos/metodo.typ" });
    expect(ws.editor()?.path).toBe("capitulos/metodo.typ");
    expect(ws.editor()?.view.state.doc.toString()).toBe("= Método\n");
    expect(root.querySelectorAll(".cm-editor")).toHaveLength(1);
    expect(
      root.querySelector("button.pt-file[data-path='capitulos/metodo.typ']")?.getAttribute("aria-current"),
    ).toBe("true");
  });
});

describe("prévia e diagnósticos (cenários 2 e 4)", () => {
  it("o PreviewUpdate vai à prévia e ao status", async () => {
    const { core, root, ws } = setup();
    await ws.ready;
    await ws.showProject(INFO);
    core.push(update());
    await settle();
    expect(core.calls("page_svg")[0]?.args).toEqual({ index: 0, hash: "h1" });
    expect(root.querySelector(".pv-page img")).not.toBeNull();
    expect(root.querySelector(".ws-compile")?.textContent).toContain("41 ms");
  });

  it("os diagnósticos do arquivo aberto aparecem no editor; status conta os erros", async () => {
    const { core, root, ws } = setup();
    await ws.ready;
    await ws.showProject(INFO);
    core.push(
      update({
        compileId: 2,
        ok: false,
        pages: [],
        changed: [],
        diagnostics: [
          { severity: "error", path: "main.typ", range: { from: 14, to: 18 }, message: "unknown variable", hints: [], kind: "typst" },
        ],
      }),
    );
    await settle();
    expect(diagnosticCount(ws.editor()!.view.state)).toBe(1);
    expect(root.querySelector(".ws-compile")?.textContent).toMatch(/1 erro/);
    expect(root.querySelector(".pv-error")?.hasAttribute("hidden")).toBe(false);
  });

  it("diagnósticos já recebidos valem para o arquivo aberto depois", async () => {
    const { core, root, ws } = setup();
    await ws.ready;
    await ws.showProject(INFO);
    core.push(
      update({
        compileId: 2,
        ok: false,
        diagnostics: [
          { severity: "error", path: "capitulos/metodo.typ", line: 1, message: "x", hints: [], kind: "typst" },
        ],
      }),
    );
    await settle();
    expect(diagnosticCount(ws.editor()!.view.state)).toBe(0);
    root.querySelector<HTMLButtonElement>("button.pt-file[data-path='capitulos/metodo.typ']")!.click();
    await settle();
    expect(diagnosticCount(ws.editor()!.view.state)).toBe(1);
  });

  it("onPreviewUpdate avisa quem observa (autoteste)", async () => {
    const { core, ws } = setup();
    await ws.ready;
    const seen: number[] = [];
    ws.onPreviewUpdate((u) => seen.push(u.compileId));
    core.push(update({ compileId: 7 }));
    expect(seen).toEqual([7]);
  });
});

describe("salvar", () => {
  it("botão Salvar chama save_file e mostra a hora", async () => {
    const { core, root, ws } = setup();
    await ws.ready;
    await ws.showProject(INFO);
    button(root, /Salvar/)!.click();
    await settle();
    expect(core.calls("save_file")[0]?.args).toEqual({ path: "main.typ" });
    expect(root.querySelector(".ws-save")?.textContent).toMatch(/Salvo/);
  });

  it("editar marca alterações pendentes", async () => {
    const { root, ws } = setup();
    await ws.ready;
    await ws.showProject(INFO);
    const view = ws.editor()!.view;
    view.dispatch({ changes: { from: 0, insert: "x" }, userEvent: "input.type" });
    await settle();
    expect(root.querySelector(".ws-save")?.textContent).toMatch(/automaticamente/);
  });

  it("edição recusada pelo núcleo recarrega o arquivo (PR #176)", async () => {
    const { core, root, ws } = setup();
    await ws.ready;
    await ws.showProject(INFO);
    core.on("edit_file", () => {
      throw "version_mismatch";
    });
    core.on("open_file", () => ({ text: "texto do núcleo", version: 5 }));
    ws.editor()!.view.dispatch({ changes: { from: 0, insert: "x" } });
    await settle(10);
    expect(ws.editor()!.view.state.doc.toString()).toBe("texto do núcleo");
    expect(ws.editor()!.version()).toBe(5);
    expect(root.querySelector(".ws-message")?.textContent).toContain("recarregue");
  });
});

describe("versão do compilador (cenário 6, FR-012)", () => {
  it("versão diferente: faixa de aviso com ação explícita de atualizar", async () => {
    const updated = { ...INFO, compiler: { embedded: "0.15.1", registered: "0.15.1" } };
    const { core, root, ws } = setup({ update_project_compiler_version: () => updated });
    await ws.ready;
    await ws.showProject({
      ...INFO,
      compiler: { embedded: "0.15.1", registered: "0.14.0" },
      warnings: ["compiler_version_mismatch"],
    });
    const banner = root.querySelector<HTMLElement>(".ws-banner")!;
    expect(banner.hidden).toBe(false);
    expect(banner.textContent).toContain("0.14.0");
    expect(core.calls("update_project_compiler_version")).toHaveLength(0);
    button(banner, /Atualizar/)!.click();
    await settle();
    expect(core.calls("update_project_compiler_version")).toHaveLength(1);
    expect(banner.hidden).toBe(true);
  });

  it("arquivo principal ausente: escolher um .typ da árvore como principal (choose_main_file)", async () => {
    const { core, root, ws } = setup({
      choose_main_file: (a) => ({ ...INFO, mainFile: a.path as string }),
    });
    await ws.ready;
    await ws.showProject({ ...INFO, mainFile: "sumiu.typ", warnings: ["main_file_missing"] });
    // Não tenta abrir o principal que não existe.
    expect(core.calls("open_file")).toHaveLength(0);
    const banner = root.querySelector<HTMLElement>(".ws-banner")!;
    expect(banner.hidden).toBe(false);
    expect(banner.textContent).toContain("arquivo principal");
    const use = button(banner, /principal/)!;
    expect(use.disabled).toBe(true);
    root.querySelector<HTMLButtonElement>("button.pt-file[data-path='capitulos/metodo.typ']")!.click();
    await settle();
    expect(use.disabled).toBe(false);
    use.click();
    await settle();
    expect(core.calls("choose_main_file")[0]?.args).toEqual({ path: "capitulos/metodo.typ" });
    expect(banner.hidden).toBe(true);
    expect(root.querySelector(".ws-project")?.textContent).toContain("capitulos/metodo.typ");
  });

  it("Sobre abre o diálogo com as versões", async () => {
    const { root, ws } = setup({
      compiler_info: () => ({ embedded: "0.15.1", registered: "0.15.1", fonts: [] }),
    });
    await ws.ready;
    await ws.showProject(INFO);
    button(root, /Sobre/)!.click();
    await settle();
    const dialog = root.querySelector<HTMLElement>("[role=dialog]")!;
    expect(dialog.hidden).toBe(false);
    expect(dialog.textContent).toMatch(/Embutido\s*0\.15\.1/);
  });
});

describe("conflito externo", () => {
  it("o evento abre o diálogo e a escolha recarrega o editor", async () => {
    const { core, root, ws } = setup({
      resolve_external_change: () => ({ text: "do disco", version: 3 }),
    });
    await ws.ready;
    await ws.showProject(INFO);
    core.emit("external_change_conflict", { path: "main.typ" });
    await settle();
    button(root, /disco/)!.click();
    await settle();
    expect(ws.editor()!.view.state.doc.toString()).toBe("do disco");
  });

  it("⌘S durante um conflito (save_file → external_change_conflict) abre o mesmo diálogo", async () => {
    const { core, root, ws } = setup({
      save_file: () => {
        throw "external_change_conflict";
      },
      resolve_external_change: () => ({ text: "meu texto", version: 4 }),
    });
    await ws.ready;
    await ws.showProject(INFO);
    button(root, /Salvar/)!.click();
    await settle();
    expect(root.querySelector("[role=alertdialog]")).not.toBeNull();
    button(root, /minha/)!.click();
    await settle();
    expect(core.calls("resolve_external_change")[0]?.args).toEqual({ path: "main.typ", keep: "mine" });
    expect(ws.editor()!.view.state.doc.toString()).toBe("meu texto");
  });
});

describe("mudança externa recarregada sem alteração local (FR-008)", () => {
  it("o evento do arquivo aberto traz o texto do disco ao editor, sem diálogo", async () => {
    let disk = { text: FILES["main.typ"]!, version: 1 };
    const { core, root, ws } = setup({ open_file: () => disk });
    await ws.ready;
    await ws.showProject(INFO);
    const status = root.querySelector(".ws-status")!.textContent;
    const trees = core.calls("list_tree").length;
    disk = { text: "= Do disco\n\n" + "linha\n".repeat(136), version: 2 };
    core.emit("external_change_reloaded", { path: "main.typ", version: 2 });
    await settle();
    const editor = ws.editor()!;
    expect(editor.view.state.doc.toString()).toBe(disk.text);
    expect(editor.view.state.doc.lines).toBe(139);
    expect(editor.version()).toBe(2);
    expect(root.querySelector("[role=alertdialog]")).toBeNull();
    expect(root.querySelector(".ws-status")!.textContent).toBe(status);
    expect(core.calls("list_tree")).toHaveLength(trees);
    expect(core.calls("edit_file")).toHaveLength(0);
  });

  it("a próxima edição vai com a versão nova", async () => {
    let disk = { text: "abc", version: 1 };
    const { core, ws } = setup({ open_file: () => disk, edit_file: (a) => ({ version: (a.version as number) + 1 }) });
    await ws.ready;
    await ws.showProject(INFO);
    disk = { text: "abc do disco", version: 4 };
    core.emit("external_change_reloaded", { path: "main.typ", version: 4 });
    await settle();
    const view = ws.editor()!.view;
    view.dispatch({ changes: { from: view.state.doc.length, insert: "!" }, userEvent: "input.type" });
    await ws.editor()!.flush();
    expect(core.calls("edit_file").at(-1)?.args).toMatchObject({ path: "main.typ", version: 4 });
  });

  it("evento de outro arquivo não mexe no editor aberto", async () => {
    const { core, ws } = setup();
    await ws.ready;
    await ws.showProject(INFO);
    const opened = core.calls("open_file").length;
    core.emit("external_change_reloaded", { path: "capitulos/metodo.typ", version: 2 });
    await settle();
    expect(core.calls("open_file")).toHaveLength(opened);
    expect(ws.editor()!.view.state.doc.toString()).toBe(FILES["main.typ"]);
  });

  it("evento com versão que o editor já tem não busca de novo", async () => {
    const { core, ws } = setup();
    await ws.ready;
    await ws.showProject(INFO);
    const opened = core.calls("open_file").length;
    core.emit("external_change_reloaded", { path: "main.typ", version: 1 });
    await settle();
    expect(core.calls("open_file")).toHaveLength(opened);
  });
});

// ---------------------------------------------------------------- painéis (T092; FR-013b)

describe("painéis ajustáveis", () => {
  afterEach(() => {
    try {
      localStorage.clear();
    } catch {
      // sem armazenamento no ambiente: nada a limpar
    }
  });
  const byLabel = (root: ParentNode, label: string) => root.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`);

  it("cada painel tem o seu botão de recolher; o da prévia fica na barra da prévia", async () => {
    const { root, ws } = setup();
    await ws.ready;
    expect(root.querySelector(".ws-col-tree .ws-pane-h button[aria-label='Recolher árvore de arquivos']")).not.toBeNull();
    expect(root.querySelector(".ws-col-editor .ws-pane-h button[aria-label='Recolher editor']")).not.toBeNull();
    expect(root.querySelector(".pvbar button[aria-label='Recolher prévia']")).not.toBeNull();
    expect(root.querySelector(".ws-top [aria-label='Painéis visíveis']")).not.toBeNull();
  });

  it("ao abrir projeto (prévia montada de novo) o botão da prévia continua na barra dela", async () => {
    const { root, ws } = setup();
    await ws.ready;
    button(root, /Abrir projeto/)!.click();
    await settle();
    expect(root.querySelector(".pvbar button[aria-label='Recolher prévia']")).not.toBeNull();
    expect(root.querySelectorAll("button[aria-label='Recolher prévia']")).toHaveLength(1);
  });

  it("só a prévia: o editor vira trilho e volta com o mesmo texto", async () => {
    const { root, ws } = setup();
    await ws.ready;
    button(root, /Abrir projeto/)!.click();
    await settle();
    byLabel(root, "Recolher editor")!.click();
    expect(root.querySelector<HTMLElement>(".ws-col-editor")!.hidden).toBe(true);
    expect(byLabel(root, "Expandir editor")!.hidden).toBe(false);
    byLabel(root, "Expandir editor")!.click();
    expect(ws.editor()?.view.state.doc.toString()).toBe(FILES["main.typ"]);
  });

  it("⌘2 com o foco no editor mostra só a prévia (o editor não usa esse atalho)", async () => {
    const { root, ws } = setup();
    await ws.ready;
    button(root, /Abrir projeto/)!.click();
    await settle();
    const content = ws.editor()!.view.contentDOM;
    const ev = new KeyboardEvent("keydown", { key: "2", metaKey: true, bubbles: true, cancelable: true });
    content.dispatchEvent(ev);
    expect(root.querySelector<HTMLElement>(".ws-col-editor")!.hidden).toBe(true);
    expect(root.querySelector<HTMLElement>(".ws-col-preview")!.hidden).toBe(false);
  });

  it("o arranjo fica lembrado pelo projeto", async () => {
    const a = setup();
    await a.ws.ready;
    button(a.root, /Abrir projeto/)!.click();
    await settle();
    byLabel(a.root, "Recolher árvore de arquivos")!.click();
    const b = setup();
    await b.ws.ready;
    expect(b.root.querySelector<HTMLElement>(".ws-col-tree")!.hidden).toBe(false);
    button(b.root, /Abrir projeto/)!.click();
    await settle();
    expect(b.root.querySelector<HTMLElement>(".ws-col-tree")!.hidden).toBe(true);
  });
});

// ---------------------------------------------------------------- zoom do editor (T094; FR-013c)

describe("zoom do editor na área de trabalho", () => {
  afterEach(() => {
    try {
      localStorage.clear();
    } catch {
      // sem armazenamento no ambiente: nada a limpar
    }
  });
  const press = (target: HTMLElement, key: string) => {
    const ev = new KeyboardEvent("keydown", { key, metaKey: true, bubbles: true, cancelable: true });
    target.dispatchEvent(ev);
    return ev;
  };

  it("o cabeçalho do editor tem o grupo de zoom", async () => {
    const { root, ws } = setup();
    await ws.ready;
    expect(root.querySelector(".ws-col-editor .ws-pane-h [aria-label='Zoom do editor']")).not.toBeNull();
  });

  it("⌘+ vale para o painel com o foco: editor ou prévia", async () => {
    const { root, ws } = setup();
    await ws.ready;
    button(root, /Abrir projeto/)!.click();
    await settle();
    const edPct = () => root.querySelector(".ed-zoom-pct")!.textContent;
    const pvLevel = () => root.querySelector(".pv-zoom-fit")!.getAttribute("aria-pressed");
    expect(press(ws.editor()!.view.contentDOM, "=").defaultPrevented).toBe(true);
    expect(edPct()).toBe("110%");
    expect(pvLevel()).toBe("true");
    press(root.querySelector<HTMLElement>(".pv-pages")!, "=");
    expect(edPct()).toBe("110%");
    expect(pvLevel()).toBe("false");
  });

  it("o zoom do editor continua ao abrir outro arquivo", async () => {
    const { root, ws } = setup();
    await ws.ready;
    button(root, /Abrir projeto/)!.click();
    await settle();
    byLabelZoom(root, "Aumentar zoom do editor").click();
    byLabelZoom(root, "Aumentar zoom do editor").click();
    await ws.openFile("capitulos/metodo.typ");
    expect(parseFloat(ws.editor()!.view.dom.style.fontSize)).toBeCloseTo(12.5 * 1.25);
  });
});

function byLabelZoom(root: ParentNode, label: string) {
  return root.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`)!;
}
