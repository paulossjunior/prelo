# Feature Specification: Esqueleto do app desktop Prelo

**Feature Branch**: `001-app-skeleton`

**Created**: 2026-11-18

**Status**: Draft

**Input**: User description: "Esqueleto do app desktop Prelo em Tauri (núcleo Rust, interface HTML/TypeScript) para macOS, Windows e Linux, com ponte interface → núcleo, suíte de testes inicial escrita em TDD, auditoria de licenças e vulnerabilidades de dependências, e trava de push (hook pre-push + CI nos três sistemas + proteção da main). Issue #1. Origem: D-01, RNF-01, RNF-06, Princípio VI. Sem editor, compilador ou rede: só o esqueleto."

**Origem**: [Issue #1](https://github.com/paulossjunior/prelo/issues/1) · D-01, RNF-01, RNF-06 · Constituição: Princípio VI, Restrições Técnicas, Fluxo de Desenvolvimento

## User Scenarios & Testing *(mandatory)*

Esta feature tem dois públicos: quem **usa** o Prelo (abre o app no próprio computador) e quem
**constrói** o Prelo (pessoas e agentes que contribuem com código). Nenhuma funcionalidade de
escrita, compilação ou rede faz parte dela.

### User Story 1 - Abrir o Prelo em qualquer sistema (Priority: P1)

Uma pessoa instala o Prelo no macOS, no Windows ou no Linux e abre o app. Uma janela do Prelo
aparece, com o nome do produto e a versão instalada.

**Why this priority**: sem um app que abre nos três sistemas (RNF-01), nenhuma outra fase do
plano tem onde existir.

**Independent Test**: gerar o pacote do app em cada sistema, abrir e verificar que a janela
aparece com nome e versão.

**Acceptance Scenarios**:

1. **Given** o app gerado para macOS, Windows ou Linux, **When** a pessoa o abre, **Then** uma
   janela intitulada "Prelo" aparece e mostra a versão do app.
2. **Given** o app aberto, **When** a pessoa fecha a janela, **Then** o app encerra sem
   mensagem de erro e sem processo remanescente.

---

### User Story 2 - Interface conversa com o núcleo (Priority: P1)

Ao abrir, a interface pede ao núcleo do app uma informação (a versão do app e o sistema em que
roda) e a exibe. Isso prova que a interface e o núcleo trocam mensagens, base de todas as
features futuras.

**Why this priority**: compilador, editor e rede (fases seguintes) dependem da comunicação
interface ↔ núcleo.

**Independent Test**: abrir o app e verificar que a versão e o sistema exibidos vieram do
núcleo; simular falha do núcleo e verificar a mensagem de erro.

**Acceptance Scenarios**:

1. **Given** o app aberto, **When** a interface consulta o núcleo, **Then** a versão do app e o
   nome do sistema operacional respondidos pelo núcleo aparecem na janela.
2. **Given** o núcleo responde com erro ou não responde, **When** a interface faz a consulta,
   **Then** a janela mostra uma mensagem de erro compreensível em vez de travar ou ficar em
   branco.
3. **Given** a interface envia uma consulta malformada ou desconhecida, **When** o núcleo a
   recebe, **Then** o núcleo a recusa com erro e continua funcionando.

---

### User Story 3 - Rodar todos os testes com um comando (Priority: P2)

Quem contribui clona o repositório e roda toda a suíte de testes (núcleo, interface e ponta a
ponta) com um único comando documentado. Cada comportamento das histórias 1 e 2 tem teste
escrito antes do código (TDD).

**Why this priority**: o Princípio VI exige TDD desde o primeiro código; sem suíte, nenhuma
feature seguinte pode ser validada.

**Independent Test**: em um clone limpo, seguir o README e rodar o comando de testes; verificar
que todos passam e que o histórico mostra os testes antes do código correspondente.

**Acceptance Scenarios**:

1. **Given** um clone limpo e as ferramentas listadas no README, **When** a pessoa roda o
   comando de testes, **Then** a suíte inteira executa e informa quantos testes passaram e
   falharam.
2. **Given** um teste falhando, **When** a suíte roda, **Then** o comando termina com falha e
   identifica o teste.
3. **Given** o histórico do repositório, **When** o QA verifica a feature, **Then** cada teste
   da ponte e da janela aparece em commit anterior ou igual ao do código que ele cobre.

---

### User Story 4 - Dependências auditadas (Priority: P2)

Toda dependência do projeto é verificada quanto a licença e vulnerabilidades conhecidas.
Dependência com licença fora da lista permitida, ou com vulnerabilidade alta ou crítica sem
justificativa registrada, faz a verificação falhar.

**Why this priority**: RNF-06 e o Princípio VI exigem licenças permissivas e auditoria de
dependências; é mais barato garantir isso desde a primeira dependência.

**Independent Test**: adicionar temporariamente uma dependência com licença proibida ou
vulnerabilidade conhecida e verificar que a auditoria falha apontando a dependência.

**Acceptance Scenarios**:

1. **Given** só dependências permitidas e sem vulnerabilidades altas ou críticas, **When** a
   auditoria roda, **Then** ela passa.
2. **Given** uma dependência com licença fora da lista permitida, **When** a auditoria roda,
   **Then** ela falha e nomeia a dependência e a licença.
3. **Given** uma vulnerabilidade alta ou crítica, **When** a auditoria roda, **Then** ela falha,
   a menos que exista justificativa registrada no repositório para aquele achado específico.

---

### User Story 5 - Push só com tudo verde (Priority: P3)

Quem contribui só consegue enviar mudanças ao repositório se a suíte completa e a auditoria
passarem. A trava existe no computador de quem contribui e no repositório remoto, para que
contornar a trava local não basta.

**Why this priority**: regra da constituição (Fluxo de Desenvolvimento); depende das histórias
3 e 4 existirem.

**Independent Test**: tentar enviar uma mudança com um teste falhando, pela trava local e
contornando-a, e verificar que a mudança não entra no ramo principal.

**Acceptance Scenarios**:

1. **Given** um teste falhando, **When** a pessoa tenta enviar a mudança, **Then** a trava
   local roda a suíte e bloqueia o envio, informando o teste que falhou.
2. **Given** a trava local foi contornada, **When** a mudança chega ao repositório remoto,
   **Then** a verificação remota roda nos três sistemas e a mudança não entra no ramo principal
   enquanto alguma verificação falhar.
3. **Given** a suíte e a auditoria verdes nos três sistemas, **When** a pessoa envia a mudança,
   **Then** ela pode entrar no ramo principal por pull request.
4. **Given** qualquer pessoa, inclusive o dono, **When** tenta enviar direto ao ramo principal,
   **Then** o envio é recusado.

### Edge Cases

- Sistema sem o componente de exibição web exigido pelo app (ex.: Linux sem a biblioteca
  necessária): a instalação ou a abertura informa o que falta, em vez de falhar em silêncio.
- Núcleo lento para responder à primeira consulta: a interface mostra estado de carregamento,
  e após um limite de tempo mostra erro.
- Teste instável (passa e falha sem mudança de código): conta como falha para a trava; não
  pode ser marcado para pular sem justificativa escrita (Princípio VI).
- Vulnerabilidade sem correção disponível: só passa com justificativa registrada para aquele
  achado, com data de revisão.
- Dependência sem licença declarada: tratada como licença proibida.
- Mudança só em documentação: a trava roda mesmo assim (regra única, sem exceção).
- Verificação remota falha em só um dos três sistemas: a mudança continua bloqueada.
- Dono tenta mesclar pull request com verificação vermelha: recusado, sem exceção.

## Clarifications

### Session 2026-11-18

- Q: Toda mudança no ramo principal passa a exigir pull request com verificação verde, inclusive
  do dono, ou o dono mantém envio direto? → A: Pull request obrigatório para todos, inclusive o
  dono; sem exceção administrativa.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: O app MUST abrir uma janela intitulada "Prelo" em macOS, Windows e Linux (RNF-01).
- **FR-002**: A janela MUST exibir a versão do app e o sistema operacional, obtidos do núcleo
  por meio da ponte interface → núcleo.
- **FR-003**: A ponte MUST recusar com erro consultas desconhecidas ou malformadas, sem derrubar
  o núcleo.
- **FR-004**: A interface MUST mostrar mensagem de erro compreensível quando o núcleo falhar ou
  não responder dentro de um limite de tempo.
- **FR-005**: Fechar a janela MUST encerrar o app por completo.
- **FR-006**: O repositório MUST oferecer um único comando documentado que roda a suíte
  completa: testes do núcleo, da interface, ponta a ponta e de segurança.
- **FR-007**: A suíte MUST incluir testes de segurança aplicáveis ao esqueleto: recusa de
  consultas malformadas na ponte e ausência de permissões além das necessárias na janela.
- **FR-008**: A auditoria MUST verificar licenças de todas as dependências contra a lista
  permitida (Apache-2.0, MIT ou compatível) e falhar em qualquer outra ou em licença ausente
  (RNF-06).
- **FR-009**: A auditoria MUST verificar vulnerabilidades conhecidas e falhar em achado alto ou
  crítico sem justificativa registrada.
- **FR-010**: Uma trava local MUST rodar suíte e auditoria antes de todo envio ao repositório
  remoto e bloquear o envio se algo falhar.
- **FR-011**: Uma verificação remota MUST rodar suíte e auditoria em macOS, Windows e Linux a
  cada mudança enviada.
- **FR-012**: O ramo principal MUST aceitar mudanças só por pull request com a verificação
  remota verde nos três sistemas. A regra MUST valer para todos, inclusive o dono e os agentes;
  envio direto ao ramo principal MUST ser recusado.
- **FR-013**: O README MUST documentar pré-requisitos, como rodar o app em desenvolvimento,
  gerar o pacote, rodar a suíte e instalar a trava local.
- **FR-014**: O esqueleto MUST NOT conter editor, compilador Typst, rede ou armazenamento de
  documentos.

### Key Entities

- **Versão do app**: identificador único da versão em execução, exibido na janela e reportado
  pelo núcleo.
- **Lista de licenças permitidas**: licenças aceitas para dependências; fonte única usada pela
  auditoria.
- **Justificativa de vulnerabilidade**: registro no repositório que aceita um achado específico,
  com motivo e data de revisão.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: O app abre e mostra versão e sistema em 100% das execuções da verificação remota
  nos três sistemas.
- **SC-002**: A janela mostra versão e sistema em até 3 segundos após abrir, em computador de
  desenvolvimento comum.
- **SC-003**: Uma pessoa nova no projeto, seguindo só o README, roda a suíte completa em até
  30 minutos a partir do clone, incluindo instalar pré-requisitos.
- **SC-004**: 100% das tentativas de envio com teste falhando ou auditoria falhando são barradas
  antes de chegar ao ramo principal, inclusive contornando a trava local.
- **SC-005**: 0 dependências com licença fora da lista permitida ou vulnerabilidade alta/crítica
  sem justificativa.
- **SC-006**: Cada requisito funcional de comportamento (FR-001 a FR-005) tem ao menos um teste
  automatizado que falhou antes da implementação.

## Assumptions

- A tecnologia já está decidida pela constituição e por D-01 (shell Tauri, núcleo Rust,
  interface HTML/TypeScript); a escolha das ferramentas de teste e auditoria fica para o
  `plan.md`.
- Pacotes gerados nesta fase são para desenvolvimento e teste: assinatura de código,
  notarização e instaladores de distribuição ficam fora do escopo.
- Versões mínimas dos sistemas operacionais: as suportadas oficialmente pela tecnologia
  escolhida no momento do plano.
- A verificação remota usa o serviço de integração contínua do GitHub, onde o repositório já
  está hospedado; o dono do repositório tem permissão para configurar a proteção do ramo
  principal.
- O limite de tempo da consulta ao núcleo (FR-004) e o tempo de abertura (SC-002) são padrões
  razoáveis, não metas dos documentos; podem ser ajustados no plano.
