// Diálogo de conflito com mudança externa (T039; FR-008; contracts/ipc.md): o núcleo emite
// `external_change_conflict` quando outro programa muda um arquivo que tem alterações ainda não
// salvas no Prelo. Nada é descartado até a pessoa escolher; a escolha vai em
// `resolve_external_change` e o editor recarrega com o texto devolvido. Diálogo na própria
// página: `alert`/`confirm` são recusados na webview.
import { toIpcError, type Ipc, type IpcError } from "../ipc";

export type Keep = "mine" | "disk";

/** Pergunta qual versão manter; só resolve com uma escolha (Escape não decide). */
export function askExternalChange(parent: HTMLElement, path: string): Promise<Keep> {
  return new Promise((resolve) => {
    const backdrop = document.createElement("div");
    backdrop.className = "ws-backdrop";
    const dialog = document.createElement("div");
    dialog.className = "ws-modal";
    dialog.setAttribute("role", "alertdialog");
    dialog.setAttribute("aria-modal", "true");
    const title = document.createElement("h2");
    title.id = `ws-conflict-${Math.random().toString(36).slice(2)}`;
    title.textContent = "O arquivo mudou fora do Prelo";
    dialog.setAttribute("aria-labelledby", title.id);
    const body = document.createElement("p");
    const code = document.createElement("code");
    code.textContent = path;
    body.append(
      code,
      " foi alterado por outro programa, e há alterações suas ainda não salvas nele. Qual versão manter? A outra será descartada.",
    );
    const actions = document.createElement("div");
    actions.className = "ws-modal-actions";
    const choice = (label: string, keep: Keep, cls: string) => {
      const b = document.createElement("button");
      b.type = "button";
      b.className = cls;
      b.textContent = label;
      b.addEventListener("click", () => {
        backdrop.remove();
        resolve(keep);
      });
      return b;
    };
    const mine = choice("Manter a minha versão", "mine", "btn primary");
    actions.append(mine, choice("Usar a versão do disco", "disk", "btn"));
    dialog.addEventListener("keydown", (e) => {
      if (e.key === "Escape") e.preventDefault();
    });
    dialog.append(title, body, actions);
    backdrop.append(dialog);
    parent.append(backdrop);
    mine.focus();
  });
}

/** O que o diálogo precisa do editor aberto. */
export interface ConflictEditor {
  readonly path: string;
  flush(): Promise<void>;
  reload(text: string, version: number): void;
}

export interface ConflictOptions {
  ipc: Ipc;
  parent: HTMLElement;
  /** Editor aberto com este caminho, se houver. */
  editorFor: (path: string) => ConflictEditor | null;
  onError?: (err: IpcError) => void;
}

/**
 * Pergunta e resolve o conflito de `path`: um só diálogo por arquivo de cada vez. Usado pelo
 * evento e quando `save_file` responde `external_change_conflict`.
 */
export function conflictResolver(opts: ConflictOptions): (path: string) => Promise<void> {
  const pending = new Set<string>();
  return async (path: string) => {
    if (typeof path !== "string" || pending.has(path)) return;
    pending.add(path);
    try {
      const keep = await askExternalChange(opts.parent, path);
      const editor = opts.editorFor(path);
      // O núcleo precisa ter todo o texto da pessoa antes de decidir.
      await editor?.flush();
      const r = await opts.ipc.resolveExternalChange(path, keep);
      opts.editorFor(path)?.reload(r.text, r.version);
    } catch (err) {
      opts.onError?.(toIpcError(err));
    } finally {
      pending.delete(path);
    }
  };
}

/** Escuta `external_change_conflict`; devolve a função que para de escutar. */
export function watchExternalChanges(
  opts: ConflictOptions,
  resolve: (path: string) => Promise<void> = conflictResolver(opts),
): Promise<() => void> {
  return opts.ipc.onExternalChangeConflict(({ path }) => void resolve(path));
}
