# Feature Specification: Editor local (fase 1)

**Feature Branch**: `003-editor-local`

**Created**: 2026-11-19

**Status**: Draft

**Input**: User description: "Fase 1 do Prelo, 'Editor local' (docs/especificacao.md §4 item 1). Abrir uma pasta de projeto Typst local existente ou criar um projeto novo; editar arquivos .typ com realce de sintaxe, autocompletar e diagnósticos (RF-01); árvore de arquivos e outline de seções (RF-04, sem contagem de pendências de revisão); prévia ao lado do editor, compilada pelo compilador Typst embutido no núcleo, sem typst instalado, na versão registrada em .prelo/project.json e exibida no app (RF-02, RF-06, RF-07, D-10); sincronização clique-no-código para prévia e clique-na-prévia para código (RF-03); bibliografia .bib/.yml/.csl e imagens dentro do projeto (RF-05, RF-09, RF-50), sem ler nada fora da raiz do projeto (RF-08, D-13, arquitetura §6); cache de imagens não alteradas (RF-51); exportar PDF final da mesma compilação (RF-42 parcial, RF-06); meta de desempenho: prévia em menos de 100 ms após edição em artigos até 40 páginas (RF-02, RNF-03); testes de segurança do Princípio VI: .typ/.bib malformados geram diagnóstico sem derrubar o app, caminhos fora do projeto recusados. Fora de escopo: comentários, git, tempo real/P2P, sugestões, papéis, arquivos pendentes, PDF com marcações e de diferenças."

**Origem**: docs/especificacao.md §4 item 1 · RF-01 a RF-09, RF-42 (parcial), RF-50, RF-51 ·
RNF-01, RNF-03, RNF-05, RNF-06 · D-10, D-13, D-14, D-15 · docs/arquitetura.md §6 · Constituição:
Princípios I, III e VI, Restrições Técnicas ("Compilação embutida", "Desempenho")

## User Scenarios & Testing *(mandatory)*

Nesta fase o Prelo é um editor Typst para uma pessoa, no próprio computador. Não há
coautores, rede, git, comentários nem sugestões. Mesmo assim, a pasta do projeto é tratada
como **entrada não confiável**: nas fases seguintes ela chegará de outros coautores, e o
projeto pode ter vindo de qualquer lugar (Princípio VI).

### User Story 1 - Escrever com prévia ao vivo (Priority: P1)

Uma pesquisadora abre no Prelo a pasta de um artigo Typst que já existe no computador. Ela vê
a árvore de arquivos, abre o `.typ`, escreve com realce de sintaxe e autocompletar, e a prévia
ao lado atualiza enquanto ela digita. Erros de Typst aparecem no editor, na linha certa, com
mensagem. Nada disso exige `typst` instalado na máquina, e o app mostra qual versão do
compilador está usando.

**Why this priority**: é o núcleo da fase 1 e a base de todas as fases seguintes (RF-01,
RF-02, RF-06, RF-07). Sem escrever e ver o resultado, o Prelo não tem uso.

**Independent Test**: num computador sem `typst` instalado, abrir um projeto de fixture,
editar o texto e verificar que a prévia muda, que um erro introduzido aparece como
diagnóstico na linha certa e que a versão do compilador aparece no app.

**Acceptance Scenarios**:

1. **Given** uma pasta com um projeto Typst válido, **When** a pessoa a abre no Prelo,
   **Then** o app mostra a árvore de arquivos, abre o arquivo principal no editor e mostra a
   prévia compilada ao lado.
2. **Given** o projeto aberto, **When** a pessoa digita no `.typ`, **Then** o código aparece
   com realce de sintaxe e a prévia reflete a mudança sem ação adicional.
3. **Given** o projeto aberto, **When** a pessoa digita o início de uma função, variável ou
   rótulo conhecido, **Then** o editor oferece opções de autocompletar.
4. **Given** uma edição que introduz um erro de Typst, **When** a compilação termina,
   **Then** o editor marca o trecho com o erro e a mensagem, e a prévia mantém a última
   versão que compilou, com indicação de que há erro.
5. **Given** um computador sem `typst` instalado, **When** a pessoa abre e edita um projeto,
   **Then** a prévia e os diagnósticos funcionam normalmente.
6. **Given** o projeto aberto, **When** a pessoa consulta a versão do compilador no app,
   **Then** o app mostra a versão embutida e a versão registrada no projeto.
