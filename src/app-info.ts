// Tela inicial (T019): consulta o núcleo e mostra "Prelo {version} · {os}".
// Estados: data-model.md "Estado da tela inicial". Contrato: contracts/ipc.md.

export const TIMEOUT_MS = 5000;

export type Invoke = (cmd: string) => Promise<unknown>;

interface AppInfo {
  version: string;
  os: "macos" | "windows" | "linux";
  arch: string;
}

class TimeoutError extends Error {}

const OS_VALUES = new Set(["macos", "windows", "linux"]);
const SEMVER = /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/;

function isAppInfo(v: unknown): v is AppInfo {
  if (typeof v !== "object" || v === null) return false;
  const o = v as Record<string, unknown>;
  return (
    typeof o.version === "string" &&
    SEMVER.test(o.version) &&
    typeof o.os === "string" &&
    OS_VALUES.has(o.os) &&
    typeof o.arch === "string" &&
    o.arch.length > 0
  );
}

function withTimeout<T>(promise: Promise<T>, ms: number): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(() => reject(new TimeoutError()), ms);
    promise.then(
      (v) => {
        clearTimeout(timer);
        resolve(v);
      },
      (e: unknown) => {
        clearTimeout(timer);
        reject(e);
      },
    );
  });
}

function paragraph(text: string): HTMLParagraphElement {
  const p = document.createElement("p");
  p.textContent = text; // nunca innerHTML: o texto vem de fora da interface
  return p;
}

/** Monta a tela e resolve quando ela chega a Pronto ou Erro. */
export async function mountAppInfo(root: HTMLElement, invoke: Invoke): Promise<void> {
  root.replaceChildren(paragraph("Carregando…"));
  try {
    const info = await withTimeout(invoke("get_app_info"), TIMEOUT_MS);
    if (!isAppInfo(info)) throw new Error("resposta fora do contrato");
    root.replaceChildren(paragraph(`Prelo ${info.version} · ${info.os}`));
  } catch (err) {
    const message =
      err instanceof TimeoutError
        ? `O núcleo do Prelo não respondeu em ${TIMEOUT_MS / 1000} s.`
        : `Não foi possível obter as informações do Prelo (${err instanceof Error ? err.message : String(err)}).`;
    const retry = document.createElement("button");
    retry.type = "button";
    retry.textContent = "Tentar de novo";
    retry.addEventListener("click", () => void mountAppInfo(root, invoke));
    root.replaceChildren(paragraph(message), retry);
  }
}
