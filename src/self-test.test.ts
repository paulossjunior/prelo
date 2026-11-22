// Testes do roteiro do autoteste (T031; SC-006): o roteiro abre o projeto, edita, espera a
// prévia mudar e o erro aparecer no editor, e devolve o que observou para o núcleo conferir.
import { afterEach, describe, expect, it } from "vitest";
import type { Change, ProjectInfo } from "./ipc";
import { ERROR_SNIPPET, runEditorScenario } from "./self-test";
import { fakeCore } from "./testing/fake-core";
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

/** Núcleo falso que "compila" a cada edição: texto com o erro → diagnóstico na linha dele. */
function compilingCore(opts: { compiles?: boolean } = {}) {
  let text = "= Introdução\n\nOlá.";
  let version = 1;
  let compileId = 0;
  const core = fakeCore({
    get_app_info: () => ({ version: "0.1.0", os: "macos", arch: "aarch64" }),
    subscribe_preview: () => null,
    list_tree: () => [{ name: "main.typ", path: "main.typ", kind: "file" }],
    open_file: () => ({ text, version }),
    page_svg: () => "<svg/>",
    edit_file: (a) => {
      for (const c of [...(a.changes as Change[])].reverse()) {
        text = text.slice(0, c.from) + c.insert + text.slice(c.to);
      }
      const at = text.indexOf(ERROR_SNIPPET);
      setTimeout(() => {
        if (opts.compiles === false) return;
        compileId++;
        if (at >= 0) {
          core.push({
            compileId, ok: false, pages: [], changed: [], durationMs: 3,
            diagnostics: [{ severity: "error", path: "main.typ", range: { from: at + 1, to: at + 5 }, message: "unknown variable", hints: [], kind: "typst" }],
          });
        } else {
          core.push({
            compileId, ok: true, changed: [0], diagnostics: [], durationMs: 3,
            pages: [{ index: 0, hash: `h${compileId}`, widthPt: 595, heightPt: 842 }],
          });
        }
      }, 0);
      return { version: ++version };
    },
  });
  return core;
}

function mount(core: ReturnType<typeof fakeCore>) {
  const root = document.createElement("main");
  document.body.append(root);
  const ws = mountWorkspace(root, {
    ipc: core.ipc,
    appInfo: (cmd) => core.invoke(cmd),
    previewUrls: { create: () => "blob:x", revoke: () => {} },
    tokenDelayMs: 0,
  });
  handles.push(ws);
  return ws;
}

describe("runEditorScenario", () => {
  it("abre, edita, vê a prévia mudar e o erro marcado na linha certa", async () => {
    const core = compilingCore();
    const ws = mount(core);
    await ws.ready;
    // Primeira compilação ao abrir o projeto.
    const open = async () => {
      setTimeout(
        () =>
          core.push({
            compileId: 0, ok: true, changed: [0], diagnostics: [], durationMs: 5,
            pages: [{ index: 0, hash: "h0", widthPt: 595, heightPt: 842 }],
          }),
        0,
      );
      return INFO;
    };
    const r = await runEditorScenario(ws, open, { timeoutMs: 2000 });
    expect(r.failure).toBeUndefined();
    expect(r.projectOpened).toBe(true);
    expect(r.previewChanged).toBe(true);
    expect(r.errorLine).toBeGreaterThan(1);
    expect(r.diagnosticLine).toBe(r.errorLine);
    expect(r.editorMarked).toBe(true);
    expect(r.compilerShown).toContain("Typst 0.15.1");
  });

  it("sem compilação: falha com motivo em vez de travar", async () => {
    const core = compilingCore({ compiles: false });
    const ws = mount(core);
    await ws.ready;
    const r = await runEditorScenario(ws, async () => INFO, { timeoutMs: 50 });
    expect(r.failure).toMatch(/prévia/);
    expect(r.previewChanged).toBe(false);
  });
});

describe("conferência de layout no app real", () => {
  const openWithFirstCompile = (core: ReturnType<typeof fakeCore>) => async () => {
    setTimeout(
      () =>
        core.push({
          compileId: 0, ok: true, changed: [0], diagnostics: [], durationMs: 5,
          pages: [{ index: 0, hash: "h0", widthPt: 595, heightPt: 842 }],
        }),
      0,
    );
    return INFO;
  };

  it("mede o layout com texto mais alto que o editor e uma linha longa, e devolve os problemas", async () => {
    const core = compilingCore();
    const ws = mount(core);
    await ws.ready;
    let lines = 0;
    let longest = 0;
    const checkLayout = () => {
      const doc = ws.editor()!.view.state.doc;
      lines = doc.lines;
      for (const l of doc.iterLines()) longest = Math.max(longest, l.length);
      return ["números de linha não estão numa coluna à esquerda do código"];
    };
    const r = await runEditorScenario(ws, openWithFirstCompile(core), { timeoutMs: 2000, checkLayout });
    expect(r.failure).toBeUndefined();
    expect(lines).toBeGreaterThanOrEqual(150);
    expect(longest).toBeGreaterThanOrEqual(400);
    expect(r.layoutProblems).toEqual(["números de linha não estão numa coluna à esquerda do código"]);
  });

  it("sem conferência (jsdom) não inventa problemas", async () => {
    const core = compilingCore();
    const ws = mount(core);
    await ws.ready;
    const r = await runEditorScenario(ws, openWithFirstCompile(core), { timeoutMs: 2000 });
    expect(r.layoutProblems).toEqual([]);
  });
});
