// Tela "Sobre" (T039; FR-012, D-15, D-16): versão do compilador embutido e a registrada no
// projeto, aviso quando diferem (com a ação explícita de atualizar), licenças das fontes
// embutidas com o texto da OFL e os avisos de atribuição dos dados do `typst-assets`.
// Tudo entra como texto, nunca como marcação.
import oflText from "../src-tauri/fonts/OFL.txt?raw";
import { toIpcError, type CompilerInfo, type FontLicense, type Ipc } from "./ipc";

export interface CompilerVersions {
  embedded: string;
  registered: string | null;
}

export interface CompilerStatus {
  mismatch: boolean;
  label: string;
}

/** Versão registrada diferente da embutida desativa a prévia (FR-012). */
export function compilerStatus(v: CompilerVersions): CompilerStatus {
  const mismatch = v.registered !== null && v.registered !== v.embedded;
  return {
    mismatch,
    label: mismatch
      ? `Typst ${v.embedded} embutido · projeto registrado com ${v.registered}`
      : `Typst ${v.embedded}`,
  };
}

export interface Notice {
  title: string;
  license: string;
  text: string;
}

/**
 * Avisos de atribuição exigidos pelos dados que o `typst-assets` 0.15.1 compila para dentro do
 * binário (NOTICE da crate; D-16). Revisar a cada atualização do `typst-assets`.
 */
export const TYPST_ASSETS_NOTICES: readonly Notice[] = [
  {
    title: "Dados de HTML derivados das especificações do WHATWG",
    license: "CC-BY-4.0",
    text:
      "Partes dos dados em files/html/data.rs do typst-assets foram derivadas de " +
      "https://html.spec.whatwg.org e https://fetch.spec.whatwg.org. " +
      "Copyright © WHATWG (Apple, Google, Mozilla, Microsoft). " +
      "This work is licensed under a Creative Commons Attribution 4.0 International License " +
      "(https://creativecommons.org/licenses/by/4.0/). To the extent portions of it are " +
      "incorporated into source code, such portions in the source code are licensed under the " +
      "BSD 3-Clause License instead. Os dados foram adaptados pelo projeto Typst para uso no " +
      "compilador; o material é fornecido no estado em que se encontra, sem garantias.",
  },
  {
    title: "Dados de HTML e MathML derivados de especificações do W3C",
    license: "W3C-20150513",
    text:
      "Partes dos dados em files/html/data.rs e files/mathml/data.rs do typst-assets foram " +
      "derivadas de https://www.w3.org/TR/wai-aria-1.1/ e https://www.w3.org/TR/referrer-policy/ " +
      "(Copyright © 2017 World Wide Web Consortium) e de https://www.w3.org/TR/mathml-core/ " +
      "(Copyright © 2025 World Wide Web Consortium). " +
      "This software or document includes material copied from or derived from the documents " +
      "above. https://www.w3.org/copyright/software-license-2023/\n\n" +
      'THIS WORK IS PROVIDED "AS IS," AND COPYRIGHT HOLDERS MAKE NO REPRESENTATIONS OR ' +
      "WARRANTIES, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO, WARRANTIES OF " +
      "MERCHANTABILITY OR FITNESS FOR ANY PARTICULAR PURPOSE OR THAT THE USE OF THE SOFTWARE OR " +
      "DOCUMENT WILL NOT INFRINGE ANY THIRD PARTY PATENTS, COPYRIGHTS, TRADEMARKS OR OTHER " +
      "RIGHTS.\n\nCOPYRIGHT HOLDERS WILL NOT BE LIABLE FOR ANY DIRECT, INDIRECT, SPECIAL OR " +
      "CONSEQUENTIAL DAMAGES ARISING OUT OF ANY USE OF THE SOFTWARE OR DOCUMENT.\n\n" +
      "The name and trademarks of copyright holders may NOT be used in advertising or publicity " +
      "pertaining to the work without specific, written prior permission. Title to copyright " +
      "in this work will at all times remain with copyright holders.",
  },
  {
    title: "Fontes Foxit (usadas ao embutir PDFs como imagem)",
    license: "BSD-3-Clause",
    text:
      "Copyright 2014 PDFium Authors. All rights reserved. Redistribution and use in source and " +
      "binary forms, with or without modification, are permitted provided that the following " +
      "conditions are met: Redistributions of source code must retain the above copyright " +
      "notice, this list of conditions and the following disclaimer. Redistributions in binary " +
      "form must reproduce the above copyright notice, this list of conditions and the following " +
      "disclaimer in the documentation and/or other materials provided with the distribution. " +
      "Neither the name of Google Inc. nor the names of its contributors may be used to endorse " +
      "or promote products derived from this software without specific prior written permission. " +
      'THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY ' +
      "EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF " +
      "MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL " +
      "THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, " +
      "SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES ARISING IN ANY WAY OUT OF THE USE OF THIS " +
      "SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.",
  },
  {
    title: "Perfis de cor ICC",
    license: "CC0-1.0",
    text: "Os perfis ICC em files/icc/*-v4.icc do typst-assets estão em domínio público (CC0 1.0 Universal).",
  },
];

/** Usado quando o núcleo não responde `compiler_info`: as fontes embutidas são fixas (D-15). */
const FALLBACK_FONTS: FontLicense[] = [
  {
    family: "Libertinus (Serif, Sans, Math, Mono)",
    file: "src-tauri/fonts/Libertinus*.otf",
    license: "OFL-1.1",
    copyright:
      'Copyright © 2012-2024 The Libertinus Project Authors, with Reserved Font Name "Linux Libertine", "Biolinum", "STIX Fonts".',
  },
];

