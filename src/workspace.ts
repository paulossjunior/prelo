// Área de trabalho do editor local (T039; US1): barra com o projeto e as ações (Abrir projeto,
// Novo projeto (US6), Salvar, Sobre), árvore | editor | prévia, e a barra de status com a compilação, o estado de
// gravação, a versão do Typst e a do Prelo. Layout e linguagem visual de prototype/index.html.
//
// Fiação (PR #176): uma assinatura de `subscribe_preview` alimenta a prévia e os diagnósticos
// do editor; a árvore abre arquivos (`open_file` → `mountEditor` → `setCurrent`); edição
// recusada pelo núcleo recarrega o arquivo.
//
// Navegação (US4): clique na prévia abre o arquivo de origem na linha; "Localizar na prévia"
// (cabeçalho do editor) rola a prévia até o trecho do cursor; o painel "Estrutura", abaixo dos
// arquivos, leva editor e prévia a cada seção (src/preview/sync.ts, src/project/outline.ts).
//
// Erros (FR-006a): o painel "Erros" (src/editor/problems.ts), acima da barra de status, lista
// todos os diagnósticos com o motivo em português; a contagem na barra de status e a faixa de
// erro da prévia o abrem, e clicar num item abre o arquivo com o cursor no trecho.
//
// Compilar (FR-014a): botão na barra da prévia e ⌘⇧B / Ctrl+Shift+B (src/preview/compile-now.ts);
// envia as edições pendentes e pede `compile_now`; desativado sem projeto e com versão diferente.
import { StateEffect } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { compilerStatus, mountAbout, type AboutHandle } from "./about";
import { mountAppInfo, type Invoke } from "./app-info";
import { mountEditor, type EditorHandle } from "./editor/editor";
import { severityIcon } from "./editor/icons";
import { isPackagePath, mountProblems, type ProblemsHandle } from "./editor/problems";
import { mountEditorZoom, type EditorZoomHandle } from "./editor/zoom";
import { mountExport } from "./export";
import {
  errorMessage,
  toIpcError,
  type Diagnostic,
  type Ipc,
  type IpcError,
  type OutlineItem,
  type PreviewUpdate,
  type ProjectInfo,
  type SourcePosition,
} from "./ipc";
import { mountPanes, type PanesHandle } from "./panes";
import { mountCompileNow, type CompileNowHandle } from "./preview/compile-now";
import { mountPreview, type BlobUrls, type PreviewHandle } from "./preview/preview";
import { mountSync, offsetOf, type SyncHandle } from "./preview/sync";
import { mountOutline, type OutlineHandle } from "./project/outline";
import {
  conflictResolver,
  mountPackageActions,
  watchCopyOffers,
  watchExternalChanges,
  watchPluginApprovals,
} from "./project/prompts";
import { mountTree, type TreeHandle } from "./project/tree";
import "./workspace.css";

export const VERSION_MISMATCH = "compiler_version_mismatch";
export const MAIN_FILE_MISSING = "main_file_missing";

/** Códigos de recusa de edição que pedem recarregar o arquivo do núcleo. */
const RELOAD_CODES = new Set(["version_mismatch", "bad_range"]);
/** `save_file` recusado porque o arquivo mudou fora e há alterações locais (PR #181). */
const CONFLICT = "external_change_conflict";
const USER_EVENTS = ["input", "delete", "undo", "redo", "move"];

export interface WorkspaceOptions {
  ipc: Ipc;
  /** Chamada de `get_app_info` (tela de 001, na barra de status). */
  appInfo: Invoke;
  previewUrls?: BlobUrls;
  tokenDelayMs?: number;
}

export interface WorkspaceHandle {
  readonly element: HTMLElement;
  /** Texto "Prelo {versão} · {sistema}" (autoteste e e2e). */
  readonly appInfoElement: HTMLElement;
  /** Resolve quando a versão do app aparece (ou falha) e a prévia foi assinada. */
  readonly ready: Promise<void>;
  showProject(info: ProjectInfo): Promise<void>;
  openFile(path: string): Promise<void>;
  editor(): EditorHandle | null;
  project(): ProjectInfo | null;
  /** Texto da versão do compilador mostrado na barra de status. */
  compilerLabel(): string;
  onPreviewUpdate(cb: (u: PreviewUpdate) => void): () => void;
  destroy(): void;
}

function el<K extends keyof HTMLElementTagNameMap>(tag: K, cls?: string, text?: string): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text !== undefined) e.textContent = text;
  return e;
}

