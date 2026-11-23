// Wrappers tipados do IPC da feature 003 (T023). Contrato: specs/003-editor-local/contracts/ipc.md.
// O núcleo responde erros como códigos estáveis em inglês; a interface mostra o texto em português.
import { Channel, invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";

// ---------------------------------------------------------------- tipos do contrato

export interface ProjectInfo {
  name: string;
  mainFile: string;
  compiler: { embedded: string; registered: string | null };
  warnings: string[];
}

/** Nó da árvore do projeto; `path` é relativo à raiz, com `/`. */
export interface TreeNode {
  name: string;
  path: string;
  kind: "file" | "dir";
  children?: TreeNode[];
}

export interface Change {
  /** Deslocamentos em unidades UTF-16 (os do CodeMirror), no texto anterior à edição. */
  from: number;
  to: number;
  insert: string;
}

export interface OpenedFile {
  text: string;
  version: number;
}

export interface Diagnostic {
  severity: "error" | "warning";
  path?: string;
  range?: { from: number; to: number };
  line?: number;
  message: string;
  hints: string[];
  kind: "typst" | "access" | "plugin" | "package" | "font" | "project";
}

export interface PageInfo {
  index: number;
  hash: string;
  widthPt: number;
  heightPt: number;
}

export interface PreviewUpdate {
  compileId: number;
  ok: boolean;
  pages: PageInfo[];
  changed: number[];
  diagnostics: Diagnostic[];
  durationMs: number;
}

/** Formato definido com o serviço de linguagem (T036). */
export type Completion = Record<string, unknown>;
/** Formato definido com o realce (T036). */
export type Token = Record<string, unknown>;

export interface SourcePosition {
  path: string;
  line: number;
  column: number;
}

export interface PagePosition {
  page: number;
  xPt: number;
  yPt: number;
}

export interface OutlineItem {
  level: number;
  title: string;
  path: string;
  line: number;
  page: number;
  yPt: number;
}

export interface FontLicense {
  family: string;
  file: string;
  license: "OFL-1.1";
  copyright: string;
}

export interface CompilerInfo {
  embedded: string;
  registered: string | null;
  fonts: FontLicense[];
}

export interface ExternalChangeConflict {
  path: string;
}

/**
 * O núcleo recarregou do disco um arquivo aberto sem alteração local (FR-008). Só o caminho e
 * a versão nova: o texto vem de `open_file`, a fonte única do estado do arquivo.
 */
export interface ExternalChangeReloaded {
  path: string;
  version: number;
}

export interface CopyIntoProjectOffer {
  requested: string;
}

/**
 * Resultado de `copy_into_project` (D-13): a cópia (`path`, relativo à raiz) e os arquivos em
 * que o núcleo trocou o caminho pedido pelo da cópia, com a versão nova de cada um.
 */
export interface CopiedIntoProject {
  path: string;
  updated: { path: string; version: number }[];
}

export interface PluginApprovalNeeded {
  path: string;
  origin: "project" | "package";
  package?: string;
  sizeBytes: number;
  sha256: string;
}

// ---------------------------------------------------------------- erros

export const ERROR_MESSAGES = {
  path_outside_project: "Acesso recusado: o caminho fica fora da pasta do projeto.",
  not_found: "Arquivo não encontrado.",
  not_utf8: "O arquivo não está em UTF-8 e não pode ser aberto no editor.",
  too_large: "Arquivo grande demais para abrir (limite de 256 MB).",
  version_mismatch: "O texto mudou enquanto a edição era enviada; recarregue o arquivo.",
  bad_range: "Edição fora do texto do arquivo; recarregue o arquivo.",
  write_failed: "Não foi possível gravar o arquivo. O texto continua no editor.",
  not_a_typ_file: "Escolha um arquivo .typ como arquivo principal.",
  folder_not_empty: "A pasta escolhida não está vazia. Escolha uma pasta vazia.",
  project_json_invalid:
    "O registro do projeto (.prelo/project.json) é inválido; usando os padrões.",
  main_file_missing:
    "O arquivo principal do projeto não foi encontrado. Escolha um .typ como arquivo principal.",
  external_change_conflict:
    "O arquivo mudou fora do Prelo e há alterações suas não salvas; escolha qual versão manter.",
  compiler_version_mismatch:
    "O projeto foi registrado com outra versão do Typst. Atualize a versão do projeto para compilar.",
  compile_errors: "O documento tem erros; corrija-os antes de exportar o PDF.",
  stale_page: "A página mudou desde o pedido; a prévia vai se atualizar.",
  network_unavailable: "Sem conexão para baixar o pacote.",
  package_rejected: "O pacote baixado foi recusado por não passar na validação.",
  no_project: "Nenhum projeto aberto. Abra ou crie um projeto.",
  not_implemented: "Recurso ainda não disponível nesta versão do Prelo.",
  invalid_arguments: "Pedido inválido da interface ao núcleo do Prelo.",
  already_exists: "Já existe um arquivo ou pasta com esse nome.",
  not_offered:
    "Esse arquivo não está mais pedido fora do projeto; nada foi copiado.",
  invalid_name:
    'Nome inválido. Use um nome que não comece com ponto, sem caracteres como \\ : * ? " < > | e sem espaço ou ponto no fim.',
} as const;

export type ErrorCode = keyof typeof ERROR_MESSAGES;

function isErrorCode(code: string): code is ErrorCode {
  return Object.prototype.hasOwnProperty.call(ERROR_MESSAGES, code);
}

/** Mensagem em português para o código de erro do núcleo. */
export function errorMessage(code: string): string {
  return isErrorCode(code)
    ? ERROR_MESSAGES[code]
    : `Erro inesperado do núcleo do Prelo (${code}).`;
}

export class IpcError extends Error {
  readonly code: string;
  readonly diagnostics?: Diagnostic[];

  constructor(code: string, diagnostics?: Diagnostic[]) {
    super(errorMessage(code));
    this.name = "IpcError";
    this.code = code;
    if (diagnostics) this.diagnostics = diagnostics;
  }
}

/** Converte a rejeição do `invoke` (código em texto ou `{ code, diagnostics }`) em `IpcError`. */
export function toIpcError(err: unknown): IpcError {
  if (err instanceof IpcError) return err;
  if (typeof err === "string" && err.length > 0) return new IpcError(err);
  if (typeof err === "object" && err !== null && !Array.isArray(err)) {
    const o = err as Record<string, unknown>;
    if (typeof o.code === "string" && o.code.length > 0) {
      return new IpcError(o.code, Array.isArray(o.diagnostics) ? (o.diagnostics as Diagnostic[]) : undefined);
    }
  }
  return new IpcError("unknown");
}

// ---------------------------------------------------------------- transporte

export type InvokeFn = (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
export type ListenFn = (event: string, handler: (payload: unknown) => void) => Promise<() => void>;
export interface ChannelLike<T> {
  onmessage: (message: T) => void;
}
export type ChannelFactory = <T>() => ChannelLike<T>;

export interface Transport {
  invoke: InvokeFn;
  listen: ListenFn;
  channel: ChannelFactory;
}

export interface Ipc {
  openProjectDialog(): Promise<ProjectInfo | null>;
  createProjectDialog(): Promise<ProjectInfo | null>;
  chooseMainFile(path: string): Promise<ProjectInfo>;
  listTree(): Promise<TreeNode[]>;
  updateProjectCompilerVersion(): Promise<ProjectInfo>;
  /** Arquivo vazio novo, sem sobrescrever (FR-034); devolve o caminho normalizado. */
  createFile(path: string): Promise<{ path: string }>;
  /** Pasta vazia nova, sem sobrescrever (FR-034); devolve o caminho normalizado. */
  createDir(path: string): Promise<{ path: string }>;
  openFile(path: string): Promise<OpenedFile>;
  editFile(path: string, version: number, changes: Change[]): Promise<{ version: number }>;
  saveFile(path: string): Promise<{ savedAt: string }>;
  resolveExternalChange(path: string, keep: "mine" | "disk"): Promise<OpenedFile>;
  complete(path: string, offset: number): Promise<Completion[]>;
  semanticTokens(path: string): Promise<Token[]>;
  subscribePreview(onUpdate: (update: PreviewUpdate) => void): Promise<void>;
  pageSvg(index: number, hash: string): Promise<string>;
  jumpFromClick(page: number, xPt: number, yPt: number): Promise<SourcePosition | null>;
  jumpFromCursor(path: string, offset: number): Promise<PagePosition[]>;
  outline(): Promise<OutlineItem[]>;
  compilerInfo(): Promise<CompilerInfo>;
  exportPdfDialog(): Promise<{ path: string } | null>;
  /**
   * O arquivo de origem é escolhido no diálogo nativo do núcleo, nunca pela interface.
   * `null`: a pessoa cancelou o diálogo.
   */
  copyIntoProject(requested: string): Promise<CopiedIntoProject | null>;
  decidePlugin(sha256: string, approve: boolean): Promise<unknown>;
  redownloadPackage(spec: string): Promise<unknown>;
  onExternalChangeConflict(handler: (e: ExternalChangeConflict) => void): Promise<() => void>;
  onExternalChangeReloaded(handler: (e: ExternalChangeReloaded) => void): Promise<() => void>;
  onCopyIntoProjectOffer(handler: (e: CopyIntoProjectOffer) => void): Promise<() => void>;
  onPluginApprovalNeeded(handler: (e: PluginApprovalNeeded) => void): Promise<() => void>;
}

export function createIpc(transport: Transport): Ipc {
  async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    try {
      const result = args === undefined ? await transport.invoke(cmd) : await transport.invoke(cmd, args);
      return result as T;
    } catch (err) {
      throw toIpcError(err);
    }
  }
  const on =
    <T>(event: string) =>
    (handler: (payload: T) => void) =>
      transport.listen(event, (payload) => handler(payload as T));

  return {
    openProjectDialog: () => call("open_project_dialog"),
    createProjectDialog: () => call("create_project_dialog"),
    chooseMainFile: (path) => call("choose_main_file", { path }),
    listTree: () => call("list_tree"),
    updateProjectCompilerVersion: () => call("update_project_compiler_version"),
    createFile: (path) => call("create_file", { path }),
    createDir: (path) => call("create_dir", { path }),
    openFile: (path) => call("open_file", { path }),
    editFile: (path, version, changes) => call("edit_file", { path, version, changes }),
    saveFile: (path) => call("save_file", { path }),
    resolveExternalChange: (path, keep) => call("resolve_external_change", { path, keep }),
    complete: (path, offset) => call("complete", { path, offset }),
    semanticTokens: (path) => call("semantic_tokens", { path }),
    subscribePreview: async (onUpdate) => {
      const channel = transport.channel<PreviewUpdate>();
      channel.onmessage = onUpdate;
      await call("subscribe_preview", { channel });
    },
    pageSvg: (index, hash) => call("page_svg", { index, hash }),
    jumpFromClick: (page, xPt, yPt) => call("jump_from_click", { page, xPt, yPt }),
    jumpFromCursor: (path, offset) => call("jump_from_cursor", { path, offset }),
    outline: () => call("outline"),
    compilerInfo: () => call("compiler_info"),
    exportPdfDialog: () => call("export_pdf_dialog"),
    copyIntoProject: (requested) => call("copy_into_project", { requested }),
    decidePlugin: (sha256, approve) => call("decide_plugin", { sha256, approve }),
    redownloadPackage: (spec) => call("redownload_package", { spec }),
    onExternalChangeConflict: on<ExternalChangeConflict>("external_change_conflict"),
    onExternalChangeReloaded: on<ExternalChangeReloaded>("external_change_reloaded"),
    onCopyIntoProjectOffer: on<CopyIntoProjectOffer>("copy_into_project_offer"),
    onPluginApprovalNeeded: on<PluginApprovalNeeded>("plugin_approval_needed"),
  };
}

/** Transporte real do Tauri. Só é tocado quando um comando é chamado. */
export const tauriTransport: Transport = {
  invoke: (cmd, args) => tauriInvoke(cmd, args),
  listen: (event, handler) => tauriListen(event, (e) => handler(e.payload)),
  channel: <T>() => new Channel<T>(),
};

export const ipc: Ipc = createIpc(tauriTransport);