7. **Given** alterações feitas no editor, **When** a pessoa salva, fecha o app ou o app
   encerra de forma inesperada, **Then** o texto editado está no disco ao reabrir o projeto,
   exceto no máximo os últimos segundos de digitação no caso de encerramento inesperado.

---

### User Story 2 - Projeto confinado à própria pasta (Priority: P1)

Um projeto recebido de outra pessoa contém um `.typ` que tenta ler arquivos de fora da pasta,
por exemplo `#image("../../.ssh/id_ed25519")`, um link simbólico para uma pasta pessoal ou um
`.bib` corrompido. O Prelo recusa toda leitura fora da pasta do projeto, explica o motivo no
editor e continua funcionando. Arquivos malformados geram diagnóstico, não travam o app.

**Why this priority**: o Princípio VI é não negociável, e o `.typ` passa a vir de outros
coautores nas fases seguintes. A regra de escopo (RF-08, D-13) precisa existir desde a
primeira compilação.

**Independent Test**: abrir os projetos de fixture de segurança (caminho com `..` além da
raiz, `/` seguido de `..`, link simbólico para fora, link simbólico interno, FIFO na raiz,
`.typ`, `.bib`, `.yml`, `.csl` e imagem malformados) e verificar, para cada um, o diagnóstico
esperado, a ausência de qualquer leitura fora da raiz e o app respondendo.

**Acceptance Scenarios**:

1. **Given** um `.typ` com `#image`, `#include`, `#import`, `read`, `bibliography` ou função de
   dados apontando para caminho que, resolvido, fica fora da raiz do projeto, **When** o
   projeto compila, **Then** a leitura é recusada sem acessar o conteúdo e o editor mostra
   diagnóstico com o caminho pedido e o motivo.
2. **Given** um link simbólico dentro do projeto que aponta para fora dele, **When** o `.typ`
   o referencia, **Then** a leitura é recusada como no cenário 1.
3. **Given** um link simbólico dentro do projeto que aponta para outro arquivo dentro dele,
   **When** o `.typ` o referencia, **Then** a leitura é permitida.
4. **Given** um caminho que começa com `/` (como `/figuras/a.png`), **When** o projeto compila,
   **Then** ele é resolvido a partir da raiz do projeto e permitido se continuar dentro dela.
5. **Given** um arquivo especial (dispositivo, FIFO, socket) dentro do projeto, **When** o
   `.typ` o referencia, **Then** a leitura é recusada com diagnóstico, sem bloquear o app.
6. **Given** um `.typ`, `.bib`, `.yml`, `.csl` ou imagem malformado, **When** o projeto
   compila, **Then** o editor mostra diagnóstico com arquivo e, quando houver, linha, e o app
   continua respondendo.
7. **Given** o autocompletar de caminhos de arquivo, **When** a pessoa digita um caminho,
   **Then** nenhuma opção fora da raiz do projeto é listada.
8. **Given** um `.prelo/project.json` malformado ou com campos inesperados, **When** a pessoa
   abre o projeto, **Then** o app informa o problema e não o executa nem o interpreta além do
   que é válido, sem encerrar.
9. **Given** um `.typ` que chama `plugin()` num projeto em que o plugin nunca foi aprovado,
   **When** o projeto compila, **Then** o plugin não roda, o app pede confirmação mostrando o
   arquivo e a origem, e a compilação gera diagnóstico até a pessoa aprovar.
10. **Given** um plugin aprovado, **When** o conteúdo do arquivo do plugin muda, **Then** o
    app pede confirmação de novo antes de rodá-lo.
11. **Given** um pacote `@preview` cujo conteúdo no cache foi alterado depois do download,
    **When** o projeto compila, **Then** o pacote é recusado com diagnóstico e o app oferece
    baixá-lo de novo.
12. **Given** um pacote baixado cujo arquivo compactado contém entrada com `..`, caminho
    absoluto ou link, **When** o app o recebe, **Then** o pacote inteiro é recusado e nada é
    gravado fora da pasta do cache.

---

### User Story 3 - Bibliografia e figuras do projeto (Priority: P2)

A pesquisadora cita referências de um `refs.bib` e de um estilo ABNT em `.csl` guardados no
projeto, e inclui figuras em PNG, JPEG, SVG e PDF. A prévia mostra citações, lista de
referências e figuras. Editar o texto de um artigo com imagens grandes não deixa a prévia
lenta, porque as imagens que não mudaram não são processadas de novo.

**Why this priority**: artigo científico sem bibliografia e figuras não é publicável (RF-05,
RF-09, RF-50), mas a história 1 já entrega valor com texto puro.

