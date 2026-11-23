// Botão "Compilar" e atalho ⌘⇧B / Ctrl+Shift+B (FR-014a): ainda não implementado (T101).
import type { Ipc, IpcError, PreviewUpdate } from "../ipc";

export const COMPILE_NOW_LABEL = "Compilar agora (⌘⇧B / Ctrl+Shift+B)";

export interface CompileNowOptions {
  ipc: Ipc;
  flush: () => Promise<void>;
  onError: (err: IpcError) => void;
  keyTarget?: EventTarget;
}

export interface CompileNowHandle {
  readonly button: HTMLButtonElement;
  setAvailable(on: boolean): void;
  update(u: PreviewUpdate): void;
  run(): Promise<void>;
  destroy(): void;
}

export function mountCompileNow(_opts: CompileNowOptions): CompileNowHandle {
  throw new Error("não implementado (T101)");
}
