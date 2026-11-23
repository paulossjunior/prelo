// Painel "Erros" (T099; FR-006a, cenário 13 de US1, emenda de 2026-11-23): todos os
// diagnósticos da última compilação, com ícone de gravidade, `arquivo:linha:coluna`, o motivo
// em português e "Como corrigir" (src/editor/explain.ts), e a mensagem original do Typst com as
// dicas como texto secundário. Fica recolhido abaixo da área de trabalho, acima da barra de
// status; a contagem na barra de status e a faixa de erro da prévia o abrem. Todo texto entra
// por `textContent` (mensagens vêm de arquivos do projeto e não são HTML).
import type { Diagnostic } from "../ipc";
import { explain } from "./explain";
import { severityIcon } from "./icons";

export interface Location {
  line: number;
  /** Coluna a partir de 1, em unidades UTF-16; só quando o texto do arquivo é conhecido. */
  column?: number;
}

/**
 * Linha e coluna de `d`. Com o texto atual do arquivo (aberto no editor), calcula as duas a
 * partir de `range.from`; sem ele, usa a linha que o núcleo mandou.
 */
export function location(d: Diagnostic, text: string | null): Location | null {
  if (text !== null && d.range && Number.isFinite(d.range.from)) {
    const at = Math.min(Math.max(0, Math.trunc(d.range.from)), text.length);
    const before = text.slice(0, at);
    const lineStart = before.lastIndexOf("\n") + 1;
    let line = 1;
    for (let i = 0; i < lineStart; i++) if (before.charCodeAt(i) === 10) line++;
    return { line, column: at - lineStart + 1 };
  }
  if (typeof d.line === "number" && Number.isFinite(d.line)) return { line: Math.trunc(d.line) };
  return null;
}

/** Pacote baixado (`@ns/nome:versão`) e o arquivo dentro dele. */
export interface PackagePath {
  spec: string;
  file: string;
}

/**
 * `@preview/nome:0.1.0/lib.typ` → `{ spec: "@preview/nome:0.1.0", file: "lib.typ" }`; caminho
 * do projeto → `null`. Arquivos de pacote não abrem no editor (o núcleo só abre os do projeto).
 */
export function packagePath(path: string): PackagePath | null {
  const m = /^(@[^/]+\/[^/:]+:[^/]+)\/(.+)$/.exec(path);
  return m ? { spec: m[1]!, file: m[2]! } : null;
}

/** Caminho que não deve ir ao `open_file`: tudo que começa com `@` é de pacote. */
export function isPackagePath(path: string): boolean {
  return path.startsWith("@");
}

function span(cls: string, text: string): HTMLSpanElement {
  const s = document.createElement("span");
  s.className = cls;
  s.textContent = text;
  return s;
}

/**
 * Motivo, "Como corrigir", original e dicas de `d` (os mesmos no painel e no tooltip do
 * sublinhado). Mensagem desconhecida: a original vira o motivo, com as dicas.
 */
export function explanationParts(d: Diagnostic): HTMLSpanElement[] {
  const e = explain(d);
  const parts: HTMLSpanElement[] = [];
  if (e) {
    parts.push(span("pb-reason", e.reason));
    const fix = span("pb-fix", "");
    const b = document.createElement("b");
    b.textContent = "Como corrigir:";
    fix.append(b, ` ${e.fix}`);
    parts.push(fix);
    parts.push(span("pb-orig", `${d.kind === "typst" ? "Typst" : "Prelo"}: ${d.message}`));
  } else {
    parts.push(span("pb-reason", d.message));
  }
  for (const h of d.hints) parts.push(span("pb-hint", `Dica: ${h}`));
  return parts;
}

/** Bloco da explicação para o tooltip do sublinhado no editor. */
export function explanationNode(d: Diagnostic): HTMLElement {
  const box = document.createElement("div");
  box.className = "pb-tip";
  box.append(...explanationParts(d));
  return box;
}

function plural(n: number, one: string, many: string): string {
  return `${n} ${n === 1 ? one : many}`;
}

/** Frase da contagem para a região viva e o rótulo do painel. */
export function countText(diags: Diagnostic[]): string {
  const errors = diags.filter((d) => d.severity === "error").length;
  const warnings = diags.length - errors;
  if (diags.length === 0) return "Nenhum erro no documento.";
  const parts: string[] = [];
  if (errors) parts.push(plural(errors, "erro", "erros"));
  if (warnings) parts.push(plural(warnings, "aviso", "avisos"));
  return `${parts.join(" e ")} no documento.`;
}

export interface ProblemsOptions {
  /** Clique num item com arquivo: abrir o arquivo com o cursor no trecho. */
  onSelect(d: Diagnostic): void;
  /** Texto atual de `path` se estiver aberto no editor (para a coluna); senão `null`. */
  textOf?(path: string): string | null;
}

