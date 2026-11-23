// Testes dos wrappers tipados do IPC (T023). Contrato: specs/003-editor-local/contracts/ipc.md.
import { describe, expect, it, vi } from "vitest";
import {
  createIpc,
  ERROR_MESSAGES,
  errorMessage,
  IpcError,
  toIpcError,
  type ChannelFactory,
  type ChannelLike,
  type Ipc,
  type PreviewUpdate,
} from "./ipc";

function setup(result: unknown = null) {
  const invoke = vi.fn((_cmd: string, _args?: Record<string, unknown>) => Promise.resolve(result));
  const unlisten = vi.fn();
  const handlers = new Map<string, (payload: unknown) => void>();
  const listen = vi.fn((event: string, handler: (payload: unknown) => void) => {
    handlers.set(event, handler);
    return Promise.resolve(unlisten);
  });
  const channels: ChannelLike<unknown>[] = [];
  const channel = vi.fn(() => {
    const c: ChannelLike<unknown> = { onmessage: () => {} };
    channels.push(c);
    return c;
  });
  const ipc = createIpc({ invoke, listen, channel: channel as ChannelFactory });
  return { ipc, invoke, listen, handlers, unlisten, channels };
}

const CHANGES = [{ from: 0, to: 1, insert: "ç" }];

// [chamada do wrapper, comando, argumentos esperados]
const CALLS: [string, (ipc: Ipc) => Promise<unknown>, string, Record<string, unknown> | undefined][] = [
  ["openProjectDialog", (i) => i.openProjectDialog(), "open_project_dialog", undefined],
  ["createProjectDialog", (i) => i.createProjectDialog(), "create_project_dialog", undefined],
  ["chooseMainFile", (i) => i.chooseMainFile("cap/a.typ"), "choose_main_file", { path: "cap/a.typ" }],
  ["listTree", (i) => i.listTree(), "list_tree", undefined],
  [
    "updateProjectCompilerVersion",
    (i) => i.updateProjectCompilerVersion(),
    "update_project_compiler_version",
    undefined,
  ],
  ["createFile", (i) => i.createFile("cap/novo.typ"), "create_file", { path: "cap/novo.typ" }],
  ["createDir", (i) => i.createDir("cap/anexos"), "create_dir", { path: "cap/anexos" }],
  ["openFile", (i) => i.openFile("main.typ"), "open_file", { path: "main.typ" }],
  [
    "editFile",
    (i) => i.editFile("main.typ", 3, CHANGES),
    "edit_file",
    { path: "main.typ", version: 3, changes: CHANGES },
  ],
  ["saveFile", (i) => i.saveFile("main.typ"), "save_file", { path: "main.typ" }],
  [
    "resolveExternalChange",
    (i) => i.resolveExternalChange("main.typ", "disk"),
    "resolve_external_change",
    { path: "main.typ", keep: "disk" },
  ],
  ["complete", (i) => i.complete("main.typ", 7), "complete", { path: "main.typ", offset: 7 }],
  ["semanticTokens", (i) => i.semanticTokens("main.typ"), "semantic_tokens", { path: "main.typ" }],
  ["pageSvg", (i) => i.pageSvg(2, "abc"), "page_svg", { index: 2, hash: "abc" }],
  ["previewAsset", (i) => i.previewAsset("ab12"), "preview_asset", { sha256: "ab12" }],
  [
    "jumpFromClick",
    (i) => i.jumpFromClick(1, 10.5, 20),
    "jump_from_click",
    { page: 1, xPt: 10.5, yPt: 20 },
  ],
  [
    "jumpFromCursor",
    (i) => i.jumpFromCursor("main.typ", 4),
    "jump_from_cursor",
    { path: "main.typ", offset: 4 },
  ],
  ["outline", (i) => i.outline(), "outline", undefined],
  ["compilerInfo", (i) => i.compilerInfo(), "compiler_info", undefined],
  ["compileNow", (i) => i.compileNow(), "compile_now", undefined],
  ["exportPdfDialog", (i) => i.exportPdfDialog(), "export_pdf_dialog", undefined],
  [
    "copyIntoProject",
    (i) => i.copyIntoProject("img/fig.png"),
    "copy_into_project",
    { requested: "img/fig.png" },
  ],
  [
    "decidePlugin",
    (i) => i.decidePlugin("ab12", true),
    "decide_plugin",
    { sha256: "ab12", approve: true },
  ],
  [
    "redownloadPackage",
    (i) => i.redownloadPackage("@preview/foo:1.0.0"),
    "redownload_package",
    { spec: "@preview/foo:1.0.0" },
  ],
];

describe("wrappers dos comandos", () => {
  it.each(CALLS)("%s chama o comando do contrato com os argumentos certos", async (_name, call, cmd, args) => {
    const { ipc, invoke } = setup({ ok: 1 });
    await expect(call(ipc)).resolves.toEqual({ ok: 1 });
    expect(invoke).toHaveBeenCalledTimes(1);
    if (args === undefined) {
      expect(invoke).toHaveBeenCalledWith(cmd);
    } else {
      expect(invoke).toHaveBeenCalledWith(cmd, args);
    }
  });

  it("diálogos cancelados devolvem null", async () => {
    const { ipc } = setup(null);
    await expect(ipc.openProjectDialog()).resolves.toBeNull();
    await expect(ipc.exportPdfDialog()).resolves.toBeNull();
  });
});

