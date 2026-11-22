// Testes do editor (T030, parte do editor; FR-006, FR-007). Contrato: contracts/ipc.md
// (`edit_file` com `{from,to,insert}` em UTF-16 sobre o texto anterior, em ordem crescente e
// sem sobreposição; `save_file`; `complete`; `semantic_tokens`).
import { CompletionContext } from "@codemirror/autocomplete";
import { EditorState } from "@codemirror/state";
import { afterEach, describe, expect, it, vi } from "vitest";
import { IpcError } from "../ipc";
import { deferred, fakeCore, settle } from "../testing/fake-core";
import { completionSource, toCompletion } from "./completion";
import { mountEditor, type EditorHandle } from "./editor";
import { normalizeTokens } from "./highlight";

let handles: EditorHandle[] = [];
afterEach(() => {
  for (const h of handles) h.destroy();
  handles = [];
  document.body.innerHTML = "";
});

function setup(text = "= Título\n\nOlá.", extra: Parameters<typeof fakeCore>[0] = {}) {
  let version = 1;
  const core = fakeCore({
    edit_file: () => ({ version: ++version }),
    save_file: () => ({ savedAt: "2026-11-22T10:00:00Z" }),
    ...extra,
  });
  const parent = document.createElement("div");
  document.body.append(parent);
  const onError = vi.fn();
  const onSaved = vi.fn();
  const editor = mountEditor(parent, {
    ipc: core.ipc,
    path: "main.typ",
    text,
    version: 1,
    onError,
    onSaved,
    tokenDelayMs: 0,
  });
  handles.push(editor);
  return { core, parent, editor, onError, onSaved };
}

function keydown(target: HTMLElement, init: KeyboardEventInit) {
  const ev = new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...init });
  target.dispatchEvent(ev);
  return ev;
}

describe("montagem", () => {
  it("mostra o texto aberto num editor acessível", () => {
    const { parent, editor } = setup("#set page(\"a4\")");
    expect(editor.view.state.doc.toString()).toBe('#set page("a4")');
    expect(parent.querySelector(".cm-editor")).not.toBeNull();
    expect(editor.view.contentDOM.getAttribute("aria-label")).toBe("Código-fonte: main.typ");
    expect(editor.version()).toBe(1);
  });
});

describe("envio de mudanças (edit_file)", () => {
  it("envia {from,to,insert} com a versão aberta e guarda a versão devolvida", async () => {
    const { core, editor } = setup("abc");
    editor.view.dispatch({ changes: { from: 1, to: 2, insert: "XY" } });
    await editor.flush();
    expect(core.calls("edit_file")).toEqual([
      {
        cmd: "edit_file",
        args: { path: "main.typ", version: 1, changes: [{ from: 1, to: 2, insert: "XY" }] },
      },
    ]);
    expect(editor.version()).toBe(2);
  });

  it("transação com várias mudanças vai em ordem crescente, sobre o texto anterior", async () => {
    const { core, editor } = setup("0123456789");
    editor.view.dispatch({
      changes: [
        { from: 8, to: 9, insert: "" },
        { from: 0, to: 1, insert: "A" },
        { from: 4, to: 4, insert: "BB" },
      ],
    });
    await editor.flush();
    expect(core.calls("edit_file")[0]?.args.changes).toEqual([
      { from: 0, to: 1, insert: "A" },
      { from: 4, to: 4, insert: "BB" },
      { from: 8, to: 9, insert: "" },
    ]);
  });

  it("deslocamentos em unidades UTF-16 (os do CodeMirror)", async () => {
    const { core, editor } = setup("ç𝑥 a");
    // "ç" = 1 unidade, "𝑥" = 2 unidades: depois de "𝑥" o deslocamento é 3.
    editor.view.dispatch({ changes: { from: 3, insert: "!" } });
    await editor.flush();
    expect(core.calls("edit_file")[0]?.args.changes).toEqual([{ from: 3, to: 3, insert: "!" }]);
  });

  it("edições seguidas esperam a resposta anterior e usam a versão nova", async () => {
    const first = deferred<{ version: number }>();
    let n = 0;
    const { core, editor } = setup("abc", {
      edit_file: () => (n++ === 0 ? first.promise : { version: 3 }),
    });
    editor.view.dispatch({ changes: { from: 0, insert: "x" } });
    editor.view.dispatch({ changes: { from: 1, insert: "y" } });
    await settle();
    expect(core.calls("edit_file")).toHaveLength(1);
    first.resolve({ version: 2 });
    await editor.flush();
    const sent = core.calls("edit_file");
    expect(sent).toHaveLength(2);
    expect(sent[1]?.args).toEqual({
      path: "main.typ",
      version: 2,
      changes: [{ from: 1, to: 1, insert: "y" }],
    });
    expect(editor.version()).toBe(3);
  });

  it("recusa do núcleo (version_mismatch) avisa e mantém o texto", async () => {
    const { editor, onError } = setup("abc", {
      edit_file: () => {
        throw "version_mismatch";
      },
    });
    editor.view.dispatch({ changes: { from: 3, insert: "d" } });
    await editor.flush();
    expect(onError).toHaveBeenCalledTimes(1);
    const err = onError.mock.calls[0]?.[0] as IpcError;
    expect(err).toBeInstanceOf(IpcError);
    expect(err.code).toBe("version_mismatch");
    expect(editor.view.state.doc.toString()).toBe("abcd");
  });

  it("reload troca o texto sem enviar edição ao núcleo", async () => {
    const { core, editor } = setup("abc");
    editor.reload("texto do disco", 7);
    await editor.flush();
    expect(editor.view.state.doc.toString()).toBe("texto do disco");
    expect(editor.version()).toBe(7);
    expect(core.calls("edit_file")).toHaveLength(0);
  });
});

