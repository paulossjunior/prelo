#!/usr/bin/env node
// Orquestrador de `npm run verify` (specs/001-app-skeleton/contracts/dev-workflow.md).
// Roda todas as etapas mesmo após falha, resume as falhas e sai com código != 0 se algo falhar.
import { spawn } from "node:child_process";
import { readdirSync, readFileSync, realpathSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

export const STEPS = [
  { name: "fmt", cmd: "cargo fmt --manifest-path src-tauri/Cargo.toml --check" },
  {
    name: "clippy",
    cmd: "cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings",
  },
  { name: "tsc", cmd: "npx --no-install tsc --noEmit && npx --no-install tsc --noEmit -p e2e" },
  {
    name: "clippy:self-test",
    cmd: "cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --features self-test -- -D warnings",
  },
  { name: "test:rust", cmd: "npm run test:rust" },
  {
    name: "test:rust:self-test",
    cmd: "cargo test --no-fail-fast --manifest-path src-tauri/Cargo.toml --features self-test",
  },
  { name: "test:ui", cmd: "npm run test:ui" },
  { name: "test:e2e", cmd: "npm run test:e2e" },
  { name: "audit", cmd: "npm run audit" },
];

const SCAN_DIRS = ["src", "src-tauri/src", "src-tauri/tests", "scripts", "e2e"];
const SCAN_EXT = /\.(rs|ts|mts|mjs|js)$/;
const JUSTIFICATION = "// justificativa:";
const RUST_SKIP = /^\s*#\[ignore\b/;
// it/test/describe/suite/bench, com modificadores encadeados (ex.: test.concurrent.skip).
const TS_SKIP = /^\s*(it|test|describe|suite|bench)(\.\w+)*\.(skip|only)\b/;
// Continuação de cadeia quebrada em linhas: a anterior termina em it/test/... e esta começa em .skip.
const TS_CONTINUATION = /^\s*(\.\w+)*\.(skip|only)\b/;
const TS_CHAIN_HEAD = /^\s*(it|test|describe|suite|bench)(\.\w+)*\s*$/;

/** Roda cada etapa em sequência; uma falha não interrompe as seguintes. */
/** Etapas que o modo rápido deixa de fora: o build e os testes ponta a ponta (gitflow). */
const FULL_ONLY = new Set(["test:e2e"]);

/** "quick" para push em feature/dev; "full" para push na main, tag e CI. */
export function stepsFor(mode) {
  return mode === "quick" ? STEPS.filter((s) => !FULL_ONLY.has(s.name)) : STEPS;
}

export function modeFromArgs(argv) {
  return argv.includes("--quick") ? "quick" : "full";
}

export async function runSteps(steps, run = runCommand) {
  const results = [];
  for (const step of steps) {
    try {
      const code = await run(step);
      results.push({ name: step.name, ok: code === 0 });
    } catch (err) {
      results.push({ name: step.name, ok: false, error: String(err?.message ?? err) });
    }
  }
  return results;
}

export function exitCode(results) {
  return results.length > 0 && results.every((r) => r.ok) ? 0 : 1;
}

export function summarize(results) {
  const lines = results.map((r) => `${r.ok ? "ok   " : "FALHA"}  ${r.name}${r.error ? ` (${r.error})` : ""}`);
  const failed = results.filter((r) => !r.ok).map((r) => r.name);
  lines.push(
    failed.length === 0 && results.length > 0
      ? "verify: todas as etapas passaram"
      : `verify: ${failed.length || "nenhuma etapa executada,"} falharam: ${failed.join(", ") || "-"}`,
  );
  return lines.join("\n");
}

/** Testes pulados ou isolados sem `// justificativa:` na mesma linha ou na anterior. */
export function findUnjustifiedSkips(file, text) {
  const pattern = file.endsWith(".rs") ? RUST_SKIP : TS_SKIP;
  const lines = text.split(/\r?\n/);
  const found = [];
  const isRust = file.endsWith(".rs");
  lines.forEach((line, i) => {
    const prev = lines[i - 1] ?? "";
    const continued = !isRust && TS_CONTINUATION.test(line) && TS_CHAIN_HEAD.test(prev);
    if (!pattern.test(line) && !continued) return;
    const justified =
      line.includes(JUSTIFICATION) ||
      prev.includes(JUSTIFICATION) ||
      (continued && (lines[i - 2] ?? "").includes(JUSTIFICATION));
    if (!justified) found.push({ file, line: i + 1 });
  });
  return found;
}

function listFiles(dir) {
  let entries;
  try {
    entries = readdirSync(dir);
  } catch {
    return [];
  }
  return entries.flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? listFiles(path) : SCAN_EXT.test(name) ? [path] : [];
  });
}

export function scanSkips(root) {
  return SCAN_DIRS.flatMap((d) => listFiles(join(root, d))).flatMap((path) =>
    findUnjustifiedSkips(relative(root, path), readFileSync(path, "utf8")),
  );
}

function runCommand(step) {
  return new Promise((resolve, reject) => {
    console.log(`\n▶ ${step.name}: ${step.cmd}`);
    const child = spawn(step.cmd, { shell: true, stdio: "inherit" });
    child.on("error", reject);
    child.on("close", (code) => resolve(code ?? 1));
  });
}

async function main() {
  const root = fileURLToPath(new URL("..", import.meta.url));
  process.chdir(root);
  const skipStep = {
    name: "testes pulados",
    cmd: "(interno)",
  };
  const mode = modeFromArgs(process.argv);
  console.log(`verify: modo ${mode === "quick" ? "rápido (sem build nem e2e)" : "completo"}`);
  const results = await runSteps([...stepsFor(mode), skipStep], async (step) => {
    if (step !== skipStep) return runCommand(step);
    const skips = scanSkips(root);
    for (const s of skips) console.error(`teste pulado sem justificativa: ${s.file}:${s.line}`);
    return skips.length === 0 ? 0 : 1;
  });
  console.log(`\n${summarize(results)}`);
  process.exitCode = exitCode(results);
}

/** Compara caminhos reais: link simbólico ou maiúsculas de unidade não podem pular main(). */
export function isMain(moduleUrl, argv1) {
  if (!argv1) return false;
  try {
    return realpathSync(fileURLToPath(moduleUrl)) === realpathSync(argv1);
  } catch {
    return false;
  }
}

if (isMain(import.meta.url, process.argv[1])) {
  await main();
}
