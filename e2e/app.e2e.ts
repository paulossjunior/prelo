import { readFileSync } from "node:fs";
import { $, browser, expect } from "@wdio/globals";

// A versão vem do package.json (a mesma que release-check.mjs exige igual à tag).
const { version } = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8")) as { version: string };

describe("janela do Prelo", () => {
  it("abre com o título Prelo", async () => {
    await expect(browser).toHaveTitle("Prelo");
  });

  it("mostra versão e sistema vindos do núcleo", async () => {
    const os = process.platform === "win32" ? "windows" : "linux";
    await expect($("[data-testid=app-info]")).toHaveText(`Prelo ${version} · ${os}`, { wait: 3000 });
  });
});
