// Testes da ação "Exportar PDF" (T068; US5 cenários 1 e 3; FR-012, FR-028, FR-029).
// Núcleo falso: contracts/ipc.md "Exportação". O diálogo de salvar é do núcleo; a interface
// só chama `export_pdf_dialog` e mostra o resultado.
import { afterEach, describe, expect, it } from "vitest";
import type { Diagnostic, PreviewUpdate, ProjectInfo } from "./ipc";
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

const MISMATCH: ProjectInfo = {
  ...INFO,
  compiler: { embedded: "0.15.1", registered: "0.14.0" },
  warnings: ["compiler_version_mismatch"],
};

const ERRORS: Diagnostic[] = [
  { severity: "error", path: "main.typ", line: 12, message: "unknown variable: x", hints: [], kind: "typst" },
  { severity: "warning", path: "main.typ", line: 3, message: "unused import", hints: [], kind: "typst" },
  { severity: "error", path: "capitulos/metodo.typ", line: 4, message: "expected expression", hints: [], kind: "typst" },
];

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
  const core = fakeCore({
    get_app_info: () => ({ version: "0.1.0", os: "macos", arch: "aarch64" }),
    subscribe_preview: () => null,
    list_tree: () => [{ name: "main.typ", path: "main.typ", kind: "file" }],
    open_file: () => ({ text: "= Olá\n", version: 1 }),
    page_svg: () => "<svg xmlns='http://www.w3.org/2000/svg'/>",
    export_pdf_dialog: () => ({ path: "/Users/ana/artigo.pdf" }),
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

const exportButton = (root: ParentNode) =>
  [...root.querySelectorAll<HTMLButtonElement>("button")].find((b) => /Exportar PDF/.test(b.textContent ?? ""));

async function opened(info: ProjectInfo = INFO, extra: Parameters<typeof fakeCore>[0] = {}) {
  const s = setup(extra);
  await s.ws.ready;
  await s.ws.showProject(info);
  return s;
}

describe("ação Exportar PDF", () => {
  it("fica na barra de cima, como ação principal", async () => {
    const { root, ws } = setup();
    await ws.ready;
    const b = exportButton(root);
    expect(b).toBeDefined();
    expect(b!.closest(".ws-top")).not.toBeNull();
    expect(b!.classList.contains("primary")).toBe(true);
  });

  it("desativada sem projeto e antes da primeira prévia", async () => {
    const { root, ws } = setup();
    await ws.ready;
    expect(exportButton(root)!.disabled).toBe(true);
    await ws.showProject(INFO);
    expect(exportButton(root)!.disabled).toBe(true);
  });

  it("com prévia, chama export_pdf_dialog sem argumentos e avisa onde gravou", async () => {
    const { core, root } = await opened();
    core.push(update());
    await settle();
    expect(exportButton(root)!.disabled).toBe(false);
    exportButton(root)!.click();
    await settle();
    expect(core.calls("export_pdf_dialog")).toHaveLength(1);
    expect(core.calls("export_pdf_dialog")[0]!.args).toEqual({});
    expect(root.querySelector(".ws-export-status")?.textContent).toContain("/Users/ana/artigo.pdf");
    expect(root.querySelector(".ws-export-errors")?.hasAttribute("hidden")).toBe(true);
  });

  it("cancelar o diálogo não mostra nada", async () => {
    const { core, root } = await opened(INFO, { export_pdf_dialog: () => null });
    core.push(update());
    await settle();
    exportButton(root)!.click();
    await settle();
    expect(core.calls("export_pdf_dialog")).toHaveLength(1);
    expect(root.querySelector(".ws-export-status")?.textContent ?? "").toBe("");
    expect(root.querySelector(".ws-message")?.hasAttribute("hidden")).toBe(true);
  });

  it("botão desativado enquanto exporta (sem dois diálogos)", async () => {
    let resolve!: (v: unknown) => void;
    const pending = new Promise((r) => (resolve = r));
    const { core, root } = await opened(INFO, { export_pdf_dialog: () => pending });
    core.push(update());
    await settle();
    exportButton(root)!.click();
    await settle();
    expect(exportButton(root)!.disabled).toBe(true);
    exportButton(root)!.click();
    resolve({ path: "/tmp/a.pdf" });
    await settle();
    expect(core.calls("export_pdf_dialog")).toHaveLength(1);
    expect(exportButton(root)!.disabled).toBe(false);
  });
});

describe("recusa com erro de compilação (FR-029)", () => {
  function refusing() {
    return opened(INFO, {
      export_pdf_dialog: () => {
        throw { code: "compile_errors", diagnostics: ERRORS };
      },
    });
  }

  it("compile_errors mostra a lista de erros com arquivo e linha", async () => {
    const { core, root } = await refusing();
    core.push(update());
    await settle();
    exportButton(root)!.click();
    await settle();
    const list = root.querySelector(".ws-export-errors")!;
    expect(list.hasAttribute("hidden")).toBe(false);
    expect(list.textContent).toContain("corrija-os antes de exportar o PDF");
    const items = [...list.querySelectorAll("li")].map((li) => li.textContent);
    expect(items).toEqual(["main.typ:12: unknown variable: x", "capitulos/metodo.typ:4: expected expression"]);
    expect(root.querySelector(".ws-export-status")?.textContent ?? "").toBe("");
  });

  it("a lista some na próxima compilação sem erro", async () => {
    const { core, root } = await refusing();
    core.push(update());
    await settle();
    exportButton(root)!.click();
    await settle();
    core.push(update({ compileId: 2 }));
    await settle();
    expect(root.querySelector(".ws-export-errors")?.hasAttribute("hidden")).toBe(true);
  });

  it("outro erro do núcleo vira mensagem, sem lista", async () => {
    const { core, root } = await opened(INFO, {
      export_pdf_dialog: () => {
        throw "write_failed";
      },
    });
    core.push(update());
    await settle();
    exportButton(root)!.click();
    await settle();
    expect(root.querySelector(".ws-message")?.textContent).toContain("Não foi possível gravar");
    expect(root.querySelector(".ws-export-errors")?.hasAttribute("hidden")).toBe(true);
  });
});

describe("versão diferente (FR-012)", () => {
  it("desativada com versão registrada diferente da embutida", async () => {
    const { root } = await opened(MISMATCH);
    expect(exportButton(root)!.disabled).toBe(true);
    expect(exportButton(root)!.title).toMatch(/versão/i);
  });

  it("volta a valer depois de atualizar a versão do projeto e compilar", async () => {
    const { core, root } = await opened(MISMATCH, { update_project_compiler_version: () => INFO });
    const upd = [...root.querySelectorAll<HTMLButtonElement>("button")].find((b) =>
      /Atualizar o projeto/.test(b.textContent ?? ""),
    )!;
    upd.click();
    await settle();
    core.push(update());
    await settle();
    expect(exportButton(root)!.disabled).toBe(false);
  });
});