**Independent Test**: abrir um projeto de fixture com `.bib`, `.yml`, `.csl` e uma figura de
cada formato suportado; verificar a prévia e comparar o PDF exportado com o do `typst` de
linha de comando na mesma versão; medir a prévia após edições de texto num artigo com imagens
grandes.

**Acceptance Scenarios**:

1. **Given** um projeto com bibliografia em BibLaTeX (`.bib`) ou Hayagriva (`.yml`/`.yaml`),
   um ou vários arquivos, **When** o `.typ` cita entradas, **Then** a prévia mostra citações e
   lista de referências.
2. **Given** um estilo `.csl` no projeto ou um estilo embutido do Typst, **When** o `.typ` o
   usa, **Then** a prévia formata citações e referências por esse estilo.
3. **Given** figuras em PNG, JPEG, GIF, SVG e, se a versão embutida suportar, PDF e WebP,
   **When** o `.typ` as inclui, **Then** todas aparecem na prévia e no PDF (GIF como imagem
   estática).
4. **Given** um arquivo com extensão de imagem e conteúdo inválido, **When** o projeto
   compila, **Then** aparece diagnóstico e o app continua respondendo.
5. **Given** um artigo com imagens grandes, **When** a pessoa edita só o texto, **Then** cada
   imagem é processada uma única vez ao longo das edições e a prévia cumpre a meta de
   desempenho.
6. **Given** um `.typ` que aponta para um arquivo fora do projeto (como um `.bib`
   compartilhado entre artigos), **When** o projeto compila, **Then** o app recusa a leitura
   (história 2) e oferece copiar o arquivo para dentro do projeto (D-13); a cópia só acontece
   com confirmação da pessoa.
7. **Given** a pessoa edita o `.bib` no próprio Prelo, **When** salva, **Then** a prévia
   recompila com as referências atualizadas.
8. **Given** um arquivo de fonte em `fonts/` no projeto, **When** o `.typ` pede essa fonte,
   **Then** a prévia e o PDF a usam; uma fonte só instalada no sistema não é usada.
9. **Given** um `.typ` que importa um pacote `@preview`, **When** o projeto compila com rede,
   **Then** o pacote é baixado e a prévia aparece; **When** o mesmo projeto compila depois
   sem rede, **Then** o pacote vem do cache e a prévia aparece igual.

---

### User Story 4 - Navegar entre código e prévia (Priority: P2)

Lendo a prévia, a pesquisadora clica num parágrafo e o editor vai para a linha que o gera.
No editor, ela pede para ver na prévia o trecho onde está o cursor, e a prévia rola até ele e
o destaca. Um painel de estrutura lista as seções do artigo e leva a cada uma; a árvore de
arquivos abre qualquer `.typ` do projeto.

**Why this priority**: acelera a revisão de um artigo longo (RF-03, RF-04), mas o editor já é
útil sem ela.

**Independent Test**: num projeto de fixture com vários arquivos e seções, clicar em pontos
conhecidos da prévia e verificar a linha aberta; posicionar o cursor em linhas conhecidas e
verificar o ponto destacado na prévia; clicar em cada seção do painel de estrutura.

**Acceptance Scenarios**:

1. **Given** a prévia de um projeto, **When** a pessoa clica num trecho de texto da prévia,
   **Then** o editor abre o arquivo de origem e posiciona o cursor na linha que gera o trecho.
2. **Given** um trecho gerado por `#include` de outro `.typ`, **When** a pessoa clica nele na
   prévia, **Then** o editor abre o arquivo incluído, não o principal.
3. **Given** o cursor numa linha do `.typ`, **When** a pessoa pede para localizar na prévia,
   **Then** a prévia rola até o ponto correspondente e o destaca.
4. **Given** o projeto aberto, **When** a pessoa abre o painel de estrutura, **Then** ele
   lista os títulos de seção do artigo em ordem e hierarquia, e clicar num título leva o
   editor e a prévia até ele.
5. **Given** um clique na prévia em conteúdo sem linha de origem no projeto (como texto gerado
   por pacote ou numeração automática), **When** a sincronização não encontra a origem,
   **Then** o editor não muda de posição e o app informa que não há origem no projeto.

---

### User Story 5 - Exportar o PDF (Priority: P2)

Com o artigo pronto, a pesquisadora exporta o PDF. O PDF é idêntico ao que a prévia mostra e
ao que o `typst` de linha de comando geraria na mesma versão.

