# Research: Comentários (004)

Fase 0 do plano. Cada item: decisão, motivo, alternativas. Itens marcados **spike** precisam
de um teste de viabilidade antes das tarefas que dependem deles (ver tasks.md).

## R1 Documento CRDT no núcleo: `yrs`

- **Decisão**: `yrs = "=0.28.2"` (MIT) no núcleo Rust. Um `yrs::Doc` por arquivo `.typ`
  aberto, com um `TextRef` chamado `"content"`, criado com `OffsetKind::Utf16` para que os
  deslocamentos de `edit_file` (UTF-16, contracts/ipc.md da 003) entrem sem conversão.
- **Motivo**: o texto já vive no núcleo (`edit_file` aplica deltas `{from,to,insert}`). Aplicar
  os mesmos deltas no `TextRef` mantém o documento CRDT idêntico ao texto do World sem tocar
  na interface. A R3 reaproveita o mesmo `Doc` para rede (decisão Q-07: sem migração).
- **Alternativas**: Yjs na interface (JS) — descartado: o núcleo é a fonte do texto e da
  validação (Princípio VI); duplicaria o estado. Automerge — descartado: a constituição fixa
  Yjs/yrs (Restrições Técnicas).

## R2 Âncoras: `StickyIndex`

- **Decisão**: início e fim de cada conversa são `yrs::StickyIndex` (posição relativa do Yjs),
  início com `Assoc::After` e fim com `Assoc::Before`, serializados em base64 do encoding v1
  do próprio yrs. Resolver: `StickyIndex::get_offset` → deslocamento UTF-16.
- **Trecho removido**: se início ≥ fim após resolver, ou o item foi apagado e o deslocamento
  colapsou, a conversa fica com `detached = true` (condição derivada, não gravada como estado).
- **Citação**: cada conversa guarda `quote` (até 500 caracteres do trecho no momento da
  criação) para exibir no painel e para reancorar à mão.
- **Alternativas**: deslocamento + texto de contexto (re-busca) — descartado pelo dono (Q-07).

## R3 Persistência do estado CRDT

- **Decisão**: `.prelo/crdt/<id>.yjs`, onde `<id>` é o sha256 (hex, 16 primeiros bytes) do
  caminho relativo normalizado do arquivo. Conteúdo: cabeçalho JSON de uma linha
  (`{"schema":1,"path":"cap/intro.typ"}`) + `\n` + estado v1 do `Doc`
  (`encode_state_as_update_v1`). Gravação atômica (tempfile + rename, como `fs/save.rs`), no
  mesmo ciclo do salvamento automático do `.typ` e antes de confirmar qualquer ação de
  comentário que dependa de uma âncora nova.
- **Limite**: 32 MiB por arquivo de estado; acima disso → aviso e âncoras daquele arquivo
  viram "trecho removido" sem perder conversas.
- **Compactação**: ao abrir, se o estado tiver mais de 4× o tamanho do texto, regrava com
  `Doc` novo só se não houver conversas ancoradas nele (âncoras dependem dos ids dos itens).
- **Spec 005**: estados v1 de dois coautores combinam aplicando os dois updates (união
  comutativa do Yjs). A regra de combinação fica na 005; aqui só garantimos que o arquivo é um
  update v1 válido.
- **Alternativas**: guardar só âncoras e recriar o `Doc` do texto a cada abertura — descartado:
  `StickyIndex` referencia ids de itens, que não sobrevivem a um `Doc` recriado.

## R4 Reconciliação com o disco (FR-008)

- **Decisão**: ao abrir um `.typ` ou em `resolve_external_change { keep: "disk" }`, se o texto
  do disco ≠ texto do `Doc`, calcular diff por caracteres com `similar = "=3.2.1"`
  (Apache-2.0; algoritmo Myers, `TextDiff::from_chars` com limite de tempo de 200 ms; acima,
  cai para diff por linhas) e aplicar as operações no `TextRef` como uma transação.
  Trechos preservados mantêm os itens e, portanto, as âncoras.
- **Sem estado CRDT** (projeto antigo ou arquivo apagado): cria `Doc` do texto do disco;
  conversas daquele arquivo ficam "trecho removido".
- **Alternativas**: substituir o texto inteiro — descartado: apagaria todos os itens e
  descolaria todas as âncoras.

## R5 Arquivo `.prelo/review.json`

- **Decisão**: JSON com `schema: 1` e `threads` como **objeto indexado por id** (não lista),
  para que a combinação da 005 seja por chave. Ids: 128 bits aleatórios em hex (`getrandom`,
  já na árvore). Mensagens com id próprio dentro da conversa. Detalhes em data-model.md.
- **Limites (FR-030)**: arquivo ≤ 16 MiB; ≤ 5.000 conversas; ≤ 500 mensagens por conversa;
  mensagem ≤ 10.000 caracteres (FR-021); nome ≤ 100 caracteres; `quote` ≤ 500; ids
  `^[0-9a-f]{32}$`; datas RFC 3339 UTC; `file` relativo, sem `..`, absoluto, byte nulo ou
  barra invertida, e dentro da raiz pela regra de escopo da 003 (file-access.md).
