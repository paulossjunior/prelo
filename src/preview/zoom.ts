// Regras do zoom da prévia (T091; FR-013a): degraus, limites, atalhos, âncora de leitura e
// memória por projeto. Sem DOM próprio: a barra e a aplicação ficam em preview.ts.

/** Degraus de "−"/"+" e dos atalhos, em %. A roda e a pinça podem parar entre eles. */
export const ZOOM_STEPS = [50, 67, 75, 90, 100, 110, 125, 150, 175, 200, 250, 300] as const;
export const MIN_ZOOM = 50;
export const MAX_ZOOM = 300;

/** 100 % = tamanho real: 1 pt = 1/72 pol. e 1 px CSS = 1/96 pol. */
export const PX_PER_PT = 96 / 72;

/** "fit" = ajustar à largura (padrão); número = porcentagem fixa. */
export type ZoomLevel = "fit" | number;

/** Folga para não "andar" zero degraus por causa de arredondamento (ex.: 99,8 % → 110 %). */
const TOL = 0.5;

export function clampZoom(pct: number): number {
  if (!Number.isFinite(pct)) return 100;
  return Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, pct));
}

export function stepIn(current: number): number {
  return ZOOM_STEPS.find((s) => s > current + TOL) ?? MAX_ZOOM;
}

export function stepOut(current: number): number {
  return [...ZOOM_STEPS].reverse().find((s) => s < current - TOL) ?? MIN_ZOOM;
}

/**
 * ⌘ (macOS) ou Ctrl (outros sistemas) + `+`/`=`, `-`/`_` e `0`. Aceita os dois modificadores em
 * qualquer sistema: quem chama só escuta eventos vindos da prévia, então nenhum atalho do editor
 * é tomado.
 */
export function zoomShortcut(e: KeyboardEvent): "in" | "out" | "fit" | null {
  if (!(e.metaKey || e.ctrlKey) || e.altKey) return null;
  switch (e.key) {
    case "+":
    case "=":
      return "in";
    case "-":
    case "_":
      return "out";
    case "0":
      return "fit";
    default:
      return null;
  }
}

export interface Box {
  top: number;
  left: number;
  width: number;
  height: number;
}

/**
 * Rolagem que deixa o mesmo ponto da página (mesma fração da largura e da altura) sob o ponto
 * `at` da tela da prévia, depois que a página mudou de `before` para `after` (coordenadas do
 * conteúdo rolável).
 */
export function anchorScroll(
  before: Box,
  after: Box,
  at: { x: number; y: number },
  scroll: { top: number; left: number },
): { top: number; left: number } {
  const fy = before.height > 0 ? (scroll.top + at.y - before.top) / before.height : 0;
  const fx = before.width > 0 ? (scroll.left + at.x - before.left) / before.width : 0;
  return {
    top: Math.max(0, after.top + fy * after.height - at.y),
    left: Math.max(0, after.left + fx * after.width - at.x),
  };
}

export type ZoomStorage = Pick<Storage, "getItem" | "setItem">;

const PREFIX = "prelo.preview.zoom:";

function defaultStorage(): ZoomStorage | null {
  try {
    return globalThis.localStorage ?? null;
  } catch {
    return null;
  }
}

/**
 * Nível guardado sob `prefix + key` (também usado pelo zoom do editor, FR-013c): número, "fit",
 * ou `null` sem chave, sem armazenamento, sem valor ou com valor inválido.
 */
export function loadLevel(
  prefix: string,
  key: string | null | undefined,
  storage: ZoomStorage | null = defaultStorage(),
): number | "fit" | null {
  if (!key || !storage) return null;
  try {
    const v = storage.getItem(prefix + key);
    if (v === null) return null;
    if (v === "fit") return "fit";
    const n = Number(v);
    return Number.isFinite(n) && v.trim() !== "" ? n : null;
  } catch {
    return null;
  }
}

/** Grava o nível sob `prefix + key`; falha do armazenamento (modo privado, cota) é ignorada. */
export function saveLevel(
  prefix: string,
  key: string | null | undefined,
  level: number | "fit",
  storage: ZoomStorage | null = defaultStorage(),
): void {
  if (!key || !storage) return;
  try {
    storage.setItem(prefix + key, level === "fit" ? "fit" : String(Math.round(level * 10) / 10));
  } catch {
    // O zoom continua valendo nesta sessão; só não é lembrado.
  }
}

/** Nível da prévia guardado para o projeto; sem valor ou inválido → "fit". */
export function loadZoom(key: string | null | undefined, storage: ZoomStorage | null = defaultStorage()): ZoomLevel {
  const v = loadLevel(PREFIX, key, storage);
  return v === null || v === "fit" ? "fit" : clampZoom(v);
}

/** Grava o nível da prévia do projeto. */
export function saveZoom(
  key: string | null | undefined,
  level: ZoomLevel,
  storage: ZoomStorage | null = defaultStorage(),
): void {
  saveLevel(PREFIX, key, level, storage);
}
