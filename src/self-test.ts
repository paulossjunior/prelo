// Autoteste ponta a ponta (T012/T020): só roda em build com VITE_SELF_TEST definido.
// Reporta ao núcleo o texto renderizado e o tempo desde o início da página; o núcleo decide
// e encerra o app (contracts/ipc.md, self_test_report). Erros de JavaScript também são
// reportados, para o autoteste falhar com motivo em vez de esperar o tempo esgotar.
import { invoke } from "@tauri-apps/api/core";

function send(rendered: string): Promise<void> {
  return invoke("self_test_report", { rendered, elapsedMs: Math.round(performance.now()) });
}

export function installSelfTestErrorReporter(): void {
  if (!import.meta.env.VITE_SELF_TEST) return;
  window.addEventListener("error", (e) => void send(`erro JS: ${e.message}`));
  window.addEventListener("unhandledrejection", (e) => void send(`promessa rejeitada: ${String(e.reason)}`));
}

export async function reportSelfTest(root: HTMLElement): Promise<void> {
  if (!import.meta.env.VITE_SELF_TEST) return;
  await send(root.textContent?.trim() ?? "");
}
