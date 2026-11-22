// Diagnósticos do núcleo no editor (T037; FR-006, cenário 4 de US1), pelo `@codemirror/lint`.
// O núcleo manda `Diagnostic` com `range` (deslocamentos UTF-16 no texto atual, como as
// edições) ou só `line` (1-based). Diagnóstico de outro arquivo ou sem posição não é marcado
// aqui; a faixa de erro da prévia e o painel de problemas mostram esses.
import { lintGutter, setDiagnostics, type Diagnostic as LintDiagnostic } from "@codemirror/lint";
import type { Extension, Text } from "@codemirror/state";
import type { EditorView } from "@codemirror/view";
import type { Diagnostic } from "../ipc";

function clamp(n: number, max: number): number {
  return Math.min(Math.max(0, Math.trunc(n)), max);
}

function position(d: Diagnostic, doc: Text): { from: number; to: number } | null {
  if (d.range && Number.isFinite(d.range.from) && Number.isFinite(d.range.to)) {
    const from = clamp(d.range.from, doc.length);
    const to = clamp(Math.max(d.range.from, d.range.to), doc.length);
    return { from, to };
  }
  if (typeof d.line === "number" && Number.isFinite(d.line)) {
    const line = doc.line(clamp(d.line, doc.lines) || 1);
    return { from: line.from, to: line.to };
  }
  return null;
}

/** Converte os diagnósticos do núcleo nos do CodeMirror, só os de `path`. */
export function toLintDiagnostics(diags: Diagnostic[], path: string, doc: Text): LintDiagnostic[] {
  const out: LintDiagnostic[] = [];
  for (const d of diags) {
    if (d.path !== path) continue;
    const pos = position(d, doc);
    if (!pos) continue;
    const message = [d.message, ...d.hints.map((h) => `Dica: ${h}`)].join("\n");
    out.push({
      from: pos.from,
      to: pos.to,
      severity: d.severity === "warning" ? "warning" : "error",
      source: d.kind === "typst" ? "Typst" : "Prelo",
      message,
    });
  }
  return out;
}

/** Troca os diagnósticos mostrados no editor pelos de `diags` (lista vazia limpa). */
export function applyDiagnostics(view: EditorView, diags: Diagnostic[], path: string): void {
  view.dispatch(setDiagnostics(view.state, toLintDiagnostics(diags, path, view.state.doc)));
}

/** Marcador na margem para as linhas com diagnóstico. */
export function diagnosticsExtension(): Extension {
  return lintGutter();
}
