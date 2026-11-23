// Explicação dos erros em português (T099; FR-006a, cenário 13 de US1, emenda de 2026-11-23:
// "Ao achar um erro, identifique o erro e mostre o motivo do erro para o usuário acertar").
//
// Função pura sobre `Diagnostic.message` e `Diagnostic.kind`: o contrato IPC não muda e a
// mensagem original continua sendo mostrada (como texto secundário) por quem chama. As
// mensagens são as do Typst 0.15.1 (typst-syntax: parser e lexer; typst-library: escopo,
// conversão de tipos, operações, rótulos, arquivos, fontes e pacotes) e as recusas do núcleo
// do Prelo (src-tauri/src/fs/scope.rs, world/packages.rs, world/fonts.rs). Mensagem sem
// regra → `null`: a interface mostra a original com as dicas do Typst.
import type { Diagnostic } from "../ipc";

export interface Explanation {
  /** Motivo do erro, em português. */
  reason: string;
  /** O que fazer para corrigir ("Como corrigir"). */
  fix: string;
}

type Input = Pick<Diagnostic, "message" | "kind"> & { hints?: string[] };

/** Nomes de tipos e de elementos de sintaxe do Typst → português, com artigo. */
const TERMS: Record<string, string> = {
  // Tipos (mensagens "expected X, found Y" e "cannot add X and Y").
  integer: "um número inteiro",
  float: "um número decimal",
  "numeric value": "um número",
  string: "um texto",
  content: "um conteúdo",
  boolean: "um valor lógico (`true`/`false`)",
  length: "um comprimento",
  "relative length": "um comprimento relativo",
  ratio: "uma porcentagem",
  angle: "um ângulo",
  fraction: "uma fração (`fr`)",
  color: "uma cor",
  gradient: "um gradiente",
  alignment: "um alinhamento",
  array: "uma lista (array)",
  dictionary: "um dicionário",
  function: "uma função",
  label: "um rótulo",
  module: "um módulo",
  type: "um tipo",
  none: "`none`",
  auto: "`auto`",
  "`none`": "`none`",
  "`auto`": "`auto`",
  // Sintaxe (mensagens "expected X" e "unexpected X" do parser).
  expression: "uma expressão",
  identifier: "um nome (identificador)",
  comma: "uma vírgula",
  semicolon: "um ponto e vírgula",
  colon: "dois-pontos (`:`)",
  "line break": "uma quebra de linha",
  block: "um bloco (`{ … }` ou `[ … ]`)",
  pattern: "um padrão (nome ou desestruturação)",
  "argument list": "a lista de argumentos (`( … )`)",
  "keyword `in`": "a palavra `in`",
  "named or keyed pair": "um par `nome: valor`",
  "closing bracket": "um `]`",
  "closing paren": "um `)`",
  "closing brace": "um `}`",
  "opening bracket": "um `[`",
  "opening paren": "um `(`",
  "opening brace": "um `{`",
  "equals sign": "um `=`",
  hash: "um `#`",
  "dollar sign": "um `$`",
  dot: "um ponto",
  star: "um `*`",
  underscore: "um `_`",
  "keyword `else`": "a palavra `else`",
};

/** Termo traduzido; o que não está na tabela volta entre crases, como veio. */
function term(raw: string): string {
  const t = raw.trim();
  return TERMS[t] ?? (t.startsWith("`") ? t : `\`${t}\``);
}

/** "a or b" / "a, b, or c" → "x ou y" com cada termo traduzido. */
function terms(raw: string): string {
  const parts = raw.split(/,\s*(?:or\s+)?|\s+or\s+/).filter(Boolean);
  if (parts.length <= 1) return term(raw);
  const out = parts.map(term);
  return `${out.slice(0, -1).join(", ")} ou ${out.at(-1)}`;
}

const VERBS: Record<string, string> = {
  add: "somar",
  subtract: "subtrair",
  multiply: "multiplicar",
  divide: "dividir",
  join: "juntar",
  compare: "comparar",
};