describe("erros do núcleo", () => {
  const CODES = [
    "path_outside_project",
    "not_found",
    "not_utf8",
    "too_large",
    "version_mismatch",
    "bad_range",
    "write_failed",
    "not_a_typ_file",
    "folder_not_empty",
    "project_json_invalid",
    "main_file_missing",
    "compiler_version_mismatch",
    "compile_errors",
    "stale_page",
    "network_unavailable",
    "package_rejected",
    "no_project",
    "not_implemented",
    "invalid_arguments",
    "already_exists",
    "invalid_name",
    "compile_suspended",
  ];

  it.each(CODES)("%s tem mensagem em português", (code) => {
    const message = errorMessage(code);
    expect(message).toBe(ERROR_MESSAGES[code as keyof typeof ERROR_MESSAGES]);
    expect(message.length).toBeGreaterThan(10);
    expect(message).not.toContain(code);
    expect(message).toMatch(/[A-ZÁÉÍÓÚ]/);
    expect(message.endsWith(".")).toBe(true);
  });

  it("mensagens citam o problema em português", () => {
    expect(errorMessage("path_outside_project")).toMatch(/fora da pasta do projeto/i);
    expect(errorMessage("not_found")).toMatch(/não encontrado/i);
    expect(errorMessage("not_utf8")).toMatch(/UTF-8/);
    expect(errorMessage("too_large")).toMatch(/grande demais/i);
    expect(errorMessage("write_failed")).toMatch(/gravar/i);
    expect(errorMessage("compiler_version_mismatch")).toMatch(/versão/i);
    expect(errorMessage("main_file_missing")).toMatch(/arquivo principal/i);
  });

  it("código desconhecido vira mensagem genérica que cita o código", () => {
    expect(errorMessage("qualquer_coisa")).toMatch(/erro inesperado/i);
    expect(errorMessage("qualquer_coisa")).toContain("qualquer_coisa");
  });

  it("rejeição com código vira IpcError com código e mensagem", async () => {
    const { ipc, invoke } = setup();
    invoke.mockRejectedValueOnce("path_outside_project");
    const err = await ipc.openFile("../fora.typ").catch((e: unknown) => e);
    expect(err).toBeInstanceOf(IpcError);
    expect((err as IpcError).code).toBe("path_outside_project");
    expect((err as IpcError).message).toBe(errorMessage("path_outside_project"));
  });

  it("compile_errors traz os diagnósticos", () => {
    const diagnostics = [
      { severity: "error", message: "x", hints: [], kind: "typst", path: "main.typ" },
    ];
    const err = toIpcError({ code: "compile_errors", diagnostics });
    expect(err.code).toBe("compile_errors");
    expect(err.diagnostics).toEqual(diagnostics);
    expect(err.message).toBe(errorMessage("compile_errors"));
  });

  it("rejeição fora do formato vira código unknown", () => {
    for (const raw of [undefined, null, 42, {}, { code: 7 }, ["x"]]) {
      const err = toIpcError(raw);
      expect(err.code).toBe("unknown");
      expect(err.message).toMatch(/erro inesperado/i);
    }
    const fromError = toIpcError(new Error("falha de transporte"));
    expect(fromError.code).toBe("unknown");
  });

  it("IpcError recebido de novo é mantido", () => {
    const original = new IpcError("not_found");
    expect(toIpcError(original)).toBe(original);
  });
});

describe("prévia e eventos", () => {
  it("subscribePreview passa um canal e repassa cada PreviewUpdate", async () => {
    const { ipc, invoke, channels } = setup();
    const updates: PreviewUpdate[] = [];
    await ipc.subscribePreview((u) => updates.push(u));
    expect(channels).toHaveLength(1);
    expect(invoke).toHaveBeenCalledWith("subscribe_preview", { channel: channels[0] });
    const update: PreviewUpdate = {
      compileId: 3,
      ok: true,
      pages: [{ index: 0, hash: "h", widthPt: 595, heightPt: 842 }],
      changed: [0],
      diagnostics: [],
      durationMs: 12,
    };
    channels[0]!.onmessage(update);
    expect(updates).toEqual([update]);
  });

  it.each([
    ["onExternalChangeConflict", "external_change_conflict", { path: "main.typ" }],
    ["onExternalChangeReloaded", "external_change_reloaded", { path: "main.typ", version: 3 }],
    ["onCopyIntoProjectOffer", "copy_into_project_offer", { requested: "../fig.png" }],
    [
      "onPluginApprovalNeeded",
      "plugin_approval_needed",
      { path: "p.wasm", origin: "project", sizeBytes: 10, sha256: "ab" },
    ],
  ] as const)("%s escuta %s e repassa o conteúdo", async (method, event, payload) => {
    const { ipc, listen, handlers, unlisten } = setup();
    const received: unknown[] = [];
    const stop = await (ipc[method] as (h: (p: unknown) => void) => Promise<() => void>)((p) =>
      received.push(p),
    );
    expect(listen).toHaveBeenCalledWith(event, expect.any(Function));
    handlers.get(event)!(payload);
    expect(received).toEqual([payload]);
    stop();
    expect(unlisten).toHaveBeenCalled();
  });
});
