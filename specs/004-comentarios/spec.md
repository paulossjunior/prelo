# Feature Specification: Comentários (fase 2, parte 1)

**Feature Branch**: `004-comentarios`

**Created**: 2026-11-26

**Status**: Draft

**Input**: User description: "004-comentarios — Release R2 (1.1.0), parte 1 de 2 (a parte 2 será a spec 005, git: Salvar com commit/push/pull, versões nomeadas, Fechar sincroniza). Escopo da 004: comentários ancorados num trecho do .typ, com respostas, menções, resolver, reabrir e excluir, guardados em .prelo/review.json (RF-16, D-03, Princípios I e V); aba "Resolvidos" (RF-17); regras por autor (apagar a própria mensagem, excluir conversa com confirmação; requisitos.md §1); contagem de comentários abertos por seção no outline (RF-04 parcial); PDF com comentários na margem (RF-43, incluído na 1.1.0 por decisão do dono). Decisões do dono (2026-11-26): Q-07 — adotar o Yjs já na R2, âncoras por posição relativa do Yjs desde o início (sem migração na R3); Q-05 — RNF-02 vale só para servidores do Prelo, o remoto git do grupo é aceito; Q-03 — RF-43 entra na 1.1.0. Critérios de saída da R2 que valem aqui: .typ continua Typst puro (nenhum comentário dentro dele; compilar com typst CLI); fechar o app não perde comentário (RNF-04, teste de encerramento forçado); testes de segurança para .prelo/review.json malformado ou adulterado (Princípio VI). Fontes: docs/release-plan.md (R2), docs/requisitos.md, docs/especificacao.md, docs/adrs (D-02, D-03), constituição."

**Origem**: docs/especificacao.md §4 item 2 · docs/release-plan.md R2 · RF-04 (parcial),
RF-16, RF-17, RF-43 · RNF-04, RNF-05 · D-02, D-03 · docs/requisitos.md §1 (regras sobre
comentários) · docs/arquitetura.md §4 (Comentários) · Constituição: Princípios I, III, V e VI

**Exceção à transição entre features**: esta spec começa antes de a `1.0.0` estar em produção
(release publicada, `.dmg` validado no macOS, issues fechadas, QA APROVADO). O dono aprovou em
2026-11-26 a exceção **só para especificação e planejamento**: spec, plan e tasks da 004 podem
ser escritos agora; a implementação só começa depois que a `1.0.0` cumprir a regra de
"em produção". A justificativa formal fica no plan.md (Complexity Tracking).

## User Scenarios & Testing *(mandatory)*

Nesta fase o Prelo ganha comentários, mas ainda não tem tempo real nem papéis. Cada pessoa
comenta no próprio computador, e os comentários chegam aos coautores pelo git do grupo, que é
a spec 005. Esta spec cobre o que acontece no computador de uma pessoa: criar, responder,
resolver, reabrir e excluir comentários, vê-los no texto, no outline e no PDF, e garantir que
eles ficam fora do `.typ`.

Como na fase 1, a pasta do projeto, inclusive `.prelo/review.json`, é **entrada não
confiável**: ela pode chegar de outro coautor pelo git, editada à mão ou adulterada
(Princípio VI).

### User Story 1 - Comentar um trecho e conversar sobre ele (Priority: P1)

Um orientador abre o artigo da orientanda no Prelo, seleciona uma frase e escreve "Qual
organização?". O trecho fica marcado no editor e o comentário aparece no painel de revisão,
com autor e hora. A orientanda responde na mesma conversa e menciona o coautor com
`@bruno`. Os dois continuam escrevendo o texto, inclusive antes e dentro do trecho comentado,
e a marca acompanha o trecho. O arquivo `.typ` continua Typst puro.

**Why this priority**: é o núcleo da fase 2 (RF-16). Sem criar e ler conversas ancoradas no
texto, nenhuma outra história desta spec tem uso.

