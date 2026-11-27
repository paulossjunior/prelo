# Feature Specification: Sincronização pelo git (fase 2, parte 2)

**Feature Branch**: `005-git-sincronizacao`

**Created**: 2026-11-27

**Status**: Draft

**Input**: User description: "Spec 005 — Release R2 (1.1.0), parte 2 de 2: git. Salvar (⌘S) recebe, envia e grava commit com push; sem conexão guarda e agenda (RF-33, RF-34, D-08); Fechar faz um último Salvar (RF-36); ao abrir, verificar e receber do git remoto antes de liberar a edição (RF-32, só git); resumo do que mudou (RF-38, só git); commit e push pelo app (RF-40); versões nomeadas em .prelo/versions.json (RF-41). Login no GitHub por GitHub App com device flow (D-18). Decisões do dono (2026-11-27): GitHub App na conta pessoal paulossjunior; device flow; conflito de .typ mesclado pelo Yjs (estados .prelo/crdt da 004); o app abre pasta que já é clone e também clona um repositório do GitHub (criar repositório fica para depois)."

**Origem**: docs/especificacao.md §4 item 2 · docs/release-plan.md R2 · RF-32 (só git), RF-33,
RF-34, RF-36, RF-38 (só git), RF-40, RF-41 · RNF-02, RNF-03, RNF-04 · D-05, D-08, D-18 ·
docs/requisitos.md §5 · docs/arquitetura.md §5 · Constituição: Princípios I, II, III, IV e VI ·
depende de `specs/004-comentarios`

**Exceção à transição entre features**: como a 004, esta spec começa antes de a `1.0.0` estar
em produção. O dono pediu a spec 005 em 2026-11-27, sob a mesma exceção aprovada em
2026-11-26 **só para especificação e planejamento**. A implementação espera a `1.0.0` em
produção e, pela ordem do plano, a 004.

## User Scenarios & Testing *(mandatory)*

A colaboração desta fase é **assíncrona**: coautores trabalham em horários diferentes e o
repositório git do grupo no GitHub é o canal (D-05). Não há tempo real nem pares (R3). O gesto
central é um só: **Salvar** recebe o que os outros enviaram, envia o próprio trabalho e grava
um commit (D-08).

Projeto **sem remoto git** continua como na `1.0.0`: edição local, Salvar só grava no disco.
Tudo abaixo vale para projeto com remoto no GitHub.

O repositório remoto, os commits que chegam e os arquivos `.prelo/` vindos de outros coautores
são **entrada não confiável** (Princípio VI).

### User Story 1 - Conectar ao GitHub e abrir o projeto do grupo (Priority: P1)

A orientanda nunca usou git. Ela escolhe "Entrar com GitHub", o app mostra um código, ela o
digita no site do GitHub e autoriza o Prelo só no repositório do artigo. Depois escolhe
"Clonar do GitHub", seleciona o repositório e uma pasta vazia, e o projeto abre. O orientador,
que já tinha o repositório clonado, só abre a pasta.

**Why this priority**: sem credencial e sem projeto ligado ao remoto, nenhuma outra história
funciona (RF-40, D-18).

**Independent Test**: com um repositório de teste no GitHub, fazer login por device flow,
clonar pelo app, abrir uma pasta já clonada e conferir que o app mostra "conectado como
@login" e o remoto do projeto; conferir que o token não aparece em nenhum arquivo do projeto,
log ou resposta à interface.

**Acceptance Scenarios**:

1. **Given** o app sem login, **When** a pessoa escolhe "Entrar com GitHub", **Then** o app
   mostra um código e abre a página de autorização do GitHub; quando a pessoa autoriza, o app
   mostra "conectado como @login".
2. **Given** a pessoa recusa a autorização ou o código expira, **When** o app recebe a
   resposta, **Then** mostra o motivo e volta ao estado "sem login", sem travar.
3. **Given** login feito, **When** a pessoa escolhe "Clonar do GitHub", **Then** o app lista
   os repositórios em que o Prelo foi autorizado; ao escolher um e uma pasta vazia, clona e
   abre o projeto.
