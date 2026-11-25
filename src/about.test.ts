// Testes da tela "Sobre" (T039; FR-012, D-15, D-16). Contrato: contracts/ipc.md
// (`compiler_info` → `{ embedded, registered, fonts }`).
import { afterEach, describe, expect, it, vi } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { parseFontLicenses } from "../scripts/audit.mjs";
import { compilerStatus, mountAbout, TYPST_ASSETS_NOTICES, UI_FONTS } from "./about";
import { fakeCore, settle } from "./testing/fake-core";

afterEach(() => {
  document.body.innerHTML = "";
});

const FONTS = [
  {
    family: "Libertinus Serif",
    file: "LibertinusSerif-Regular.otf",
    license: "OFL-1.1",
    copyright: "Copyright © 2012-2024 The Libertinus Project Authors",
  },
];

describe("compilerStatus", () => {
  it("mesma versão (ou sem registro) não é diferença", () => {
    expect(compilerStatus({ embedded: "0.15.1", registered: "0.15.1" })).toEqual({
      mismatch: false,
      label: "Typst 0.15.1",
    });
    expect(compilerStatus({ embedded: "0.15.1", registered: null }).mismatch).toBe(false);
  });

  it("versão registrada diferente vira aviso com as duas versões", () => {
    const s = compilerStatus({ embedded: "0.15.1", registered: "0.14.0" });
    expect(s.mismatch).toBe(true);
    expect(s.label).toContain("0.15.1");
    expect(s.label).toContain("0.14.0");
  });
});

