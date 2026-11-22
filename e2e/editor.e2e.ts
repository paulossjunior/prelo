// T031 (SC-006, US1 Independent Test): com `typst` fora do PATH, a fixture `artigo-basico`
// abre (build de e2e: `self_test_open_project`), a edição muda a prévia e um erro aparece
// como diagnóstico no editor. Linux e Windows; no macOS o autoteste cobre o mesmo roteiro.
import { $, $$, browser, expect } from "@wdio/globals";

async function pageSources(): Promise<string> {
  const srcs: string[] = [];
  for (const img of await $$(".pv-page img")) srcs.push(String(await img.getAttribute("src")));
  return srcs.join(",");
}

describe("editor com prévia ao vivo (US1)", () => {
  it("abre a fixture: árvore, arquivo principal no editor e prévia", async () => {
    await expect($("button.pt-file[data-path='main.typ']")).toHaveAttribute("aria-current", "true", {
      wait: 10_000,
    });
    await expect($(".cm-content")).toHaveText(expect.stringContaining("Artigo básico"), { wait: 10_000 });
    await $(".pv-page img").waitForExist({ timeout: 20_000 });
    await expect($(".ws-compiler")).toHaveText(expect.stringContaining("Typst 0.15.1"));
  });

  it("digitar muda a prévia", async () => {
    await browser.waitUntil(async () => (await pageSources()).length > 0, { timeout: 20_000 });
    const before = await pageSources();
    await $(".cm-content").click();
    await browser.keys(["Control", "Home"]);
    await browser.keys(["Control"]);
    await browser.keys("Parágrafo novo do teste ponta a ponta.\n");
    await browser.waitUntil(async () => (await pageSources()) !== before, {
      timeout: 20_000,
      timeoutMsg: "a prévia não mudou depois da edição",
    });
  });

  it("um erro aparece como diagnóstico no editor", async () => {
    await browser.keys("#funcao-inexistente-do-e2e()\n");
    await $(".cm-lintRange-error").waitForExist({ timeout: 20_000 });
    await expect($(".pv-error")).toBeDisplayed();
    await expect($(".ws-compile")).toHaveText(expect.stringMatching(/erro/));
  });
});
