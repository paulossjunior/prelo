#!/usr/bin/env node
// Etapa `test:e2e` (contracts/dev-workflow.md): autoteste em todos os sistemas e WebDriver
// em Linux e Windows. No macOS não existe driver WebDriver para a webview (research R3).
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));

// Comandos fixos, sem entrada externa interpolada.
function step(cmd) {
  const r = spawnSync(cmd, { cwd: root, stdio: "inherit", shell: true });
  return r.status ?? 1;
}

// Roda todas as etapas mesmo após falha, para o diagnóstico ficar completo.
const codes = [step("node scripts/self-test.mjs")];
if (process.platform === "darwin") {
  console.log("e2e: WebDriver não disponível no macOS; coberto pelo autoteste");
} else {
  const built = step("npx --no-install tauri build --debug --no-bundle");
  codes.push(built);
  if (process.platform === "win32") {
    // Processos do WebView2 que sobraram do autoteste prendem a pasta de dados do app.
    step("taskkill /F /IM msedgewebview2.exe /T >NUL 2>&1 || exit 0");
  }
  if (built === 0) {
    const wdio = step("npx --no-install wdio run e2e/wdio.conf.ts");
    codes.push(wdio);
    const log = process.env.PRELO_NATIVE_DRIVER_LOG;
    if (wdio !== 0 && log && existsSync(log)) {
      const lines = readFileSync(log, "utf8").split(/\r?\n/);
      console.error(`e2e: últimas linhas de ${log}:\n${lines.slice(-120).join("\n")}`);
    }
  }
}
process.exitCode = codes.every((c) => c === 0) ? 0 : 1;
