// Diálogo de conflito com mudança externa (T039; FR-008; contracts/ipc.md): o núcleo emite
// `external_change_conflict` quando outro programa muda um arquivo que tem alterações ainda não
// salvas no Prelo. Nada é descartado até a pessoa escolher; a escolha vai em
// `resolve_external_change` e o editor recarrega com o texto devolvido. Diálogo na própria
// página: `alert`/`confirm` são recusados na webview.
//
// Também os pedidos de plugin e de pacote (T049; FR-018, FR-019): `plugin_approval_needed`
// abre a confirmação com caminho, origem e tamanho, e a decisão vai em `decide_plugin`; o
// diagnóstico de pacote adulterado ganha a ação "Baixar de novo" (`redownload_package`).
//
// E a oferta de cópia para o projeto (T059; D-13, FR-023): `copy_into_project_offer` traz só o
// caminho pedido no `.typ`; a pessoa decide, e "Escolher arquivo…" chama `copy_into_project`
// com `{ requested }`. O arquivo de origem é escolhido no diálogo nativo do núcleo: a webview
// nunca manda caminho de fora do projeto.
import {
  toIpcError,
  type CopiedIntoProject,
  type Diagnostic,
  type Ipc,
  type IpcError,
  type PluginApprovalNeeded,
} from "../ipc";

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

// ---------------------------------------------------------------- plugins (FR-019)

