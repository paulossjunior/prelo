// Testes do diálogo de conflito com mudança externa (T039; FR-008). Contrato: contracts/ipc.md
// (evento `external_change_conflict` `{ path }`; `resolve_external_change { path, keep }`).
import { afterEach, describe, expect, it, vi } from "vitest";
import { fakeCore, settle } from "../testing/fake-core";
import { askExternalChange, watchExternalChanges } from "./prompts";

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