**Independent Test**: abrir um projeto de fixture, selecionar um trecho, comentar, responder
com uma menção, editar o texto antes e dentro do trecho, fechar e reabrir o app; verificar que
a conversa está lá, ancorada no trecho certo, e que o `.typ` compila no `typst` de linha de
comando sem nenhum vestígio do comentário.

**Acceptance Scenarios**:

1. **Given** um projeto aberto e um trecho selecionado no editor, **When** a pessoa escolhe
   "Comentar" e escreve o texto, **Then** o trecho fica marcado no editor e uma conversa nova
   aparece no painel de revisão com o texto, o nome do autor e a hora.
2. **Given** uma conversa aberta, **When** a pessoa escreve uma resposta, **Then** a resposta
   aparece na mesma conversa, abaixo da anterior, com autor e hora.
3. **Given** a pessoa digitando `@` numa mensagem, **When** ela continua o nome, **Then** o
   app sugere os nomes de pessoas já conhecidas no projeto e, ao escolher um, a menção fica
   destacada na mensagem gravada.
4. **Given** uma conversa ancorada num trecho, **When** a pessoa insere ou apaga texto antes
   do trecho, ou edita dentro dele, **Then** a marca continua sobre o mesmo trecho lógico.
5. **Given** um clique numa conversa do painel, **When** o trecho existe, **Then** o editor
   rola até o trecho e o seleciona; um clique na marca do editor destaca a conversa no painel.
6. **Given** comentários criados, **When** a pessoa compila o `.typ` com o `typst` de linha de
   comando, **Then** a compilação é idêntica à de um projeto sem comentários: nenhuma marca,
   nenhum caractere extra no `.typ`.
7. **Given** um comentário confirmado na tela, **When** o app é encerrado à força logo depois,
   **Then** ao reabrir o projeto o comentário está lá.

---

### User Story 2 - Resolver, reabrir e consultar os resolvidos (Priority: P1)

A orientanda ajusta a frase e marca a conversa como resolvida. A marca some do editor e a
conversa vai para a aba "Resolvidos". Semanas depois o orientador percebe que a dúvida
voltou, encontra a conversa em "Resolvidos" e a reabre.

**Why this priority**: sem resolver, o painel acumula conversas encerradas e perde utilidade
(RF-16, RF-17, D-03). Comentário é resolvido, não aceito.

**Independent Test**: com conversas de fixture, resolver uma, conferir que sai do painel e da
marcação do editor e aparece em "Resolvidos"; reabri-la e conferir que volta ao painel no
mesmo trecho.

**Acceptance Scenarios**:

1. **Given** uma conversa aberta, **When** a pessoa escolhe "Resolver", **Then** a conversa
   sai da lista de abertas, a marca some do editor e a conversa aparece na aba "Resolvidos" com
   quem resolveu e quando.
2. **Given** uma conversa resolvida, **When** a pessoa escolhe "Reabrir" na aba "Resolvidos",
   **Then** ela volta às abertas com o histórico completo e a marca reaparece no trecho.
3. **Given** uma conversa resolvida, **When** a pessoa responde a ela, **Then** a resposta é
   gravada e a conversa continua resolvida até alguém reabrir.
4. **Given** a aba "Resolvidos", **When** a pessoa a abre, **Then** vê as conversas
   resolvidas da mais recente para a mais antiga, cada uma com o trecho a que se refere.
5. **Given** comentários e sugestões no mesmo painel em fases futuras, **When** a pessoa age
   sobre um comentário, **Then** as únicas ações são responder, resolver, reabrir e excluir;
   não existe aceitar ou rejeitar comentário (D-03).

---

### User Story 3 - Apagar mensagens e excluir conversas com segurança (Priority: P2)

O coautor escreveu uma resposta errada e a apaga. Quem abriu a conversa decide excluí-la
inteira, e o app pede confirmação. Ao fim da revisão, o dono do projeto limpa todos os
resolvidos de uma vez.