4. **Given** uma pasta que já é clone de um repositório do GitHub, **When** a pessoa a abre,
   **Then** o app reconhece o remoto e passa a sincronizar por ele.
5. **Given** um repositório em que o Prelo não foi autorizado, **When** a pessoa tenta
   clonar ou sincronizar, **Then** o app explica que é preciso autorizar o Prelo naquele
   repositório e oferece o link para isso.
6. **Given** a pessoa escolhe "Sair do GitHub", **When** confirma, **Then** o app apaga a
   credencial do cofre do sistema e passa a tratar os projetos como offline.

---

### User Story 2 - Salvar sincroniza (Priority: P1)

A orientanda termina uma seção e aperta ⌘S. O editor fica em leitura por um instante; o app
recebe dois commits do orientador, mescla com o texto dela, envia o trabalho e grava um
commit. Aparece "Paulo fez 2 commits, deixou 1 comentário". Os comentários da 004 que os dois
fizeram aparecem juntos.

**Why this priority**: é o gesto central da fase 2 (RF-33, RF-40, D-08, Princípio IV).

**Independent Test**: dois clones do mesmo repositório de teste (A e B); editar e comentar em
ambos; Salvar em A e depois em B; conferir que B termina com o texto e os comentários dos
dois, que o remoto tem os dois commits e que o `.typ` continua Typst puro.

**Acceptance Scenarios**:

1. **Given** um projeto com remoto e edições locais, **When** a pessoa aperta ⌘S, **Then** o
   app bloqueia a edição (o editor continua legível), recebe do remoto, mescla, grava um commit
   com as mudanças, envia ao remoto e libera a edição.
2. **Given** commits novos no remoto que mexem em outro trecho do mesmo `.typ`, **When** a
   pessoa salva, **Then** o texto final tem as duas mudanças, sem marca de conflito no `.typ`.
3. **Given** commits novos no remoto que mexem no mesmo trecho que a pessoa editou, **When**
   ela salva, **Then** o texto é mesclado pelo documento CRDT (as duas inserções ficam; nada é
   perdido em silêncio) e o resumo avisa que houve edição no mesmo trecho.
4. **Given** comentários novos dos dois lados em `.prelo/review.json`, **When** a pessoa
   salva, **Then** o resultado tem todas as conversas e mensagens dos dois; resolver ou excluir
   feito por um lado vale no resultado.
5. **Given** a sincronização terminou, **When** algo chegou do remoto, **Then** o app mostra
   um resumo por pessoa (commits e comentários novos) que a pessoa pode fechar.
6. **Given** nada mudou localmente nem no remoto, **When** a pessoa salva, **Then** nenhum
   commit vazio é criado e o app diz que está tudo sincronizado.
7. **Given** o push é recusado porque o remoto avançou entre o pull e o push, **When** o app
   recebe a recusa, **Then** repete receber → mesclar → enviar até 3 vezes antes de avisar.

---

### User Story 3 - Sem conexão, nada se perde (Priority: P1)

No avião, a orientanda abre o projeto. O app mostra "offline": ela lê, comenta e responde, mas
não edita o texto (Princípio IV). Ao salvar, o app guarda tudo no computador e agenda. No hotel,
com Wi-Fi, o app sincroniza sozinho e libera a edição.

**Why this priority**: RF-34, Princípios III e IV. Sem isso a pessoa perde trabalho ou edita
sobre versão velha.

**Independent Test**: com o remoto inacessível, abrir o projeto, comentar, salvar e matar o
app; restaurar a rede e reabrir; conferir que os comentários foram enviados e que a edição só
foi liberada depois de receber do remoto.

**Acceptance Scenarios**:

1. **Given** o remoto inacessível ao abrir, **When** o projeto abre, **Then** o app mostra
   "offline", permite ler, comentar, responder e resolver, e mantém a edição de texto
   bloqueada.
2. **Given** offline, **When** a pessoa salva, **Then** o app grava as ações locais num commit
   local, mostra "guardado no computador; será enviado quando a conexão voltar" e agenda.
