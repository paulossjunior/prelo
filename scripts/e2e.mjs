#!/usr/bin/env node
// Etapa `test:e2e` (contracts/dev-workflow.md): autoteste em todos os sistemas e WebDriver
// em Linux e Windows. No macOS não existe driver WebDriver para a webview (research R3).
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));

// Comandos fixos, sem entrada externa interpolada.
function step(cmd) {
  const r = spawnSync(cmd, { cwd: root, stdio: "inherit", shell: true });
  return r.status ?? 1;
}

let code = step("node scripts/self-test.mjs");
if (process.platform === "darwin") {
  console.log("e2e: WebDriver não disponível no macOS; coberto pelo autoteste");
} else {
  const built = step("npx --no-install tauri build --debug --no-bundle");
  code ||= built || step("npx --no-install wdio run e2e/wdio.conf.ts");
}
process.exitCode = code;