**Why this priority**: entrega o produto final da escrita (RF-42, parte do PDF final;
RF-06). Depende da história 1.

**Independent Test**: exportar o PDF de projetos de fixture e comparar com o PDF do `typst`
de linha de comando na versão registrada; tentar exportar com erro de compilação.

**Acceptance Scenarios**:

1. **Given** um projeto que compila sem erro, **When** a pessoa exporta o PDF e escolhe onde
   gravar, **Then** o PDF é gravado e tem o mesmo conteúdo visual da prévia.
2. **Given** o mesmo projeto, **When** o PDF é comparado ao gerado pelo `typst` de linha de
   comando na versão registrada em `.prelo/project.json`, **Then** os dois coincidem página a
   página.
3. **Given** um projeto com erro de compilação, **When** a pessoa tenta exportar, **Then** a
   exportação é recusada e o app lista os erros que impedem a compilação.

---

### User Story 6 - Criar um projeto novo (Priority: P3)

Quem ainda não tem um artigo cria um projeto novo no Prelo: escolhe a pasta e o nome, e o app
cria um projeto Typst mínimo que já compila e abre no editor com a prévia.

**Why this priority**: conveniência; quem já tem um projeto usa a história 1. Depende dela.

**Independent Test**: criar um projeto numa pasta vazia, verificar os arquivos criados,
verificar que a prévia aparece e que o mesmo projeto compila com o `typst` de linha de
comando.

**Acceptance Scenarios**:

1. **Given** uma pasta vazia, **When** a pessoa cria um projeto novo nela, **Then** o app cria
   um arquivo principal `.typ` mínimo e o registro do projeto em `.prelo/` com a versão do
   compilador, e abre o projeto com a prévia.
2. **Given** uma pasta que já tem arquivos, **When** a pessoa tenta criar um projeto novo
   nela, **Then** o app avisa e não sobrescreve nenhum arquivo existente.
3. **Given** o projeto recém-criado, **When** compilado fora do Prelo pelo `typst` de linha de
   comando, **Then** compila sem erro (Princípio I).

### Edge Cases

- **Pasta sem `.prelo/`**: um projeto Typst comum, nunca aberto no Prelo, abre normalmente; o
  app cria `.prelo/project.json` com a versão do compilador embutido sem alterar nenhum outro
  arquivo.
- **Versão registrada diferente da embutida**: o app compila com a versão embutida, mostra
  aviso com as duas versões e não altera o registro sem ação explícita da pessoa (D-10; quem
  decide a troca de versão fica para fase posterior).
- **Vários `.typ` sem `main.typ`**: o app pede para a pessoa escolher o arquivo principal e
  guarda a escolha em `.prelo/project.json`.
- **Arquivo alterado fora do Prelo** (outro editor, gerenciador de referências que reescreve o
  `.bib`): sem alterações não salvas no Prelo, o app recarrega e recompila; com alterações não
  salvas, o app avisa e a pessoa escolhe qual versão manter, sem perder nenhuma em silêncio
  (Princípio III).
- **Arquivo referenciado inexistente**: diagnóstico de arquivo não encontrado, com o caminho.
  Arquivos "pendentes" (RF-52) não existem nesta fase.
- **Compilação que não termina** (recursão profunda, laço, plugin que não retorna): o editor
  continua respondendo; a compilação é abandonada por limite de tempo e vira diagnóstico.
- **Edições rápidas seguidas**: compilações obsoletas são descartadas; a prévia sempre
  converge para o texto mais recente.
- **Link simbólico trocado entre a verificação e a leitura**: a leitura não escapa da raiz
  (docs/arquitetura.md §6.2, passo 6).
- **`..` que continua dentro da raiz** (como `../utils.typ` a partir de `capitulos/`):
  permitido.
- **Arquivo `.typ` muito grande ou com codificação inválida** (não UTF-8): diagnóstico, sem
  encerrar o app.
- **Pacote `@preview` sem rede**: se já está no cache, compila; se não está, diagnóstico que
  cita o pacote, e a prévia mantém a última compilação bem-sucedida.
- **Fonte pedida que não existe** nem embutida nem em `fonts/`: o Typst usa a fonte de
  reserva e o editor mostra aviso com o nome da fonte.
- **Plugin aprovado num projeto e usado em outro projeto**: pede confirmação de novo.
- **Pasta do projeto removida ou renomeada com o app aberto**: o app avisa e mantém o texto em
  memória até a pessoa escolher onde salvar.
