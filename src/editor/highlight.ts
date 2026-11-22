// Realce de sintaxe pelos tokens do núcleo (T037; FR-006; research R3, D-17): o mesmo
// analisador do compilador (`typst-syntax`), aplicado como marcas com classe `tok-<tipo>`.
// Formato esperado de T036: `{ from, to, kind }` em deslocamentos UTF-16 do texto atual.
// Enquanto o serviço de linguagem não existe (`not_implemented`), o editor segue sem realce.
import { StateEffect, StateField, type Extension, type Text } from "@codemirror/state";
import { Decoration, EditorView, type DecorationSet } from "@codemirror/view";
import { toIpcError, type Ipc, type Token } from "../ipc";

export interface HighlightToken {
  from: number;
  to: number;
  kind: string;
}

function num(v: unknown): number | null {
  return typeof v === "number" && Number.isInteger(v) ? v : null;
}

/** Nome de classe seguro: só letras minúsculas, dígitos e hífen. */
function cleanKind(v: unknown): string | null {
  if (typeof v !== "string") return null;
  const k = v
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return k.length > 0 ? k : null;
}

/** Valida os tokens do núcleo: descarta intervalo vazio, invertido ou fora do texto. */
export function normalizeTokens(raw: Token[], length: number): HighlightToken[] {
  const out: HighlightToken[] = [];
  for (const t of raw) {
    const from = num(t.from);
    const to = num(t.to);
    const kind = cleanKind(t.kind);
    if (from === null || to === null || kind === null) continue;
    if (from < 0 || to <= from || to > length) continue;
    out.push({ from, to, kind });
  }
  return out;
}

const setTokens = StateEffect.define<HighlightToken[]>();

function build(tokens: HighlightToken[]): DecorationSet {
  const sorted = [...tokens].sort((a, b) => a.from - b.from || a.to - b.to);
  return Decoration.set(
    sorted.map((t) => Decoration.mark({ class: `tok-${t.kind}` }).range(t.from, t.to)),
  );
}

const tokenField = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(deco, tr) {
    let next = deco.map(tr.changes);
    for (const e of tr.effects) if (e.is(setTokens)) next = build(e.value);
    return next;
  },
  provide: (f) => EditorView.decorations.from(f),
});

export function highlightExtension(): Extension {
  return tokenField;
}

export type RefreshResult = "applied" | "discarded" | "unavailable" | "failed";

/**
 * Pede os tokens ao núcleo e aplica. Se o texto mudou enquanto o pedido estava em curso, a
 * resposta é descartada (outro pedido virá depois da próxima edição).
 */
export async function refreshTokens(view: EditorView, ipc: Ipc, path: string): Promise<RefreshResult> {
  const doc: Text = view.state.doc;
  let raw: Token[];
  try {
    raw = await ipc.semanticTokens(path);
  } catch (err) {
    return toIpcError(err).code === "not_implemented" ? "unavailable" : "failed";
  }
  if (view.state.doc !== doc) return "discarded";
  if (!Array.isArray(raw)) return "failed";
  view.dispatch({ effects: setTokens.of(normalizeTokens(raw, doc.length)) });
  return "applied";
}
