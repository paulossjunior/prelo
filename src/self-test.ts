// Autoteste ponta a ponta (T012/T020 e T031): só roda em build com VITE_SELF_TEST definido.
// Além da versão do app (001), percorre US1 no app real: abre a fixture `artigo-basico`
// (cópia temporária feita pelo núcleo), edita, espera a prévia mudar, introduz um erro e
// espera o diagnóstico marcado no editor; depois grava o arquivo aberto "por fora" (sem
// alteração local) e espera o editor mostrar o texto do disco com o diagnóstico na linha
// certa (FR-008, T084). Reporta ao núcleo o que observou; o núcleo confere,
// inclusive que não há `typst` no PATH (SC-006), e encerra o app
// (contracts/ipc.md, `self_test_report`).
import { diagnosticCount } from "@codemirror/lint";
import { invoke } from "@tauri-apps/api/core";
import type { PreviewUpdate, ProjectInfo } from "./ipc";
import { layoutProblems } from "./layout-check";
import type { WorkspaceHandle } from "./workspace";

/** Chamada a algo que não existe: o Typst acusa erro nesta linha. */
export const ERROR_SNIPPET = "#funcao-inexistente-do-autoteste()";
const NEW_TEXT = "Parágrafo novo escrito pelo autoteste do Prelo.";

export interface ScenarioResult {
  projectOpened: boolean;
  previewChanged: boolean;
  /** Linha (1-based) onde o erro foi escrito. */
  errorLine: number;
  /** Linha do diagnóstico de erro do arquivo principal, se veio. */
  diagnosticLine: number | null;
  editorMarked: boolean;
  compilerShown: string;
  /** O que a conferência de layout achou no app real (vazio em jsdom, sem conferência). */
  layoutProblems: string[];
  /** O editor passou a mostrar o texto gravado por fora (sem alteração local). */
  externalReloaded: boolean;
  /** Linha (1-based) do erro no texto gravado por fora. */
  externalErrorLine: number;
  /** Linha do diagnóstico de erro no editor recarregado, se veio. */
  externalDiagnosticLine: number | null;
  failure?: string;
}

export interface ScenarioOptions {
  /** Espera máxima por cada compilação. */
  timeoutMs?: number;
  /** Mede o layout na webview real (src/layout-check.ts); ausente em jsdom. */
  checkLayout?: () => string[];
  /** Grava o arquivo principal como outro programa faria (`self_test_write_external`). */
  writeExternal?: (text: string) => Promise<void>;
}

function waitFor(
  ws: WorkspaceHandle,
  what: string,
  pred: (u: PreviewUpdate) => boolean,
  timeoutMs: number,
): Promise<PreviewUpdate> {
  return new Promise((resolve, reject) => {
    const stop = ws.onPreviewUpdate((u) => {
      if (!pred(u)) return;
      clearTimeout(timer);
      stop();
      resolve(u);
    });
    const timer = setTimeout(() => {
      stop();
      reject(new Error(`a prévia não ${what} em ${timeoutMs} ms`));
    }, timeoutMs);
  });
}

const hashes = (u: PreviewUpdate) => u.pages.map((p) => p.hash).join(",");

/** Espera `pred` ficar verdadeiro, conferindo a cada 50 ms. */
async function until(what: string, pred: () => boolean, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (!pred()) {
    if (Date.now() > deadline) throw new Error(`${what} em ${timeoutMs} ms`);
    await new Promise((r) => setTimeout(r, 50));
  }
}

/** Texto que "outro programa" grava: bem mais longo que o aberto, com o erro no fim. */
export function externalText(): { text: string; errorLine: number } {
  const lines = Array.from({ length: 130 }, (_, i) => `Linha ${i + 1} gravada por outro programa.`);
  lines.push(ERROR_SNIPPET, "");
  return { text: lines.join("\n"), errorLine: 131 };
}

