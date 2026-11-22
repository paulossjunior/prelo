// Área de trabalho do editor local (T039; US1): barra com o projeto e as ações (Abrir projeto,
// Salvar, Sobre), árvore | editor | prévia, e a barra de status com a compilação, o estado de
// gravação, a versão do Typst e a do Prelo. Layout e linguagem visual de prototype/index.html.
//
// Fiação (PR #176): uma assinatura de `subscribe_preview` alimenta a prévia e os diagnósticos
// do editor; a árvore abre arquivos (`open_file` → `mountEditor` → `setCurrent`); edição
// recusada pelo núcleo recarrega o arquivo.
import { StateEffect } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { compilerStatus, mountAbout, type AboutHandle } from "./about";
import { mountAppInfo, type Invoke } from "./app-info";
import { mountEditor, type EditorHandle } from "./editor/editor";
import { errorMessage, toIpcError, type Ipc, type IpcError, type PreviewUpdate, type ProjectInfo } from "./ipc";
import { mountPreview, type BlobUrls, type PreviewHandle } from "./preview/preview";
import { conflictResolver, watchExternalChanges } from "./project/prompts";
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
  const saveBtn = button("Salvar");
  saveBtn.title = "Salvar (⌘S / Ctrl+S)";
  saveBtn.disabled = true;
  const aboutBtn = button("Sobre");
  top.append(projectEl, el("div", "ws-spacer"), openBtn, saveBtn, aboutBtn);

  const message = el("div", "ws-message");
  message.setAttribute("role", "status");
  message.hidden = true;
  const banner = el("div", "ws-banner");
  banner.hidden = true;

  const body = el("div", "ws-body");
  const side = el("aside", "ws-side");
  side.append(el("h4", undefined, "Arquivos"));
  const editorBox = el("div", "ws-editor");
  const previewBox = el("div", "ws-preview");
  body.append(side, editorBox, previewBox);

  const status = el("footer", "ws-status");
  const compileEl = el("span", "ws-compile", "Sem compilação");
  const saveEl = el("span", "ws-save");
  const compilerEl = el("span", "ws-compiler");
  const appInfoEl = el("span", "ws-appinfo");
  appInfoEl.dataset.testid = "app-info";
  status.append(compileEl, saveEl, el("div", "ws-spacer"), compilerEl, appInfoEl);

  app.append(top, message, banner, body, status);
  root.replaceChildren(app);

  // ---------------------------------------------------------------- estado
  let info: ProjectInfo | null = null;
  let editor: EditorHandle | null = null;
  let preview: PreviewHandle = mountPreview(previewBox, { ipc, urls: opts.previewUrls });
  let last: PreviewUpdate | null = null;
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
      p.append(b);
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

  const tree: TreeHandle = mountTree(side, { ipc, onOpen: (path) => void openFile(path) });

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
      compileEl.textContent =
        errors === 0 ? "● A compilação falhou" : `● ${errors} ${errors === 1 ? "erro" : "erros"} no documento`;
    }
  }

  function onUpdate(u: PreviewUpdate) {
    last = u;
    preview.update(u);
    editor?.setDiagnostics(u.diagnostics);
    renderCompile(u);
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

  async function showProject(next: ProjectInfo) {
    const ticket = ++opening;
    await closeEditor();
    info = next;
    last = null;
    showMessage(null);
    // O núcleo recomeça a prévia ao abrir outro projeto: a interface também.
    preview.destroy();
    // O zoom da prévia é lembrado por projeto (FR-013a); `ProjectInfo` não traz o caminho da
    // raiz, então a chave é o nome da pasta.
    preview = mountPreview(previewBox, { ipc, urls: opts.previewUrls, zoomKey: next.name });
    compileEl.textContent = "Compilando…";
    compileEl.classList.remove("is-ok", "is-err");
    saveEl.textContent = "";
    saveBtn.disabled = true;
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
    try {
      next = await ipc.openProjectDialog();
    } catch (err) {
      showError(toIpcError(err));
      return;
    }
    if (next) await showProject(next);
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
    saveEl.textContent = "Sem alterações";
    saveBtn.disabled = false;
    tree.setCurrent(path);
    if (last) handle.setDiagnostics(last.diagnostics);
    refreshChooseMain();
  }

  // ---------------------------------------------------------------- ações
  openBtn.addEventListener("click", () => void openProject());
  saveBtn.addEventListener("click", () => void editor?.save());
  aboutBtn.addEventListener("click", () => void about.open());

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
      preview.destroy();
      tree.destroy();
      app.remove();
    },
  };
}
