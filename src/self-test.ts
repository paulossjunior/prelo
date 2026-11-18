// Autoteste ponta a ponta (T012/T020): só roda em build com VITE_SELF_TEST definido.
// Reporta ao núcleo o texto renderizado e o tempo desde o início da página; o núcleo decide
// e encerra o app (contracts/ipc.md, self_test_report).
import { invoke } from "@tauri-apps/api/core";

export async function reportSelfTest(root: HTMLElement): Promise<void> {
  if (!import.meta.env.VITE_SELF_TEST) return;
  await invoke("self_test_report", {
    rendered: root.textContent?.trim() ?? "",
    elapsedMs: Math.round(performance.now()),
  });
}
