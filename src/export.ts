// Ação "Exportar PDF" (T069; US5; FR-012, FR-028, FR-029; contracts/ipc.md "Exportação").
// O núcleo confere o projeto, mostra o diálogo nativo de salvar e grava o PDF da última prévia
// `Ok`; a interface só chama `export_pdf_dialog` (sem argumentos: o caminho nunca passa por
// aqui) e mostra o resultado: onde gravou, ou a lista de erros que impedem a compilação.
import { errorMessage, toIpcError, type Diagnostic, type Ipc, type IpcError, type PreviewUpdate, type ProjectInfo } from "./ipc";

const COMPILE_ERRORS = "compile_errors";
const VERSION_MISMATCH = "compiler_version_mismatch";

export interface ExportOptions {
  ipc: Ipc;
  /** Erro do núcleo que não é `compile_errors` (mostrado na mensagem da área de trabalho). */
  onError(err: IpcError): void;
}

export interface ExportHandle {
  /** Botão "Exportar PDF" (barra de cima). */
  readonly button: HTMLButtonElement;
  /** Lista de erros da recusa (abaixo da barra). */
  readonly errors: HTMLElement;
  /** Onde o último PDF foi gravado (barra de status). */
  readonly status: HTMLElement;
  /** Projeto aberto (ou nenhum), e a cada mudança dos avisos dele. */
  setProject(info: ProjectInfo | null): void;
  /** Outro projeto aberto: a prévia recomeça, então a exportação espera a nova compilação. */
  reset(): void;
  /** Cada `PreviewUpdate`: libera a ação e, sem erro, tira a lista da recusa anterior. */
  update(u: PreviewUpdate): void;
  run(): Promise<void>;
}

/** "arquivo:linha: mensagem" (só o que houver). */
export function describeDiagnostic(d: Diagnostic): string {
  const where = d.path ? (d.line !== undefined ? `${d.path}:${d.line}` : d.path) : "";
  return where ? `${where}: ${d.message}` : d.message;
}

export function mountExport(opts: ExportOptions): ExportHandle {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "btn primary";
  button.textContent = "Exportar PDF";

  const errors = document.createElement("div");
  errors.className = "ws-message ws-export-errors";
  errors.setAttribute("role", "alert");
  errors.hidden = true;

  const status = document.createElement("span");
  status.className = "ws-export-status";
  status.setAttribute("role", "status");

  let info: ProjectInfo | null = null;
  let compiled = false;
  let busy = false;

  function refresh() {
    const mismatch = info?.warnings.includes(VERSION_MISMATCH) ?? false;
    button.disabled = busy || !info || mismatch || !compiled;
    button.title = !info
      ? "Abra um projeto para exportar o PDF."
      : mismatch
        ? "Atualize a versão do Typst do projeto para exportar o PDF."
        : !compiled
          ? "Aguardando a primeira compilação."
          : "Exportar o PDF do documento";
  }

  function showErrors(list: Diagnostic[] | null) {
    if (!list) {
      errors.replaceChildren();
      errors.hidden = true;
      return;
    }
    const title = document.createElement("strong");
    title.textContent = errorMessage(COMPILE_ERRORS);
    const ul = document.createElement("ul");
    for (const d of list.filter((d) => d.severity === "error")) {
      const li = document.createElement("li");
      li.textContent = describeDiagnostic(d);
      ul.append(li);
    }
    errors.replaceChildren(title, ul);
    errors.hidden = false;
  }

  async function run() {
    if (button.disabled) return;
    busy = true;
    refresh();
    status.textContent = "";
    try {
      const result = await opts.ipc.exportPdfDialog();
      showErrors(null);
      if (result) status.textContent = `PDF exportado em ${result.path}`;
    } catch (raw) {
      const err = toIpcError(raw);
      if (err.code === COMPILE_ERRORS) {
        showErrors(err.diagnostics ?? []);
      } else {
        showErrors(null);
        opts.onError(err);
      }
    } finally {
      busy = false;
      refresh();
    }
  }

  button.addEventListener("click", () => void run());
  refresh();

  return {
    button,
    errors,
    status,
    setProject(next) {
      info = next;
      refresh();
    },
    reset() {
      compiled = false;
      status.textContent = "";
      showErrors(null);
      refresh();
    },
    update(u) {
      compiled = compiled || u.ok || u.pages.length > 0;
      if (u.ok) showErrors(null);
      refresh();
    },
    run,
  };
}