- **Disco cheio ou sem permissão de escrita ao salvar ou exportar**: o app avisa, não perde o
  texto do editor e não deixa arquivo parcialmente gravado no lugar do original.

## Clarifications

### Session 2026-11-19

- Q: Quais fontes o compilador pode usar? → A: Fontes embutidas no app mais uma pasta de
  fontes dentro do projeto, sujeita à regra de escopo; sem fontes do sistema. Licença OFL-1.1
  aceita só para arquivos de fonte (D-15).
- Q: Como o app trata pacotes `@preview` nesta fase? → A: Baixa sob demanda e guarda em cache
  local para uso offline; a pasta do pacote vira a raiz de escopo.
- Q: Plugins WASM (`plugin()`) rodam sem pergunta? → A: Pedem confirmação na primeira vez em
  cada projeto, e a decisão fica guardada.

## Requirements *(mandatory)*

### Functional Requirements

**Projeto**

- **FR-001**: A pessoa MUST poder abrir uma pasta local como projeto Typst; a raiz do projeto
  é essa pasta (RF-04, RF-08).
- **FR-002**: A pessoa MUST poder criar um projeto novo numa pasta vazia, com um arquivo
  principal `.typ` mínimo que compila no Prelo e fora dele, sem sobrescrever arquivos
  existentes (Princípio I, RNF-05).
- **FR-003**: Ao abrir ou criar um projeto, o app MUST garantir um registro em
  `.prelo/project.json` com a versão do compilador e o arquivo principal, sem alterar outros
  arquivos do projeto (RF-07, docs/arquitetura.md §2).
- **FR-004**: O app MUST mostrar a árvore de arquivos do projeto e abrir no editor qualquer
  `.typ`, `.bib` ou `.yml` dela (RF-04, D-14).
- **FR-005**: O app MUST mostrar a estrutura de seções (outline) do artigo e levar editor e
  prévia até a seção escolhida. A contagem de pendências de revisão por seção fica para a
  fase de revisão (RF-04).

**Edição**

- **FR-006**: O editor MUST oferecer realce de sintaxe Typst, autocompletar e diagnósticos
  (erros e avisos) posicionados no trecho de origem (RF-01).
- **FR-007**: Alterações MUST ser gravadas no disco ao Salvar (⌘S / Ctrl+S), ao fechar o app e
  automaticamente pouco depois de a pessoa parar de digitar, de modo que um encerramento
  inesperado perca no máximo os últimos segundos de digitação (Princípio III, RNF-04).
- **FR-008**: Mudanças feitas por outro programa num arquivo aberto MUST ser detectadas; o app
  MUST recarregar quando não houver alteração local não salva, e perguntar à pessoa quando
  houver (Princípio III).
- **FR-009**: Gravar arquivo MUST ser atômico: falha na gravação MUST NOT deixar o arquivo
  original truncado ou corrompido (Princípio III).

**Compilação e prévia**

- **FR-010**: O app MUST compilar Typst com o compilador embutido, sem depender de `typst`
  instalado na máquina (RF-06, D-10, Restrição "Compilação embutida").
- **FR-011**: Prévia, diagnósticos, autocompletar e PDF exportado MUST vir da mesma
  compilação, na mesma versão do compilador (RF-06, D-10).
- **FR-012**: O app MUST mostrar a versão do compilador embutido e a versão registrada no
  projeto; se diferirem, MUST avisar e MUST NOT alterar o registro sem ação explícita da
  pessoa (RF-07, D-10).
- **FR-013**: A prévia MUST ficar ao lado do editor e atualizar enquanto se digita, sem ação
  da pessoa (RF-02).
- **FR-014**: Quando o texto não compila, a prévia MUST manter a última versão bem-sucedida e
  indicar que há erro (RF-02).
- **FR-015**: Uma compilação lenta, que não termina ou que excede o limite de tempo MUST NOT
  travar o editor; ela MUST ser abandonada, virar diagnóstico, e a prévia MUST voltar a
  compilar na edição seguinte (Princípio VI, entrada não confiável).
- **FR-016**: As fontes disponíveis para a compilação MUST ser só as fontes embutidas no app
  e os arquivos de fonte dentro da pasta `fonts/` do projeto; fontes instaladas no sistema
  MUST NOT ser usadas, para que todos os coautores compilem igual (D-10, D-15). Arquivos da
  pasta de fontes MUST seguir FR-021, e arquivo de fonte malformado MUST gerar diagnóstico.
  As fontes embutidas MUST ter licença permitida por RNF-06 ou OFL-1.1 (D-15), e o app MUST
  exibir as licenças das fontes embutidas.