/** Tamanho legível: bytes até 1 KB, depois KB/MB com uma casa quando preciso. */
export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} bytes`;
  const units = ["KB", "MB", "GB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  const text = Number.isInteger(value) ? String(value) : value.toFixed(1).replace(".", ",");
  return `${text} ${units[unit]}`;
}

/** Pergunta se o plugin pode rodar neste projeto; só resolve com uma escolha. */
export function askPluginApproval(parent: HTMLElement, req: PluginApprovalNeeded): Promise<boolean> {
  return new Promise((resolve) => {
    const backdrop = document.createElement("div");
    backdrop.className = "ws-backdrop";
    const dialog = document.createElement("div");
    dialog.className = "ws-modal";
    dialog.setAttribute("role", "alertdialog");
    dialog.setAttribute("aria-modal", "true");
    const title = document.createElement("h2");
    title.id = `ws-plugin-${Math.random().toString(36).slice(2)}`;
    title.textContent = "Executar plugin?";
    dialog.setAttribute("aria-labelledby", title.id);

    const body = document.createElement("p");
    const code = document.createElement("code");
    code.textContent = req.path;
    body.append("O documento quer executar o plugin ", code, " ");
    if (req.origin === "package" && req.package) {
      const pkg = document.createElement("code");
      pkg.textContent = req.package;
      body.append("do pacote ", pkg);
    } else {
      body.append("do projeto");
    }
    body.append(` (${formatSize(req.sizeBytes)}).`);
    const note = document.createElement("p");
    note.textContent =
      "Plugins são programas. Permita só se você confia na origem. A decisão fica guardada neste computador, para este projeto e este conteúdo.";

    const actions = document.createElement("div");
    actions.className = "ws-modal-actions";
    const choice = (label: string, approve: boolean, cls: string) => {
      const b = document.createElement("button");
      b.type = "button";
      b.className = cls;
      b.textContent = label;
      b.addEventListener("click", () => {
        backdrop.remove();
        resolve(approve);
      });
      return b;
    };
    const refuse = choice("Não permitir", false, "btn");
    actions.append(choice("Permitir neste projeto", true, "btn primary"), refuse);
    dialog.addEventListener("keydown", (e) => {
      if (e.key === "Escape") e.preventDefault();
    });
    dialog.append(title, body, note, actions);
    backdrop.append(dialog);
    parent.append(backdrop);
    // Foco na escolha mais segura.
    refuse.focus();
  });
}

const SHA256 = /^[0-9a-f]{64}$/;

function isPluginRequest(e: unknown): e is PluginApprovalNeeded {
  if (typeof e !== "object" || e === null) return false;
  const o = e as Record<string, unknown>;
  return (
    typeof o.path === "string" &&
    (o.origin === "project" || o.origin === "package") &&
    (o.package === undefined || typeof o.package === "string") &&
    typeof o.sizeBytes === "number" &&
    typeof o.sha256 === "string" &&
    SHA256.test(o.sha256)
  );
}

export interface PluginOptions {
  ipc: Ipc;
  parent: HTMLElement;
  onError?: (err: IpcError) => void;
}

/** Escuta `plugin_approval_needed`: um pedido por plugin de cada vez; devolve quem para. */
export function watchPluginApprovals(opts: PluginOptions): Promise<() => void> {
  const pending = new Set<string>();
  return opts.ipc.onPluginApprovalNeeded((e: unknown) => {
    if (!isPluginRequest(e) || pending.has(e.sha256)) return;
    pending.add(e.sha256);
    void (async () => {
      try {
        const approve = await askPluginApproval(opts.parent, e);
        await opts.ipc.decidePlugin(e.sha256, approve);
      } catch (err) {
        opts.onError?.(toIpcError(err));
      } finally {
        pending.delete(e.sha256);
      }
    })();
  });
}

// ---------------------------------------------------------------- pacotes (FR-018)

// Mensagem do contrato (contracts/file-access.md, `PacoteAdulterado`).
const TAMPERED = /^o pacote `(@preview\/[A-Za-z0-9_-]+:\d+\.\d+\.\d+)` foi alterado depois do download$/;

/** `@preview/<nome>:<versão>` do diagnóstico de pacote adulterado, ou `null`. */
export function tamperedPackage(d: Diagnostic): string | null {
  if (d.kind !== "package") return null;
  return TAMPERED.exec(d.message)?.[1] ?? null;
}

export interface PackageActionsOptions {
  ipc: Ipc;
  onError?: (err: IpcError) => void;
}

export interface PackageActions {
  /** Mostra uma linha com "Baixar de novo" por pacote adulterado nos diagnósticos. */
  update(diagnostics: Diagnostic[]): void;
}

/** Faixa com a ação "Baixar de novo" para pacotes adulterados; escondida sem nenhum. */
export function mountPackageActions(box: HTMLElement, opts: PackageActionsOptions): PackageActions {
  box.hidden = true;
  const busy = new Set<string>();
  return {
    update(diagnostics) {
      const specs = [...new Set(diagnostics.map(tamperedPackage).filter((s): s is string => s !== null))];
      const rows = specs.map((spec) => {
        const row = document.createElement("div");
        row.className = "ws-banner-row";
        const text = document.createElement("span");
        const code = document.createElement("code");
        code.textContent = spec;
        text.append("O pacote ", code, " foi alterado depois do download e não foi usado.");
        const b = document.createElement("button");
        b.type = "button";
        b.className = "btn";
        b.textContent = "Baixar de novo";
        b.disabled = busy.has(spec);
        b.addEventListener("click", async () => {
          busy.add(spec);
          b.disabled = true;
          try {
            await opts.ipc.redownloadPackage(spec);
          } catch (err) {
            opts.onError?.(toIpcError(err));
          } finally {
            busy.delete(spec);
            b.disabled = false;
          }
        });
        row.append(text, b);
        return row;
      });
      box.replaceChildren(...rows);
      box.hidden = rows.length === 0;
    },
  };
}

// ---------------------------------------------------------------- cópia para o projeto (D-13)

/** Pergunta se a pessoa quer trazer o arquivo para o projeto; `true` = escolher o arquivo. */
export function askCopyIntoProject(parent: HTMLElement, requested: string): Promise<boolean> {
  return new Promise((resolve) => {
    const backdrop = document.createElement("div");
    backdrop.className = "ws-backdrop";
    const dialog = document.createElement("div");
    dialog.className = "ws-modal";
    dialog.setAttribute("role", "alertdialog");
    dialog.setAttribute("aria-modal", "true");
    const title = document.createElement("h2");
    title.id = `ws-copy-${Math.random().toString(36).slice(2)}`;
    title.textContent = "Trazer o arquivo para o projeto?";
    dialog.setAttribute("aria-labelledby", title.id);

    const body = document.createElement("p");
    const code = document.createElement("code");
    code.textContent = requested;
    body.append(
      "O documento pede ",
      code,
      ", que fica fora da pasta do projeto. O Prelo só lê arquivos de dentro do projeto, para que ele compile igual no computador de todos os coautores.",
    );
    const note = document.createElement("p");
    note.textContent =
      "Você pode escolher esse arquivo para o Prelo copiar para dentro do projeto, ao lado do .typ que o pede; o caminho no .typ passa a apontar para a cópia. Nada é sobrescrito.";

    const actions = document.createElement("div");
    actions.className = "ws-modal-actions";
    const choice = (label: string, copy: boolean, cls: string) => {
      const b = document.createElement("button");
      b.type = "button";
      b.className = cls;
      b.textContent = label;
      b.addEventListener("click", () => {
        backdrop.remove();
        resolve(copy);
      });
      return b;
    };
    const copy = choice("Escolher arquivo para copiar…", true, "btn primary");
    actions.append(copy, choice("Agora não", false, "btn"));
    dialog.addEventListener("keydown", (e) => {
      if (e.key === "Escape") e.preventDefault();
    });
    dialog.append(title, body, note, actions);
    backdrop.append(dialog);
    parent.append(backdrop);
    copy.focus();
  });
}

function isCopyOffer(e: unknown): e is { requested: string } {
  if (typeof e !== "object" || e === null) return false;
  const requested = (e as Record<string, unknown>).requested;
  return typeof requested === "string" && requested.length > 0;
}

export interface CopyOfferOptions {
  ipc: Ipc;
  parent: HTMLElement;
  /** Antes de copiar: o texto digitado chega ao núcleo (ele pode trocar o caminho no `.typ`). */
  beforeCopy?: () => Promise<void>;
  /** A cópia foi feita; `updated` lista os arquivos cujo texto o núcleo mudou. */
  onCopied?: (result: CopiedIntoProject) => void;
  onError?: (err: IpcError) => void;
}

/** Escuta `copy_into_project_offer`: uma pergunta por caminho de cada vez; devolve quem para. */
export function watchCopyOffers(opts: CopyOfferOptions): Promise<() => void> {
  const pending = new Set<string>();
  return opts.ipc.onCopyIntoProjectOffer((e: unknown) => {
    if (!isCopyOffer(e) || pending.has(e.requested)) return;
    const { requested } = e;
    pending.add(requested);
    void (async () => {
      try {
        if (!(await askCopyIntoProject(opts.parent, requested))) return;
        await opts.beforeCopy?.();
        const result = await opts.ipc.copyIntoProject(requested);
        if (result) opts.onCopied?.(result);
      } catch (err) {
        opts.onError?.(toIpcError(err));
      } finally {
        pending.delete(requested);
      }
    })();
  });
}