**Why this priority**: as regras por autor evitam que uma pessoa apague o que outra escreveu
(docs/requisitos.md §1, Princípio V). Não bloqueiam o uso básico, mas sem elas a conversa não
é confiável.

**Independent Test**: com conversas de fixture de autores diferentes, conferir que cada ação
proibida está indisponível e é recusada pelo núcleo, e que cada ação permitida funciona, com
confirmação onde exigida.

**Acceptance Scenarios**:

1. **Given** uma mensagem escrita pela própria pessoa, **When** ela escolhe "Apagar
   mensagem", **Then** a mensagem some e a conversa mostra que houve uma mensagem apagada.
2. **Given** uma mensagem de outra pessoa, **When** a pessoa tenta apagá-la, **Then** a ação
   não aparece; se chegar ao núcleo mesmo assim, é recusada e nada muda.
3. **Given** uma conversa aberta pela própria pessoa, ou a pessoa é o dono do projeto,
   **When** ela escolhe "Excluir conversa", **Then** o app pede confirmação e só exclui
   depois dela; cancelar não muda nada.
4. **Given** uma conversa aberta por outra pessoa e quem age não é o dono, **When** tenta
   excluí-la, **Then** a ação não aparece e o núcleo recusa.
5. **Given** a aba "Resolvidos" e a pessoa é o dono, **When** escolhe "Excluir todos os
   resolvidos" e confirma, **Then** todas as conversas resolvidas são excluídas e as abertas
   continuam intactas; para quem não é o dono a ação não aparece e o núcleo recusa.
6. **Given** a primeira mensagem da conversa apagada pelo autor, **When** a conversa tem
   respostas, **Then** a conversa continua com as respostas e o trecho ancorado.

---

### User Story 4 - Ver onde há comentários pelo outline (Priority: P3)

Antes de uma reunião, a orientanda olha o outline e vê quantos comentários abertos há em cada
seção. Clica na seção com mais comentários e vai direto para ela.

**Why this priority**: ajuda a priorizar a revisão em artigos longos (RF-04, contagem de
pendências; nesta fase só comentários, sugestões entram na R4). Complementa o painel, não o
substitui.

**Independent Test**: com um projeto de fixture com comentários em seções diferentes, conferir
que o outline mostra a contagem certa por seção e que ela muda ao criar, resolver, reabrir e
excluir.

**Acceptance Scenarios**:

1. **Given** comentários abertos em seções diferentes, **When** a pessoa olha o outline,
   **Then** cada seção mostra o número de conversas abertas ancoradas nela; seção sem
   comentários abertos não mostra número.
2. **Given** uma conversa resolvida, reaberta, criada ou excluída, **When** a ação termina,
   **Then** a contagem da seção atualiza sem recarregar o projeto.
3. **Given** uma conversa cujo trecho atravessa duas seções, **When** o outline é exibido,
   **Then** ela conta só na seção onde o trecho começa.

---

### User Story 5 - Exportar o PDF com comentários na margem (Priority: P3)

O orientador prefere ler no tablet. A orientanda exporta um PDF com os comentários abertos na
margem, cada um ligado ao seu trecho, e envia por e-mail.

**Why this priority**: pedido para orientadores (RF-43), incluído na `1.1.0` por decisão do
dono (Q-03). Depende das histórias 1 e 2.

**Independent Test**: com um projeto de fixture com comentários abertos e resolvidos,
exportar o PDF com comentários e conferir que cada comentário aberto aparece na margem junto
ao seu trecho, que os resolvidos não aparecem, e que o `.typ` e o PDF final continuam iguais
aos de antes.

**Acceptance Scenarios**:

1. **Given** um projeto que compila e tem conversas abertas, **When** a pessoa escolhe
   "Exportar PDF com comentários" e um destino, **Then** o app grava um PDF com o documento e,
   na margem de cada página, as conversas abertas daquela página (autor, hora e mensagens),
   cada uma ligada visualmente ao seu trecho.
