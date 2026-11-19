// Testes da tela inicial (T016). Estados: data-model.md "Estado da tela inicial".
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { mountAppInfo, TIMEOUT_MS } from "./app-info";

const INFO = { version: "0.1.0", os: "macos", arch: "aarch64" };

let root: HTMLElement;

beforeEach(() => {
  vi.useFakeTimers();
  root = document.createElement("main");
});

afterEach(() => {
  vi.useRealTimers();
});

function text(): string {
  return root.textContent?.replace(/\s+/g, " ").trim() ?? "";
}

describe("tela inicial", () => {
  it("mostra Carregando e depois Prelo versão · sistema", async () => {
    let resolve!: (v: unknown) => void;
    const invoke = vi.fn(() => new Promise((r) => (resolve = r)));
    const done = mountAppInfo(root, invoke);
    expect(text()).toContain("Carregando");
    resolve(INFO);
    await done;
    expect(text()).toBe("Prelo 0.1.0 · macos");
    expect(invoke).toHaveBeenCalledWith("get_app_info");
  });

  it("erro do núcleo mostra mensagem e botão Tentar de novo, nunca tela vazia", async () => {
    const invoke = vi.fn(() => Promise.reject(new Error("unsupported_os")));
    await mountAppInfo(root, invoke);
    expect(text()).toMatch(/não foi possível/i);
    expect(root.querySelector("button")?.textContent).toBe("Tentar de novo");
  });

  it(`sem resposta em ${TIMEOUT_MS} ms vira erro`, async () => {
    const invoke = vi.fn(() => new Promise(() => {}));
    const done = mountAppInfo(root, invoke);
    await vi.advanceTimersByTimeAsync(TIMEOUT_MS - 1);
    expect(text()).toContain("Carregando");
    await vi.advanceTimersByTimeAsync(1);
    await done;
    expect(text()).toMatch(/não respondeu/i);
    expect(root.querySelector("button")).not.toBeNull();
  });

  it("Tentar de novo volta a Carregando e consulta outra vez", async () => {
    const invoke = vi
      .fn()
      .mockRejectedValueOnce(new Error("falhou"))
      .mockResolvedValueOnce(INFO);
    await mountAppInfo(root, invoke);
    root.querySelector("button")!.click();
    expect(text()).toContain("Carregando");
    await vi.runAllTimersAsync();
    expect(text()).toBe("Prelo 0.1.0 · macos");
    expect(invoke).toHaveBeenCalledTimes(2);
  });

  it("resposta fora do contrato é tratada como erro", async () => {
    const invoke = vi.fn(() => Promise.resolve({ version: "0.1.0", os: "beos", arch: "x" }));
    await mountAppInfo(root, invoke);
    expect(root.querySelector("button")).not.toBeNull();
  });

  it("texto vindo do núcleo não é interpretado como HTML", async () => {
    const invoke = vi.fn(() =>
      Promise.resolve({ version: "<img src=x onerror=alert(1)>", os: "linux", arch: "x86_64" }),
    );
    await mountAppInfo(root, invoke);
    expect(root.querySelector("img")).toBeNull();
  });
});