3. **Given** commits locais pendentes de envio, **When** a conexão volta (detectada por
   tentativa periódica), **Then** o app sincroniza sem ação da pessoa e libera a edição.
4. **Given** commits locais pendentes, **When** o app é encerrado à força e reaberto, **Then**
   os commits continuam lá e são enviados na próxima sincronização.
5. **Given** o token expirado durante o trabalho, **When** o app tenta sincronizar, **Then**
   renova o token sem incomodar a pessoa; se a renovação falhar, pede novo login e mantém
   tudo pendente guardado (D-18).

---

### User Story 4 - Abrir e fechar sincronizam (Priority: P2)

O orientador abre o projeto de manhã. O editor fica em leitura enquanto o app recebe o que a
orientanda enviou à noite; depois libera. Ao fechar, o app faz um último Salvar, para que ela
receba o trabalho dele mesmo com o computador dele desligado.

**Why this priority**: RF-32 e RF-36 (Princípios III e IV). Sem isso, quem abre edita sobre
versão velha e quem fecha esquece de enviar.

**Independent Test**: com dois clones, salvar em A; abrir B e conferir que a edição só libera
após receber; editar em B e fechar o app; conferir no remoto o commit de B.

**Acceptance Scenarios**:

1. **Given** commits novos no remoto, **When** a pessoa abre o projeto, **Then** o editor fica
   em leitura com "Recebendo…" até terminar; depois libera e mostra o resumo.
2. **Given** um projeto aberto com mudanças não enviadas, **When** a pessoa fecha a janela ou
   o app, **Then** o app faz um Salvar completo antes de fechar, mostrando o progresso.
3. **Given** o último Salvar falhou por falta de conexão, **When** o app fecha, **Then** o
   commit fica local e o app avisa, na próxima abertura, que há trabalho a enviar.
4. **Given** o último Salvar demora mais que 15 s, **When** a pessoa escolhe "Fechar mesmo
   assim", **Then** o app guarda tudo localmente (commit local) e fecha.

---

### User Story 5 - Versões nomeadas (Priority: P3)

Antes de enviar o artigo à revista, a orientanda marca a versão atual como "Submissão 1".
Meses depois, ela consulta a lista de versões e vê quando cada uma foi marcada e por quem.

**Why this priority**: RF-41. Útil para submissões e para o PDF de diferenças da R4, mas não
bloqueia o uso diário.

**Independent Test**: marcar uma versão, sincronizar, abrir em outro clone e conferir que a
versão aparece com nome, autor e data, apontando para o commit certo.

**Acceptance Scenarios**:

1. **Given** um projeto sincronizado, **When** a pessoa escolhe "Marcar versão" e dá um nome,
   **Then** o app sincroniza, registra a versão apontando para o commit resultante e a mostra
   na lista.
2. **Given** um nome já usado, **When** a pessoa tenta reutilizá-lo, **Then** o app recusa e
   pede outro nome.
3. **Given** uma versão marcada por outro coautor, **When** a pessoa sincroniza, **Then** a
   versão aparece na lista dela.
4. **Given** a lista de versões, **When** a pessoa escolhe uma, **Then** o app mostra nome,
   autor, data e o resumo de commits desde a versão anterior (sem restaurar nem comparar
   nesta fase).

---

### Edge Cases

- **`.typ` alterado fora do Prelo por outro coautor** (sem estado CRDT correspondente): o app
  aplica a mudança como edição de um autor "externo" via reconciliação da 004 (FR-008 da 004);
  se o mesmo trecho foi editado localmente, o app mostra os dois textos e a pessoa escolhe
  qual fica, sem perder o outro (vai para o histórico do git).
- **Conflito git em arquivo que não é texto do Prelo** (imagem, `.bib`, outro arquivo): o app
  não mescla; mostra o arquivo e deixa a pessoa escolher "manter o meu" ou "manter o
  recebido"; o descartado fica no histórico do git.
- **Conflito em `.prelo/project.json`**: campos combinados por chave; `owner` divergente
  mantém o mais antigo registrado e avisa.
