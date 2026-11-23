// Testes do diálogo de conflito com mudança externa (T039; FR-008). Contrato: contracts/ipc.md
// (evento `external_change_conflict` `{ path }`; `resolve_external_change { path, keep }`).
import { afterEach, describe, expect, it, vi } from "vitest";
import { fakeCore, settle } from "../testing/fake-core";
import type { Diagnostic, PluginApprovalNeeded } from "../ipc";
import {
  askExternalChange,
  askPluginApproval,
  mountPackageActions,
  tamperedPackage,
  watchExternalChanges,
  watchCopyOffers,
  watchPluginApprovals,
} from "./prompts";

afterEach(() => {
  document.body.innerHTML = "";
});

function buttons(root: ParentNode) {
  return [...root.querySelectorAll<HTMLButtonElement>("button")];
}
const byText = (root: ParentNode, re: RegExp) => buttons(root).find((b) => re.test(b.textContent ?? ""));

describe("askExternalChange", () => {
  it("pergunta com o caminho e resolve com a escolha da pessoa", async () => {
    const parent = document.createElement("div");
    document.body.append(parent);
    const p = askExternalChange(parent, "capitulos/metodo.typ");
    const dialog = parent.querySelector("[role=alertdialog]");
    expect(dialog).not.toBeNull();
    expect(dialog?.textContent).toContain("capitulos/metodo.typ");
    byText(parent, /disco/)!.click();
    await expect(p).resolves.toBe("disk");
    expect(parent.querySelector("[role=alertdialog]")).toBeNull();
  });

  it("Escape não escolhe por ninguém: nada é descartado sem decisão", async () => {
    const parent = document.createElement("div");
    document.body.append(parent);
    const p = askExternalChange(parent, "main.typ");
    let settled = false;
    void p.then(() => (settled = true));
    parent
      .querySelector("[role=alertdialog]")!
      .dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    await settle();
    expect(settled).toBe(false);
    byText(parent, /minha/)!.click();
    await expect(p).resolves.toBe("mine");
  });

  it("caminho entra como texto", () => {
    const parent = document.createElement("div");
    void askExternalChange(parent, "<img src=x>.typ");
    expect(parent.querySelector("img")).toBeNull();
    expect(parent.textContent).toContain("<img src=x>.typ");
  });
});

