# Research: Editor local (fase 1)

**Feature**: [spec.md](spec.md) · **Date**: 2026-11-19

Versões consultadas em 2026-11-19 (crates.io e npm). Fixar no lockfile; atualizar por PR. A
árvore de dependências Rust candidata foi resolvida num crate de rascunho (490 crates) para
levantar licenças antes do plano; o resultado está em R12.

## R1. Compilador embutido e versão

- **Decision**: `typst` 0.15.1 (com `typst-library`, `typst-syntax`, `typst-svg`, `typst-pdf`,
  `typst-ide` na mesma versão, todos Apache-2.0). `.prelo/project.json` registra
  `"typst": "0.15.1"`. Versão exibida no app vem de uma constante gerada no build a partir do
  `Cargo.lock`, e um teste confere que ela é igual à do lockfile.
- **Rationale**: é a versão estável atual; a partir da 0.14.0 o Typst aceita PDF e WebP como
  imagem, então FR-026 entra completo (fecha TODO(versão-typst) de RF-50). Rust mínimo do
  `typst` é 1.92; o projeto já fixa 1.95.
- **Alternatives considered**: 0.14.x (perde correções; nenhuma vantagem); chamar o `typst`
  instalado (descartado por D-10).

## R2. Serviço de linguagem (`typst-ide` oficial; Tinymist para World, VFS e pacotes)

> **Revisado em 2026-11-19 (D-17).** O `tinymist-query` 0.15.x publicado não compila com o
> `typst` 0.15.1 oficial: o `tinymist-analysis` usa `typst::foundations::FuncInner` e
> `Func::inner()`, que só existem no *fork* do Typst do Tinymist, e o `typlite` 0.15.12 não
> compila contra `tinymist-world` 0.15.0. O dono decidiu (D-17) usar o `typst-ide` oficial
> para a análise do editor e manter o Tinymist só para World, VFS e pacotes.

- **Decision**: `tinymist-world`, `tinymist-vfs` e `tinymist-package` 0.15.0 (Apache-2.0),
  **sem** as features `system`, `fonts`, `http-registry`. O núcleo implementa:
  - `PathAccessModel` (trait de `tinymist-vfs`): único ponto de leitura de disco, com as regras
    de escopo de RF-08 ([contracts/file-access.md](contracts/file-access.md));
  - `PackageRegistry` (trait de `tinymist-package`): resolve `@preview` para o cache próprio do
    Prelo (R7);
  - resolvedor de fontes alimentado só por fontes embutidas e `fonts/` do projeto (R6).
  Os textos abertos no editor entram como *overlay* em memória do VFS; o disco só é lido para o
  que não está aberto.
  `typst-ide` 0.15.1 fornece autocompletar (`autocomplete`), *hover* (`tooltip`),
  `jump_from_click` e `jump_from_cursor` (R5), sobre o mesmo World da prévia. Diagnósticos
  vêm da própria compilação. O realce sai de `typst-syntax` (`LinkedNode` + `highlight`, que
  devolve a `Tag` de cada nó), convertido em *tokens* pela camada `lang_service.rs`.
- **Versão travada**: `tinymist-*` com requisito exato no `Cargo.toml` (`=0.15.0`), assim como
  `typst-*` (`=0.15.1`). Atualizar só por PR com a suíte completa verde (decisão do dono,
  2026-11-19).
- **Rationale**: D-10 quer prévia, diagnósticos e autocompletar do mesmo World e do mesmo
  compilador oficial, e SC-005 exige PDF igual ao do `typst` CLI. `typst-ide` é publicado
  junto com o `typst`, na mesma versão. As traits `PathAccessModel` e `PackageRegistry`
  deixam a política de acesso inteira no núcleo do Prelo, sem *fork*. A constituição pede
  nova D-xx para mudança de componente: D-17.