- **Repositório com histórico divergente após force-push de alguém**: o app não sobrescreve o
  remoto; recusa enviar, avisa e mantém tudo local.
- **Branch atual não é a padrão do remoto**: o app sincroniza a branch atual com a de mesmo
  nome no remoto; não troca de branch.
- **Arquivos grandes** (figuras de dezenas de MB): sincronizam normalmente; o progresso aparece.
- **Repositório com submódulos, LFS ou hooks**: submódulos e LFS não são suportados nesta
  fase (aviso ao abrir); hooks do repositório **nunca** são executados pelo app.
- **Pasta não vazia ao clonar**: recusado, como "Novo projeto" na 003.
- **URL de remoto que não é GitHub** (GitLab, SSH, servidor próprio): o app trata o projeto
  como sem remoto e avisa que só o GitHub é suportado nesta versão (D-18).
- **Credenciais do git do sistema**: o app não lê nem usa `~/.gitconfig` de credenciais, SSH
  ou helpers; só o token do GitHub App.
- **Commit vindo do remoto com `.prelo/` malformado ou adulterado**: tratado como na 004
  (FR-030 a FR-034 da 004); sincronização não aborta e nada local é sobrescrito sem cópia.
- **Caminhos maliciosos no repositório** (`..`, links simbólicos para fora, nomes reservados
  do Windows, diferença só de maiúsculas): o clone e o pull recusam ou isolam o arquivo, sem
  escrever fora da pasta do projeto.
- **Relógio do computador errado**: a ordem de commits e o resumo não dependem do relógio
  local para mesclar; datas exibidas podem estar erradas.
- **Duas janelas do Prelo no mesmo projeto**: a segunda abre em leitura com aviso.

## Requirements *(mandatory)*

### Functional Requirements

**Login e credenciais (D-18)**

- **FR-001**: O app MUST autenticar no GitHub por OAuth Device Flow de um GitHub App
  registrado na conta pessoal `paulossjunior` (decisão do dono, 2026-11-27), com permissões
  por repositório só `Contents: read & write` e `Metadata: read`.
- **FR-002**: Tokens de acesso e de renovação MUST ficar só no cofre de credenciais do sistema
  (Keychain no macOS; equivalente no Windows e Linux) e MUST NOT aparecer em arquivos do
  projeto, `.prelo/`, logs, mensagens de erro ou respostas à interface.
- **FR-003**: Só o núcleo MUST falar com a rede e com o git; a interface recebe apenas estados
  (por exemplo "conectado como @login", "login necessário").
- **FR-004**: O app MUST renovar o token expirado automaticamente e, se a renovação falhar,
  pedir novo login sem perder nada pendente.
- **FR-005**: O app MUST permitir sair do GitHub, apagando as credenciais do cofre.
- **FR-006**: O app MUST respeitar os intervalos e erros de espera do device flow (sem
  pedidos mais rápidos que o GitHub permite).

**Projeto e remoto**

- **FR-007**: O app MUST clonar um repositório do GitHub, escolhido numa lista dos
  repositórios autorizados, para uma pasta vazia, e abri-lo como projeto.
- **FR-008**: O app MUST reconhecer uma pasta que já é clone com remoto HTTPS do GitHub e
  sincronizar por ele; remoto de outro tipo MUST ser tratado como "sem remoto" com aviso.
- **FR-009**: Projeto sem remoto MUST funcionar como na `1.0.0`.
- **FR-010**: O app MUST NOT executar hooks do repositório, nem usar credenciais, helpers ou
  configuração de credenciais do git do sistema.
- **FR-011**: Clone e pull MUST obedecer ao confinamento do projeto (RF-08, D-13): nenhum
  arquivo escrito fora da pasta; caminhos com `..`, absolutos, links simbólicos para fora e
  nomes inválidos no sistema MUST ser recusados.

**Estados e edição (Princípio IV, requisitos.md §5)**

- **FR-012**: Projeto com remoto MUST ter os estados Verificando, Recebendo, Sincronizado e
  Offline (arquitetura.md §5), visíveis na interface.