export interface AboutOptions {
  ipc: Ipc;
  /** Versões do projeto aberto, usadas se o núcleo não tiver `compiler_info`. */
  compiler: () => CompilerVersions | null;
  /** Ação explícita "atualizar o projeto para a versão embutida" (FR-012). */
  onUpdateVersion?: () => void | Promise<void>;
}

export interface AboutHandle {
  readonly element: HTMLElement;
  open(): Promise<void>;
  close(): void;
  isOpen(): boolean;
}

function el<K extends keyof HTMLElementTagNameMap>(tag: K, text?: string, cls?: string): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (text !== undefined) e.textContent = text;
  if (cls) e.className = cls;
  return e;
}

export function mountAbout(parent: HTMLElement, opts: AboutOptions): AboutHandle {
  const backdrop = el("div", undefined, "ws-backdrop");
  backdrop.hidden = true;
  const dialog = el("div", undefined, "ws-modal ws-about");
  dialog.setAttribute("role", "dialog");
  dialog.setAttribute("aria-modal", "true");
  dialog.setAttribute("aria-labelledby", "ws-about-title");
  dialog.tabIndex = -1;
  dialog.hidden = true;
  backdrop.append(dialog);
  parent.append(backdrop);
  let lastFocus: Element | null = null;

  function close() {
    dialog.hidden = true;
    backdrop.hidden = true;
    if (lastFocus instanceof HTMLElement) lastFocus.focus();
  }

  dialog.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      e.preventDefault();
      close();
    }
  });
  backdrop.addEventListener("click", (e) => {
    if (e.target === backdrop) close();
  });

  function render(info: CompilerInfo | null) {
    const project = opts.compiler();
    const embedded = info?.embedded ?? project?.embedded ?? "—";
    const registered = info ? info.registered : (project?.registered ?? null);
    const status = compilerStatus({ embedded, registered });

    const title = el("h2", "Sobre o Prelo");
    title.id = "ws-about-title";

    const compiler = el("section");
    compiler.append(el("h3", "Compilador Typst"));
    const dl = el("dl", undefined, "ws-dl");
    dl.append(el("dt", "Embutido"), el("dd", embedded));
    const regText = registered ?? (project || info ? "ainda não registrado" : "nenhum projeto aberto");
    dl.append(el("dt", "Registrado no projeto"), el("dd", regText));
    compiler.append(dl);
    compiler.append(
      el(
        "p",
        "O Prelo compila com o Typst embutido; não precisa de typst instalado no computador.",
        "ws-muted",
      ),
    );
    if (status.mismatch) {
      const warn = el(
        "p",
        `Este projeto foi registrado com o Typst ${registered}, e o Prelo usa o ${embedded}. A prévia e a exportação ficam desativadas até você atualizar o projeto.`,
        "ws-warn",
      );
      warn.setAttribute("role", "alert");
      compiler.append(warn);
      if (opts.onUpdateVersion) {
        const btn = el("button", `Atualizar o projeto para Typst ${embedded}`, "btn primary");
        btn.type = "button";
        btn.addEventListener("click", async () => {
          btn.disabled = true;
          try {
            await opts.onUpdateVersion?.();
            close();
          } finally {
            btn.disabled = false;
          }
        });
        compiler.append(btn);
      }
    }

    const fonts = el("section");
    fonts.append(el("h3", "Fontes embutidas"));
    const table = el("table", undefined, "ws-fonts");
    const head = el("tr");
    for (const h of ["Família", "Arquivo", "Licença", "Copyright"]) head.append(el("th", h));
    table.append(head);
    const list = info && info.fonts.length > 0 ? info.fonts : FALLBACK_FONTS;
    for (const f of list) {
      const tr = el("tr");
      tr.append(el("td", f.family), el("td", f.file), el("td", f.license), el("td", f.copyright));
      table.append(tr);
    }
    fonts.append(table);
    const ofl = el("details");
    ofl.append(el("summary", "Texto da licença SIL Open Font License 1.1"), el("pre", oflText, "ws-license"));
    fonts.append(ofl);

    const notices = el("section");
    notices.append(el("h3", "Avisos de terceiros (dados do typst-assets)"));
    for (const n of TYPST_ASSETS_NOTICES) {
      const d = el("details");
      d.append(el("summary", `${n.title} · ${n.license}`), el("pre", n.text, "ws-license"));
      notices.append(d);
    }

    const closeBtn = el("button", "Fechar", "btn");
    closeBtn.type = "button";
    closeBtn.addEventListener("click", close);
    const footer = el("div", undefined, "ws-modal-actions");
    footer.append(closeBtn);

    dialog.replaceChildren(title, compiler, fonts, notices, footer);
  }

  return {
    element: dialog,
    async open() {
      lastFocus = document.activeElement;
      let info: CompilerInfo | null = null;
      try {
        info = await opts.ipc.compilerInfo();
      } catch (err) {
        // Núcleo sem `compiler_info`: mostra as versões do projeto e as licenças fixas.
        if (toIpcError(err).code !== "not_implemented") console.warn("sobre: compiler_info", err);
      }
      render(info);
      backdrop.hidden = false;
      dialog.hidden = false;
      dialog.focus();
    },
    close,
    isOpen: () => !dialog.hidden,
  };
}