/** Percorre US1 sobre a área de trabalho montada; nunca lança: falha vai em `failure`. */
export async function runEditorScenario(
  ws: WorkspaceHandle,
  open: () => Promise<ProjectInfo>,
  opts: ScenarioOptions = {},
): Promise<ScenarioResult> {
  const timeoutMs = opts.timeoutMs ?? 15_000;
  const r: ScenarioResult = {
    projectOpened: false,
    previewChanged: false,
    errorLine: 0,
    diagnosticLine: null,
    editorMarked: false,
    compilerShown: "",
    layoutProblems: [],
    externalReloaded: false,
    externalErrorLine: 0,
    externalDiagnosticLine: null,
  };
  try {
    const first = waitFor(ws, "compilou ao abrir o projeto", (u) => u.ok && u.pages.length > 0, timeoutMs);
    const info = await open();
    await ws.showProject(info);
    const editor = ws.editor();
    r.projectOpened = editor?.path === info.mainFile;
    r.compilerShown = ws.compilerLabel();
    if (!editor) throw new Error("o arquivo principal não abriu no editor");
    const before = hashes(await first);

    // Cenário 2: digitar muda a prévia sem ação adicional.
    const changed = waitFor(ws, "mudou depois da edição", (u) => u.ok && hashes(u) !== before, timeoutMs);
    const view = editor.view;
    view.dispatch({
      changes: { from: view.state.doc.length, insert: `\n\n${NEW_TEXT}\n` },
      userEvent: "input.type",
    });
    await changed;
    r.previewChanged = true;

    // Cenário 4: erro marcado no editor, na linha certa.
    const failed = waitFor(
      ws,
      "acusou o erro introduzido",
      (u) => !u.ok && u.diagnostics.some((d) => d.severity === "error" && d.path === info.mainFile),
      timeoutMs,
    );
    const at = view.state.doc.length;
    view.dispatch({ changes: { from: at, insert: `\n${ERROR_SNIPPET}\n` }, userEvent: "input.type" });
    r.errorLine = view.state.doc.lineAt(at + 1).number;
    const u = await failed;
    const d = u.diagnostics.find((x) => x.severity === "error" && x.path === info.mainFile)!;
    if (d.range) r.diagnosticLine = view.state.doc.lineAt(Math.min(d.range.from, view.state.doc.length)).number;
    else if (typeof d.line === "number") r.diagnosticLine = d.line;
    r.editorMarked = diagnosticCount(view.state) > 0;

    // FR-008: outro programa grava o arquivo aberto sem alteração local → o editor mostra o
    // texto do disco (sem diálogo) e o diagnóstico cai numa linha que o editor tem.
    if (opts.writeExternal) {
      if (!(await editor.save())) throw new Error("não salvou antes da mudança externa");
      let latest: PreviewUpdate | null = null;
      const stop = ws.onPreviewUpdate((u) => (latest = u));
      try {
        const ext = externalText();
        r.externalErrorLine = ext.errorLine;
        await opts.writeExternal(ext.text);
        const shown = () => ws.editor()?.view.state.doc.toString() === ext.text;
        await until("o editor não mostrou o texto gravado no disco", shown, timeoutMs);
        r.externalReloaded = true;
        const diagLine = (): number | null => {
          const v = ws.editor()?.view;
          const d = (latest as PreviewUpdate | null)?.diagnostics.find(
            (x) => x.severity === "error" && x.path === info.mainFile && x.range,
          );
          if (!v || !d?.range || !shown()) return null;
          return v.state.doc.lineAt(Math.min(d.range.from, v.state.doc.length)).number;
        };
        await until(
          "o diagnóstico do texto do disco não apareceu na linha do erro",
          () => (r.externalDiagnosticLine = diagLine()) === ext.errorLine,
          timeoutMs,
        );
      } finally {
        stop();
      }
    }

    // Layout no app real: texto mais alto que o editor e uma linha mais larga que ele, para
    // conferir gutter em coluna, rolagem vertical e quebra da linha longa.
    if (opts.checkLayout) {
      const filler = Array.from({ length: 160 }, (_, i) => `Linha de enchimento ${i + 1}.`).join("\n");
      const longLine = "palavra ".repeat(60);
      view.dispatch({
        changes: { from: view.state.doc.length, insert: `\n${longLine}\n${filler}\n` },
        userEvent: "input.type",
      });
      await nextFrame();
      r.layoutProblems = opts.checkLayout();
    }
  } catch (err) {
    r.failure = err instanceof Error ? err.message : String(err);
  }
  return r;
}

const nextFrame = () =>
  new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));

/** Roda o autoteste no app real e entrega o resultado ao núcleo. */
export async function reportSelfTest(
  ws: WorkspaceHandle,
  cspViolations: () => string[] = () => [],
): Promise<void> {
  if (!import.meta.env.VITE_SELF_TEST) return;
  await ws.ready;
  // SC-002: versão e sistema visíveis em até 3 s desde o início da página.
  const elapsedMs = Math.round(performance.now());
  const rendered = ws.appInfoElement.textContent?.trim() ?? "";
  const scenario = await runEditorScenario(ws, () => invoke<ProjectInfo>("self_test_open_project"), {
    checkLayout: () => [...cspViolations(), ...layoutProblems(document)],
    writeExternal: (text) => invoke("self_test_write_external", { text }),
  });
  await invoke("self_test_report", { rendered, elapsedMs, scenario });
}

/** Build de e2e (WebDriver): abre a fixture logo ao iniciar, sem o diálogo nativo. */
export async function openE2eFixture(ws: WorkspaceHandle): Promise<void> {
  if (!import.meta.env.VITE_E2E) return;
  await ws.ready;
  await ws.showProject(await invoke<ProjectInfo>("self_test_open_project"));
}