type Rule = [RegExp, (m: RegExpMatchArray) => Explanation];

const TYPE_FIX =
  "Troque o valor pelo tipo esperado: números sem aspas (`12`), textos entre aspas (`\"abc\"`), " +
  "comprimentos com unidade (`12pt`, `2cm`, `1em`) e conteúdo entre colchetes (`[texto]`).";

/** Regras do Typst (inglês). A primeira que casar vale. */
const TYPST_RULES: Rule[] = [
  [
    /^unknown variable: (.+)$/,
    (m) => ({
      reason: `\`${m[1]}\` não existe neste ponto: não é uma variável nem uma função definida antes, nem faz parte do Typst.`,
      fix:
        `Confira a grafia (maiúsculas contam); defina antes de usar (\`#let ${m[1]} = …\`) ou importe o arquivo ou pacote que a define (\`#import\`). ` +
        "Se queria uma subtração, ponha espaços em volta do sinal: `a - b`.",
    }),
  ],
  [
    /^unclosed delimiter$/,
    () => ({
      reason: "Um parêntese, colchete ou chave foi aberto e não foi fechado.",
      fix: "Feche com o `)`, `]` ou `}` correspondente; o trecho marcado é onde ele foi aberto.",
    }),
  ],
  [
    /^unclosed string$/,
    () => ({
      reason: "Um texto entre aspas foi aberto e não foi fechado.",
      fix: 'Feche o texto com outra aspa dupla (`"`) na mesma linha.',
    }),
  ],
  [
    /^unclosed raw text$/,
    () => ({
      reason: "Um trecho de código literal (entre crases) foi aberto e não foi fechado.",
      fix: "Feche com a mesma quantidade de crases (`) usada para abrir.",
    }),
  ],
  [
    /^unclosed label$/,
    () => ({
      reason: "Um rótulo `<…>` foi aberto e não foi fechado.",
      fix: "Feche o rótulo com `>`, por exemplo `<intro>`; o nome não pode ter espaços.",
    }),
  ],
  [
    /^unclosed Unicode escape sequence$/,
    () => ({
      reason: "Um código Unicode `\\u{…}` foi aberto e não foi fechado.",
      fix: "Feche com `}`, por exemplo `\\u{1F600}`.",
    }),
  ],
  [
    /^unexpected end of block comment$/,
    () => ({
      reason: "Há um `*/` que fecha um comentário que não foi aberto.",
      fix: "Apague o `*/` sobrando ou abra o comentário com `/*` antes dele.",
    }),
  ],
  [
    /^expected (.+?), found (.+)$/,
    (m) => ({
      reason: `O Typst esperava ${terms(m[1]!)}, mas recebeu ${term(m[2]!)}.`,
      fix: TYPE_FIX,
    }),
  ],
  [
    /^expected (.+)$/,
    (m) => ({
      reason: `A escrita do código está incompleta: faltou ${terms(m[1]!)} aqui.`,
      fix:
        "Complete o trecho marcado. Depois de `#`, escreva uma expressão (`#nome`, `#func(…)`); " +
        "se queria o caractere literal, escape com barra invertida (`\\#`).",
    }),
  ],
  [
    /^unexpected (closing (?:bracket|paren|brace))$/,
    (m) => ({
      reason: `Há ${term(m[1]!)} sobrando: ele fecha algo que não foi aberto.`,
      fix: "Apague o caractere sobrando ou acrescente o que o abre antes dele; para o caractere literal, escape com `\\`.",
    }),
  ],
  [
    /^unexpected argument: (.+)$/,
    (m) => ({
      reason: `A função não aceita o argumento \`${m[1]}\`.`,
      fix: "Confira o nome do argumento na documentação da função (a grafia conta) ou remova-o.",
    }),
  ],
  [
    /^unexpected argument$/,
    () => ({
      reason: "A função recebeu um argumento a mais, que ela não aceita.",
      fix: "Remova o argumento sobrando ou dê nome a ele (`nome: valor`), conforme a documentação da função.",
    }),
  ],
  [
    /^unexpected (.+)$/,
    (m) => ({
      reason: `O Typst encontrou ${term(m[1]!)} num lugar onde não cabe.`,
      fix: "Apague ou mova o trecho marcado; para o caractere literal no texto, escape com `\\`.",
    }),
  ],
  [
    /^file not found[ ,(]+searched at (.+?)\)?$/,
    (m) => ({
      reason: `O arquivo pedido não existe no projeto (procurado em ${m[1]}).`,
      fix:
        "Confira o caminho e a grafia, inclusive maiúsculas e a extensão. O caminho é relativo ao arquivo " +
        "atual; começando com `/`, é relativo à raiz do projeto. Se o arquivo falta, copie-o para a pasta do projeto.",
    }),
  ],
  [
    /^unknown font family: (.+)$/,
    (m) => ({
      reason: `A fonte \`${m[1]}\` não está disponível: não vem com o Prelo nem está na pasta do projeto. O texto sai em outra fonte.`,
      fix: "Confira o nome da família, ou ponha os arquivos da fonte (`.ttf` ou `.otf`) dentro da pasta do projeto.",
    }),
  ],
  [
    /^label `(<[^`]+>)` does not exist in the document$/,
    (m) => ({
      reason: `A referência aponta para o rótulo \`${m[1]}\`, que não existe no documento nem na bibliografia.`,
      fix: `Confira a grafia; ponha \`${m[1]}\` logo depois do título, figura ou equação que quer citar, ou acrescente a entrada à bibliografia.`,
    }),
  ],
  [
    /^label `(<[^`]+>)` occurs multiple times in the document$/,
    (m) => ({
      reason: `O rótulo \`${m[1]}\` aparece em mais de um lugar, e a referência não sabe para qual apontar.`,
      fix: "Dê um nome único a cada rótulo.",
    }),
  ],
  [
    /^label `(<[^`]+>)` occurs both in the document and a bibliography$/,
    (m) => ({
      reason: `O rótulo \`${m[1]}\` existe no documento e também é uma chave da bibliografia.`,
      fix: "Renomeie o rótulo do documento ou a chave da bibliografia para os dois serem diferentes.",
    }),
  ],
  [
    /^cannot (add|subtract|multiply|divide|join|compare) (.+?) (?:and|with) (.+)$/,
    (m) => ({
      reason: `Não dá para ${VERBS[m[1]!]} ${term(m[2]!)} e ${term(m[3]!)}: os tipos não combinam.`,
      fix:
        "Converta um dos lados antes: `str(n)` vira texto, `int(\"3\")` vira número; para juntar texto a conteúdo, use `[#a #b]`.",
    }),
  ],
  [
    /^missing argument: (.+)$/,
    (m) => ({
      reason: `A função precisa do argumento \`${m[1]}\`, que não foi passado.`,
      fix: `Acrescente \`${m[1]}\` na chamada; argumentos posicionais vão na ordem da documentação da função.`,
    }),
  ],
  [
    /^duplicate argument: (.+)$/,
    (m) => ({
      reason: `O argumento \`${m[1]}\` foi passado duas vezes na mesma chamada.`,
      fix: "Deixe um só e apague o repetido.",
    }),
  ],
  [
    /^type (.+) has no method `(.+)`$|^(.+) has no method `(.+)`$/,
    (m) => ({
      reason: `O método \`${m[2] ?? m[4]}\` não existe para ${term(m[1] ?? m[3] ?? "")}.`,
      fix: "Confira o nome do método e o tipo do valor na documentação do Typst.",
    }),
  ],
  [
    /^package not found \(searched for (.+)\)$/,
    (m) => ({
      reason: `O pacote ${m[1]} não foi encontrado.`,
      fix: "Confira o nome e a versão no `#import` (por exemplo `@preview/nome:0.1.0`).",
    }),
  ],
  [
    /^package found, but version (.+) does not exist \(latest is (.+)\)$/,
    (m) => ({
      reason: `O pacote existe, mas não na versão ${m[1]}.`,
      fix: `Use uma versão que existe no \`#import\`; a mais recente é ${m[2]}.`,
    }),
  ],
  [
    /^failed to download package/,
    () => ({
      reason: "Não foi possível baixar o pacote.",
      fix: "Confira a conexão com a internet e compile de novo; depois de baixado, o pacote funciona sem rede.",
    }),
  ],
];