2. **Given** conversas resolvidas, **When** o PDF com comentários é gerado, **Then** elas não
   aparecem.
3. **Given** um projeto com erro de compilação, **When** a pessoa tenta exportar o PDF com
   comentários, **Then** o app recusa como no PDF final (fase 1), mostrando os erros.
4. **Given** o PDF com comentários exportado, **When** a pessoa exporta o PDF final ou compila
   com o `typst` de linha de comando, **Then** o resultado é idêntico ao de antes: o PDF com
   comentários não altera o `.typ` nem o PDF final.

---

### Edge Cases

- **Trecho comentado apagado por inteiro**: a conversa não some em silêncio (Princípio III).
  Ela fica marcada como "trecho removido" no painel, e a pessoa decide se ancora num trecho
  novo ou exclui a conversa.
- **`.typ` alterado fora do Prelo** (outro editor, ou `git pull` na spec 005): ao reabrir ou
  detectar a mudança, o app reconcilia o texto e reposiciona as âncoras; âncora que não puder
  ser recuperada com segurança vira "trecho removido" e segue a regra acima. Nenhuma conversa é
  descartada.
- **Comentário em arquivo que não é o principal** (outro `.typ` do projeto): funciona igual;
  o painel indica o arquivo, e clicar abre o arquivo no trecho.
- **Arquivo comentado renomeado ou excluído do projeto**: as conversas desse arquivo ficam
  como "trecho removido" com o nome antigo do arquivo, até a pessoa reancorar ou excluir.
- **Seleção vazia**: "Comentar" fica indisponível sem trecho selecionado.
- **Mensagem vazia ou só com espaços**: não é gravada.
- **Mensagem muito longa**: o app aceita até o limite definido em FR-021 e recusa acima dele
  com aviso, sem cortar o texto em silêncio.
- **Menção a nome que não existe no projeto**: fica como texto comum, sem destaque.
- **Nome da pessoa ainda não definido**: na primeira vez que for comentar, o app pede o nome
  que vai aparecer como autor; sem nome, não grava o comentário.
- **Dois comentários sobre o mesmo trecho ou trechos sobrepostos**: cada um é uma conversa
  própria; o editor mostra a sobreposição e o clique escolhe entre elas.
- **`.prelo/review.json` malformado, truncado ou adulterado**: o app abre o projeto e o
  editor normalmente, avisa que os comentários não puderam ser lidos, não sobrescreve o
  arquivo e guarda uma cópia antes de qualquer gravação nova (FR-030 a FR-034).
- **`.prelo/review.json` com versão de formato mais nova que a do app**: o app mostra os
  comentários só para leitura e avisa que é preciso atualizar o Prelo; não grava nada no
  arquivo.
- **Texto de mensagem com marcação ou código** (HTML, script, sintaxe Typst, caracteres de
  controle): aparece como texto literal no app e no PDF com comentários, sem ser executado,
  interpretado ou quebrar a compilação.
- **Disco cheio ou sem permissão de escrita em `.prelo/`**: a ação não é confirmada; o app
  mostra o erro e mantém o texto digitado na caixa para tentar de novo.
- **Projeto com centenas de conversas**: painel, outline e PDF continuam utilizáveis
  (SC-005).

## Requirements *(mandatory)*

### Functional Requirements

**Criar e conversar**

- **FR-001**: A pessoa MUST poder criar uma conversa a partir de um trecho selecionado num
  arquivo `.typ` do projeto, com uma primeira mensagem (RF-16).
- **FR-002**: Cada mensagem MUST registrar autor, data e hora e texto; cada conversa MUST ter
  um identificador único, o arquivo e o trecho a que se refere, o estado (aberta ou
  resolvida) e o autor que a abriu.
- **FR-003**: A pessoa MUST poder responder a qualquer conversa, aberta ou resolvida; a
  resposta não muda o estado.
