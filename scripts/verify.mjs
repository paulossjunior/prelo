#!/usr/bin/env node
// Orquestrador de `npm run verify` (specs/001-app-skeleton/contracts/dev-workflow.md).
// Roda todas as etapas mesmo após falha, resume as falhas e sai com código != 0 se algo falhar.
import { spawn } from "node:child_process";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

export const STEPS = [
  { name: "fmt", cmd: "cargo fmt --manifest-path src-tauri/Cargo.toml --check" },
  {
    name: "clippy",
    cmd: "cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings",
  },
  { name: "tsc", cmd: "npx tsc --noEmit" },
  { name: "test:rust", cmd: "npm run test:rust" },
  { name: "test:ui", cmd: "npm run test:ui" },
  { name: "test:e2e", cmd: "npm run test:e2e" },
  { name: "audit", cmd: "npm run audit" },
];

const SCAN_DIRS = ["src", "src-tauri/src", "src-tauri/tests", "scripts", "e2e"];
const SCAN_EXT = /\.(rs|ts|mts|mjs|js)$/;
const JUSTIFICATION = "// justificativa:";
const RUST_SKIP = /^\s*#\[ignore\b/;
const TS_SKIP = /^\s*(it|test|describe)\.(skip|only)\b/;

/** Roda cada etapa em sequência; uma falha não interrompe as seguintes. */
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
  lines.forEach((line, i) => {
    if (!pattern.test(line)) return;
    const justified = line.includes(JUSTIFICATION) || (lines[i - 1] ?? "").includes(JUSTIFICATION);
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
  const results = await runSteps([...STEPS, skipStep], async (step) => {
    if (step !== skipStep) return runCommand(step);
    const skips = scanSkips(root);
    for (const s of skips) console.error(`teste pulado sem justificativa: ${s.file}:${s.line}`);
    return skips.length === 0 ? 0 : 1;
  });
  console.log(`\n${summarize(results)}`);
  process.exitCode = exitCode(results);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  await main();
}