describe("salvar (⌘S / Ctrl+S)", () => {
  it("Ctrl+S chama save_file depois de enviar as edições pendentes", async () => {
    const { core, editor, onSaved } = setup("abc");
    editor.view.dispatch({ changes: { from: 0, insert: "z" } });
    const ev = keydown(editor.view.contentDOM, { key: "s", ctrlKey: true });
    expect(ev.defaultPrevented).toBe(true);
    await settle();
    expect(core.calls().map((c) => c.cmd).filter((c) => c !== "semantic_tokens")).toEqual([
      "edit_file",
      "save_file",
    ]);
    expect(core.calls("save_file")[0]?.args).toEqual({ path: "main.typ" });
    expect(onSaved).toHaveBeenCalledWith("2026-11-22T10:00:00Z");
  });

  it("⌘S (Meta+S) também salva", async () => {
    const { core, editor } = setup("abc");
    keydown(editor.view.contentDOM, { key: "s", metaKey: true });
    await settle();
    expect(core.calls("save_file")).toHaveLength(1);
  });

  it("falha ao gravar (write_failed) avisa e mantém o texto no editor", async () => {
    const { editor, onError } = setup("abc", {
      save_file: () => {
        throw "write_failed";
      },
    });
    expect(await editor.save()).toBe(false);
    expect((onError.mock.calls[0]?.[0] as IpcError).code).toBe("write_failed");
    expect(editor.view.state.doc.toString()).toBe("abc");
  });
});

describe("realce (semantic_tokens)", () => {
  it("aplica os tokens do núcleo como marcas com classe", async () => {
    const { editor, parent } = setup("= Título\n#let x = 1", {
      semantic_tokens: () => [
        { from: 0, to: 8, kind: "heading" },
        { from: 9, to: 13, kind: "keyword" },
      ],
    });
    await settle();
    const heading = parent.querySelector(".tok-heading");
    expect(heading?.textContent).toBe("= Título");
    expect(parent.querySelector(".tok-keyword")?.textContent).toBe("#let");
    expect(editor.view.state.doc.toString()).toContain("#let");
  });

  it("sem serviço de linguagem (not_implemented) o editor segue sem realce e sem erro", async () => {
    const { parent, onError } = setup("= Título");
    await settle();
    expect(parent.querySelector("[class*='tok-']")).toBeNull();
    expect(onError).not.toHaveBeenCalled();
  });

  it("normalizeTokens descarta intervalos inválidos e limpa o nome da classe", () => {
    expect(
      normalizeTokens(
        [
          { from: 0, to: 3, kind: "function" },
          { from: 5, to: 2, kind: "bad" },
          { from: 0, to: 99, kind: "fora" },
          { from: 1, to: 2, kind: "x\" onclick=\"y" },
          { from: 1, to: 2 },
        ],
        10,
      ),
    ).toEqual([
      { from: 0, to: 3, kind: "function" },
      { from: 1, to: 2, kind: "x-onclick-y" },
    ]);
  });
});

describe("autocompletar (complete)", () => {
  function ctx(doc: string, pos: number, explicit = false) {
    return new CompletionContext(EditorState.create({ doc }), pos, explicit);
  }

  it("pede ao núcleo com caminho e deslocamento depois de enviar as edições", async () => {
    const flush = vi.fn(async () => {});
    const core = fakeCore({
      complete: () => [
        { label: "lorem", kind: "function", detail: "lorem(words)" },
        { label: "let", kind: "keyword", apply: "let " },
      ],
    });
    const source = completionSource({ ipc: core.ipc, path: "main.typ", flush });
    const result = await source(ctx("#lor", 4));
    expect(flush).toHaveBeenCalled();
    expect(core.calls("complete")[0]?.args).toEqual({ path: "main.typ", offset: 4 });
    expect(result?.from).toBe(1);
    expect(result?.options.map((o) => o.label)).toEqual(["lorem", "let"]);
    expect(result?.options[0]).toMatchObject({ type: "function", detail: "lorem(words)" });
    expect(result?.options[1]).toMatchObject({ apply: "let " });
  });

  it("not_implemented (antes de T036) não quebra e para de perguntar", async () => {
    const core = fakeCore();
    const source = completionSource({ ipc: core.ipc, path: "main.typ", flush: async () => {} });
    expect(await source(ctx("#lor", 4, true))).toBeNull();
    expect(await source(ctx("#lor", 4, true))).toBeNull();
    expect(core.calls("complete")).toHaveLength(1);
  });

  it("sem palavra nem gatilho e sem pedido explícito não chama o núcleo", async () => {
    const core = fakeCore({ complete: () => [] });
    const source = completionSource({ ipc: core.ipc, path: "main.typ", flush: async () => {} });
    expect(await source(ctx("a ", 2))).toBeNull();
    expect(core.calls("complete")).toHaveLength(0);
  });

  it("toCompletion ignora itens sem rótulo", () => {
    expect(toCompletion({ kind: "x" })).toBeNull();
    expect(toCompletion({ label: "heading" })).toMatchObject({ label: "heading" });
  });
});
