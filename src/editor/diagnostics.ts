// Diagnósticos do núcleo no editor (T037; FR-006, cenário 4 de US1), pelo `@codemirror/lint`.
// O núcleo manda `Diagnostic` com `range` (deslocamentos UTF-16 no texto atual, como as
// edições) ou só `line` (1-based). Diagnóstico de outro arquivo ou sem posição não é marcado
// aqui; a faixa de erro da prévia e o painel "Erros" (src/editor/problems.ts) mostram esses.
import { forEachDiagnostic, setDiagnostics, type Diagnostic as LintDiagnostic } from "@codemirror/lint";
import { RangeSet, type EditorState, type Extension, type Text } from "@codemirror/state";
import { gutter, GutterMarker, type EditorView } from "@codemirror/view";
import type { Diagnostic } from "../ipc";
import { severityIcon, type Severity } from "./icons";
import { explanationNode } from "./problems";

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
      // Tooltip ao passar o ponteiro no sublinhado: a mesma explicação do painel "Erros"
      // (FR-006a). Só DOM com `textContent`; os estilos vêm de workspace.css e o tema do lint
      // entra pelo nonce de estilo (EditorView.cspNonce, PR #187).
      renderMessage: () => explanationNode(d),
    });
  }
  return out;
}

/** Troca os diagnósticos mostrados no editor pelos de `diags` (lista vazia limpa). */
export function applyDiagnostics(view: EditorView, diags: Diagnostic[], path: string): void {
  view.dispatch(setDiagnostics(view.state, toLintDiagnostics(diags, path, view.state.doc)));
}

const RANK: Record<string, number> = { hint: 0, info: 1, warning: 2, error: 3 };

/** Marcador da margem: ícone SVG da gravidade mais alta da linha, com a mensagem no `title`. */
class SeverityMarker extends GutterMarker {
  constructor(
    readonly severity: Severity,
    readonly title: string,
  ) {
    super();
  }
  eq(other: SeverityMarker): boolean {
    return other.severity === this.severity && other.title === this.title;
  }
  toDOM(): Node {
    return severityIcon(this.severity, this.title);
  }
}

function markers(state: EditorState): RangeSet<GutterMarker> {
  const byLine = new Map<number, { rank: number; severity: Severity; titles: string[] }>();
  forEachDiagnostic(state, (d, from) => {
    const line = state.doc.lineAt(Math.min(from, state.doc.length)).from;
    const severity: Severity = d.severity === "error" || d.severity === "warning" ? d.severity : "info";
    const rank = RANK[d.severity] ?? 1;
    const title = d.message.split("\n")[0] ?? "";
    const cur = byLine.get(line);
    if (!cur) byLine.set(line, { rank, severity, titles: [title] });
    else {
      cur.titles.push(title);
      if (rank > cur.rank) Object.assign(cur, { rank, severity });
    }
  });
  const ranges = [...byLine.entries()]
    .sort(([a], [b]) => a - b)
    .map(([pos, m]) => new SeverityMarker(m.severity, m.titles.join("\n")).range(pos));
  return RangeSet.of(ranges);
}

/**
 * Margem com o ícone de cada linha com diagnóstico. Substitui o `lintGutter()` do
 * @codemirror/lint: o tema dele desenha o marcador com uma imagem `data:` na propriedade
 * `content` do próprio elemento, que a CSP (`img-src` sem `data:`) bloqueia; o PR #187 dependia
 * de sobrescrever isso por CSS, e o dono viu no app real um quadrado vazio no lugar do marcador.
 * Aqui o ícone é SVG montado pelo DOM, sem nada a sobrescrever, e o autoteste confere no app
 * real que ele aparece em vermelho (src/layout-check.ts, `lintMarkerProblems`).
 */
export function diagnosticsExtension(): Extension {
  return gutter({
    class: "prelo-lint-gutter",
    markers: (view) => markers(view.state),
  });
}