/** Regras das mensagens do núcleo do Prelo (já em português): explicam o porquê. */
const PRELO_RULES: Rule[] = [
  [
    /^acesso recusado: .* fica fora da pasta do projeto$/,
    () => ({
      reason: "Por segurança, o Prelo só lê arquivos dentro da pasta do projeto, e este caminho sai dela.",
      fix: "Copie o arquivo para dentro da pasta do projeto e use o caminho relativo a ela.",
    }),
  ],
  [
    /^acesso recusado: .* não é um arquivo comum$/,
    () => ({
      reason: "O caminho aponta para algo que não é um arquivo comum (por exemplo, um dispositivo ou atalho especial).",
      fix: "Use um arquivo comum dentro da pasta do projeto.",
    }),
  ],
  [
    /^arquivo não encontrado: /,
    () => ({
      reason: "O arquivo pedido não existe na pasta do projeto.",
      fix: "Confira o caminho e a grafia, inclusive maiúsculas e a extensão, ou copie o arquivo para a pasta do projeto.",
    }),
  ],
  [
    /^(?:arquivo|estilo) grande demais: /,
    () => ({
      reason: "O arquivo é grande demais para o Prelo ler com segurança.",
      fix: "Reduza o arquivo (por exemplo, divida os dados ou comprima a imagem).",
    }),
  ],
  [
    /^plugin .* aguardando sua confirmação$/,
    () => ({
      reason: "Plugins (`.wasm`) executam código, por isso o Prelo pede sua aprovação antes de rodar um.",
      fix: "Aprove o plugin na janela que o Prelo mostrou, se você confia na origem do projeto.",
    }),
  ],
  [
    /^plugin .* recusado neste projeto/,
    () => ({
      reason: "Você recusou este plugin neste projeto, então ele não é executado.",
      fix: "Remova o uso do plugin do documento, ou reabra o projeto para decidir de novo.",
    }),
  ],
  [
    /^o pacote .* foi alterado depois do download$/,
    () => ({
      reason: "O pacote guardado no computador foi alterado depois do download e não é mais confiável.",
      fix: "Use \"Baixar de novo\" na faixa de aviso para trocar pela cópia original.",
    }),
  ],
  [
    /^pacote .* não está no computador e não há conexão$/,
    () => ({
      reason: "O pacote ainda não foi baixado e não há conexão com a internet.",
      fix: "Conecte-se à internet e compile de novo; depois de baixado, o pacote funciona sem rede.",
    }),
  ],
  [
    /^o pacote .* foi recusado: /,
    () => ({
      reason: "O pacote baixado não passou na validação do Prelo.",
      fix: "Confira o nome e a versão do pacote no `#import`, ou use outra versão.",
    }),
  ],
  [
    /^arquivo de fonte inválido: /,
    () => ({
      reason: "O arquivo de fonte do projeto está corrompido ou não é uma fonte, e foi ignorado.",
      fix: "Troque-o por um `.ttf` ou `.otf` válido, ou apague-o do projeto.",
    }),
  ],
];

/** Explica `d` em português; `null` quando a mensagem não é conhecida. */
export function explain(d: Input): Explanation | null {
  const message = d.message.trim();
  if (!message) return null;
  const rules = d.kind === "typst" ? [...TYPST_RULES, ...PRELO_RULES] : [...PRELO_RULES, ...TYPST_RULES];
  for (const [re, make] of rules) {
    const m = message.match(re);
    if (m) return make(m);
  }
  return null;
}