- **Leitura defensiva**: mesmo padrão de `project/record.rs`: JSON inválido, schema
  desconhecido ou tipo errado no topo → `review_json_invalid`, nada é gravado até a pessoa
  agir; antes da primeira gravação nova, cópia para
  `.prelo/review.json.invalid-<AAAAMMDDThhmmssZ>` (FR-031). Conversa individual inválida →
  guardada como JSON bruto e regravada intacta (FR-032). `schema` > 1 → só leitura (FR-034).
- **Gravação atômica** (FR-024): tempfile no mesmo diretório + `fsync` + rename; a resposta do
  comando IPC só volta depois do rename (FR-023).

## R6 Identidade local e dono

- **Decisão**: nome de exibição em `<config do app>/settings.json` (`dirs::config_dir()`,
  pasta `Prelo`), campo `displayName`. Dono em `.prelo/project.json`, campo novo
  `owner: { name }` (o registro já preserva campos desconhecidos; passa a ler `owner`).
- **Permissões no núcleo** (FR-020): todo comando de comentário recebe o autor do
  `settings.json` lido pelo núcleo, nunca da interface. Assim a interface não pode pedir uma
  ação "em nome de" outra pessoa.
- **Limite aceito**: sem prova criptográfica até a R3 (spec, Assumptions).

## R7 Menções

- **Decisão**: no texto gravado a menção é `@{nome}` e a mensagem lista `mentions: [nome]`.
  Nomes conhecidos = autores em `review.json` ∪ dono ∪ nome local. Nome desconhecido fica
  como texto. Autocompletar na interface com `@codemirror/autocomplete` numa caixa de texto
  CodeMirror mínima (já é dependência).

## R8 Interface

- **Decisão**: marcas no editor como `Decoration.mark` do CodeMirror num `StateField`
  alimentado pelo evento `ReviewUpdate` (faixas UTF-16 resolvidas pelo núcleo). Painel de
  revisão à direita da prévia, recolhível como os painéis da 003 (`panes.ts`), com abas
  "Abertos" e "Resolvidos". Texto das mensagens inserido com `textContent` (FR-014).
- **Desempenho**: o núcleo só manda faixas do arquivo aberto; a interface aplica decorações
  por diff de ids.

## R9 PDF com comentários (RF-43) — **spike**

- **Decisão**: duas passagens, sem mudar o leiaute do documento (SC-001):
  1. Compilar o documento normal e exportar o PDF em memória. Achar a posição de cada âncora
     aberta com o mesmo caminho do `jump_from_cursor` da 003 (`typst-ide`, span → posição na
     página).
  2. Compilar um documento sintético em memória (nunca gravado no projeto) que, para cada
     página `n`, coloca `image(pdf_bytes, page: n)` à esquerda e uma coluna de notas de 6 cm à
     direita, com linha ligando cada nota à altura do trecho. Texto das notas entra como
     string Typst escapada (`str` em `#let`, nunca como marcação; FR-014).
- **Spike**: confirmar que `image` com PDF e parâmetro `page` funciona no typst 0.15.1 com
  fidelidade vetorial. **Fallback**: SVG da página (`typst-svg`, já usado na prévia) como
  imagem; perde texto selecionável no PDF de comentários, aceitável para leitura.
- **Colisão de notas**: notas empilham em ordem de y; se não couberem na coluna da página,
  continuam numa página de continuação com referência numerada.
- **Alternativas**: injetar notas de margem no fonte (`place` no trecho) — descartado: muda o
  leiaute e pode quebrar código/math; `set page` do usuário sobrepõe o nosso.

## R10 Outline com contagem (FR-013)

- **Decisão**: o `outline` da 003 já dá os títulos com posição de origem. O núcleo devolve
  `openComments` em cada `OutlineItem`: conta conversas abertas, não destacadas, cujo início
  resolvido fica entre o título e o próximo título de nível igual ou menor no mesmo arquivo.
  Conversas num arquivo incluído por `#include` contam na seção do próprio arquivo incluído,
  não na seção onde o `#include` aparece (limitação documentada desta fase).

## R11 Testes

- Rust: testes unitários em `src-tauri/src/review/*` e integração em `src-tauri/tests/`
  (`review_anchor.rs`, `review_json_security.rs`, `review_crash.rs` com processo filho morto
  por `SIGKILL`/`TerminateProcess`, `review_pdf.rs`, `review_typ_pure.rs` comparando com o
  `typst` CLI como em `equivalence` da 003).
- Propriedade (SC-003): 200 edições aleatórias com semente fixa; sem nova dependência
  (gerador simples com `getrandom` semeado).
- TS: vitest/jsdom para painel, decorações e menções; e2e wdio para criar/resolver.
- Auditoria: `cargo deny`/`cargo audit` e `license-checker` devem aceitar `yrs` (MIT) e
  `similar` (Apache-2.0).
