// Testes das regras do zoom da prévia (T090; FR-013a): degraus e limites, atalhos de teclado,
// âncora de leitura e memória por projeto.
import { describe, expect, it } from "vitest";
import {
  MAX_ZOOM,
  MIN_ZOOM,
  ZOOM_STEPS,
  anchorScroll,
  clampZoom,
  loadZoom,
  saveZoom,
  stepIn,
  stepOut,
  zoomShortcut,
} from "./zoom";

describe("degraus e limites", () => {
  it("vai de 50 % a 300 % nos degraus combinados", () => {
    expect(ZOOM_STEPS).toEqual([50, 67, 75, 90, 100, 110, 125, 150, 175, 200, 250, 300]);
    expect(MIN_ZOOM).toBe(50);
    expect(MAX_ZOOM).toBe(300);
  });

  it("'+' vai ao próximo degrau acima e para em 300 %", () => {
    expect(stepIn(100)).toBe(110);
    expect(stepIn(103)).toBe(110);
    expect(stepIn(250)).toBe(300);
    expect(stepIn(300)).toBe(300);
    expect(stepIn(42)).toBe(50);
  });

  it("'−' vai ao degrau abaixo e para em 50 %", () => {
    expect(stepOut(100)).toBe(90);
    expect(stepOut(103)).toBe(100);
    expect(stepOut(67)).toBe(50);
    expect(stepOut(50)).toBe(50);
    expect(stepOut(400)).toBe(300);
  });

  it("valor de roda ou pinça fica dentro dos limites", () => {
    expect(clampZoom(10)).toBe(50);
    expect(clampZoom(999)).toBe(300);
    expect(clampZoom(133.3)).toBeCloseTo(133.3);
    expect(clampZoom(Number.NaN)).toBe(100);
  });
});

describe("atalhos (só com o foco na prévia)", () => {
  const key = (k: string, mods: Partial<KeyboardEventInit> = {}) => new KeyboardEvent("keydown", { key: k, ...mods });

  it("⌘ ou Ctrl com +, =, -, 0", () => {
    expect(zoomShortcut(key("+", { metaKey: true }))).toBe("in");
    expect(zoomShortcut(key("=", { ctrlKey: true }))).toBe("in");
    expect(zoomShortcut(key("-", { metaKey: true }))).toBe("out");
    expect(zoomShortcut(key("_", { ctrlKey: true, shiftKey: true }))).toBe("out");
    expect(zoomShortcut(key("0", { ctrlKey: true }))).toBe("fit");
  });

  it("sem modificador, com Alt, ou outra tecla: não é atalho de zoom", () => {
    expect(zoomShortcut(key("+"))).toBeNull();
    expect(zoomShortcut(key("0", { metaKey: true, altKey: true }))).toBeNull();
    expect(zoomShortcut(key("s", { metaKey: true }))).toBeNull();
  });
});

describe("âncora de leitura", () => {
  it("mantém o mesmo ponto da página sob o ponto da tela", () => {
    const before = { top: 1000, left: 20, width: 400, height: 600 };
    const after = { top: 2000, left: 20, width: 800, height: 1200 };
    // Ponto da tela (250, 300) com rolagem 1100/0: documento y = 1400, 2/3 da página... (400/600).
    const s = anchorScroll(before, after, { x: 250, y: 300 }, { top: 1100, left: 0 });
    const fy = (1100 + 300 - before.top) / before.height;
    const fx = (0 + 250 - before.left) / before.width;
    expect(s.top).toBeCloseTo(after.top + fy * after.height - 300);
    expect(s.left).toBeCloseTo(after.left + fx * after.width - 250);
  });

  it("não devolve rolagem negativa", () => {
    const s = anchorScroll(
      { top: 18, left: 18, width: 400, height: 600 },
      { top: 18, left: 18, width: 200, height: 300 },
      { x: 300, y: 300 },
      { top: 0, left: 0 },
    );
    expect(s.top).toBeGreaterThanOrEqual(0);
    expect(s.left).toBeGreaterThanOrEqual(0);
  });
});

describe("memória por projeto", () => {
  function memory() {
    const m = new Map<string, string>();
    return {
      getItem: (k: string) => m.get(k) ?? null,
      setItem: (k: string, v: string) => void m.set(k, v),
      map: m,
    };
  }

  it("grava e lê o nível por chave de projeto; padrão é ajustar à largura", () => {
    const st = memory();
    expect(loadZoom("artigo", st)).toBe("fit");
    saveZoom("artigo", 150, st);
    saveZoom("outro", "fit", st);
    expect(loadZoom("artigo", st)).toBe(150);
    expect(loadZoom("outro", st)).toBe("fit");
  });

  it("valor inválido guardado vira o padrão; valor fora dos limites é limitado", () => {
    const st = memory();
    saveZoom("a", 150, st);
    const k = [...st.map.keys()].find((x) => x.includes("a"))!;
    st.setItem(k, "banana");
    expect(loadZoom("a", st)).toBe("fit");
    st.setItem(k, "900");
    expect(loadZoom("a", st)).toBe(300);
  });

  it("armazenamento que lança erro não quebra nada", () => {
    const broken = {
      getItem: () => {
        throw new Error("SecurityError");
      },
      setItem: () => {
        throw new Error("QuotaExceededError");
      },
    };
    expect(loadZoom("a", broken)).toBe("fit");
    expect(() => saveZoom("a", 200, broken)).not.toThrow();
  });

  it("sem chave de projeto não lê nem grava", () => {
    const st = memory();
    saveZoom(null, 200, st);
    expect(st.map.size).toBe(0);
    expect(loadZoom(null, st)).toBe("fit");
  });
});