- **FR-004**: A pessoa MUST poder mencionar outra com `@nome`, com sugestão dos nomes já
  conhecidos no projeto (autores de mensagens e nomes registrados no projeto). A menção MUST
  ficar gravada de forma identificável na mensagem e aparecer destacada (RF-16). Nesta fase a
  menção não gera notificação (notificações são da R5).
- **FR-005**: Antes do primeiro comentário, o app MUST pedir o nome de exibição da pessoa e
  guardá-lo nas preferências locais do app, fora do projeto; a pessoa MUST poder alterá-lo
  depois. Mudar o nome não reescreve mensagens antigas.

**Âncoras**

- **FR-006**: A âncora de cada conversa MUST ser feita por posição relativa do documento CRDT
  (Yjs), e não por número de linha ou deslocamento fixo (Princípio V; decisão do dono Q-07,
  2026-11-26). A âncora MUST acompanhar o trecho quando houver edição antes, depois ou dentro
  dele.
- **FR-007**: O app MUST manter, para cada `.typ` comentado, o estado do documento CRDT
  necessário para resolver as âncoras, guardado em `.prelo/`, nunca no `.typ`
  (Princípio I).
- **FR-008**: Quando o conteúdo de um `.typ` no disco diferir do conhecido pelo app (edição
  externa ou chegada pelo git), o app MUST reconciliar o documento CRDT com o conteúdo do disco
  como uma edição, preservando as âncoras dos trechos que continuam existindo.
- **FR-009**: Conversa cujo trecho deixou de existir MUST ficar no estado "trecho removido",
  visível no painel; a pessoa MUST poder reancorá-la num trecho novo ou excluí-la. O app MUST
  NOT descartar conversa em silêncio (Princípio III).

**Exibir**

- **FR-010**: O editor MUST marcar os trechos com conversas abertas; trechos de conversas
  resolvidas MUST NOT ficar marcados.
- **FR-011**: O app MUST ter um painel de revisão com as conversas abertas, na ordem em que
  aparecem no documento, e uma aba "Resolvidos" com as resolvidas, da mais recente para a mais
  antiga (RF-17).
- **FR-012**: Clicar numa conversa MUST levar o editor ao trecho (abrindo o arquivo se
  preciso); clicar numa marca do editor MUST destacar a conversa correspondente.
- **FR-013**: O outline MUST mostrar, por seção, o número de conversas abertas cujo trecho
  começa nela, atualizado após cada ação sobre comentários e cada edição que mova trechos
  entre seções (RF-04, parcial).
- **FR-014**: Texto de mensagens MUST ser exibido como texto literal, no app e no PDF: nenhuma
  marcação HTML, script ou sintaxe Typst contida nele é interpretada.

**Resolver, reabrir, apagar, excluir**

- **FR-015**: A pessoa MUST poder resolver uma conversa aberta e reabrir uma resolvida; o app
  MUST registrar quem fez e quando (RF-16, RF-17, D-03).
- **FR-016**: Comentários MUST NOT ter ações de aceitar ou rejeitar (D-03).
- **FR-017**: Só o autor de uma mensagem MUST poder apagá-la. Mensagem apagada MUST deixar
  na conversa uma indicação de que existiu, sem o texto.
- **FR-018**: Só quem abriu a conversa e o dono do projeto MUST poder excluí-la inteira, e só
  após confirmação explícita (Princípio V).
- **FR-019**: Só o dono do projeto MUST poder "Excluir todos os resolvidos", após confirmação
  explícita.
- **FR-020**: As permissões de FR-017 a FR-019 MUST ser verificadas no núcleo, não só na
  interface: um pedido proibido que chegue ao núcleo MUST ser recusado sem mudar nada
  (Princípio VI, permissões).
- **FR-021**: O app MUST recusar mensagem vazia e mensagem acima de 10.000 caracteres, com
  aviso, sem cortar o texto em silêncio.