- **FR-017**: Pacotes `@preview` importados pelo `.typ` MUST ser baixados sob demanda do
  registro do Typst, por conexão criptografada, e guardados num cache local, de modo que o
  mesmo pacote compile depois sem rede (D-10). Sem rede e sem o pacote no cache, o app MUST
  gerar diagnóstico que cita o pacote. Arquivos de pacote MUST seguir FR-021, com a pasta do
  pacote no cache como raiz (RF-08). O download MUST enviar ao registro só nome e versão do
  pacote, nunca conteúdo do projeto (Princípio II).
- **FR-018**: O pacote baixado MUST ser validado antes de entrar no cache: arquivo compactado
  malformado, entrada com caminho absoluto ou com `..`, link simbólico, link físico ou arquivo
  especial MUST fazer o pacote inteiro ser recusado, com diagnóstico. O app MUST registrar a
  impressão digital (hash) de cada pacote ao guardá-lo e, se o conteúdo no cache não
  corresponder mais a ela (pacote adulterado), MUST recusar o pacote com diagnóstico e
  oferecer baixá-lo de novo (Princípio VI).
- **FR-019**: Um plugin WASM (`plugin()`), do projeto ou de pacote, MUST NOT ser executado sem
  confirmação. Na primeira vez em cada projeto, o app MUST pedir confirmação, mostrando o
  arquivo e a origem (projeto ou pacote); enquanto não houver confirmação, ou se a pessoa
  recusar, a compilação MUST gerar diagnóstico e o plugin MUST NOT rodar. A decisão MUST ficar
  guardada neste computador, fora da pasta do projeto, ligada ao projeto e ao conteúdo do
  plugin: plugin com conteúdo alterado MUST pedir confirmação de novo. O arquivo do plugin
  MUST seguir FR-021.

**Sincronização código ↔ prévia**

- **FR-020**: Clicar num trecho da prévia MUST abrir o arquivo de origem e posicionar o cursor
  na linha que o gera, inclusive para conteúdo de `.typ` incluído; pedir "localizar na
  prévia" a partir do cursor MUST rolar a prévia até o ponto correspondente e destacá-lo
  (RF-03). Quando não houver origem no projeto, o app MUST informar sem mover o cursor.

**Acesso a arquivos (segurança)**

- **FR-021**: O compilador MUST ler só arquivos regulares cujo caminho real, depois de
  resolver `..` e links simbólicos, fique dentro da raiz do projeto. Caminho que sairia da
  raiz, link simbólico para fora e arquivo especial (dispositivo, FIFO, socket) MUST ser
  recusados sem ler o conteúdo, com diagnóstico que cita o caminho pedido e o motivo. Vale
  para `#include`, `#import`, `image`, `bibliography`, estilo `.csl`, `read`, funções de dados
  (`json`, `yaml`, `toml`, `csv`, `xml`, `cbor`), `plugin` e autocompletar de caminhos.
  Caminho iniciado por `/` é relativo à raiz e permitido (RF-08, docs/arquitetura.md §6.2).
- **FR-022**: A verificação de FR-021 MUST valer para o arquivo efetivamente lido, de modo que
  um link simbólico trocado entre a verificação e a leitura não escape da raiz
  (docs/arquitetura.md §6.2, passo 6).
- **FR-023**: O app MUST NOT oferecer configuração de pastas extras fora do projeto. Quando o
  `.typ` aponta para fora, o app MUST oferecer copiar o arquivo para dentro do projeto, só com
  confirmação da pessoa (D-13).
- **FR-024**: Arquivos do projeto (`.typ`, `.bib`, `.yml`, `.csl`, imagens, dados e
  `.prelo/project.json`) MUST ser tratados como entrada não confiável: conteúdo malformado
  MUST gerar diagnóstico com arquivo e, quando houver, linha, sem encerrar nem travar o app
  (RF-09, Princípio VI).

**Bibliografia e figuras**

- **FR-025**: A bibliografia MUST aceitar BibLaTeX `.bib` e Hayagriva `.yml`/`.yaml`, um ou
  vários arquivos, estilos `.csl` do projeto e estilos embutidos do Typst (RF-05, RF-09).
- **FR-026**: Figuras MUST aceitar PNG, JPEG, GIF (estática) e SVG, e PDF e WebP se a versão
  embutida do compilador os suportar (RF-50).
- **FR-027**: Uma edição que não altera os bytes de uma imagem MUST NOT decodificá-la de
  novo; o mesmo vale para bibliografia já processada (RF-51, docs/arquitetura.md §6.2,
  passo 7).

