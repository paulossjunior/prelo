import { browser, expect } from "@wdio/globals";

describe("janela do Prelo", () => {
  it("abre com o título Prelo", async () => {
    await expect(browser).toHaveTitle("Prelo");
  });
});