export interface ProblemsHandle {
  readonly element: HTMLElement;
  /** Troca a lista pelos diagnósticos da compilação mais recente. */
  update(diags: Diagnostic[]): void;
  /** Recalcula os locais (ex.: outro arquivo aberto no editor passa a dar a coluna). */
  refresh(): void;
  /** Abre o painel e põe o foco no primeiro item. */
  open(): void;
  setOpen(open: boolean): void;
  isOpen(): boolean;
  destroy(): void;
}

let seq = 0;

export function mountProblems(parent: HTMLElement, opts: ProblemsOptions): ProblemsHandle {
  const root = document.createElement("section");
  root.className = "pb";
  root.setAttribute("aria-label", "Erros");

  const head = document.createElement("div");
  head.className = "pb-h";
  const toggle = document.createElement("button");
  toggle.type = "button";
  toggle.className = "pb-toggle";
  toggle.setAttribute("aria-expanded", "false");
  const count = span("pb-count", "0");
  toggle.append(span("pb-title", "Erros"), count);
  head.append(toggle);

  const live = document.createElement("div");
  live.className = "pb-live";
  live.setAttribute("role", "status");
  live.setAttribute("aria-live", "polite");

  const list = document.createElement("ul");
  list.className = "pb-list";
  list.id = `pb-list-${++seq}`;
  list.hidden = true;
  toggle.setAttribute("aria-controls", list.id);

  root.append(head, live, list);
  parent.append(root);

  let diags: Diagnostic[] = [];
  let lastCount: string | null = null;

  function render() {
    const errors = diags.filter((d) => d.severity === "error").length;
    count.textContent = String(diags.length);
    root.classList.toggle("has-errors", errors > 0);
    root.classList.toggle("has-warnings", errors === 0 && diags.length > 0);
    toggle.title = countText(diags);
    if (diags.length === 0) {
      const li = document.createElement("li");
      li.className = "pb-empty";
      li.textContent = "Nenhum erro.";
      list.replaceChildren(li);
      return;
    }
    list.replaceChildren(
      ...diags.map((d) => {
        const li = document.createElement("li");
        // Arquivo de pacote baixado: mostra onde está, mas não é link (não abre no editor).
        const inPackage = d.path ? isPackagePath(d.path) : false;
        const loc = d.path ? location(d, inPackage ? null : (opts.textOf?.(d.path) ?? null)) : null;
        let where = d.path ? [d.path, loc?.line, loc?.column].filter((x) => x !== undefined).join(":") : "";
        if (d.path && inPackage) {
          const pkg = packagePath(d.path);
          const file = pkg ? [pkg.file, loc?.line].filter((x) => x !== undefined).join(":") : where;
          where = pkg ? `pacote ${pkg.spec} · ${file}` : `pacote ${where}`;
        }
        const item = d.path && !inPackage ? document.createElement("button") : document.createElement("div");
        item.className = `pb-item is-${d.severity}${inPackage ? " is-package" : ""}`;
        const icon = severityIcon(d.severity === "error" ? "error" : "warning");
        icon.classList.add("pb-icon");
        const body = span("pb-body", "");
        if (where) body.append(span("pb-loc", where));
        body.append(...explanationParts(d));
        item.append(icon, body);
        if (item instanceof HTMLButtonElement) {
          item.type = "button";
          const sev = d.severity === "error" ? "Erro" : "Aviso";
          const pos = [loc ? `linha ${loc.line}` : null, loc?.column ? `coluna ${loc.column}` : null].filter(Boolean);
          const reason = explain(d)?.reason ?? d.message;
          item.setAttribute("aria-label", `${sev} em ${[d.path, ...pos].join(", ")}: ${reason}`);
          item.addEventListener("click", () => opts.onSelect(d));
        }
        li.append(item);
        return li;
      }),
    );
  }

  function setOpen(open: boolean) {
    list.hidden = !open;
    toggle.setAttribute("aria-expanded", String(open));
    root.classList.toggle("is-open", open);
  }

  toggle.addEventListener("click", () => setOpen(toggle.getAttribute("aria-expanded") !== "true"));
  render();

  return {
    element: root,
    update(next) {
      diags = next;
      render();
      // Só a mudança da contagem é anunciada (não cada recompilação enquanto se digita).
      const text = countText(diags);
      if (text !== lastCount) {
        live.textContent = text;
        lastCount = text;
      }
    },
    refresh: render,
    open() {
      setOpen(true);
      list.querySelector<HTMLElement>(".pb-item")?.focus();
    },
    setOpen,
    isOpen: () => !list.hidden,
    destroy() {
      root.remove();
    },
  };
}