function button(label: string, cls = "btn"): HTMLButtonElement {
  const b = el("button", cls, label);
  b.type = "button";
  return b;
}

function newProjectButton(): HTMLButtonElement {
  const b = button("Novo projeto…");
  b.dataset.action = "new-project";
  b.title = "Criar um projeto Typst mínimo numa pasta vazia";
  return b;
}

function clock(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  return d.toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

export function mountWorkspace(root: HTMLElement, opts: WorkspaceOptions): WorkspaceHandle {
  const { ipc } = opts;

  // ---------------------------------------------------------------- estrutura
  const app = el("div", "ws");
  const top = el("header", "ws-top");
  const projectEl = el("div", "ws-project");
  projectEl.append(el("strong", undefined, "Prelo"), el("small", undefined, "nenhum projeto aberto"));
  const openBtn = button("Abrir projeto…", "btn primary");
  // "Novo projeto" (US6): o núcleo mostra o diálogo de pasta e cria o projeto mínimo.
  const newBtn = newProjectButton();
  const saveBtn = button("Salvar");
  saveBtn.title = "Salvar (⌘S / Ctrl+S)";
  saveBtn.disabled = true;
  const aboutBtn = button("Sobre");
  // "Exportar PDF" (US5): a ação principal da barra; o resto em src/export.ts.
  const exporter = mountExport({ ipc, onError: (err) => showError(err) });
  const topSpacer = el("div", "ws-spacer");
  top.append(projectEl, topSpacer, openBtn, newBtn, saveBtn, aboutBtn, exporter.button);

  const message = el("div", "ws-message");
  message.setAttribute("role", "status");
  message.hidden = true;
  const banner = el("div", "ws-banner");
  banner.hidden = true;
  // Pacote adulterado: ação "Baixar de novo" (FR-018).
  const packageBanner = el("div", "ws-banner");
  const packages = mountPackageActions(packageBanner, { ipc, onError: (err) => showError(err) });

  // Cada painel numa coluna com cabeçalho e botão de recolher (FR-013b); a prévia usa a própria
  // barra como cabeçalho.
  const body = el("div", "ws-body");
  const treeCol = el("div", "ws-col ws-col-tree");
  const treeHead = el("div", "ws-pane-h");
  treeHead.append(el("h4", undefined, "Arquivos"));
  const side = el("aside", "ws-side");
  treeCol.append(treeHead, side);
  const editorCol = el("div", "ws-col ws-col-editor");
  const editorHead = el("div", "ws-pane-h");
  const editorPath = el("span", "ws-pane-path");
  // "Localizar na prévia" (US4 cenário 3): rola a prévia até o trecho do cursor.
  const locateBtn = button("Localizar na prévia", "btn sm ws-locate");
  locateBtn.title = "Mostrar na prévia o trecho onde está o cursor";
  locateBtn.disabled = true;
  editorHead.append(el("span", "ws-pane-title", "Editor"), editorPath, locateBtn);
  const editorBox = el("div", "ws-editor");
  editorCol.append(editorHead, editorBox);
  // Zoom do código (FR-013c): controles no cabeçalho; atalhos, roda e pinça só na área do editor.
  const editorZoom: EditorZoomHandle = mountEditorZoom({ header: editorHead, target: editorBox, key: null });
  const previewCol = el("div", "ws-col ws-col-preview");
  const previewBox = el("div", "ws-preview");
  previewCol.append(previewBox);
  body.append(treeCol, editorCol, previewCol);

  const status = el("footer", "ws-status");
  // A contagem de erros abre o painel "Erros" (FR-006a).
  const compileEl = button("Sem compilação", "ws-compile");
  compileEl.title = "Mostrar o painel Erros";
  const saveEl = el("span", "ws-save");
  const compilerEl = el("span", "ws-compiler");
  const appInfoEl = el("span", "ws-appinfo");
  appInfoEl.dataset.testid = "app-info";
  status.append(compileEl, saveEl, el("div", "ws-spacer"), compilerEl, appInfoEl);

  status.insertBefore(exporter.status, compilerEl.previousSibling);

  app.append(top, message, exporter.errors, banner, packageBanner, body, status);
  root.replaceChildren(app);

  // Painel "Erros" entre o corpo e a barra de status (FR-006a).
  const problems: ProblemsHandle = mountProblems(app, {
    onSelect: (d) => void revealDiagnostic(d),
    textOf: (p) => (editor?.path === p ? editor.view.state.doc.toString() : null),
  });
  app.insertBefore(problems.element, status);

  // ---------------------------------------------------------------- estado
  let info: ProjectInfo | null = null;
  let editor: EditorHandle | null = null;
  const panes: PanesHandle = mountPanes({
    body,
    tree: treeCol,
    editor: editorCol,
    preview: previewCol,
    treeHeader: treeHead,
    editorHeader: editorHead,
    key: null,
    // Editor que volta a aparecer ou muda de largura mede de novo as linhas.
    onChange: () => editor?.view.requestMeasure(),
  });
  topSpacer.after(panes.modeControl);
  let sync: SyncHandle | null = null;
  // "Compilar" (FR-014a): na barra da prévia; ⌘⇧B / Ctrl+Shift+B em qualquer lugar da janela.
  const compileNow: CompileNowHandle = mountCompileNow({
    ipc,
    flush: () => editor?.flush() ?? Promise.resolve(),
    onError: (err) => showError(err),
  });
  function attachPreview(p: PreviewHandle): PreviewHandle {
    p.bar.append(compileNow.button, panes.previewButton);
    // Clique na prévia → código; cursor → prévia (US4).
    sync?.destroy();
    sync = mountSync({
      ipc,
      preview: p,
      openAt: (pos) => openAt(pos),
      cursor: () => (editor ? { path: editor.path, offset: editor.view.state.selection.main.head } : null),
      flush: () => editor?.flush() ?? Promise.resolve(),
      notify: (text) => showMessage(text),
    });
    return p;
  }
  const showErrors = () => problems.open();
  let preview: PreviewHandle = attachPreview(
    mountPreview(previewBox, { ipc, urls: opts.previewUrls, onShowErrors: showErrors }),
  );
  let last: PreviewUpdate | null = null;
  /** Conta as `PreviewUpdate` recebidas: diz se alguma chegou durante a abertura de um projeto. */
  let updateSeq = 0;
  let opening = 0;
  const observers = new Set<(u: PreviewUpdate) => void>();
  const stops: (() => void)[] = [];

  function showMessage(text: string | null) {
    message.textContent = text ?? "";
    message.hidden = text === null;
  }
  const showError = (err: IpcError) => showMessage(err.message);

  function setEmptyEditor(text: string) {
    const p = el("div", "ws-empty");
    p.append(el("p", undefined, text));
    if (!info) {
      const b = button("Abrir projeto…", "btn primary");
      b.addEventListener("click", () => void openProject());
      const n = newProjectButton();
      n.addEventListener("click", () => void createProject());
      const actions = el("div", "ws-empty-actions");
      actions.append(b, n);
      p.append(actions);
    }
    editorBox.replaceChildren(p);
  }
  setEmptyEditor("Nenhum projeto aberto.");

  const conflictOpts = {
    ipc,
    parent: app,
    editorFor: (p: string) => (editor?.path === p ? editor : null),
    onError: (err: IpcError) => showError(err),
  };
  const resolveConflict = conflictResolver(conflictOpts);

  // "Recolher tudo"/"Expandir tudo" no cabeçalho da árvore; pastas recolhidas por projeto (FR-004a).
  const tree: TreeHandle = mountTree(side, { ipc, onOpen: (path) => void openFile(path), header: treeHead, key: null });
  // Estrutura do artigo abaixo dos arquivos, como no protótipo (FR-005).
  const outline: OutlineHandle = mountOutline(side, { ipc, onSelect: (item) => void goToSection(item) });

  const about: AboutHandle = mountAbout(app, {
    ipc,
    compiler: () => info?.compiler ?? null,
    onUpdateVersion: () => updateVersion(),
  });

  // ---------------------------------------------------------------- prévia e status
  function renderCompile(u: PreviewUpdate) {
    const errors = u.diagnostics.filter((d) => d.severity === "error").length;
    compileEl.classList.toggle("is-err", !u.ok);
    compileEl.classList.toggle("is-ok", u.ok);
    if (u.ok) {
      compileEl.textContent = `● Compilado em ${u.durationMs} ms`;
    } else {
      // Ícone de erro desenhado em SVG (sem `data:`), como na faixa da prévia e na margem.
      compileEl.replaceChildren(
        severityIcon("error"),
        errors === 0 ? " A compilação falhou" : ` ${errors} ${errors === 1 ? "erro" : "erros"} no documento`,
      );
    }
  }

  function onUpdate(u: PreviewUpdate) {
    updateSeq += 1;
    last = u;
    preview.update(u);
    editor?.setDiagnostics(u.diagnostics);
    problems.update(u.diagnostics);
    renderCompile(u);
    exporter.update(u);
    compileNow.update(u);
    packages.update(u.diagnostics);
    // A estrutura vem do último documento bom: só muda quando a compilação dá certo.
    if (u.ok) void outline.refresh();
    for (const cb of [...observers]) cb(u);
  }

  // ---------------------------------------------------------------- projeto
  function renderProject() {
    if (!info) return;
    const small = el("small", undefined, info.mainFile);
    projectEl.replaceChildren(el("strong", undefined, info.name), small);
    const st = compilerStatus(info.compiler);
    compilerEl.textContent = st.label;
    compilerEl.classList.toggle("is-warn", st.mismatch);
    exporter.setProject(info);
    // Sem compilação com versão diferente (FR-012): "Compilar" também fica desativado.
    compileNow.setAvailable(!info.warnings.includes(VERSION_MISMATCH));
    renderBanner();
  }

  function renderBanner() {
    const rows: HTMLElement[] = [];
    for (const w of info?.warnings ?? []) {
      const row = el("div", "ws-banner-row");
      if (w === VERSION_MISMATCH && info) {
        const { embedded, registered } = info.compiler;
        row.append(
          el(
            "span",
            undefined,
            `Este projeto foi registrado com o Typst ${registered ?? "?"}; o Prelo usa o ${embedded}. A prévia fica desativada até você atualizar o projeto.`,
          ),
        );
        const b = button(`Atualizar o projeto para Typst ${embedded}`, "btn sm primary");
        b.addEventListener("click", () => void updateVersion());
        row.append(b);
      } else if (w === MAIN_FILE_MISSING) {
        row.append(
          el(
            "span",
            undefined,
            `${errorMessage(w)} Abra um .typ na árvore e use-o como arquivo principal.`,
          ),
        );
        const b = button("Usar o arquivo aberto como principal", "btn sm primary");
        b.dataset.action = "choose-main";
        b.disabled = !(editor && /\.typ$/i.test(editor.path));
        b.addEventListener("click", () => void chooseMain());
        row.append(b);
      } else {
        row.append(el("span", undefined, errorMessage(w)));
      }
      rows.push(row);
    }
    banner.replaceChildren(...rows);
    banner.hidden = rows.length === 0;
  }

  function refreshChooseMain() {
    const b = banner.querySelector<HTMLButtonElement>("[data-action=choose-main]");
    if (b) b.disabled = !(editor && /\.typ$/i.test(editor.path));
  }

  async function updateVersion() {
    try {
      info = await ipc.updateProjectCompilerVersion();
      // O núcleo liga uma agenda nova: os `compileId` recomeçam.
      compileNow.reset();
      renderProject();
    } catch (err) {
      showError(toIpcError(err));
    }
  }

  async function chooseMain() {
    if (!editor) return;
    try {
      info = await ipc.chooseMainFile(editor.path);
      renderProject();
    } catch (err) {
      showError(toIpcError(err));
    }
  }

  /**
   * `since`: valor de `updateSeq` antes de pedir ao núcleo o projeto. O núcleo já compila o
   * projeto novo ao abri-lo, e um erro (sintaxe, pacote indisponível sem rede) volta em poucos
   * ms, antes do diálogo ou de `closeEditor`. Essa atualização é reaplicada depois da limpeza;
   * sem isso a barra ficava em "Compilando…" para sempre, com ERROS 0.
   */
  async function showProject(next: ProjectInfo, since = updateSeq) {
    const ticket = ++opening;
    await closeEditor();
    const early = updateSeq > since ? last : null;
    info = next;
    last = null;
    problems.update([]);
    showMessage(null);
    exporter.reset();
    compileNow.reset();
    outline.clear();
    // O núcleo recomeça a prévia ao abrir outro projeto: a interface também.
    preview.destroy();
    // O zoom da prévia é lembrado por projeto (FR-013a); `ProjectInfo` não traz o caminho da
    // raiz, então a chave é o nome da pasta.
    preview = attachPreview(
      mountPreview(previewBox, { ipc, urls: opts.previewUrls, zoomKey: next.name, onShowErrors: showErrors }),
    );
    // O arranjo dos painéis também é lembrado por projeto (FR-013b).
    panes.setKey(next.name);
    editorZoom.setKey(next.name);
    // Projeto novo, árvore limpa: sem campo de criação, erro, pasta escolhida ou arquivo do
    // projeto anterior; pastas recolhidas do projeto novo (FR-004a, FR-034).
    tree.resetForProject(next.name);
    compileEl.textContent = "Compilando…";
    compileEl.classList.remove("is-ok", "is-err");
    saveEl.textContent = "";
    saveBtn.disabled = true;
    if (early) onUpdate(early);
    renderProject();
    await tree.refresh();
    if (ticket !== opening) return;
    if (next.warnings.includes(MAIN_FILE_MISSING)) {
      setEmptyEditor("Escolha na árvore o arquivo a abrir.");
      return;
    }
    await openFile(next.mainFile);
  }

  async function openProject() {
    let next: ProjectInfo | null;
    const since = updateSeq;
    try {
      next = await ipc.openProjectDialog();
    } catch (err) {
      showError(toIpcError(err));
      return;
    }
    if (next) await showProject(next, since);
  }

  /** "Novo projeto" (US6): pasta não vazia → aviso `folder_not_empty`, nada muda. */
  async function createProject() {
    let next: ProjectInfo | null;
    const since = updateSeq;
    try {
      next = await ipc.createProjectDialog();
    } catch (err) {
      showError(toIpcError(err));
      return;
    }
    if (next) await showProject(next, since);
  }

  // ---------------------------------------------------------------- editor
  async function closeEditor() {
    if (!editor) return;
    const old = editor;
    editor = null;
    // As edições já digitadas chegam ao núcleo antes de trocar de arquivo.
    await old.flush();
    old.destroy();
  }

  async function reload(path: string) {
    try {
      const r = await ipc.openFile(path);
      if (editor?.path === path) editor.reload(r.text, r.version);
    } catch (err) {
      showError(toIpcError(err));
    }
  }

  /**
   * O núcleo recarregou do disco um arquivo sem alteração local (FR-008): o editor aberto com
   * esse caminho passa a mostrar o texto novo, sem diálogo. Árvore e status não mudam. Versão
   * que o editor já tem (evento repetido ou atrasado) não busca de novo.
   */
  async function externalReload(path: string, version: number) {
    if (editor?.path !== path || editor.version() >= version) return;
    try {
      const r = await ipc.openFile(path);
      const current = editor as EditorHandle | null;
      if (current?.path === path && r.version > current.version()) current.reload(r.text, r.version);
    } catch (err) {
      showError(toIpcError(err));
    }
  }

  async function openFile(path: string) {
    const ticket = ++opening;
    let opened;
    try {
      opened = await ipc.openFile(path);
    } catch (err) {
      showError(toIpcError(err));
      return;
    }
    if (ticket !== opening) return;
    await closeEditor();
    editorBox.replaceChildren();
    const handle = mountEditor(editorBox, {
      ipc,
      path,
      text: opened.text,
      version: opened.version,
      tokenDelayMs: opts.tokenDelayMs,
      onError: (err) => {
        if (err.code === CONFLICT) {
          void resolveConflict(path);
          return;
        }
        showError(err);
        if (RELOAD_CODES.has(err.code)) void reload(path);
      },
      onSaved: (savedAt) => {
        saveEl.textContent = `Salvo às ${clock(savedAt)}`;
      },
    });
    handle.view.dispatch({
      effects: StateEffect.appendConfig.of(
        EditorView.updateListener.of((u) => {
          // Só edições da pessoa; `reload` (texto do núcleo) não deixa nada pendente.
          if (u.docChanged && u.transactions.some((t) => USER_EVENTS.some((e) => t.isUserEvent(e)))) {
            saveEl.textContent = "Alterações pendentes · salvas automaticamente após 2 s parado";
          }
        }),
      ),
    });
    editor = handle;
    editorZoom.attach(handle.view);
    saveEl.textContent = "Sem alterações";
    saveBtn.disabled = false;
    tree.setCurrent(path);
    editorPath.textContent = path;
    if (last) handle.setDiagnostics(last.diagnostics);
    // Com o arquivo aberto, o painel "Erros" passa a mostrar a coluna dos diagnósticos dele.
    problems.refresh();
    refreshChooseMain();
    refreshLocate();
  }

  // ---------------------------------------------------------------- navegação (US4)
  /** Abre `pos.path` (se ainda não estiver aberto) com o cursor na linha e coluna dadas; caminho de pacote (`@…`) é ignorado. */
  async function openAt(pos: SourcePosition) {
    // Código de pacote baixado: o núcleo não abre (só arquivos do projeto); não faz nada.
    if (isPackagePath(pos.path)) return;
    if (editor?.path !== pos.path) await openFile(pos.path);
    const current = editor as EditorHandle | null;
    if (current?.path !== pos.path) return;
    const view = current.view;
    const anchor = offsetOf(view.state.doc.toString(), pos.line, pos.column);
    view.dispatch({
      selection: { anchor },
      effects: EditorView.scrollIntoView(anchor, { y: "center" }),
    });
    view.focus();
  }

  /** Item do painel "Erros": abre o arquivo e seleciona o trecho (ou o início da linha). */
  async function revealDiagnostic(d: Diagnostic) {
    if (!d.path || isPackagePath(d.path)) return;
    if (editor?.path !== d.path) await openFile(d.path);
    const current = editor as EditorHandle | null;
    if (current?.path !== d.path) return;
    const view = current.view;
    const doc = view.state.doc;
    const clamp = (n: number) => Math.min(Math.max(0, Math.trunc(n)), doc.length);
    let anchor: number;
    let head: number;
    if (d.range && Number.isFinite(d.range.from) && Number.isFinite(d.range.to)) {
      anchor = clamp(d.range.from);
      head = clamp(Math.max(d.range.from, d.range.to));
    } else if (typeof d.line === "number" && Number.isFinite(d.line)) {
      anchor = head = doc.line(Math.min(Math.max(1, Math.trunc(d.line)), doc.lines)).from;
    } else {
      return;
    }
    view.dispatch({
      selection: { anchor, head },
      effects: EditorView.scrollIntoView(anchor, { y: "center" }),
    });
    view.focus();
  }

  async function goToSection(item: OutlineItem) {
    sync?.reveal({ page: item.page, yPt: item.yPt });
    await openAt({ path: item.path, line: item.line, column: 1 });
  }

  function refreshLocate() {
    locateBtn.disabled = !(editor && /\.typ$/i.test(editor.path));
  }

  // ---------------------------------------------------------------- ações
  locateBtn.addEventListener("click", () => void sync?.locate());
  openBtn.addEventListener("click", () => void openProject());
  newBtn.addEventListener("click", () => void createProject());
  saveBtn.addEventListener("click", () => void editor?.save());
  aboutBtn.addEventListener("click", () => void about.open());
  compileEl.addEventListener("click", showErrors);

  const ready = (async () => {
    const appReady = mountAppInfo(appInfoEl, opts.appInfo);
    try {
      await ipc.subscribePreview(onUpdate);
    } catch (err) {
      showError(toIpcError(err));
    }
    // Sem permissão de eventos na webview, escutar falha: o resto da área segue funcionando.
    watchExternalChanges(conflictOpts, resolveConflict).then(
      (stop) => stops.push(stop),
      (err: unknown) => console.warn("conflito externo: sem escuta de eventos", err),
    );
    watchPluginApprovals({ ipc, parent: app, onError: (err) => showError(err) }).then(
      (stop) => stops.push(stop),
      (err: unknown) => console.warn("plugins: sem escuta de eventos", err),
    );
    // Arquivo fora do projeto (D-13): oferta de cópia; o núcleo troca o caminho no `.typ`.
    watchCopyOffers({
      ipc,
      parent: app,
      beforeCopy: async () => {
        await editor?.flush();
      },
      onCopied: (r) => {
        showMessage(`Arquivo copiado para ${r.path}.`);
        for (const u of r.updated) void externalReload(u.path, u.version);
        void tree.refresh();
      },
      onError: (err) => showError(err),
    }).then(
      (stop) => stops.push(stop),
      (err: unknown) => console.warn("cópia para o projeto: sem escuta de eventos", err),
    );
    ipc.onExternalChangeReloaded(({ path, version }) => void externalReload(path, version)).then(
      (stop) => stops.push(stop),
      (err: unknown) => console.warn("mudança externa: sem escuta de eventos", err),
    );
    await appReady;
  })();

  return {
    element: app,
    appInfoElement: appInfoEl,
    ready,
    showProject,
    openFile,
    editor: () => editor,
    project: () => info,
    compilerLabel: () => compilerEl.textContent ?? "",
    onPreviewUpdate(cb) {
      observers.add(cb);
      return () => observers.delete(cb);
    },
    destroy() {
      for (const s of stops) s();
      editor?.destroy();
      editor = null;
      sync?.destroy();
      compileNow.destroy();
      preview.destroy();
      panes.destroy();
      editorZoom.destroy();
      tree.destroy();
      outline.destroy();
      problems.destroy();
      app.remove();
    },
  };
}
