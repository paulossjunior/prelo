import { $, browser, expect } from "@wdio/globals";

describe("janela do Prelo", () => {
  it("abre com o título Prelo", async () => {
    await expect(browser).toHaveTitle("Prelo");
  });

  it("mostra versão e sistema vindos do núcleo", async () => {
    const os = process.platform === "win32" ? "windows" : "linux";
    await expect($("#app")).toHaveText(`Prelo 0.1.0 · ${os}`, { wait: 3000 });
  });
});