**Persistência**

- **FR-022**: Conversas, mensagens e estados MUST ser gravados em `.prelo/review.json` (D-02),
  e nunca dentro de um `.typ` (Princípio I, RNF-05).
- **FR-023**: Cada ação sobre comentários MUST estar gravada no disco antes de ser confirmada
  na tela; uma ação confirmada MUST sobreviver a encerramento forçado do app (Princípio III,
  RNF-04).
- **FR-024**: A gravação de `.prelo/review.json` MUST ser atômica: uma interrupção no meio da
  gravação MUST deixar no disco a versão anterior ou a nova, nunca um arquivo parcial.
- **FR-025**: O formato de `.prelo/review.json` MUST ter número de versão e MUST guardar cada
  conversa por identificador próprio, de modo que conversas criadas em computadores diferentes
  não colidam e possam ser combinadas pela spec 005.
- **FR-026**: Ações de comentário MUST funcionar sem conexão de rede (Princípio IV: offline,
  comentar é permitido).

**Dono do projeto**

- **FR-027**: Até a R5 (papéis), "dono do projeto" MUST ser o nome registrado como dono em
  `.prelo/project.json`. Projetos criados no Prelo registram como dono o nome de exibição de
  quem criou; projetos abertos sem dono registrado pedem, na primeira ação que exige dono, que
  a pessoa confirme se é o dono, e registram a resposta.

**PDF com comentários**

- **FR-028**: A pessoa MUST poder exportar um "PDF com comentários" com as conversas abertas
  na margem, ao lado da página do seu trecho e ligadas visualmente a ele, a partir da mesma
  compilação da prévia (RF-43, RF-06). Conversas resolvidas e em "trecho removido" MUST NOT
  aparecer.
- **FR-029**: Exportar o PDF com comentários MUST NOT alterar o `.typ`, `.prelo/review.json`
  nem o PDF final; com erro de compilação, a exportação MUST ser recusada como a do PDF final.

**Entrada não confiável**

- **FR-030**: O app MUST validar `.prelo/review.json` ao ler: formato, versão, tipos, tamanho
  máximo do arquivo e de cada campo, identificadores únicos e âncoras que se refiram a
  arquivos do projeto.
- **FR-031**: Com `.prelo/review.json` inválido, o app MUST abrir o projeto e editar
  normalmente, avisar que os comentários não puderam ser lidos e MUST NOT sobrescrever o
  arquivo; antes de gravar um arquivo novo, MUST guardar uma cópia do inválido em `.prelo/`.
- **FR-032**: Conversas individuais inválidas num arquivo válido MUST ser ignoradas na exibição
  e listadas num aviso, sem derrubar as demais; elas MUST ser preservadas na próxima gravação.
- **FR-033**: Caminhos de arquivo em `.prelo/review.json` MUST obedecer ao confinamento do
  projeto (RF-08, D-13): âncora para arquivo fora da raiz MUST ser recusada.
- **FR-034**: Arquivo `.prelo/review.json` com versão de formato mais nova que a suportada
  MUST ser exibido só para leitura, sem gravação.

**Fora de escopo**

- **FR-035**: Ficam fora desta spec: sincronização pelo git, commit, push, versões nomeadas e
  combinação de `.prelo/review.json` entre coautores (spec 005); tempo real, presença e
  identidade por chave (R3); sugestões e contagem de sugestões no outline (R4); papéis,
  convites e notificações (R5).

### Key Entities

- **Conversa**: identificador único; arquivo do projeto; âncora de início e fim (posições
  relativas do CRDT); estado (aberta, resolvida, trecho removido como condição derivada);
  autor que abriu; data de abertura; quem resolveu ou reabriu por último e quando; lista de
  mensagens.
- **Mensagem**: identificador único na conversa; autor; data e hora; texto; menções; marca de
  apagada.
