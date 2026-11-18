// Autoteste ponta a ponta (T012): só roda em build com VITE_SELF_TEST definido.
// Reporta ao núcleo o texto renderizado; o núcleo decide e encerra o app.
import { invoke } from "@tauri-apps/api/core";

export async function reportSelfTest(root: HTMLElement, startedAt: number): Promise<void> {
  if (!import.meta.env.VITE_SELF_TEST) return;
  await invoke("self_test_report", {
    rendered: root.textContent?.trim() ?? "",
    elapsedMs: Math.round(performance.now() - startedAt),
  });
}