- **FR-013**: Edição de texto MUST ficar bloqueada em Verificando, Recebendo e Offline; o
  editor continua legível. Comentar, responder e resolver (004) MUST funcionar em Offline.
- **FR-014**: Ao abrir um projeto com remoto, o app MUST verificar e receber do remoto antes
  de liberar a edição (RF-32).
- **FR-015**: Em Offline, o app MUST tentar reconectar periodicamente (no máximo a cada 30 s,
  com espera crescente até 5 min) e sincronizar ao conseguir, sem ação da pessoa.

**Salvar (RF-33, RF-34, RF-40, D-08)**

- **FR-016**: Salvar MUST seguir a ordem: bloquear edição → receber do remoto → mesclar →
  gravar commit → enviar → liberar (Princípio IV).
- **FR-017**: O commit MUST ter a mensagem `Prelo sync · <nome> · <resumo>` e autor com o nome
  de exibição da pessoa e o e-mail `noreply` do GitHub do login (`<id>+<login>@users.noreply.github.com`).
- **FR-018**: Salvar sem mudanças locais nem remotas MUST NOT criar commit.
- **FR-019**: Sem conexão, Salvar MUST gravar commit local, avisar e agendar o envio (RF-34).
- **FR-020**: Push recusado por remoto que avançou MUST repetir receber → mesclar → enviar até
  3 vezes; push recusado por histórico divergente (force-push alheio) MUST parar, avisar e
  manter tudo local, sem forçar.
- **FR-021**: Commits locais MUST sobreviver a encerramento forçado (Princípio III).

**Mesclagem**

- **FR-022**: `.typ` editado dos dois lados MUST ser mesclado pela união dos estados CRDT
  (`.prelo/crdt/` da 004) e o `.typ` gravado a partir do documento mesclado (decisão do dono,
  2026-11-27). O resultado MUST NOT conter marca de conflito.
- **FR-023**: Quando um lado mudou o `.typ` sem estado CRDT correspondente (edição fora do
  Prelo), o app MUST reconciliar como edição externa; se a reconciliação tocar trecho também
  editado localmente, a pessoa MUST escolher qual texto fica, e o outro MUST continuar no
  histórico do git.
- **FR-024**: `.prelo/review.json` MUST ser mesclado por chave: conversas e mensagens pela
  união por id; estado da conversa pelo registro de `statusAt` mais recente; exclusões de
  conversa e de mensagem MUST prevalecer sobre respostas e mudanças de estado concorrentes, por
  meio de registros de exclusão (lápides) que a 004 grava.
- **FR-025**: `.prelo/versions.json` MUST ser mesclado pela união por id.
- **FR-026**: Outros arquivos em conflito MUST ser apresentados à pessoa para escolher qual
  versão fica; o app MUST NOT gravar marcas de conflito do git em nenhum arquivo.
- **FR-027**: O app MUST validar o que chega do remoto como entrada não confiável, com as
  regras da 004 para `.prelo/` e de FR-011 para caminhos.

**Fechar e resumo**

- **FR-028**: Fechar a janela ou o app MUST fazer um último Salvar (RF-36), com progresso e a
  opção "Fechar mesmo assim" depois de 15 s, que grava commit local e fecha.
- **FR-029**: Depois de cada sincronização que recebeu algo, o app MUST mostrar um resumo por
  pessoa: commits recebidos, comentários novos e resolvidos (RF-38, só git).

**Versões nomeadas (RF-41)**

- **FR-030**: A pessoa MUST poder marcar uma versão com um nome único no projeto; a marcação
  MUST sincronizar antes e registrar a versão em `.prelo/versions.json` com nome, autor, data e
  commit.
- **FR-031**: O app MUST listar as versões nomeadas com nome, autor, data e resumo de commits
  desde a anterior. Restaurar e comparar versões ficam fora (R4, RF-44).
- **FR-032**: `.prelo/versions.json` MUST ser lido como entrada não confiável, no padrão de
  `.prelo/review.json` (validação, limites, arquivo inválido preservado).

