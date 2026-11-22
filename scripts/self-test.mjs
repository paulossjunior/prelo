#!/usr/bin/env node
// Autoteste ponta a ponta (T009, T031; research R3): roda nos três sistemas, inclusive macOS,
// onde não há WebDriver. Compila o app com a feature `self-test`, abre o binário real e
// espera que ele se encerre sozinho com código 0 depois que a interface reportar ao núcleo.
// Depois confirma que nenhum processo do app sobrou (FR-005).
import { execFileSync, spawn, spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { pathWithoutTypst } from "./no-typst-path.mjs";

// Inclui o roteiro do editor (T031): abrir a fixture, compilar, editar e esperar o erro.
const TIMEOUT_MS = 60_000;
const root = fileURLToPath(new URL("..", import.meta.url));
const exe = process.platform === "win32" ? "prelo.exe" : "prelo";
const binary = join(root, "src-tauri", "target", "debug", exe);

function fail(msg) {
  console.error(`self-test: FALHA: ${msg}`);
  process.exit(1);
}

function build() {
  console.log("self-test: compilando com --features self-test");
  const r = spawnSync("npx --no-install tauri build --debug --no-bundle --features self-test", {
    cwd: root,
    stdio: "inherit",
    shell: true,
    env: { ...process.env, VITE_SELF_TEST: "1" },
  });
  if (r.status !== 0) fail(`build terminou com código ${r.status}`);
  if (!existsSync(binary)) fail(`binário não encontrado em ${binary}`);
}

function run() {
  return new Promise((resolve) => {
    const started = Date.now();
    // SC-006: o app roda sem `typst` no PATH; o núcleo confere de novo ao receber o relatório.
    const env = { ...process.env, PATH: pathWithoutTypst(process.env.PATH) };
    const child = spawn(binary, [], { cwd: root, env, stdio: ["ignore", "inherit", "pipe"] });
    let stderr = "";
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
      process.stderr.write(chunk);
    });
    const timer = setTimeout(() => {
      diagnose(child.pid);
      child.kill("SIGKILL");
      resolve({ code: null, timedOut: true, ms: Date.now() - started, stderr });
    }, TIMEOUT_MS);
    child.on("exit", (code) => {
      clearTimeout(timer);
      resolve({ code, timedOut: false, ms: Date.now() - started, stderr });
    });
  });
}

// Ao esgotar o tempo: mostra os processos do app (ex.: WebKitWebProcess, msedgewebview2)
// para saber se a webview chegou a subir.
function diagnose(pid) {
  try {
    const out =
      process.platform === "win32"
        ? execFileSync(
            "powershell",
            ["-NoProfile", "-Command", "Get-Process prelo,msedgewebview2 -ErrorAction SilentlyContinue | Format-Table Id,ProcessName,StartTime -AutoSize | Out-String"],
            { encoding: "utf8" },
          )
        : execFileSync("ps", ["-e", "-o", "pid,ppid,stat,etime,args"], { encoding: "utf8" })
            .split("\n")
            .filter((l) => /prelo|WebKit|PID/.test(l))
            .join("\n");
    console.error(`self-test: processos ao esgotar o tempo (pid ${pid}):\n${out}`);
  } catch (err) {
    console.error(`self-test: diagnóstico indisponível: ${err}`);
  }
}

function lingering() {
  try {
    if (process.platform === "win32") {
      // Só o binário deste build: um Prelo instalado pelo usuário não conta.
      const out = execFileSync(
        "powershell",
        [
          "-NoProfile",
          "-Command",
          "Get-Process -Name prelo -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Path",
        ],
        { encoding: "utf8" },
      );
      return out
        .split(/\r?\n/)
        .some((p) => p.trim().toLowerCase() === binary.toLowerCase());
    }
    execFileSync("pgrep", ["-f", binary], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

build();
const result = await run();
if (result.timedOut) fail(`o app não se encerrou em ${TIMEOUT_MS / 1000} s`);
if (result.code !== 0) fail(`o app saiu com código ${result.code}`);
// US1 cenário 2: fechar sem mensagem de erro.
// Só sinais do próprio Prelo: ruído do WebKit/WebView2 no stderr não conta.
if (/panicked|self-test: FALHA/.test(result.stderr)) {
  fail("o app escreveu erro no stderr");
}
if (lingering()) fail("processo do app continua em execução depois de fechar a janela");
console.log(`self-test: ok em ${result.ms} ms`);
