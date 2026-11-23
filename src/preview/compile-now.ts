// Botão "Compilar" e atalho ⌘⇧B / Ctrl+Shift+B (T101; FR-014a, cenário 14 de US1;
// contracts/ipc.md `compile_now`). Fica na barra da prévia, no estilo dos botões de `.seg` do
// protótipo.
//
// Fluxo: envia as edições pendentes do editor (`flush`), pede `compile_now` e fica em
// "Compilando…", desativado, até chegar uma `PreviewUpdate` com `compileId` igual ou maior que
// o devolvido (uma edição durante a espera pode substituir a compilação pedida na fila de 1 do
// núcleo; qualquer prévia mais nova também responde). A prévia pode chegar antes da resposta de
// `compile_now`, por isso o maior `compileId` visto fica guardado.
//
// Ícones em SVG montados pelo DOM com `currentColor` (sem `data:`, que a CSP `img-src` bloqueia,
// e sem atributo `style`, que `style-src` bloqueia); o giro do arco é CSS em workspace.css,
// parado com `prefers-reduced-motion`. O atalho escuta o documento: o mapa de teclas do
// CodeMirror não usa Mod-Shift-b, então vale também com o foco no editor.
import { toIpcError, type Ipc, type IpcError, type PreviewUpdate } from "../ipc";

export const COMPILE_NOW_LABEL = "Compilar agora (⌘⇧B / Ctrl+Shift+B)";
const BUSY_LABEL = "Compilando…";

export interface CompileNowOptions {
  ipc: Ipc;
  /** Espera as edições do editor chegarem ao núcleo. */
  flush: () => Promise<void>;
  onError: (err: IpcError) => void;
  /** Onde escutar o atalho (padrão: o documento). */
  keyTarget?: EventTarget;
}

export interface CompileNowHandle {
  readonly button: HTMLButtonElement;
  /** Projeto aberto e compilável (sem versão diferente, FR-012). */
  setAvailable(on: boolean): void;
  /** Cada `PreviewUpdate`: libera o botão quando chega a compilação pedida. */
  update(u: PreviewUpdate): void;
  /** Compila agora (o que o clique e o atalho fazem); nada se indisponível ou já esperando. */
  run(): Promise<void>;
  /** Projeto aberto de novo ou agenda recriada: os `compileId` recomeçam. */
  reset(): void;
  destroy(): void;
}

const NS = "http://www.w3.org/2000/svg";

function svgNode(tag: string, attrs: Record<string, string | number>): SVGElement {
  const e = document.createElementNS(NS, tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, String(v));
  return e;
}

function icon(busy: boolean): SVGElement {
  const svg = svgNode("svg", {
    viewBox: "0 0 16 16",
    width: 14,
    height: 14,
    focusable: "false",
    "aria-hidden": "true",
    class: busy ? "cn-icon cn-spin" : "cn-icon",
  });
  if (busy) {
    // Arco de progresso: três quartos de círculo.
    svg.append(
      svgNode("path", {
        d: "M8 2.5a5.5 5.5 0 1 1-5.5 5.5",
        fill: "none",
        stroke: "currentColor",
        "stroke-width": 1.8,
        "stroke-linecap": "round",
      }),
    );
  } else {
    // "Play" de compilar.
    svg.append(svgNode("path", { d: "M5 3.2v9.6L12.8 8Z", fill: "currentColor", "stroke-linejoin": "round" }));
  }
  return svg;
}

/** ⌘⇧B no macOS, Ctrl+Shift+B nos demais (as duas valem em qualquer sistema). */
export function isCompileShortcut(e: KeyboardEvent): boolean {
  if (!(e.metaKey || e.ctrlKey) || !e.shiftKey || e.altKey) return false;
  return e.code === "KeyB" || e.key.toLowerCase() === "b";
}

export function mountCompileNow(opts: CompileNowOptions): CompileNowHandle {
  const { ipc } = opts;
  const button = document.createElement("button");
  button.type = "button";
  button.className = "pv-compile";
  button.setAttribute("aria-label", COMPILE_NOW_LABEL);
  button.title = COMPILE_NOW_LABEL;

  let available = false;
  /** `compileId` esperado; `0` enquanto a resposta de `compile_now` não chegou. */
  let waiting: number | null = null;
  /** Maior `compileId` visto numa prévia. */
  let seen = 0;

  function render() {
    const busy = waiting !== null;
    const label = document.createElement("span");
    label.className = "pv-compile-label";
    label.textContent = busy ? BUSY_LABEL : "Compilar";
    button.replaceChildren(icon(busy), label);
    button.setAttribute("aria-busy", String(busy));
    button.disabled = !available || busy;
  }

  function done() {
    waiting = null;
    render();
  }

  async function run() {
    if (!available || waiting !== null) return;
    waiting = 0;
    render();
    try {
      await opts.flush();
      const { compileId } = await ipc.compileNow();
      if (waiting === null) return;
      if (seen >= compileId) done();
      else waiting = compileId;
    } catch (err) {
      done();
      opts.onError(toIpcError(err));
    }
  }

  const onKey = (e: Event) => {
    const k = e as KeyboardEvent;
    if (!isCompileShortcut(k)) return;
    k.preventDefault();
    void run();
  };
  const keyTarget = opts.keyTarget ?? document;
  keyTarget.addEventListener("keydown", onKey);
  button.addEventListener("click", () => void run());
  render();

  return {
    button,
    setAvailable(on) {
      available = on;
      if (!on) waiting = null;
      render();
    },
    update(u) {
      seen = Math.max(seen, u.compileId);
      if (waiting !== null && waiting !== 0 && u.compileId >= waiting) done();
    },
    run,
    reset() {
      seen = 0;
      waiting = null;
      render();
    },
    destroy() {
      keyTarget.removeEventListener("keydown", onKey);
      button.remove();
    },
  };
}