- **Pessoa (nome de exibição)**: nome que identifica autor de mensagens e menções nesta fase;
  guardado nas preferências locais. Sem verificação criptográfica até a R3.
- **Dono do projeto**: nome registrado em `.prelo/project.json` (FR-027).
- **Estado do documento CRDT**: estado por arquivo `.typ` comentado, em `.prelo/`, usado para
  resolver as âncoras.
- **Arquivo de revisão (`.prelo/review.json`)**: versão do formato e conversas por
  identificador.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Em 100% dos projetos de fixture com comentários, o `.typ` compila com o `typst`
  de linha de comando na mesma versão e gera PDF idêntico, página a página, ao do mesmo projeto
  sem comentários (Princípio I, RNF-05).
- **SC-002**: Em 100% das execuções do teste de encerramento forçado, todo comentário
  confirmado na tela está presente ao reabrir (RNF-04).
- **SC-003**: Em um roteiro de fixture com 200 edições aleatórias antes, dentro e depois de
  trechos comentados, 100% das âncoras de trechos que continuam existindo apontam para o mesmo
  texto lógico, e 100% das de trechos apagados viram "trecho removido" (nenhuma some).
- **SC-004**: 100% dos casos de fixture de `.prelo/review.json` malformado, truncado,
  adulterado ou com caminho fora do projeto são recusados ou isolados sem perda do arquivo
  original, e o app continua editando o projeto; cada caso responde em até 1 s.
- **SC-005**: Com 500 conversas num artigo de 40 páginas, criar, resolver ou reabrir uma
  conversa aparece na tela em até 200 ms, e a prévia continua atualizando em menos de 100 ms
  após edição (RF-02, RNF-03).
- **SC-006**: 100% das ações proibidas pelas regras de comentário (docs/requisitos.md §1) têm
  teste de recusa no núcleo.
- **SC-007**: Numa revisão manual do PDF com comentários de fixture, 100% das conversas abertas
  aparecem na página certa, junto ao trecho, e nenhuma resolvida aparece.
- **SC-008**: Uma pessoa que nunca usou o Prelo cria, responde e resolve um comentário em menos
  de 1 minuto, sem ajuda, no roteiro do quickstart.

## Assumptions

- **Identidade nesta fase**: o autor é um nome de exibição local, sem verificação. Quem edita
  `.prelo/review.json` à mão ou muda o próprio nome pode se passar por outra pessoa; isso é
  aceito até a R3, que traz identidade por chave criptográfica (Princípio II). As regras por
  autor (FR-017 a FR-020) protegem contra engano na interface, não contra um coautor mal
  intencionado com acesso ao repositório.
- **Papéis**: sem papéis até a R5. Toda pessoa que abre o projeto pode comentar, resolver e
  reabrir; "dono" segue FR-027. Leitor (que não comenta) só existe a partir da R5.
- **Yjs desde já** (Q-07): o documento CRDT existe nesta fase só para dar âncoras estáveis aos
  comentários; não há rede nem tempo real. A R3 reaproveita o mesmo documento.
- **RNF-02** (Q-05): vale para servidores do Prelo. O remoto git do grupo, usado na spec 005,
  é aceito.
- **Combinação entre coautores**: esta spec garante um formato combinável (FR-025); a regra de
  combinação e o tratamento de conflitos ficam na spec 005.
- **PDF com comentários**: o leiaute da margem (largura, fonte, ligação ao trecho) é definido
  no plano; o requisito é legibilidade e ligação clara a cada trecho. TODO(export-revisao) de
  docs/requisitos.md (arquivos pendentes) não se aplica antes da R3.
- **Limites**: 10.000 caracteres por mensagem e limites de tamanho de `.prelo/review.json`
  definidos no plano, com folga para 500 conversas (SC-005).
- **Plataformas**: macOS, Windows e Linux (RNF-01); validação manual de produção no macOS
  (constituição v1.4.0).