describe("mountAbout", () => {
  function setup(handlers: Parameters<typeof fakeCore>[0] = {}, registered: string | null = "0.15.1") {
    const core = fakeCore(handlers);
    const parent = document.createElement("div");
    document.body.append(parent);
    const onUpdateVersion = vi.fn(async () => {});
    const about = mountAbout(parent, {
      ipc: core.ipc,
      compiler: () => ({ embedded: "0.15.1", registered }),
      onUpdateVersion,
    });
    return { core, parent, about, onUpdateVersion };
  }

  it("começa fechada e abre como diálogo acessível", async () => {
    const { about } = setup({ compiler_info: () => ({ embedded: "0.15.1", registered: "0.15.1", fonts: FONTS }) });
    expect(about.isOpen()).toBe(false);
    await about.open();
    expect(about.isOpen()).toBe(true);
    expect(about.element.getAttribute("role")).toBe("dialog");
    expect(about.element.getAttribute("aria-modal")).toBe("true");
    expect(about.element.textContent).toContain("Sobre o Prelo");
  });

  it("mostra versão embutida e registrada vindas de compiler_info", async () => {
    const { about, core } = setup({
      compiler_info: () => ({ embedded: "0.15.1", registered: "0.15.1", fonts: FONTS }),
    });
    await about.open();
    expect(core.calls("compiler_info")).toHaveLength(1);
    const text = about.element.textContent ?? "";
    expect(text).toMatch(/Embutido\s*0\.15\.1/);
    expect(text).toMatch(/Registrado no projeto\s*0\.15\.1/);
    expect(about.element.querySelector("[role=alert]")).toBeNull();
  });

  it("versão diferente: aviso e ação explícita de atualizar (FR-012)", async () => {
    const { about, onUpdateVersion } = setup({
      compiler_info: () => ({ embedded: "0.15.1", registered: "0.14.0", fonts: FONTS }),
    });
    await about.open();
    const alert = about.element.querySelector("[role=alert]");
    expect(alert?.textContent).toContain("0.14.0");
    const btn = [...about.element.querySelectorAll("button")].find((b) => /Atualizar/.test(b.textContent ?? ""));
    expect(btn).toBeDefined();
    btn!.click();
    await settle();
    expect(onUpdateVersion).toHaveBeenCalledTimes(1);
  });

  it("lista as fontes com licença e copyright, e o texto da OFL (D-15)", async () => {
    const { about } = setup({
      compiler_info: () => ({ embedded: "0.15.1", registered: null, fonts: FONTS }),
    });
    await about.open();
    const text = about.element.textContent ?? "";
    expect(text).toContain("Libertinus Serif");
    expect(text).toContain("LibertinusSerif-Regular.otf");
    expect(text).toContain("OFL-1.1");
    expect(text).toContain("The Libertinus Project Authors");
    expect(text).toContain("SIL OPEN FONT LICENSE Version 1.1");
    expect(text).toMatch(/Registrado no projeto\s*ainda não registrado/);
  });

  it("mostra o texto de cada licença das fontes do compilador (GUST, Bitstream Vera, GPL+FE+DE)", async () => {
    const entries = parseFontLicenses(readFileSync(resolve(process.cwd(), "src-tauri/fonts/licenses.toml"), "utf8"));
    const { about } = setup({
      compiler_info: () => ({ embedded: "0.15.1", registered: null, fonts: entries }),
    });
    await about.open();
    const text = about.element.textContent ?? "";
    for (const family of ["New Computer Modern", "New Computer Modern Math", "DejaVu Sans Mono"]) {
      expect(text).toContain(family);
    }
    expect(text).toContain("SIL OPEN FONT LICENSE Version 1.1");
    expect(text).toContain("GUST Font License");
    expect(text).toContain("Bitstream Vera Fonts Copyright");
    expect(text).toContain("Distribution Exception (DE)");
    expect(text).toContain("GNU GENERAL PUBLIC LICENSE");
  });

  it("mostra os avisos de atribuição do typst-assets (CC-BY-4.0 e W3C, D-16)", async () => {
    const { about } = setup({
      compiler_info: () => ({ embedded: "0.15.1", registered: null, fonts: FONTS }),
    });
    await about.open();
    const text = about.element.textContent ?? "";
    expect(TYPST_ASSETS_NOTICES.map((n) => n.license)).toEqual(
      expect.arrayContaining(["CC-BY-4.0", "W3C-20150513"]),
    );
    for (const n of TYPST_ASSETS_NOTICES) expect(text).toContain(n.text.slice(0, 40));
    expect(text).toContain("WHATWG");
    expect(text).toContain("World Wide Web Consortium");
    expect(text).toContain("creativecommons.org/licenses/by/4.0");
  });

  it("sem compiler_info no núcleo, usa as versões do projeto e ainda mostra licenças", async () => {
    const { about } = setup({}, "0.15.1");
    await about.open();
    const text = about.element.textContent ?? "";
    expect(text).toMatch(/Embutido\s*0\.15\.1/);
    expect(text).toContain("Libertinus");
    expect(text).toContain("New Computer Modern");
    expect(text).toContain("DejaVu Sans Mono");
    expect(text).toContain("SIL OPEN FONT LICENSE Version 1.1");
  });

  it("texto vindo do núcleo entra como texto, nunca como marcação", async () => {
    const { about } = setup({
      compiler_info: () => ({
        embedded: "0.15.1",
        registered: "<img src=x onerror=alert(1)>",
        fonts: [{ ...FONTS[0], family: "<b>x</b>" }],
      }),
    });
    await about.open();
    expect(about.element.querySelector("img")).toBeNull();
    expect(about.element.querySelector("b")).toBeNull();
    expect(about.element.textContent).toContain("<b>x</b>");
  });

  it("fecha pelo botão e pela tecla Escape", async () => {
    const { about } = setup({ compiler_info: () => ({ embedded: "0.15.1", registered: null, fonts: [] }) });
    await about.open();
    const close = [...about.element.querySelectorAll("button")].find((b) => b.textContent === "Fechar");
    close!.click();
    expect(about.isOpen()).toBe(false);
    await about.open();
    about.element.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    expect(about.isOpen()).toBe(false);
  });
});

describe("fontes da interface (OFL-1.1, D-15)", () => {
  it("a lista mostrada no Sobre é a de src/fonts/licenses.toml", () => {
    const entries = parseFontLicenses(readFileSync(resolve(process.cwd(), "src/fonts/licenses.toml"), "utf8"));
    expect(entries.length).toBeGreaterThan(0);
    const fromToml = [...new Map(entries.map((e) => [e.family, e])).values()].map((e) => ({
      family: e.family,
      license: e.license,
      copyright: e.copyright,
    }));
    expect(UI_FONTS.map((f) => ({ family: f.family, license: f.license, copyright: f.copyright }))).toEqual(fromToml);
  });

  it("o Sobre lista cada família da interface com licença, copyright e o texto da OFL", async () => {
    const parent = document.createElement("div");
    document.body.append(parent);
    const about = mountAbout(parent, {
      ipc: fakeCore({ compiler_info: () => ({ embedded: "0.15.1", registered: null, fonts: [] }) }).ipc,
      compiler: () => ({ embedded: "0.15.1", registered: null }),
    });
    await about.open();
    const text = about.element.textContent ?? "";
    expect(text).toContain("Fontes da interface");
    for (const f of ["IBM Plex Sans", "JetBrains Mono", "Bricolage Grotesque"]) expect(text).toContain(f);
    expect(text).toContain("IBM Corp.");
    expect(text).toMatch(/SIL OPEN FONT LICENSE Version 1\.1/);
  });
});
