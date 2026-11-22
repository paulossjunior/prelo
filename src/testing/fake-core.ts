// Núcleo falso para os testes de interface: o `Ipc` real (createIpc) sobre um transporte
// injetado. Cada comando responde pelo handler registrado; comando sem handler recusa com
// `not_implemented`, como o núcleo faz com recursos ainda não entregues.
import { vi, type Mock } from "vitest";
import { createIpc, type ChannelFactory, type ChannelLike, type Ipc } from "../ipc";

export type Handler = (args: Record<string, unknown>) => unknown;

export interface FakeCore {
  ipc: Ipc;
  invoke: Mock<(cmd: string, args?: Record<string, unknown>) => Promise<unknown>>;
  /** Comandos chamados, em ordem, com os argumentos. */
  calls(cmd?: string): { cmd: string; args: Record<string, unknown> }[];
  on(cmd: string, handler: Handler): void;
  channels: ChannelLike<unknown>[];
  /** Emite um evento do núcleo (ex.: `external_change_conflict`) para quem escuta. */
  emit(event: string, payload: unknown): void;
  /** Envia um `PreviewUpdate` pelo último canal de `subscribe_preview`. */
  push(update: unknown): void;
}

export function fakeCore(handlers: Record<string, Handler> = {}): FakeCore {
  const table = new Map(Object.entries(handlers));
  const log: { cmd: string; args: Record<string, unknown> }[] = [];
  const invoke = vi.fn(async (cmd: string, args?: Record<string, unknown>) => {
    log.push({ cmd, args: args ?? {} });
    const h = table.get(cmd);
    if (!h) throw "not_implemented";
    return h(args ?? {});
  });
  const channels: ChannelLike<unknown>[] = [];
  const channel = (() => {
    const c: ChannelLike<unknown> = { onmessage: () => {} };
    channels.push(c);
    return c;
  }) as ChannelFactory;
  const listeners = new Map<string, Set<(payload: unknown) => void>>();
  const listen = async (event: string, handler: (payload: unknown) => void) => {
    const set = listeners.get(event) ?? new Set();
    listeners.set(event, set);
    set.add(handler);
    return () => void set.delete(handler);
  };
  const ipc = createIpc({ invoke, listen, channel });
  return {
    ipc,
    invoke,
    calls: (cmd) => (cmd ? log.filter((c) => c.cmd === cmd) : [...log]),
    on: (cmd, handler) => table.set(cmd, handler),
    channels,
    emit: (event, payload) => {
      for (const h of [...(listeners.get(event) ?? [])]) h(payload);
    },
    push: (update) => channels.at(-1)?.onmessage(update),
  };
}

/** Promessa controlada pelo teste, para segurar uma resposta do núcleo. */
export function deferred<T>() {
  let resolve!: (v: T) => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

/** Deixa rodar as promessas e microtarefas pendentes. */
export async function settle(rounds = 5): Promise<void> {
  for (let i = 0; i < rounds; i++) await new Promise((r) => setTimeout(r, 0));
}
