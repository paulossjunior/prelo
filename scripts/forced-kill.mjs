// Encerramento forçado (T077; SC-009, Princípio III): regras do roteiro de
// scripts/self-test.mjs, separadas para teste (scripts/forced-kill.test.ts).
//
// O app, em modo `PRELO_SELF_TEST_CRASH=write`, digita marcas `marca-<n>` (uma por linha) no
// arquivo principal e avisa cada uma no stderr (`self-test: marca <n>`) só depois que o núcleo
// já tem o texto.
// O roteiro mata o processo KILL_AFTER_MS depois da última marca, reabre o app em modo
// `reopen` e recebe o texto que o editor mostrou (`self-test: reaberto <json>`). Toda marca
// avisada MAX_LOSS_MS ou mais antes da morte tem de estar no texto reaberto.

/** Espera entre a última edição e a morte do processo (tasks.md T077). */
export const KILL_AFTER_MS = 2000;
/** Digitação que um encerramento forçado pode perder (SC-009). */
export const MAX_LOSS_MS = 2000;

const MARK_LINE = /^self-test: marca (\d+)\s*$/;
const REOPENED_LINE = /^self-test: reaberto (.*)$/;

/** Texto da marca `n` no arquivo. */
export function markText(n) {
  return `marca-${n}`;
}

/** Número da marca avisada nesta linha do stderr, ou `null`. */
export function parseMark(line) {
  const m = MARK_LINE.exec(line.trim());
  return m ? Number(m[1]) : null;
}

/** Texto reaberto (JSON) avisado nesta linha do stderr, ou `null`. */
export function parseReopened(line) {
  const m = REOPENED_LINE.exec(line.trim());
  if (!m) return null;
  try {
    const text = JSON.parse(m[1]);
    return typeof text === "string" ? text : null;
  } catch {
    return null;
  }
}

/**
 * Confere o texto reaberto. `marks`: `{ n, at }` na ordem em que foram avisadas (`at` em ms);
 * `killedAt`: quando o processo foi morto; `original`: o arquivo antes das marcas.
 * Devolve as falhas (vazio = ok).
 */
export function checkReopened({ marks, killedAt, original, reopened, maxLossMs = MAX_LOSS_MS }) {
  const failures = [];
  if (marks.length === 0) return ["nenhuma marca foi digitada antes da morte do processo"];
  if (typeof reopened !== "string") return ["o app reaberto não mostrou o arquivo principal"];
  if (!reopened.startsWith(original.trimEnd())) {
    failures.push("o texto reaberto não começa com o conteúdo original do arquivo");
  }
  // Cada marca numa linha própria (`marca-1` não pode casar com `marca-10`).
  const lines = new Set(reopened.split(/\r?\n/).map((l) => l.trim()));
  const present = marks.filter((m) => lines.has(markText(m.n)));
  for (const m of marks) {
    const age = killedAt - m.at;
    if (age >= maxLossMs && !present.includes(m)) {
      failures.push(`${markText(m.n)} digitada ${age} ms antes da morte e perdida (limite ${maxLossMs} ms)`);
    }
  }
  // Gravação atômica (FR-009): o que chegou ao disco é um prefixo das marcas, nunca um buraco.
  const firstMissing = marks.findIndex((m) => !present.includes(m));
  if (firstMissing >= 0 && present.some((m) => marks.indexOf(m) > firstMissing)) {
    failures.push("o texto reaberto tem marcas fora de ordem ou com buraco");
  }
  return failures;
}