- **Alternatives considered**: rodar o servidor LSP `tinymist` como processo separado (exige
  binário extra por sistema, a política de acesso ficaria fora do Prelo e o World não seria o
  mesmo da prévia); `tinymist-query` 0.15.12 (não compila com o `typst` oficial; ver acima);
  `[patch]` com o *fork* git do Typst (compilador deixaria de ser o oficial e a auditoria
  proíbe fontes git).

## R3. Editor de código

- **Decision**: CodeMirror 6 (`@codemirror/state` 6.7, `view` 6.43, `commands` 6.11,
  `language` 6.13, `autocomplete` 6.20, `lint` 6.9, `search` 6.7; todos MIT), interface em
  TypeScript sem framework de UI. Realce por *tokens* vindos do núcleo (`typst-syntax`,
  D-17) aplicados como decorações; diagnósticos pelo `@codemirror/lint`;
  autocompletar pelo `@codemirror/autocomplete` com fonte que chama o núcleo.
- **Rationale**: CodeMirror 6 está na constituição e é a base de `y-codemirror.next` da fase 3.
  Realce pelo núcleo usa o mesmo analisador do compilador, sem gramática JS paralela que
  divergiria. A tela tem quatro painéis simples (árvore, editor, prévia, estrutura); um
  framework não paga o custo agora, e adiar mantém a decisão de 001 R2 aberta sem D-xx.
- **Alternatives considered**: pacote npm de gramática Typst para CodeMirror (segundo
  analisador, licença e manutenção a verificar); Monaco (fora da stack); framework de UI
  (Svelte/Solid/Vue) — adiado para a fase de revisão, que tem painel com estado rico.

## R4. Prévia

- **Decision**: SVG por página com `typst-svg`. Depois de cada compilação o núcleo calcula um
  hash por página (`Frame` → hash de `comemo`/`siphasher`) e envia à interface, por
  `tauri::ipc::Channel`, só as páginas novas ou alteradas, com número e tamanho de todas. A
  interface mostra cada página como `<img>` com *blob URL* de SVG (o SVG nunca entra no DOM
  como marcação, então não executa script nem carrega recurso externo) e só pede SVG das
  páginas visíveis ± 2 (virtualização). Erro de compilação mantém as páginas atuais e mostra
  a faixa "há erro" (FR-014).
  Imagens dentro do SVG vêm como `data:` URL; para não reenviar imagens grandes a cada edição
  na mesma página, o núcleo troca cada `data:` URL por `prelo-asset://<sha256>` servido por
  protocolo próprio do Tauri, com cache por conteúdo (RF-51, SC-002).