**Exportação**

- **FR-028**: A pessoa MUST poder exportar o PDF do artigo para um local escolhido; o PDF MUST
  vir da mesma compilação da prévia (RF-06, RF-42 parcial).
- **FR-029**: Exportar MUST ser recusado quando o projeto tem erro de compilação, listando os
  erros.

**Testes**

- **FR-030**: A suíte MUST incluir testes de segurança com projetos de fixture para:
  - escape da raiz: `..` além da raiz, `/` seguido de `..`, `#include` subindo além da raiz,
    link simbólico versionado para fora, link simbólico interno (aceito), FIFO na raiz, troca
    de link entre verificação e leitura, caminho de pacote saindo da pasta do pacote, fonte
    fora de `fonts/` e autocompletar de caminhos;
  - entrada malformada: `.typ` inválido e não UTF-8, `.bib` com chave não fechada, YAML
    inválido, XML de CSL inválido, imagem com conteúdo inválido, arquivo de fonte inválido,
    `.prelo/project.json` malformado e compilação que não termina;
  - pacotes: arquivo compactado malformado, entrada com `..`, caminho absoluto, link
    simbólico ou link físico, pacote adulterado no cache e pacote ausente sem rede;
  - plugins: plugin sem confirmação não roda, plugin recusado não roda, plugin aprovado com
    conteúdo alterado pede confirmação de novo, aprovação gravada dentro da pasta do projeto
    é ignorada.

  Cada caso recusado MUST verificar o diagnóstico e a ausência de leitura fora da raiz ou de
  execução não autorizada (Princípio VI, RF-08, RF-09, RF-50, D-15).
- **FR-031**: A suíte MUST incluir teste de equivalência: projetos de fixture com `.bib`,
  `.yml`, `.csl` e uma figura de cada formato geram, no Prelo, o mesmo PDF que o `typst` de
  linha de comando na versão registrada (RF-09, RF-50).
- **FR-032**: A suíte MUST incluir teste que conta decodificações de imagem ao longo de uma
  sequência de edições de texto (uma por imagem) e medição automatizada da meta de prévia
  (RF-51, RNF-03).

**Fora de escopo**

- **FR-033**: Esta feature MUST NOT incluir: comentários e painel de revisão (RF-16, RF-17);
  git, commit, push e versões nomeadas (RF-40, RF-41); Salvar que sincroniza (RF-33 a RF-38);
  edição em tempo real, presença, rede P2P, mDNS, relay e documento compartilhado entre
  computadores (RF-20, RF-23); convites, identidade por chave e papéis (RF-21, RF-22, RF-24);
  modos Sugerindo e Lendo, sugestões e prévia com marcações (RF-10 a RF-15, RF-18, RF-19);
  arquivos pendentes (RF-52 a RF-54), que só existem com transporte entre coautores (D-12);
  PDF com comentários e PDF de diferenças (RF-43, RF-44); exportar `.typ` limpo e tela
  "Preparar submissão" (RF-42 parte `.typ`, RF-45).

### Key Entities

- **Projeto**: pasta local cuja raiz delimita tudo o que o compilador pode ler; contém o
  arquivo principal, outros `.typ`, bibliografia, estilos, figuras e `.prelo/`.
- **Registro do projeto** (`.prelo/project.json`): versão do compilador e arquivo principal.
  Nas fases seguintes ganha participantes e papéis; nesta fase só esses dois campos são
  usados.
- **Arquivo principal**: o `.typ` a partir do qual o artigo compila.
- **Compilação**: resultado de compilar o projeto num instante; gera prévia, diagnósticos e,
  sob pedido, PDF.
- **Diagnóstico**: erro ou aviso com arquivo, posição, mensagem e, nas recusas de acesso, o
  caminho pedido e o motivo.
- **Versão do compilador**: versão embutida no app e versão registrada no projeto; podem
  diferir.
- **Pasta de fontes do projeto** (`fonts/`): arquivos de fonte do projeto, somados às fontes
  embutidas no app.
- **Cache de pacotes**: pacotes `@preview` baixados, por nome e versão, cada um com a impressão
  digital registrada no download; local a este computador, fora do projeto.
- **Aprovação de plugin**: decisão da pessoa (aprovado ou recusado) para um plugin, ligada ao
  projeto e ao conteúdo do plugin; guardada neste computador, fora do projeto.
