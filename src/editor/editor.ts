// Editor de código (T037; FR-006, FR-007; research R3): CodeMirror 6 que envia cada edição ao
// núcleo como `{from,to,insert}` com a versão (contracts/ipc.md, `edit_file`), salva com
// ⌘S/Ctrl+S e mostra realce, autocompletar e diagnósticos vindos do núcleo.
//
// Mudanças: deslocamentos UTF-16 (os do CodeMirror) no texto anterior à transação, em ordem
// crescente e sem sobreposição (decisão do PR #174). As transações vão em fila: cada envio
// espera a resposta do anterior e usa a versão devolvida.
import { closeBrackets, closeBracketsKeymap, completionKeymap } from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { lintKeymap } from "@codemirror/lint";
import { highlightSelectionMatches, search, searchKeymap } from "@codemirror/search";
import { Annotation, EditorState, type Extension } from "@codemirror/state";
import {
  drawSelection,
  EditorView,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
} from "@codemirror/view";
import { toIpcError, type Change, type Diagnostic, type Ipc, type IpcError } from "../ipc";
import "../workspace.css";
import { completionExtension } from "./completion";
import { styleNonce } from "./csp";
import { applyDiagnostics, diagnosticsExtension } from "./diagnostics";
import { highlightExtension, refreshTokens } from "./highlight";

export interface EditorOptions {
  ipc: Ipc;
  /** Caminho relativo à raiz do projeto, como em `open_file`. */
  path: string;
  /** Texto e versão devolvidos por `open_file`. */
  text: string;
  version: number;
  /** Recusa do núcleo ao editar ou salvar (texto continua no editor). */
  onError?: (err: IpcError) => void;
  onSaved?: (savedAt: string) => void;
  /** Espera depois da última edição antes de pedir os tokens de realce (ms). */
  tokenDelayMs?: number;
}

export interface EditorHandle {
  readonly view: EditorView;
  readonly path: string;
  /** Versão do texto no núcleo depois da última edição confirmada. */
  version(): number;
  /** Espera todas as edições da fila chegarem ao núcleo. */
  flush(): Promise<void>;
  /** Envia as edições pendentes e chama `save_file`; `false` se o núcleo recusou. */
  save(): Promise<boolean>;
  /** Mostra os diagnósticos deste arquivo (vindos de `PreviewUpdate.diagnostics`). */
  setDiagnostics(diags: Diagnostic[]): void;
  /** Troca o texto sem enviar edição (ex.: `resolve_external_change`). */
  reload(text: string, version: number): void;
  destroy(): void;
}

/** Marca transações que não vêm da pessoa e não devem ir ao núcleo. */
const fromCore = Annotation.define<boolean>();

const isTypst = (path: string) => /\.typ$/i.test(path);

export function mountEditor(parent: HTMLElement, opts: EditorOptions): EditorHandle {
  const { ipc, path } = opts;
  const tokenDelay = opts.tokenDelayMs ?? 150;
  let version = opts.version;
  let chain: Promise<void> = Promise.resolve();
  // Depois de uma recusa de edição o núcleo e o editor divergem; nada mais é enviado até
  // `reload` (a mensagem de `version_mismatch` pede para recarregar o arquivo).
  let desynced = false;
  let tokensAvailable = isTypst(path);
  let tokenTimer: ReturnType<typeof setTimeout> | undefined;
  let destroyed = false;
  // `reload` troca o texto inteiro: edições enfileiradas antes dele não valem mais.
  let generation = 0;

  const report = (err: unknown) => opts.onError?.(toIpcError(err));

  function scheduleTokens() {
    if (!tokensAvailable || destroyed) return;
    clearTimeout(tokenTimer);
    tokenTimer = setTimeout(() => {
      if (destroyed) return;
      // Os tokens valem para o texto que o núcleo tem: envia o que falta antes de pedir.
      void flush()
        .then(() => (destroyed ? "discarded" : refreshTokens(view, ipc, path)))
        .then((r) => {
          if (r === "unavailable") tokensAvailable = false;
        });
    }, tokenDelay);
  }

  function enqueue(changes: Change[]) {
    const gen = generation;
    chain = chain.then(async () => {
      if (desynced || destroyed || gen !== generation) return;
      try {
        const r = await ipc.editFile(path, version, changes);
        if (gen !== generation) return;
        version = r.version;
        scheduleTokens();
      } catch (err) {
        if (gen !== generation) return;
        desynced = true;
        report(err);
      }
    });
  }

  async function flush() {
    let current: Promise<void>;
    do {
      current = chain;
      await current;
    } while (current !== chain);
  }

  async function save(): Promise<boolean> {
    await flush();
    if (desynced) {
      report("version_mismatch");
      return false;
    }
    try {
      const r = await ipc.saveFile(path);
      opts.onSaved?.(r.savedAt);
      return true;
    } catch (err) {
      report(err);
      return false;
    }
  }

  const sendChanges = EditorView.updateListener.of((update) => {
    for (const tr of update.transactions) {
      if (!tr.docChanged || tr.annotation(fromCore)) continue;
      const changes: Change[] = [];
      tr.changes.iterChanges((fromA, toA, _fromB, _toB, inserted) => {
        changes.push({ from: fromA, to: toA, insert: inserted.toString() });
      });
      if (changes.length > 0) enqueue(changes);
    }
  });

  const saveCommand = () => {
    void save();
    return true;
  };

  const extensions: Extension[] = [
    lineNumbers(),
    highlightActiveLineGutter(),
    history(),
    drawSelection(),
    highlightActiveLine(),
    closeBrackets(),
    search(),
    highlightSelectionMatches(),
    EditorView.lineWrapping,
    EditorView.contentAttributes.of({ "aria-label": `Código-fonte: ${path}` }),
    keymap.of([
      // ⌘S no macOS e Ctrl+S nos demais; Meta+S também em Windows/Linux por segurança.
      { key: "Mod-s", run: saveCommand, preventDefault: true },
      { key: "Meta-s", run: saveCommand, preventDefault: true },
      ...closeBracketsKeymap,
      ...defaultKeymap,
      ...searchKeymap,
      ...historyKeymap,
      ...completionKeymap,
      ...lintKeymap,
      indentWithTab,
    ]),
    diagnosticsExtension(),
    highlightExtension(),
    sendChanges,
  ];
  if (isTypst(path)) extensions.push(completionExtension({ ipc, path, flush }));
  // Sem o nonce a CSP bloqueia o tema base do CodeMirror (ver ./csp.ts).
  const nonce = styleNonce(parent.ownerDocument);
  if (nonce) extensions.push(EditorView.cspNonce.of(nonce));

  const view = new EditorView({
    parent,
    state: EditorState.create({ doc: opts.text, extensions }),
  });
  view.dom.classList.add("prelo-editor");
  scheduleTokens();

  return {
    view,
    path,
    version: () => version,
    flush,
    save,
    setDiagnostics: (diags) => applyDiagnostics(view, diags, path),
    reload(text, newVersion) {
      generation++;
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: text },
        annotations: fromCore.of(true),
      });
      version = newVersion;
      desynced = false;
      scheduleTokens();
    },
    destroy() {
      destroyed = true;
      clearTimeout(tokenTimer);
      view.destroy();
    },
  };
}
