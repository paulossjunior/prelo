# Feature Specification: Pipeline de release dos instaladores

**Feature Branch**: `002-release-pipeline`

**Created**: 2026-11-19

**Status**: Draft

**Input**: User description: "Pipeline de release: tag vX.Y.Z na main dispara build e publicação dos instaladores do Prelo no GitHub Releases (rascunho), para macOS (.dmg), Windows (.msi/.exe) e Linux (.AppImage/.deb), com checksums SHA-256, verify nos três sistemas antes do build, versão da tag conferida contra package.json/tauri.conf.json/Cargo.toml, sem a feature self-test, permissões mínimas, sem assinatura de código por ora. Issue #35. Origem: RNF-01, D-01."

**Origem**: [Issue #35](https://github.com/paulossjunior/prelo/issues/35) · RNF-01, D-01 · Spec 001 (Assumptions: instaladores de distribuição e assinatura fora do escopo) · Constituição: Princípio VI, Restrições Técnicas, Fluxo de Desenvolvimento, Governance (TODO(LICENSE))

## User Scenarios & Testing *(mandatory)*

Esta feature tem dois públicos: o **dono** do repositório, que decide quando sai uma versão
do Prelo, e quem **baixa** o Prelo para instalar no próprio computador. Ela não muda o
comportamento do app; só gera e publica os instaladores.

Decisões do dono já tomadas (2026-11-18, issue #35): publicar no GitHub Releases; o gatilho é
uma tag `vX.Y.Z` em commit da `main`; sem assinatura de código por enquanto.

### User Story 1 - Gerar uma release a partir de uma tag de versão (Priority: P1)

O dono atualiza a versão do Prelo, mescla a mudança na `main` e cria a tag `vX.Y.Z` nesse
commit. Sem nenhum passo manual a mais, os instaladores dos três sistemas são gerados e
anexados a uma release em rascunho com o nome da versão. O dono revisa o rascunho e decide
quando publicá-lo.

**Why this priority**: sem isso não há como distribuir o Prelo nos três sistemas (RNF-01); é o
núcleo da feature.

**Independent Test**: criar uma tag de versão de teste em commit da `main` com a versão
correspondente nos arquivos do projeto e verificar que surge um rascunho de release com os
instaladores de macOS, Windows e Linux.

**Acceptance Scenarios**:

1. **Given** um commit da `main` cuja versão declarada no projeto é `X.Y.Z`, **When** o dono
   cria a tag `vX.Y.Z` nesse commit, **Then** uma release em rascunho chamada `vX.Y.Z` é criada
   com os instaladores de macOS (`.dmg` universal), Windows (`.exe`) e Linux (`.AppImage` e
   `.deb`).
2. **Given** o rascunho criado, **When** alguém de fora do projeto procura a release, **Then**
   ela não está visível ao público até o dono publicá-la manualmente.
3. **Given** a geração falha em um dos sistemas, **When** o processo termina, **Then** ele
   aparece como falho, identifica o sistema que falhou e não deixa rascunho incompleto.

---

### User Story 2 - Barrar release inválida antes de gerar instaladores (Priority: P1)

Uma tag criada fora da `main`, com formato errado ou com versão diferente da declarada no
projeto não gera release. A suíte completa e a auditoria (o mesmo `verify` da spec 001) rodam
nos três sistemas antes de qualquer instalador ser gerado; se algo falhar, nada é publicado.

**Why this priority**: uma release com versão errada, de código não revisado ou com teste
vermelho chega a quem baixa e é difícil de recolher; a constituição exige suíte verde antes de
qualquer envio (Fluxo de Desenvolvimento).

**Independent Test**: criar tags de teste em situações inválidas (fora da `main`, versão
divergente, suíte falhando) e verificar que nenhum rascunho é criado e que a falha explica o
motivo.

**Acceptance Scenarios**:

1. **Given** uma tag `vX.Y.Z` em commit que não pertence à `main`, **When** o processo roda,
   **Then** ele falha antes de gerar instaladores e informa que o commit não pertence à `main`.
2. **Given** uma tag `vX.Y.Z` e um dos três arquivos de versão do projeto declarando outra
   versão, **When** o processo roda, **Then** ele falha e nomeia o arquivo e a versão
   divergentes.
3. **Given** a suíte ou a auditoria falhando em qualquer um dos três sistemas, **When** o
   processo roda, **Then** nenhum instalador é gerado nem publicado.
4. **Given** uma tag fora do formato `vX.Y.Z` (ex.: `v1.2`, `1.2.3`, `release-1`), **When** ela
   é criada, **Then** nenhuma release é gerada.
5. **Given** o repositório sem arquivo de licença na raiz, **When** o processo roda, **Then** ele
   falha antes de gerar instaladores e informa que falta a licença.

---

### User Story 3 - Baixar e conferir o instalador (Priority: P2)

Quem baixa o Prelo encontra, na página da release, o instalador do seu sistema e um arquivo de
checksums SHA-256. Com ele, a pessoa confere que o arquivo baixado é o mesmo que foi gerado.

**Why this priority**: sem assinatura de código, o checksum é a forma de a pessoa conferir a
integridade do instalador; depende da história 1.

**Independent Test**: baixar cada instalador de um rascunho, calcular o SHA-256 localmente e
comparar com o arquivo de checksums publicado.

**Acceptance Scenarios**:

1. **Given** uma release gerada, **When** a pessoa abre a página, **Then** encontra um arquivo
   de checksums com uma linha por instalador anexado.
2. **Given** um instalador baixado, **When** a pessoa calcula o SHA-256, **Then** o valor
   coincide com o publicado.
3. **Given** um instalador alterado depois da geração, **When** a pessoa confere o checksum,
   **Then** o valor não coincide.

---

### User Story 4 - Release segura por construção (Priority: P2)

O que vai para quem baixa não contém recursos de teste, e o processo de release não tem mais
poder sobre o repositório do que precisa.

**Why this priority**: o Princípio VI exige testes de segurança junto com os funcionais; a
feature `self-test` da spec 001 expõe comandos de diagnóstico que não podem chegar ao usuário.

**Independent Test**: verificar por teste automatizado que a feature `self-test` não entra nos
instaladores e que só a etapa de publicação tem permissão de escrita.

**Acceptance Scenarios**:

1. **Given** um instalador gerado pela release, **When** um teste automatizado o verifica,
   **Then** a feature `self-test` não está incluída.
2. **Given** a configuração do processo de release, **When** um teste automatizado a lê,
   **Then** só a etapa de publicação tem permissão de escrever no repositório, e as demais têm
   apenas leitura.
3. **Given** o processo de release, **When** ele roda, **Then** nenhum segredo além do
   fornecido automaticamente pelo GitHub ao próprio repositório é necessário.

### Edge Cases

- Tag `vX.Y.Z` já usada antes (release ou rascunho existente com o mesmo nome): o processo
  falha e não substitui os arquivos já anexados.
- Tag removida e recriada: tratada como nova execução; se o rascunho anterior ainda existir,
  vale a regra acima.
- Tag de pré-release (ex.: `v1.0.0-rc.1`): fora do formato aceito nesta feature; não gera
  release.
- Duas tags de versão criadas quase ao mesmo tempo: cada uma gera seu próprio rascunho, sem
  misturar instaladores.
- Commit da tag está na `main`, mas não é o mais recente: aceito, desde que pertença ao
  histórico da `main`.
- Falha em só um dos três sistemas durante a geração: o processo inteiro é falho e não
  publica rascunho incompleto (US1, cenário 3).
- Instalador sem assinatura: macOS e Windows alertam ao abrir; as notas do rascunho orientam
  como abrir mesmo assim (FR-014).
- Arquivo de licença ausente: o processo falha antes de gerar instaladores (FR-016).
- Linux sem as bibliotecas exigidas pelo app: o `.deb` declara as dependências; o `.AppImage`
  segue o comportamento já previsto na spec 001 (informa o que falta).

## Clarifications

### Session 2026-11-19

- Q: Para quais arquiteturas os instaladores são gerados? → A: macOS universal (Apple Silicon
  e Intel num só `.dmg`); Windows e Linux só x86_64.
- Q: Qual o formato do instalador de Windows: `.msi`, `.exe` (NSIS) ou ambos? → A: Só `.exe`
  (NSIS).
- Q: O processo deve recusar criar o rascunho sem arquivo de licença, ou basta registrar o
  pré-requisito? → A: Recusar criar o rascunho sem arquivo de licença, com teste automatizado.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: O processo de release MUST ser disparado apenas por tag no formato estrito
  `vX.Y.Z` (X, Y e Z inteiros, sem sufixo).
- **FR-002**: O processo MUST falhar antes de gerar instaladores se o commit da tag não
  pertencer ao histórico da `main`.
- **FR-003**: O processo MUST falhar antes de gerar instaladores se a versão da tag (sem o
  `v`) diferir da versão declarada em qualquer um dos três arquivos de versão do projeto
  (`package.json`, `tauri.conf.json` e `Cargo.toml` do núcleo), nomeando o arquivo divergente.
- **FR-004**: A suíte completa e a auditoria (`npm run verify`) MUST rodar e passar em macOS,
  Windows e Linux antes de qualquer instalador ser gerado.
- **FR-005**: O processo MUST gerar um instalador de macOS (`.dmg`) universal, que roda em
  Apple Silicon e Intel no mesmo arquivo, e instaladores de Linux (`.AppImage` e `.deb`) para
  x86_64.
- **FR-006**: O instalador de Windows MUST ser gerado só no formato `.exe` (instalador NSIS),
  para x86_64; `.msi` MUST NOT ser gerado.
- **FR-007**: Os instaladores MUST ser anexados a uma release em **rascunho** chamada `vX.Y.Z`
  no GitHub Releases; o processo MUST NOT publicar a release. Publicar é ato manual do dono.
- **FR-008**: O rascunho MUST ser criado só se os instaladores de todos os sistemas forem
  gerados; falha em qualquer sistema MUST deixar o processo falho e sem rascunho incompleto.
- **FR-009**: O processo MUST anexar um arquivo de checksums SHA-256 com uma linha por
  instalador anexado.
- **FR-010**: Os instaladores MUST NOT incluir a feature `self-test`; um teste automatizado
  MUST verificar isso.
- **FR-011**: Só a etapa de publicação MUST ter permissão de escrita no conteúdo do
  repositório; as demais etapas MUST ter só leitura. Um teste automatizado MUST verificar a
  configuração de permissões.
- **FR-012**: O processo MUST NOT exigir segredos além do token automático do repositório e
  MUST NOT assinar nem notarizar os instaladores nesta feature.
- **FR-013**: Se já existir release ou rascunho com o mesmo nome, o processo MUST falhar sem
  alterar os arquivos existentes.
- **FR-014**: As notas do rascunho MUST informar que os instaladores não são assinados, como
  abrir o app mesmo assim em macOS e Windows, e como conferir o checksum.
- **FR-015**: A documentação para contribuidores MUST descrever como preparar uma versão
  (atualizar os três arquivos, mesclar na `main`, criar a tag) e como publicar o rascunho.
- **FR-016**: O processo MUST recusar criar o rascunho, antes de gerar instaladores, se o
  repositório não tiver arquivo de licença na raiz (constituição, Governance: TODO(LICENSE)),
  informando o motivo. Um teste automatizado MUST verificar essa recusa.

### Key Entities

- **Tag de versão**: marca `vX.Y.Z` num commit da `main`; dispara o processo e dá nome à
  release.
- **Versão declarada**: número de versão registrado nos três arquivos do projeto; precisa ser
  igual nos três e igual ao da tag.
- **Release em rascunho**: página no GitHub Releases, invisível ao público, com instaladores,
  checksums e notas; publicada só pelo dono.
- **Instalador**: arquivo de instalação de um sistema e arquitetura (`.dmg` universal,
  `.exe` x86_64, `.AppImage` e `.deb` x86_64).
- **Arquivo de checksums**: lista do SHA-256 de cada instalador anexado.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% das tags `vX.Y.Z` válidas em commit da `main`, com suíte verde, resultam em
  rascunho com instaladores dos três sistemas, sem passo manual do dono além de criar a tag.
- **SC-002**: 100% das tags inválidas (fora da `main`, formato errado, versão divergente, suíte
  vermelha, nome já existente, repositório sem licença) terminam sem rascunho criado ou alterado.
- **SC-003**: O rascunho fica disponível em até 60 minutos após a criação da tag.
- **SC-004**: 100% dos instaladores anexados têm checksum publicado que coincide com o arquivo
  baixado.
- **SC-005**: 0 instaladores publicados contêm a feature `self-test`.
- **SC-006**: Em cada sistema, uma pessoa consegue instalar e abrir o Prelo a partir do
  instalador baixado seguindo só as notas do rascunho, mesmo com o alerta de app não
  assinado.

## Assumptions

- A tecnologia de geração dos instaladores e o serviço de integração contínua (o mesmo da
  spec 001) vêm de D-01; a escolha das ações e ferramentas fica para o `plan.md`.
- A proteção da `main` da spec 001 (FR-012) garante que todo commit da `main` passou por pull
  request com verificação verde; mesmo assim o `verify` roda de novo na release, porque a tag
  pode apontar para commit antigo e as dependências externas mudam com o tempo.
- Criar tags é ato do dono; regras de proteção de tags são desejáveis, mas não exigidas nesta
  feature.
- Assinatura de código, notarização (macOS), atualização automática do app e publicação em
  lojas ou gerenciadores de pacotes ficam fora do escopo; a falta de assinatura é pendência
  registrada (decisão do dono, 2026-11-18).
- Pré-releases (`-rc`, `-beta`) e changelog automático ficam fora do escopo; as notas têm
  texto fixo (FR-014), e o dono pode editá-las no rascunho.
- Versões mínimas dos sistemas operacionais seguem as da spec 001.
- O limite de 60 minutos (SC-003) é padrão razoável para os três sistemas em paralelo, não meta
  dos documentos; pode ser ajustado no plano.