- **Posição de origem**: ligação entre um ponto da prévia e um arquivo e linha do projeto,
  usada na navegação código ↔ prévia.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Em 95% das edições num artigo de fixture de 40 páginas, a prévia mostra a
  mudança em menos de 100 ms após a edição, em computador de referência definido no plano
  (RF-02, RNF-03).
- **SC-002**: A meta de SC-001 vale também para edições de texto num artigo de fixture com
  imagens grandes que a edição não toca (RF-51).
- **SC-003**: 100% dos casos de fixture de acesso fora da raiz são recusados com diagnóstico,
  e em 0 casos há leitura de conteúdo fora da raiz (RF-08).
- **SC-004**: 100% dos casos de fixture de entrada malformada geram diagnóstico e o app
  continua respondendo a edições em até 1 s depois (RF-09, Princípio VI).
- **SC-005**: 100% dos projetos de fixture de equivalência geram PDF idêntico, página a
  página, ao do `typst` de linha de comando na versão registrada (RF-06, RF-09, RF-50).
- **SC-006**: Prelo, prévia e exportação funcionam em macOS, Windows e Linux sem `typst`
  instalado, verificado na verificação remota nos três sistemas (RNF-01, RF-06).
- **SC-007**: Em 95% dos cliques na prévia sobre texto do projeto, o editor vai para a linha
  correta de origem (RF-03).
- **SC-008**: Uma pessoa que já escreve em Typst abre um projeto existente, edita e exporta o
  PDF em até 5 minutos no primeiro uso, sem consultar documentação.
- **SC-009**: Nenhum trabalho salvo é perdido em testes de encerramento forçado do app; no
  máximo os últimos 2 s de digitação não salvos (Princípio III).
- **SC-010**: Cada requisito funcional de comportamento (FR-001 a FR-029) tem ao menos um teste
  automatizado que falhou antes da implementação (Princípio VI).

## Assumptions

- A tecnologia já está decidida pela constituição e por D-01 e D-10 (shell Tauri, núcleo Rust,
  compilador Typst e Tinymist embutidos como bibliotecas, editor CodeMirror 6); detalhes ficam
  para o `plan.md`.
- **Versão do Typst embutida**: a versão estável mais recente no momento do plano, que se
  espera ser 0.14.0 ou posterior; se for anterior, PDF e WebP saem de FR-026
  (TODO(versão-typst) de RF-50). O plano registra a versão escolhida.
- **Conteúdo de um projeto novo**: um `main.typ` mínimo (título, autor, um parágrafo, sem
  pacotes nem fontes específicas) que compila em qualquer computador, mais
  `.prelo/project.json`. Modelos de artigo (ABNT, IEEE etc.) ficam para depois.
- **Pasta de fontes**: `fonts/` na raiz do projeto, com subpastas. Fora do Prelo, o mesmo
  projeto compila com `typst compile --font-path fonts` (Princípio I).
- **Aprovação de plugin fora do projeto**: guardá-la em `.prelo/` permitiria que um coautor
  pré-aprovasse plugins para os outros nas fases seguintes; por isso fica nos dados locais do
  app, ligada à raiz do projeto e à impressão digital do plugin.
- **Registro de pacotes**: o registro oficial do Typst; pacotes `@local` e outros namespaces
  ficam fora (D-13).
- **Arquivo principal**: `main.typ` quando existir; caso contrário, a pessoa escolhe, e a
  escolha fica no registro do projeto.
- **Salvamento automático**: até 2 s depois que a pessoa para de digitar. Nesta fase, Salvar
  grava só no disco; Salvar que sincroniza (RF-33) vem com git na fase 2.
- **Documento compartilhado (Yjs)**: nesta fase o texto vem do disco e do editor local; se o
  plano já adotar o documento Yjs para `.typ`, `.bib` e `.yml` (D-02, D-14), o comportamento
  visível desta spec não muda.
- **Imagem grande de referência**: para SC-002, uma imagem de pelo menos 4000×3000 pixels e
  10 MB; a meta da primeira compilação depois que a imagem muda fica sem meta nesta fase
  (TODO(imagem-grande) de RF-51).
- **Computador de referência e limite de tempo de compilação** (FR-015) são
  definidos no plano.
- **Exportar `.typ` limpo** (RF-42) não se aplica nesta fase: sem sugestões nem comentários, o
  `.typ` no disco já é o texto final.
- Arquivos especiais e links simbólicos são tratados como em docs/arquitetura.md §6.2 nos três
  sistemas; o plano detalha o equivalente no Windows (junções e atalhos).