describe("watchExternalChanges", () => {
  function setup() {
    const core = fakeCore({
      resolve_external_change: (a) => ({ text: a.keep === "disk" ? "do disco" : "meu", version: 9 }),
    });
    const parent = document.createElement("div");
    document.body.append(parent);
    const editor = { path: "main.typ", flush: vi.fn(async () => {}), reload: vi.fn() };
    return { core, parent, editor };
  }

  it("no evento pergunta, envia a escolha ao núcleo e recarrega o editor", async () => {
    const { core, parent, editor } = setup();
    await watchExternalChanges({ ipc: core.ipc, parent, editorFor: (p) => (p === "main.typ" ? editor : null) });
    core.emit("external_change_conflict", { path: "main.typ" });
    await settle();
    byText(parent, /disco/)!.click();
    await settle();
    expect(editor.flush).toHaveBeenCalled();
    expect(core.calls("resolve_external_change")[0]?.args).toEqual({ path: "main.typ", keep: "disk" });
    expect(editor.reload).toHaveBeenCalledWith("do disco", 9);
  });

  it("um só diálogo por arquivo, mesmo com eventos repetidos", async () => {
    const { core, parent, editor } = setup();
    await watchExternalChanges({ ipc: core.ipc, parent, editorFor: () => editor });
    core.emit("external_change_conflict", { path: "main.typ" });
    core.emit("external_change_conflict", { path: "main.typ" });
    await settle();
    expect(parent.querySelectorAll("[role=alertdialog]")).toHaveLength(1);
  });

  it("arquivo que não está no editor: resolve sem recarregar nada", async () => {
    const { core, parent } = setup();
    await watchExternalChanges({ ipc: core.ipc, parent, editorFor: () => null });
    core.emit("external_change_conflict", { path: "outro.typ" });
    await settle();
    byText(parent, /minha/)!.click();
    await settle();
    expect(core.calls("resolve_external_change")[0]?.args).toEqual({ path: "outro.typ", keep: "mine" });
  });

  it("falha do núcleo vai para onError", async () => {
    const { core, parent, editor } = setup();
    core.on("resolve_external_change", () => {
      throw "write_failed";
    });
    const onError = vi.fn();
    await watchExternalChanges({ ipc: core.ipc, parent, editorFor: () => editor, onError });
    core.emit("external_change_conflict", { path: "main.typ" });
    await settle();
    byText(parent, /minha/)!.click();
    await settle();
    expect(onError).toHaveBeenCalledWith(expect.objectContaining({ code: "write_failed" }));
    expect(editor.reload).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------- plugins e pacotes (T044)
// Contrato: evento `plugin_approval_needed` `{ path, origin, package?, sizeBytes, sha256 }`;
// `decide_plugin { sha256, approve }`; `redownload_package { spec }` (contracts/ipc.md).

const SHA = "a".repeat(64);
const pluginEvent = (over: Record<string, unknown> = {}) =>
  ({
    path: "plugins/teste.wasm",
    origin: "project",
    sizeBytes: 2048,
    sha256: SHA,
    ...over,
  }) as PluginApprovalNeeded;

describe("askPluginApproval", () => {
  it("mostra caminho, origem e tamanho e resolve com a decisão", async () => {
    const parent = document.createElement("div");
    document.body.append(parent);
    const p = askPluginApproval(parent, pluginEvent());
    const dialog = parent.querySelector("[role=alertdialog]");
    expect(dialog).not.toBeNull();
    const text = dialog?.textContent ?? "";
    expect(text).toContain("plugins/teste.wasm");
    expect(text).toMatch(/do projeto/);
    expect(text).toContain("2 KB");
    byText(parent, /^Permitir/)!.click();
    await expect(p).resolves.toBe(true);
    expect(parent.querySelector("[role=alertdialog]")).toBeNull();
  });

  it("plugin de pacote mostra o pacote como origem", async () => {
    const parent = document.createElement("div");
    const p = askPluginApproval(
      parent,
      pluginEvent({ path: "plugin.wasm", origin: "package", package: "@preview/plug:0.1.0", sizeBytes: 300 }),
    );
    const text = parent.textContent ?? "";
    expect(text).toContain("@preview/plug:0.1.0");
    expect(text).toContain("plugin.wasm");
    expect(text).toContain("300 bytes");
    byText(parent, /Não permitir/)!.click();
    await expect(p).resolves.toBe(false);
  });

  it("Escape não decide por ninguém", async () => {
    const parent = document.createElement("div");
    document.body.append(parent);
    const p = askPluginApproval(parent, pluginEvent());
    let settled = false;
    void p.then(() => (settled = true));
    parent
      .querySelector("[role=alertdialog]")!
      .dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    await settle();
    expect(settled).toBe(false);
  });

  it("caminho e pacote entram como texto", () => {
    const parent = document.createElement("div");
    void askPluginApproval(parent, pluginEvent({ path: "<img src=x>.wasm", origin: "package", package: "<b>x</b>" }));
    expect(parent.querySelector("img, b")).toBeNull();
    expect(parent.textContent).toContain("<img src=x>.wasm");
  });
});

describe("watchPluginApprovals", () => {
  function setup() {
    const core = fakeCore({ decide_plugin: () => null });
    const parent = document.createElement("div");
    document.body.append(parent);
    return { core, parent };
  }

  it("aprovar chama decide_plugin com approve: true", async () => {
    const { core, parent } = setup();
    await watchPluginApprovals({ ipc: core.ipc, parent });
    core.emit("plugin_approval_needed", pluginEvent());
    await settle();
    byText(parent, /^Permitir/)!.click();
    await settle();
    expect(core.calls("decide_plugin")[0]?.args).toEqual({ sha256: SHA, approve: true });
  });

  it("recusar chama decide_plugin com approve: false", async () => {
    const { core, parent } = setup();
    await watchPluginApprovals({ ipc: core.ipc, parent });
    core.emit("plugin_approval_needed", pluginEvent());
    await settle();
    byText(parent, /Não permitir/)!.click();
    await settle();
    expect(core.calls("decide_plugin")[0]?.args).toEqual({ sha256: SHA, approve: false });
  });

  it("um só pedido por plugin, mesmo com eventos repetidos", async () => {
    const { core, parent } = setup();
    await watchPluginApprovals({ ipc: core.ipc, parent });
    core.emit("plugin_approval_needed", pluginEvent());
    core.emit("plugin_approval_needed", pluginEvent());
    await settle();
    expect(parent.querySelectorAll("[role=alertdialog]")).toHaveLength(1);
  });

  it("evento malformado é ignorado", async () => {
    const { core, parent } = setup();
    await watchPluginApprovals({ ipc: core.ipc, parent });
    core.emit("plugin_approval_needed", { path: 1 });
    core.emit("plugin_approval_needed", pluginEvent({ sha256: "../x" }));
    await settle();
    expect(parent.querySelector("[role=alertdialog]")).toBeNull();
  });

  it("falha do núcleo vai para onError", async () => {
    const { core, parent } = setup();
    core.on("decide_plugin", () => {
      throw "write_failed";
    });
    const onError = vi.fn();
    await watchPluginApprovals({ ipc: core.ipc, parent, onError });
    core.emit("plugin_approval_needed", pluginEvent());
    await settle();
    byText(parent, /^Permitir/)!.click();
    await settle();
    expect(onError).toHaveBeenCalledWith(expect.objectContaining({ code: "write_failed" }));
  });
});

describe("pacote adulterado: Baixar de novo", () => {
  const tampered: Diagnostic = {
    severity: "error",
    path: "main.typ",
    line: 1,
    message: "o pacote `@preview/demo:0.1.0` foi alterado depois do download",
    hints: [],
    kind: "package",
  };

  it("reconhece só o diagnóstico de pacote adulterado", () => {
    expect(tamperedPackage(tampered)).toBe("@preview/demo:0.1.0");
    expect(tamperedPackage({ ...tampered, kind: "typst" })).toBeNull();
    expect(
      tamperedPackage({
        ...tampered,
        message: "pacote `@preview/demo:0.1.0` não está no computador e não há conexão",
      }),
    ).toBeNull();
  });

  it("a ação Baixar de novo chama redownload_package", async () => {
    const core = fakeCore({ redownload_package: () => null });
    const box = document.createElement("div");
    const actions = mountPackageActions(box, { ipc: core.ipc });
    actions.update([tampered]);
    expect(box.hidden).toBe(false);
    expect(box.textContent).toContain("@preview/demo:0.1.0");
    byText(box, /Baixar de novo/)!.click();
    await settle();
    expect(core.calls("redownload_package")[0]?.args).toEqual({ spec: "@preview/demo:0.1.0" });
  });

  it("um botão por pacote, some com o diagnóstico, e falha vai para onError", async () => {
    const core = fakeCore();
    core.on("redownload_package", () => {
      throw "network_unavailable";
    });
    const onError = vi.fn();
    const box = document.createElement("div");
    const actions = mountPackageActions(box, { ipc: core.ipc, onError });
    actions.update([tampered, tampered]);
    expect(buttons(box)).toHaveLength(1);
    byText(box, /Baixar de novo/)!.click();
    await settle();
    expect(onError).toHaveBeenCalledWith(expect.objectContaining({ code: "network_unavailable" }));
    actions.update([]);
    expect(buttons(box)).toHaveLength(0);
    expect(box.hidden).toBe(true);
  });
});

// ---------------------------------------------------------------- cópia para o projeto (D-13)

describe("watchCopyOffers", () => {
  const offer = { requested: "../compartilhado/refs.bib" };
  const copied = { path: "refs.bib", updated: [{ path: "main.typ", version: 3 }] };

  function setup(result: unknown = copied) {
    const core = fakeCore({ copy_into_project: () => result });
    const parent = document.createElement("div");
    document.body.append(parent);
    const beforeCopy = vi.fn(async () => {});
    const onCopied = vi.fn();
    const onError = vi.fn();
    return { core, parent, beforeCopy, onCopied, onError };
  }

  it("mostra o caminho pedido e explica que a cópia fica no projeto", async () => {
    const { core, parent } = setup();
    await watchCopyOffers({ ipc: core.ipc, parent });
    core.emit("copy_into_project_offer", offer);
    await settle();
    const dialog = parent.querySelector("[role=alertdialog]");
    expect(dialog?.textContent).toContain("../compartilhado/refs.bib");
    expect(dialog?.textContent).toMatch(/fora da pasta do projeto/);
    expect(core.calls("copy_into_project")).toHaveLength(0);
  });

  it("copiar envia só { requested }, depois de gravar o editor, e avisa o resultado", async () => {
    const { core, parent, beforeCopy, onCopied } = setup();
    await watchCopyOffers({ ipc: core.ipc, parent, beforeCopy, onCopied });
    core.emit("copy_into_project_offer", offer);
    await settle();
    byText(parent, /^Escolher arquivo/)!.click();
    await settle();
    expect(beforeCopy).toHaveBeenCalled();
    expect(core.calls("copy_into_project")[0]?.args).toEqual({ requested: "../compartilhado/refs.bib" });
    expect(onCopied).toHaveBeenCalledWith(copied);
    expect(parent.querySelector("[role=alertdialog]")).toBeNull();
  });

  it("'Agora não' não chama o núcleo", async () => {
    const { core, parent, onCopied } = setup();
    await watchCopyOffers({ ipc: core.ipc, parent, onCopied });
    core.emit("copy_into_project_offer", offer);
    await settle();
    byText(parent, /Agora não/)!.click();
    await settle();
    expect(core.calls("copy_into_project")).toHaveLength(0);
    expect(onCopied).not.toHaveBeenCalled();
    expect(parent.querySelector("[role=alertdialog]")).toBeNull();
  });

  it("diálogo nativo cancelado (null) não avisa cópia", async () => {
    const { core, parent, onCopied } = setup(null);
    await watchCopyOffers({ ipc: core.ipc, parent, onCopied });
    core.emit("copy_into_project_offer", offer);
    await settle();
    byText(parent, /^Escolher arquivo/)!.click();
    await settle();
    expect(core.calls("copy_into_project")).toHaveLength(1);
    expect(onCopied).not.toHaveBeenCalled();
  });

  it("uma pergunta por caminho; evento malformado é ignorado", async () => {
    const { core, parent } = setup();
    await watchCopyOffers({ ipc: core.ipc, parent });
    core.emit("copy_into_project_offer", offer);
    core.emit("copy_into_project_offer", offer);
    core.emit("copy_into_project_offer", { requested: 1 });
    core.emit("copy_into_project_offer", { requested: "" });
    core.emit("copy_into_project_offer", null);
    await settle();
    expect(parent.querySelectorAll("[role=alertdialog]")).toHaveLength(1);
  });

  it("caminho entra como texto", async () => {
    const { core, parent } = setup();
    await watchCopyOffers({ ipc: core.ipc, parent });
    core.emit("copy_into_project_offer", { requested: "../<img src=x>.png" });
    await settle();
    expect(parent.querySelector("img")).toBeNull();
    expect(parent.textContent).toContain("../<img src=x>.png");
  });

  it("falha do núcleo (ex.: já existe) vai para onError", async () => {
    const { core, parent, onError, onCopied } = setup();
    core.on("copy_into_project", () => {
      throw "already_exists";
    });
    await watchCopyOffers({ ipc: core.ipc, parent, onError, onCopied });
    core.emit("copy_into_project_offer", offer);
    await settle();
    byText(parent, /^Escolher arquivo/)!.click();
    await settle();
    expect(onError).toHaveBeenCalledWith(expect.objectContaining({ code: "already_exists" }));
    expect(onCopied).not.toHaveBeenCalled();
  });
});
