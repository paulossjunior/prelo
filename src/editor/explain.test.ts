// Testes da explicação dos erros em português (T098; FR-006a, cenário 13 de US1, emenda de
// 2026-11-23). Tabela: mensagem do Typst 0.15 (ou do núcleo do Prelo) → motivo e "Como
// corrigir". As mensagens são as do código do Typst 0.15.1 (typst-syntax, typst-library).
import { describe, expect, it } from "vitest";
import type { Diagnostic } from "../ipc";
import { explain } from "./explain";

type Row = [message: string, kind: Diagnostic["kind"], reason: RegExp, fix: RegExp];

const KNOWN: Row[] = [
  // Nome desconhecido (variável ou função).
  ["unknown variable: titulo", "typst", /`titulo` não existe/, /#let titulo/],
  ["unknown variable: a-b", "typst", /`a-b` não existe/, /grafia/],
  // Delimitadores e trechos sem fechar.
  ["unclosed delimiter", "typst", /aberto e não foi fechado/, /\)|\]|\}/],
  ["unclosed string", "typst", /texto entre aspas/, /"/],
  ["unclosed raw text", "typst", /código literal/, /`/],
  ["unclosed label", "typst", /rótulo/, />/],
  ["unexpected end of block comment", "typst", /\*\//, /\/\*/],
  // Esperado X, veio Y (tipos).
  ["expected integer, found string", "typst", /esperava um número inteiro, mas recebeu um texto/, /tipo/],
  ["expected length, found content", "typst", /esperava um comprimento, mas recebeu um conteúdo/, /tipo/],
  ["expected string or content, found none", "typst", /esperava um texto ou um conteúdo, mas recebeu `none`/, /tipo/],
  // Sintaxe: esperado algo / inesperado.
  ["expected expression", "typst", /faltou uma expressão/, /#/],
  ["expected comma or semicolon", "typst", /faltou uma vírgula ou um ponto e vírgula/, /./],
  ["expected semicolon or line break", "typst", /faltou um ponto e vírgula ou uma quebra de linha/, /./],
  ["unexpected closing bracket", "typst", /um `\]` sobrando/, /abre/],
  ["unexpected closing paren", "typst", /um `\)` sobrando/, /abre/],
  ["unexpected comma", "typst", /uma vírgula/, /./],
  // Arquivos e fontes.
  ["file not found (searched at /proj/fig/a.png)", "typst", /não existe.*\/proj\/fig\/a\.png/, /caminho/],
  ["unknown font family: garamond", "typst", /fonte `garamond`/, /\.ttf|\.otf/],
  // Rótulos e referências.
  ["label `<intro>` does not exist in the document", "typst", /`<intro>`.*não existe/, /<intro>/],
  ["label `<fig>` occurs multiple times in the document", "typst", /`<fig>`.*mais de um lugar/, /único/],
  // Operações entre tipos.
  ["cannot add integer and string", "typst", /somar um número inteiro e um texto/, /str\(/],
  ["cannot subtract string and integer", "typst", /subtrair um texto e um número inteiro/, /./],
  // Argumentos.
  ["missing argument: body", "typst", /argumento `body`/, /./],
  ["unexpected argument: size", "typst", /argumento `size`/, /documentação|nome/],
  ["unexpected argument", "typst", /argumento a mais/, /./],
  ["duplicate argument: fill", "typst", /`fill`.*duas vezes/, /um só/],
  // Métodos e campos.
  ["type string has no method `foo`", "typst", /`foo`/, /./],
  // Pacotes do Typst.
  ["package not found (searched for @preview/xyz:0.1.0)", "typst", /pacote.*@preview\/xyz:0\.1\.0/, /nome|versão/],
  ["package found, but version 9.9.9 does not exist (latest is 0.3.0)", "typst", /9\.9\.9/, /0\.3\.0/],
  ["failed to download package (connection refused)", "typst", /baixar o pacote/, /conexão/],
  // Recusas do núcleo do Prelo (mensagem já em português; a explicação diz o porquê).
  ["acesso recusado: `../../.ssh/id` fica fora da pasta do projeto", "access", /só lê arquivos dentro da pasta do projeto/, /[Cc]opie/],
  ["acesso recusado: `fig/x` não é um arquivo comum", "access", /arquivo comum/, /./],
  ["arquivo não encontrado: `fig/a.png`", "access", /não existe/, /caminho/],
  ["arquivo grande demais: `dados.csv` (limite 256 MB)", "access", /grande demais/, /./],
  ["plugin `calc.wasm` aguardando sua confirmação", "plugin", /aprovação/, /[Aa]prove/],
  ["plugin `calc.wasm` recusado neste projeto; ele não é executado", "plugin", /recusou/, /./],
  ["o pacote `@preview/a:0.1.0` foi alterado depois do download", "package", /alterado/, /Baixar de novo/],
  ["pacote `@preview/a:0.1.0` não está no computador e não há conexão", "package", /conexão/, /./],
  ["arquivo de fonte inválido: `fontes/x.ttf` (ignorado)", "font", /fonte/, /./],
];

describe("explain", () => {
  it.each(KNOWN)("%s", (message, kind, reason, fix) => {
    const e = explain({ message, kind, hints: [] });
    expect(e).not.toBeNull();
    expect(e!.reason).toMatch(reason);
    expect(e!.fix).toMatch(fix);
  });

  it("mensagem desconhecida não tem explicação (a interface mostra a original e as dicas)", () => {
    expect(explain({ message: "something entirely new happened", kind: "typst", hints: ["try x"] })).toBeNull();
    expect(explain({ message: "", kind: "typst", hints: [] })).toBeNull();
  });

  it("explicação é em português e não repete a mensagem em inglês", () => {
    const e = explain({ message: "unknown variable: foo", kind: "typst", hints: [] })!;
    expect(e.reason).not.toContain("unknown variable");
  });

  it("tipo sem tradução aparece como veio, entre crases", () => {
    const e = explain({ message: "expected frobnicator, found string", kind: "typst", hints: [] })!;
    expect(e.reason).toMatch(/`frobnicator`/);
  });

  it("não interpreta HTML: o texto volta como texto", () => {
    const e = explain({ message: "unknown variable: <img src=x>", kind: "typst", hints: [] })!;
    expect(e.reason).toContain("`<img src=x>`");
  });
});
