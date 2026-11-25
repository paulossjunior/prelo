// Autocompletar pelo núcleo (T037; FR-006, FR-011; research R3): a fonte do
// `@codemirror/autocomplete` chama `complete` no mesmo World da prévia, depois de enviar as
// edições pendentes (o núcleo precisa ver o texto ainda não salvo). Formato esperado de T036:
// `{ label, kind?, detail?, info?, apply? }`. Enquanto o comando responde `not_implemented`,
// a fonte para de perguntar e o editor segue sem sugestões.
import {
  autocompletion,
  type Completion as CmCompletion,
  type CompletionContext,
  type CompletionResult,
  type CompletionSource,
} from "@codemirror/autocomplete";
import type { Extension } from "@codemirror/state";
import { toIpcError, type Completion, type Ipc } from "../ipc";

export interface CompletionOptions {
  ipc: Ipc;
  path: string;
  /** Espera as edições pendentes chegarem ao núcleo. */
  flush: () => Promise<void>;
}

const str = (v: unknown): string | undefined => (typeof v === "string" && v.length > 0 ? v : undefined);

/** Converte um item do núcleo; item sem rótulo é descartado. */
export function toCompletion(raw: Completion): CmCompletion | null {
  const label = str(raw.label);
  if (!label) return null;
  const out: CmCompletion = { label };
  const type = str(raw.kind);
  const detail = str(raw.detail);
  const info = str(raw.info);
  const apply = str(raw.apply);
  if (type) out.type = type;
  if (detail) out.detail = detail;
  if (info) out.info = info;
  if (apply) out.apply = apply;
  return out;
}

// Palavra em curso: o identificador Typst (letras, dígitos, `_`, `-`) depois do último `.`.
// O `.` fica fora: em `calc.ab`, escolher `abs` troca só `ab` (bug do PR #179).
const WORD = /[\p{L}\p{N}_-]+$/u;
// Depois destes caracteres o Typst tem o que sugerir mesmo sem palavra (`#`, `@`, `<`, e `.`
// para campos e métodos).
const TRIGGER = /[#@<.]$/;
// `validFor` é testado contra o texto de `from` até o cursor: ancorado nos dois lados, um `.`
// digitado depois invalida a lista e pede de novo ao núcleo.
const VALID_FOR = /^[\p{L}\p{N}_-]*$/u;

export function completionSource(opts: CompletionOptions): CompletionSource {
  let available = true;
  return async (context: CompletionContext): Promise<CompletionResult | null> => {
    if (!available) return null;
    const word = context.matchBefore(WORD);
    const before = context.state.sliceDoc(Math.max(0, context.pos - 1), context.pos);
    if (!context.explicit && !word && !TRIGGER.test(before)) return null;
    await opts.flush();
    if (context.aborted) return null;
    let raw: Completion[];
    try {
      raw = await opts.ipc.complete(opts.path, context.pos);
    } catch (err) {
      if (toIpcError(err).code === "not_implemented") available = false;
      return null;
    }
    if (!Array.isArray(raw)) return null;
    const options = raw.map(toCompletion).filter((c): c is CmCompletion => c !== null);
    if (options.length === 0) return null;
    return { from: word ? word.from : context.pos, options, validFor: VALID_FOR };
  };
}

export function completionExtension(opts: CompletionOptions): Extension {
  return autocompletion({ override: [completionSource(opts)] });
}