**Fora de escopo**

- **FR-033**: Ficam fora: GitLab e outros remotos (D-18), SSH, criar repositório no GitHub,
  submódulos e LFS, tempo real e pares (R3), restaurar e comparar versões (R4), papéis e
  convites (R5).

### Key Entities

- **Credencial do GitHub**: tokens de acesso e de renovação, login, id; só no cofre do sistema.
- **Remoto do projeto**: URL HTTPS do GitHub, branch acompanhada; derivado do clone.
- **Estado de sincronização**: Verificando, Recebendo, Sincronizado, Offline; mais contagem de
  commits locais não enviados.
- **Commit do Prelo**: mensagem `Prelo sync · <nome> · <resumo>`, autor, data.
- **Resumo da sincronização**: por pessoa, commits recebidos e comentários novos/resolvidos.
- **Versão nomeada**: id, nome único, autor, data, commit; em `.prelo/versions.json`.
- **Lápide**: registro de exclusão de conversa ou mensagem em `.prelo/review.json` (id, quem,
  quando), usado na mesclagem.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Em rede de referência (TODO(rede-comum) do release-plan: definir no plano), com
  o projeto de fixture de 40 páginas, Salvar com uma edição local e um commit remoto novo
  termina em cerca de 1 s no p50 e em até 3 s no p95 (RNF-03).
- **SC-002**: Em 100 rodadas de teste com dois clones editando o mesmo `.typ` (trechos
  diferentes e o mesmo trecho), 100% terminam com as edições dos dois, nenhuma marca de
  conflito e `.typ` compilável pelo `typst` CLI (Princípio I).
- **SC-003**: Em 100% dos testes de encerramento forçado durante Salvar e de remoto
  indisponível, nenhum comentário, edição salva ou commit local se perde (RNF-04).
- **SC-004**: Em 100% dos casos de teste, o token não aparece em arquivo do projeto, log,
  mensagem de erro nem resposta à interface (Princípio II, D-18).
- **SC-005**: 100% dos repositórios de fixture maliciosos (caminhos fora da raiz, links
  simbólicos, hooks, `.prelo/` adulterado) são recusados ou isolados sem escrita fora da pasta
  e sem execução de código.
- **SC-006**: Em 100% dos testes, a edição de texto só é liberada depois de receber tudo o que
  o remoto tem (Princípio IV).
- **SC-007**: Uma pessoa que nunca usou git faz login, clona e salva pela primeira vez em menos
  de 3 minutos, sem ajuda, no roteiro do quickstart.

## Assumptions

- **GitHub App**: o dono registra o app na conta `paulossjunior` com device flow habilitado
  antes da implementação das tarefas de login (ação fora do código; D-18, pendência 1 resolvida
  em 2026-11-27). Pendências 2 (permissões) e 3 (fluxo) de D-18 resolvidas com os padrões
  recomendados: `Contents` e `Metadata`; device flow.
- **Biblioteca git**: a constituição fixa o gitoxide. Se ele não cobrir push por HTTPS com
  token na versão disponível, o plano registra a alternativa como nova decisão D-xx
  (constituição, Restrições Técnicas).
- **Offline bloqueia edição**: segue o Princípio IV e requisitos.md §5, inclusive para quem
  trabalha sozinho num projeto com remoto. Quem quer editar offline sem remoto usa projeto sem
  remoto (FR-009).
- **Mudança na 004**: a mesclagem de `review.json` precisa de lápides de exclusão (FR-024);
  o data-model da 004 é emendado para gravá-las desde a primeira versão.
- **Nome e e-mail do commit**: nome de exibição da 004 e e-mail `noreply` do GitHub, para não
  expor e-mail pessoal.
- **Branch**: o app trabalha na branch atual do clone; não cria nem troca branch.
- **RNF-02** (Q-05): o repositório no GitHub é do grupo e é aceito; o Prelo não opera servidor.
- **Plataformas**: macOS, Windows e Linux (RNF-01); validação manual de produção no macOS.