- **Rationale**: SVG é o que docs/especificacao.md §4 e a arquitetura citam ("prévia SVG
  incremental"); a compilação incremental do Typst (`comemo`) mais diff por página mantém o
  trabalho proporcional ao que mudou. `<img>` isola conteúdo não confiável.
- **Alternatives considered**: `tinymist-preview` (renderizador incremental próprio, mas sobe
  servidor HTTP/WebSocket local e JS de terceiros na webview; mais superfície de ataque);
  raster com `typst-render` (texto não nítido no zoom, mais memória); PDF.js sobre o PDF
  (lento para recompilar a cada tecla).

## R5. Sincronização código ↔ prévia

- **Decision**: clique na prévia → interface envia `(página, x, y)` em pontos → núcleo chama
  `typst_ide::jump_from_click` sobre o último documento compilado → `Jump::File(arquivo,
  offset)` vira `(caminho, linha, coluna)` para o editor; `Jump::Url`/`Position` internos são
  tratados como "sem origem no projeto" quando caem fora de arquivo do projeto. Do cursor →
  `typst_ide::jump_from_cursor` → lista de `(página, x, y)`; a interface rola até a primeira e
  desenha um destaque temporário. Estrutura de seções (FR-005) sai do introspector do
  documento compilado (elementos `heading` com nível e posição), com o *span* de cada título
  convertido em arquivo e linha, o que cobre títulos de `.typ` incluídos.
- **Rationale**: APIs oficiais do Typst, usadas também pelo Tinymist; trabalham sobre o mesmo
  documento da prévia, então não há desalinhamento.
- **Alternatives considered**: outline pelo `DocumentSymbol` do `tinymist-query` (é por
  arquivo, perde a ordem real com `#include`).

## R6. Fontes (D-15)

- **Decision**: o app embute a família **Libertinus** 7.051 (Serif, Sans, Math, Mono; SIL
  OFL-1.1), com os arquivos em `src-tauri/fonts/` junto do `OFL.txt` e do aviso de copyright.
  `typst-assets` entra **sem** a feature `fonts`. O resolvedor de fontes carrega (a) as fontes
  embutidas e (b) todo arquivo `.ttf`, `.otf`, `.ttc`, `.otc` sob `fonts/` do projeto, lido
  pelo mesmo `PathAccessModel` (escopo RF-08); fonte malformada vira diagnóstico. Nenhuma
  fonte do sistema é lida (sem `fontdb`, sem `typst-kit/scan-fonts`). A tela "Sobre" lista as
  fontes embutidas com licença. Uma verificação nova em `scripts/audit.mjs` lê
  `src-tauri/fonts/licenses.toml` (arquivo → licença) e falha se houver arquivo de fonte sem
  entrada ou com licença fora de `security/licenses-allow.txt` + `OFL-1.1`.
- **Rationale**: as fontes do `typst-assets` 0.15.1 têm licenças GUST/LPPL (New Computer
  Modern), GPL-3.0 com Font Exception (NewCM10-Regular) e Bitstream Vera (DejaVu Sans Mono);
  D-15 só abre exceção para OFL-1.1. Libertinus é um único projeto OFL que cobre texto,
  matemática e código, e Libertinus Serif já é a fonte de texto padrão do Typst.
- **Consequência**: matemática e blocos de código usam Libertinus Math/Mono quando o `.typ` não
  pede fonte; o `typst` de linha de comando, com as fontes dele, usaria New Computer Modern
  Math e DejaVu Sans Mono. O teste de equivalência (FR-031) roda o `typst` de linha de comando
  com `--ignore-system-fonts --ignore-embedded-fonts --font-path <fontes do Prelo> --font-path
  <projeto>/fonts` (flags confirmadas em `typst-cli` 0.15.1). O Prelo define o padrão de fonte
  de matemática e de `raw` no World, sem escrever nada no `.typ` (Princípio I).
- **Alternatives considered**: embutir o conjunto do `typst-assets` (viola D-15); só Libertinus
  Serif (matemática sem fonte de matemática gera aviso e layout ruim).

## R7. Pacotes `@preview`

- **Decision**: `PackageRegistry` próprio, com cache em `<dados locais do app>/packages/
  preview/<nome>/<versão>/` (fora do projeto e fora do cache do `typst` CLI, que outros
  programas podem alterar). Download por `ureq` 3.5 com `rustls` (provedor `ring`) e
  `rustls-native-certs` (certificados do sistema passados ao `ureq` como raízes), **sem**
  `webpki-roots` nem `webpki-root-certs` (licença CDLA-Permissive-2.0, fora da lista; o
  `rustls-platform-verifier` 0.7 traz `webpki-root-certs` no alvo `wasm32`, e a auditoria
  confere todos os alvos — decisão do dono, 2026-11-19), de `https://packages.typst.org/preview/<nome>-<versão>.tar.gz`.
  Extração (`flate2` + `tar`) numa pasta temporária do cache com validação de cada entrada:
  recusa caminho absoluto, `..`, link simbólico, link físico, dispositivo/FIFO e entradas além
  de limites (50 MB compactado, 200 MB e 5 000 arquivos descompactados); qualquer recusa
  descarta o pacote inteiro. Depois, cálculo do hash SHA-256 da árvore (caminhos ordenados +
  conteúdo), gravação no índice `<cache>/packages/index.json` e `rename` atômico para o
  destino. Na primeira resolução de cada pacote por sessão, o hash é recalculado; diferente do
  índice → pacote recusado ("pacote adulterado"), com ação "Baixar de novo". Sem rede e sem
  cache → diagnóstico citando o pacote. Namespaces fora de `preview` (incl. `@local`) →
  diagnóstico (D-13).
- **Rationale**: atende Q2 do dono e FR-017/FR-018. O registro do Typst não publica assinatura
  dos pacotes; o que dá para garantir localmente é transporte TLS, validação do arquivo
  compactado e integridade do cache depois do download. Só nome e versão saem na URL
  (Princípio II).
- **Alternatives considered**: `typst-kit` com `universe-packages` + `system-downloader` (usa
  OpenSSL nativo e extrai sem as validações exigidas); `tinymist-world` com `http-registry`
  (traz `reqwest` e a mesma lacuna); reusar o cache do `typst` CLI (alterável por fora, sem
  índice de hash).

## R8. Plugins WASM

- **Decision**: o `PathAccessModel` identifica bytes que começam com a assinatura WASM
  (`\0asm`) e só os entrega ao compilador se houver aprovação para (raiz canônica do projeto,
  SHA-256 do arquivo). Sem aprovação, devolve erro "plugin aguardando confirmação" (vira
  diagnóstico) e emite o evento `plugin_approval_needed` com caminho, origem (projeto ou
  pacote `nome@versão`) e tamanho. Aprovar ou recusar grava em `<dados locais do app>/
  plugin-approvals.json` e dispara nova compilação. Arquivo aprovado com conteúdo novo tem
  outro hash → pergunta de novo. Aprovação nunca é lida de `.prelo/`.
  A execução continua no sandbox do próprio Typst (`wasmi` 1.1, interpretador sem acesso a
  arquivo, rede ou relógio; só funções que o Typst expõe). Plugin que não retorna fica coberto
  pelo limite de tempo da compilação (R9).
- **Rationale**: o World não sabe se um arquivo foi pedido por `plugin()` ou por `read()`;
  barrar pela assinatura WASM cobre todo caminho de execução com custo nulo para o resto
  (ler um `.wasm` como dado também passa a pedir confirmação, efeito aceitável).
- **Alternatives considered**: detectar `plugin(...)` na sintaxe (frágil: caminho calculado,
  funções de pacote); guardar aprovação em `.prelo/` (coautor poderia pré-aprovar para os
  outros nas fases seguintes).

## R9. Agendamento da compilação e limite de tempo

- **Decision**: uma thread de compilação no núcleo, alimentada por fila de tamanho 1 (a edição
  mais nova substitui a pendente). Cada compilação tem *watchdog* de 10 s: estourado, o núcleo
  marca a compilação como abandonada, emite diagnóstico "compilação excedeu 10 s", cria um
  World novo numa thread nova e descarta o resultado da antiga quando ela terminar. Limites
  internos do Typst (profundidade de chamada, laço com 10 000 iterações, iterações de layout)
  continuam valendo.
- **Rationale**: o Typst não tem cancelamento cooperativo; uma thread não pode ser morta com
  segurança. Abandonar mantém o editor responsivo (FR-015), e o caso real de compilação sem
  fim é raro (plugin aprovado ou cálculo enorme).
- **Alternatives considered**: compilar em processo filho (dá para matar e limitar memória,
  mas perde o cache incremental a cada reinício e custa IPC por página; reavaliar se os
  testes mostrarem acúmulo de threads abandonadas).

## R10. Arquivos: salvar, gravação atômica, mudanças externas

- **Decision**: o núcleo guarda o texto dos arquivos abertos. Gravação: `tempfile` 3.27 na
  mesma pasta + `fsync` + `persist` (rename atômico); erro mantém o texto em memória e avisa.
  Salvamento automático 2 s após a última edição, em ⌘S/Ctrl+S e no fechamento (o fechamento
  espera a gravação, com limite). Mudanças externas: `notify` 8 (CC0-1.0) observa a raiz; o
  núcleo ignora eventos cujo hash é igual ao último que ele mesmo gravou; sem alteração local
  pendente → recarrega; com alteração pendente → evento `external_change_conflict` e a pessoa
  escolhe (manter a minha / usar a do disco); nada é descartado sem escolha.
- **Rationale**: Princípio III e FR-007 a FR-009 sem servidor nem git (git só na fase 2).
- **Alternatives considered**: gravar direto no arquivo (corrompe em falha); `notify` 9 (ainda
  RC).

## R11. Regras de escopo de arquivo (RF-08)

- **Decision**: ver [contracts/file-access.md](contracts/file-access.md). Resumo: raiz
  canonicalizada uma vez ao abrir; cada pedido junta raiz e caminho virtual (com `/` inicial
  relativo à raiz), canonicaliza (resolve `..` e links), confere prefixo da raiz canônica,
  abre o arquivo e confere **no descritor aberto** que é arquivo regular e que o caminho real
  do descritor continua sob a raiz (`/proc/self/fd` ou `fcntl(F_GETPATH)` ou
  `GetFinalPathNameByHandleW`); só então lê. Em Unix a abertura usa `O_NOFOLLOW` no último
  componente e `O_NONBLOCK` (FIFO não bloqueia). No Windows, junções e links simbólicos são
  tratados como links (resolvidos e conferidos). Tamanho máximo por arquivo lido: 256 MB.
- **Rationale**: atende RF-08 e o passo 6 de arquitetura §6.2 (conferência sobre o arquivo já
  aberto, contra troca de link entre conferência e leitura).
- **Alternatives considered**: confiar na normalização do Typst (a arquitetura proíbe);
  `cap-std` (abordagem por capability, boa, mas cobre mal o caminho de retorno ao Tinymist e
  é mais uma dependência; reavaliar se o teste de corrida falhar).

## R12. Dependências novas e licenças

| Dependência | Versão | Licença | Uso |
|---|---|---|---|
| `typst`, `typst-library`, `typst-syntax`, `typst-svg`, `typst-pdf`, `typst-ide` | 0.15.1 | Apache-2.0 | Compilador, prévia, PDF, saltos, autocompletar, *hover* e realce (D-17) |
| `typst-assets` (sem `fonts`) | 0.15.1 | Apache-2.0; dados internos com partes CC-BY-4.0 e W3C, aceitas só para esta crate (D-16) | Dependência do `typst` |
| `tinymist-world`, `tinymist-vfs`, `tinymist-package` | 0.15.0 | Apache-2.0 | World, VFS, traits |
| `ureq` | 3.5 | MIT OR Apache-2.0 | Download de pacotes |
| `rustls` / `ring` / `rustls-native-certs` | 0.23 / 0.17 / 0.8 | Apache-2.0 OR ISC OR MIT / Apache-2.0 AND ISC / Apache-2.0 OR ISC OR MIT | TLS com certificados do sistema |
| `flate2`, `tar` | 1.1 / 0.4 | MIT OR Apache-2.0 | Extração de pacotes |
| `sha2` | 0.10 | MIT OR Apache-2.0 | Hash de pacotes e plugins |
| `tempfile` | 3.27 | MIT OR Apache-2.0 | Gravação atômica; fixtures |
| `notify` | 8.2 | CC0-1.0 | Mudanças externas |
| `dirs` | 6 | MIT OR Apache-2.0 | Pasta de dados locais (traz `option-ext`, já exceção D-11) |
| `tauri-plugin-dialog` | 2.x | MIT OR Apache-2.0 | Diálogos nativos, chamados só pelo núcleo |
| Libertinus (arquivos de fonte) | 7.051 | OFL-1.1 (D-15) | Fontes embutidas |
| `@codemirror/*` (7 pacotes) | 6.x | MIT | Editor |
| `@tauri-apps/plugin-dialog` | — | — | **Não** usado: a webview não ganha permissão de diálogo |

Licenças transitivas encontradas na árvore candidata, todas já na lista ou combinadas com uma
dela por `OR`: MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib, Unicode-3.0, 0BSD, CC0-1.0,
Apache-2.0 WITH LLVM-exception; `Unlicense OR MIT`, `Apache-2.0 OR BSL-1.0` (escolha
MIT/Apache); `ring` = `Apache-2.0 AND ISC` (ambas permitidas); `option-ext` MPL-2.0 (exceção
D-11 já existe). **Achado**: `webpki-roots`/`webpki-root-certs` (CDLA-Permissive-2.0) entram
se `ureq` usar a feature padrão de raízes ou se o `rustls-platform-verifier` 0.7 estiver na
árvore (no alvo `wasm32`); a configuração escolhida usa `rustls-native-certs` e os exclui, e a
auditoria confere todos os alvos.

**Resultado de T008 (2026-11-19)**: `tinymist-query` 0.15.12 removido (D-17, R2);
`rustls-platform-verifier` trocado por `rustls-native-certs` (decisão do dono). O
`cargo-deny` acusa `quick-xml` 0.38.4 (RUSTSEC-2026-0194 e RUSTSEC-2026-0195), tratado em
R14 com exceção aprovada pelo dono (registrada em `security/vuln-exceptions.toml` e
`[advisories].ignore`, revisão até 2027-02-17). Com isso, `npm run audit` passa por inteiro:
exceções válidas, lista de licenças em sincronia, licenças de arquivos embutidos (fontes
Libertinus OFL-1.1 e dados do `typst-assets`), `cargo-deny` (avisos, licenças, *bans*,
fontes) e `audit-ci`/licenças npm. A exceção D-16 aparece como
`license-exception-not-encountered` (aviso, não erro), porque o `cargo-deny` lê só a licença
declarada do `typst-assets` (Apache-2.0); ela documenta e trava o escopo.

## R13. Testes por camada

| Camada | Ferramenta | O que cobre |
|---|---|---|
| Núcleo (unidade) | `cargo test` | Escopo de caminho, validação de `project.json`, índice de pacotes, aprovação de plugins, diff de páginas, agenda de compilação |
| Segurança (integração) | `cargo test` em `src-tauri/tests/security_*.rs` com projetos de fixture gerados em pasta temporária (links e FIFO criados no teste, não versionados) | FR-030 inteiro; contador de leituras fora da raiz via `PathAccessModel` instrumentado |
| Compilação (integração) | `cargo test` em `tests/compile_*.rs` | Diagnósticos, bibliografia, formatos de imagem, fontes, decodificações de imagem (FR-032), saltos |
| Equivalência | `cargo test --features equivalence` + `typst` CLI 0.15.1 instalado só no CI | FR-031 / SC-005: PDF renderizado página a página e comparado por imagem (`typst-render`) |
| Desempenho | `cargo test --release` em `tests/perf_preview.rs` com fixture de 40 páginas | SC-001/SC-002: p95 edição→SVG pronto; limite 100 ms no computador de referência, 250 ms no CI como guarda de regressão |
| Contrato IPC | `tauri::test` (mock runtime) | Comandos novos, argumentos malformados, caminhos de arquivo vindos da webview recusados fora da raiz |
| Configuração de segurança | `tests/security_config.rs` (já existe) | Capabilities sem diálogo/fs/http; CSP com `img-src 'self' blob: prelo-asset:` e nada mais |
| Interface | Vitest + jsdom | Editor, diagnósticos, árvore, estrutura, prévia virtualizada, diálogos de confirmação |
| Ponta a ponta | WebdriverIO (Linux/Windows) + autoteste (3 sistemas) | Abrir fixture, editar, ver prévia, clicar na prévia, exportar PDF, sem `typst` no PATH |

Dados de fixture em `src-tauri/tests/fixtures/` (projetos pequenos versionados) e geradores em
código para links, FIFO e pacotes `.tar.gz` maliciosos. O CI remove `typst` do PATH nos jobs
de E2E (SC-006).

## R14. `quick-xml` 0.38.4 (RUSTSEC-2026-0194, RUSTSEC-2026-0195)

- **Achado**: o `cargo-deny` acusa dois avisos em `quick-xml` 0.38.4: tempo quadrático ao
  conferir nomes de atributo repetidos numa *tag* (0194) e alocação sem limite para
  declarações de namespace no `NsReader` (0195). A correção só existe a partir da 0.41. A
  crate entra por `typst-library` 0.15.1 → `hayagriva` 0.10.1 → `citationberg` 0.7.0
  (`quick-xml ^0.38.1`), que lê os estilos `.csl` (XML) do projeto. Não há versão de
  `citationberg`, `hayagriva` ou `typst` compatível que a corrija.
- **Decision (dono, 2026-11-19)**: exceção para os dois avisos em
  `security/vuln-exceptions.toml` (`ecosystem = "cargo"`, `review_by = "2027-02-17"`) e em
  `src-tauri/deny.toml` `[advisories].ignore`, com mitigação testada:
  - limite de 1 MiB para arquivos `.csl` lidos pelo compilador, aplicado no ponto único de
    leitura (`access_model.rs`), com diagnóstico claro ("estilo grande demais"); estilos CSL
    reais têm poucas dezenas de KB;
  - limite de 10 s da compilação (R9): uma compilação presa no XML é abandonada e vira
    diagnóstico, com o editor respondendo.
- **Testes**: `src-tauri/tests/security_malformed.rs` com `.csl` dentro do limite contendo (a)
  uma *tag* com milhares de atributos distintos e (b) milhares de declarações `xmlns:*`;
  cada caso termina em diagnóstico ou abandono por tempo, e o núcleo responde a uma edição em
  até 1 s; `.csl` acima de 1 MiB é recusado sem ser entregue ao compilador.
- **Rationale**: sem versão corrigida compatível, a alternativa seria remover a bibliografia
  CSL (RF-05) ou fazer *fork*. O ataque exige um `.csl` malicioso no projeto; o limite de
  tamanho reduz o custo máximo, e o limite de tempo garante que o app não trava.
- **Revisão**: em 2027-02-17 ou na próxima versão do `typst`, o que vier primeiro.

## Riscos

- **API do `typst-ide`** muda entre versões do Typst. Mitigação: versão travada (`=0.15.1`),
  atualização só com a suíte verde, e camada fina `lang_service.rs` que isola o uso (D-17).
- **`quick-xml` 0.38 com avisos de segurança** (R14): exceção com data de revisão e
  mitigação testada.
- **Dados internos do `typst-assets`** (perfis ICC CC0, dados HTML e MathML com partes
  CC-BY-4.0 e W3C, fontes Foxit BSD-3-Clause para PDFs embutidos) entram no binário mesmo sem
  a feature `fonts`. **Decidido em D-16**: CC-BY-4.0 e W3C-20150513 aceitas só como exceção
  da crate `typst-assets` em `src-tauri/deny.toml`, fora da lista geral. Como o `cargo-deny` lê
  só a licença declarada da crate (Apache-2.0), a exceção documenta e trava o escopo; a
  verificação de arquivos embutidos (`scripts/audit.mjs`) lista os dados e seus avisos.
- **Imagens grandes no SVG**: a troca de `data:` URL por `prelo-asset://` depende do formato
  de saída do `typst-svg`; se não for estável, alternativa é renderizar a imagem à parte e
  posicioná-la por cima. Medido pela tarefa de desempenho com imagens grandes.
- **Threads abandonadas** (R9) podem acumular CPU; o teste de compilação sem fim conta
  threads vivas.
- **Diferenças de fonte** entre Prelo e `typst` CLI padrão (R6) podem surpreender quem compila
  fora do Prelo; o README e a tela "Sobre" explicam.
