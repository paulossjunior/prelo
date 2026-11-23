// Regras do teste de encerramento forçado (T077; SC-009): o roteiro em scripts/self-test.mjs
// mata o app 2 s depois da última edição e reabre; no máximo os últimos 2 s se perdem.
import { describe, expect, it } from "vitest";
import {
  KILL_AFTER_MS,
  MAX_LOSS_MS,
  checkReopened,
  markText,
  parseMark,
  parseReopened,
  type Mark,
} from "./forced-kill.mjs";

const original = "= Artigo\n\nTexto.\n";
/** Marcas a cada 250 ms, de 0 a 3 s; morte 2 s depois da última. */
const marks: Mark[] = Array.from({ length: 13 }, (_, i) => ({ n: i + 1, at: i * 250 }));
const last = marks[marks.length - 1]!.at;
const killedAt = last + KILL_AFTER_MS;
const withMarks = (upTo: number) =>
  original + marks.filter((m) => m.n <= upTo).map((m) => `\n${markText(m.n)}`).join("");

describe("forced-kill: leitura do stderr", () => {
  it("lê a marca e o texto reaberto", () => {
    expect(parseMark("self-test: marca 7")).toBe(7);
    expect(parseMark("self-test: marca 7\r")).toBe(7);
    expect(parseMark("self-test: marca x")).toBeNull();
    expect(parseMark("outra coisa")).toBeNull();
    expect(parseReopened(`self-test: reaberto ${JSON.stringify("a\nb")}`)).toBe("a\nb");
    expect(parseReopened("self-test: reaberto {nada")).toBeNull();
    expect(parseReopened("self-test: reaberto 42")).toBeNull();
  });

  it("morre 2 s depois da última edição e tolera no máximo 2 s de perda", () => {
    expect(KILL_AFTER_MS).toBe(2000);
    expect(MAX_LOSS_MS).toBe(2000);
  });
});

describe("forced-kill: conferência do texto reaberto", () => {
  it("aceita tudo gravado", () => {
    expect(checkReopened({ marks, killedAt, original, reopened: withMarks(13) })).toEqual([]);
  });

  it("aceita perder só o que foi digitado nos últimos 2 s", () => {
    // Morte 1 s depois da última: as marcas dos últimos 2 s podem faltar.
    const early = last + 1000;
    const kept = marks.filter((m) => early - m.at >= MAX_LOSS_MS).length;
    expect(kept).toBeGreaterThan(0);
    expect(checkReopened({ marks, killedAt: early, original, reopened: withMarks(kept) })).toEqual([]);
  });

  it("recusa perder edição de mais de 2 s antes da morte", () => {
    const failures = checkReopened({ marks, killedAt, original, reopened: withMarks(12) });
    expect(failures).toHaveLength(1);
    expect(failures[0]).toContain("marca-13");
    // A regra antiga (2 s ociosos) não gravava nada enquanto se digitava.
    expect(checkReopened({ marks, killedAt, original, reopened: original })).toHaveLength(13);
  });

  it("marca-1 não casa com marca-10", () => {
    const only10 = original + "\nmarca-10";
    const two = marks.slice(0, 10);
    const failures = checkReopened({ marks: two, killedAt: 99_999, original, reopened: only10 });
    expect(failures.join()).toContain("marca-1 ");
  });

  it("recusa buraco nas marcas e texto original corrompido", () => {
    const hole = withMarks(13).replace("\nmarca-5\n", "\n");
    expect(checkReopened({ marks, killedAt, original, reopened: hole }).join()).toContain("buraco");
    expect(checkReopened({ marks, killedAt, original, reopened: "lixo" }).join()).toContain("original");
  });

  it("recusa roteiro sem marcas ou sem texto reaberto", () => {
    expect(checkReopened({ marks: [], killedAt, original, reopened: original })).toHaveLength(1);
    expect(checkReopened({ marks, killedAt, original, reopened: null })).toHaveLength(1);
  });
});
