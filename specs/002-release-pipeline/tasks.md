---

description: "Tasks for 002-release-pipeline"
---

# Tasks: Pipeline de release dos instaladores

**Input**: Design documents from `/specs/002-release-pipeline/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: obrigatórios (Princípio VI, TDD). Em cada história, as tarefas de teste (funcionais
e de segurança) vêm antes da implementação. Elas MUST falhar pelo motivo certo antes de a
implementação começar e MUST ser commitadas antes do código que cobrem, ou junto com ele. O
agente `qa` pode executar as tarefas de teste em modo projetar e roda o modo validar ao fim
de cada história.

**Organization**: por história de usuário (spec.md), em ordem de prioridade.

**Issue-mãe**: [#35](https://github.com/paulossjunior/prelo/issues/35)

## Format: `[ID] [P?] [Story] Description`

- **[P]**: pode rodar em paralelo (arquivos diferentes, sem dependência pendente)
- **[Story]**: história do spec.md (US1…US4)

## Path Conventions

Layout do [plan.md](plan.md#source-code-repository-root): workflows em `.github/workflows/`,
scripts e seus testes Vitest em `scripts/` (projeto `scripts` do `vitest.config.ts`, incluído
no `npm run verify`). Nenhum arquivo de `src/` ou `src-tauri/src/` muda.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: dependência para os testes lerem YAML.

- [X] T001 Adicionar `yaml` (ISC, versão exata) em `devDependencies` de `package.json` e atualizar `package-lock.json`; confirmar `npm run audit` verde

**Checkpoint**: `import { parse } from "yaml"` funciona num teste do projeto `scripts`.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: o arquivo `release.yml` existe com gatilho, concorrência e permissões de topo, e
o `ci.yml` pode ser chamado. Todas as histórias editam esse esqueleto.

- [X] T002 Escrever em `scripts/release-workflow.test.ts` o carregador de `.github/workflows/release.yml` e `.github/workflows/ci.yml` e os testes: gatilho só `push.tags` com o padrão `v[0-9]+.[0-9]+.[0-9]+`, sem outros gatilhos ([contracts/release-workflow.md](contracts/release-workflow.md#gatilho), FR-001); `concurrency.group` começa com `release-` e difere do grupo do `ci.yml`; `ci.yml` aceita `workflow_call` e mantém `pull_request`, `push: main`, `permissions: contents: read` e o job `verify` com nome `verify (${{ matrix.os }})`. Confirmar que falham
- [X] T003 Criar `.github/workflows/release.yml` com gatilho, `concurrency` (`cancel-in-progress: false`) e `permissions: {}`, e acrescentar `workflow_call:` em `.github/workflows/ci.yml` até T002 passar

**Checkpoint**: `npx vitest run --project scripts` verde; nenhum check obrigatório da `main` mudou de nome.

---

## Phase 3: User Story 1 - Gerar uma release a partir de uma tag de versão (Priority: P1) 🎯 MVP

**Goal**: tag válida gera um rascunho com os quatro instaladores, sem rascunho parcial.

**Independent Test**: quickstart, cenário 1 (tag de teste na `main` → rascunho completo, invisível sem login).

### Tests for User Story 1 ⚠️ escrever primeiro, ver falhar

- [X] T004 [P] [US1] Testes em `scripts/release-artifacts.test.ts` para `expectedInstallers` e `matchInstallers` ([contracts/release-scripts.md](contracts/release-scripts.md)): nomes exatos de `data-model.md` Instalador (arquivo esperado por sistema, com `<v>` = `data-model.md` Tag de versão.version); conjunto completo → sem `missing` e `extra`; faltando `.exe` → `missing`; `.msi` ou `.rpm` a mais, ou instalador de outra versão → `extra`; e CLI `collect` saindo com 1 e nomeando o arquivo nesses casos
- [X] T005 [US1] Testes em `scripts/release-workflow.test.ts` ([contracts/release-workflow.md](contracts/release-workflow.md)): matriz de `build` igual à tabela (`os`, `target`, `bundles`) e `fail-fast: true`; `publish.needs` contém `preflight`, `verify`, `build`; existe passo que exige release inexistente (`gh release view` com falha esperada) antes da criação; `gh release create` com `--draft` e `--verify-tag`; ausência de `--draft=false`, `--latest`, `--clobber` e `gh release upload`; passo de limpeza com `if: failure() && steps.create.outputs.created == 'true'` e `gh release delete` sem `--cleanup-tag` (FR-007, FR-008, FR-013). Confirmar que falham

### Implementation for User Story 1

- [X] T006 [US1] Implementar `expectedInstallers`, `matchInstallers` e o comando `collect` em `scripts/release-artifacts.mjs`, com tipos em `scripts/release-artifacts.d.mts`, até T004 passar (`data-model.md` Instalador.arquivo esperado, com `<v>` de `data-model.md` Tag de versão.version)
- [X] T007 [US1] Adicionar o job `build` em `.github/workflows/release.yml` ([research.md](research.md) R4): matriz do contrato; dependências de sistema do Linux iguais às do `ci.yml`; `rustup target add aarch64-apple-darwin x86_64-apple-darwin` no macOS; Node por `.nvmrc`; `npm ci`; `npx tauri build --target <alvo> --bundles <lista>`; `upload-artifact` com nome `installers-<os>`
- [X] T008 [US1] Adicionar o job `publish` em `.github/workflows/release.yml` ([research.md](research.md) R5): `download-artifact` com `merge-multiple`; `collect`; checagem de release inexistente; `gh release create` em um só comando com `id: create` gravando `created=true`, nome e título iguais a `data-model.md` Release em rascunho.tag_name / .name e `--draft` (`data-model.md` Release em rascunho.draft); conferência dos assets anexados (`gh release view --json assets`) contra `data-model.md` Release em rascunho.assets; passo de limpeza; tag lida de `env` (`TAG: ${{ github.ref_name }}`), nunca interpolada no `run:`. Rodar até T005 passar

**Checkpoint**: testes de US1 verdes; `qa` modo validar.

---

## Phase 4: User Story 2 - Barrar release inválida antes de gerar instaladores (Priority: P1)

**Goal**: tag inválida, versão divergente, falta de licença ou `verify` vermelho impedem qualquer build e publicação.

**Independent Test**: quickstart, cenários 4, 5, 6 e 8 (nenhum rascunho criado, motivo na mensagem).

### Tests for User Story 2 ⚠️ escrever primeiro, ver falhar

- [X] T009 [P] [US2] Testes em `scripts/release-check.test.ts` ([contracts/release-scripts.md](contracts/release-scripts.md)):
  - `parseTag` aceita `v0.1.0` e `v10.20.30` e recusa `v1.2`, `1.2.3`, `release-1`, `v1.2.3-rc.1`, `v01.2.3`, `v1.2.3 ` e `v1.2.3;rm -rf` (`data-model.md` Tag de versão.name; o último caso é teste de segurança).
  - `readVersions` e `versionMismatches` nomeiam cada arquivo divergente com o valor (`data-model.md` Versão declarada).
  - `findLicense` cobre ausente, vazio e cada nome aceito (`data-model.md` Arquivo de licença).
  - `isOnMain` usa repositórios git temporários, com commit da `main` e commit só de outra branch (`data-model.md` Tag de versão.commit; padrão de `scripts/hooks.test.ts`, sem herdar `GIT_*`).
  - `checkRelease` devolve as mensagens exatas do contrato.
  - A CLI sai com 1 em cada falha.
- [X] T010 [US2] Testes em `scripts/release-workflow.test.ts`: job `preflight` com `contents: read`, checkout `fetch-depth: 0` e execução de `node scripts/release-check.mjs` com a tag vinda de `env`; job `verify` com `uses: ./.github/workflows/ci.yml` e `needs: preflight`; `build.needs` contém `verify` (FR-002, FR-003, FR-004, FR-016). Confirmar que falham

### Implementation for User Story 2

- [X] T011 [US2] Implementar `scripts/release-check.mjs` (funções exportadas e CLI) e `scripts/release-check.d.mts` até T009 passar, aplicando `data-model.md` Tag de versão.name (formato), Tag de versão.commit (na `main`), Versão declarada (as três fontes) e Arquivo de licença (nomes aceitos, não vazio)
- [X] T012 [US2] Adicionar os jobs `preflight` e `verify` em `.github/workflows/release.yml` e ligar `build.needs: verify` até T010 passar; o job `verify` declara `permissions: contents: read` explicitamente, porque o topo é `permissions: {}` e o workflow chamado só pode reduzir permissões

**Checkpoint**: testes de US1 e US2 verdes; `qa` modo validar. **MVP** (US1 + US2).

---

## Phase 5: User Story 3 - Baixar e conferir o instalador (Priority: P2)

**Goal**: rascunho traz `SHA256SUMS.txt` e notas explicando app não assinado e conferência.

**Independent Test**: quickstart, cenário 2 (`sha256sum -c` confere tudo) e leitura das notas.

### Tests for User Story 3 ⚠️ escrever primeiro, ver falhar

- [X] T013 [P] [US3] Testes em `scripts/release-artifacts.test.ts` ([contracts/release-scripts.md](contracts/release-scripts.md)):
  - `sha256File` com vetor conhecido.
  - `formatSums` em ordem alfabética, `<hex>  <nome>` e `\n` final (`data-model.md` Arquivo de checksums).
  - `verifySums` acusa arquivo adulterado e aceita os íntegros.
  - O comando `sums` grava `SHA256SUMS.txt` e falha com diretório vazio.
- [X] T014 [P] [US3] Testes em `scripts/release-workflow.test.ts` e sobre `.github/release-notes.md`:
  - O `publish` roda `sums` antes de criar e anexa `SHA256SUMS.txt`.
  - `--notes-file` aponta para as notas com `{{VERSION}}` substituído.
  - O modelo contém `{{VERSION}}`, o aviso de app não assinado, as instruções de abertura no macOS (Gatekeeper) e no Windows (SmartScreen) e os comandos de conferência nos três sistemas (FR-009, FR-014; `data-model.md` Release em rascunho.body).
  - Confirmar que falham.

### Implementation for User Story 3

- [X] T015 [US3] Implementar `sha256File`, `formatSums`, `verifySums` e o comando `sums` em `scripts/release-artifacts.mjs` (e tipos em `.d.mts`) até T013 passar, no formato de `data-model.md` Arquivo de checksums (`SHA256SUMS.txt`).linha
- [X] T016 [P] [US3] Escrever `.github/release-notes.md` ([research.md](research.md) R9), conteúdo de `data-model.md` Release em rascunho.body
- [X] T017 [US3] Ligar `sums`, a renderização das notas e o anexo de `SHA256SUMS.txt` no job `publish` de `.github/workflows/release.yml` até T014 passar

**Checkpoint**: testes de US1–US3 verdes; `qa` modo validar.

---

## Phase 6: User Story 4 - Release segura por construção (Priority: P2)

**Goal**: sem `self-test` no artefato, escrita só no `publish`, sem segredos extras, ações fixadas.

**Independent Test**: `npx vitest run --project scripts` (testes de segurança) e quickstart, cenário 9.

### Tests for User Story 4 ⚠️ escrever primeiro, ver falhar

- [ ] T018 [P] [US4] Testes em `scripts/release-artifacts.test.ts` para `containsSelfTestMarker` e o comando `scan`:
  - binário falso com a marca `self_test_report` → `true` e saída 1;
  - binário falso sem a marca → `false` e saída 0;
  - marca no limite entre blocos de leitura → detectada.
  - Referência: FR-010, [research.md](research.md) R7.
- [ ] T019 [US4] Testes de segurança em `scripts/release-workflow.test.ts` ([contracts/release-workflow.md](contracts/release-workflow.md#permissões-e-segredos)):
  - `permissions: {}` no topo.
  - Só `publish` tem `contents: write`; `preflight`, `verify` e `build` têm exatamente
    `contents: read`.
  - Nenhum `secrets.*` além de `GITHUB_TOKEN`.
  - Nenhuma variável de assinatura (`APPLE_`, `TAURI_SIGNING_`, `WINDOWS_CERT`).
  - Nenhum `--features` ou `self-test` no job `build`.
  - Toda ação externa fixada por SHA de 40 caracteres.
  - Nenhum `${{ github.ref_name }}`, `${{ github.event.* }}` ou `${{ inputs.* }}` dentro de `run:` (injeção de script).
  - O job `build` tem passo `scan` após o build.
  - Referência: FR-010, FR-011, FR-012.
  - Confirmar que falham.

### Implementation for User Story 4

- [ ] T020 [US4] Implementar `containsSelfTestMarker` e o comando `scan` em `scripts/release-artifacts.mjs` até T018 passar (regra de `data-model.md` Instalador.binário: sem a marca `self_test_report`)
- [ ] T021 [US4] Em `.github/workflows/release.yml`: adicionar o passo `scan` no `build` (binário `prelo` / `prelo.exe` em `src-tauri/target/<alvo>/release/`, conforme `data-model.md` Instalador.origem); fixar todas as ações externas por SHA com a versão em comentário (SHA obtido via `gh api repos/<dono>/<ação>/git/ref/tags/<versão>`); revisar permissões por job até T019 passar

**Checkpoint**: suíte inteira verde; `qa` modo validar.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [ ] T022 Documentar em `README.md` a seção "Publicar uma versão" (FR-015): atualizar os três arquivos de versão, PR e merge, `git tag vX.Y.Z origin/main && git push origin vX.Y.Z`, revisar e publicar o rascunho, conferir checksums; citar o pré-requisito de licença (FR-016)
- [ ] T023 Abrir PR e confirmar `npm run verify` verde nos três sistemas pelo CI (checks `verify (<sistema>)` inalterados)
- [ ] T024 Rodar o [quickstart.md](quickstart.md): cenários locais L1–L4 e, no GitHub, 4, 5, 6 e 8 (possíveis sem licença); apagar tags e rascunhos de teste
- [ ] T025 Rodar os cenários 1, 2, 3, 7 e 9 do [quickstart.md](quickstart.md) **depois** que o dono adicionar o arquivo de licença (TODO(LICENSE), [research.md](research.md) R10); medir o tempo até o rascunho (SC-003)
- [ ] T026 Validação final pelo agente `qa` (modo validar): suíte completa, cobertura FR-001–FR-016, testes de segurança, ordem TDD pelo histórico git; anexar o veredito na issue #35

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (1)** → **Foundational (2)** → histórias.
- **US1 (3)** depende de 2.
- **US2 (4)** depende de 2. Os testes (T009) podem correr em paralelo a US1. T012 edita o
  mesmo `release.yml` de T007 e T008: fazer depois deles.
- **US3 (5)** depende de US1, porque edita o job `publish`.
- **US4 (6)** depende de US1 e US2, porque revisa todos os jobs e fixa as ações.
- **Polish (7)** depende de todas. T025 depende também do arquivo de licença, decisão do dono.

### Within Each User Story

- Testes primeiro, falhando pelo motivo certo; commit dos testes antes ou junto da implementação.
- Implementação até os testes da história passarem; nada além.
- `scripts/release-workflow.test.ts` e `.github/workflows/release.yml` são compartilhados:
  tarefas que tocam esses arquivos não rodam em paralelo entre si.

### Parallel Opportunities

- T004 (artifacts) com T005 (workflow), após a fase 2.
- T009 (`release-check.test.ts`) em paralelo a toda a US1.
- T011 (`release-check.mjs`) em paralelo a T006–T008.
- T013, T014 e T016 juntos.
- T018 em paralelo a T013 só se o mesmo executor não tocar `release-artifacts.test.ts` nos
  dois; senão, em sequência.

---

## Parallel Example: User Story 2

```bash
Task: "Testes de release-check em scripts/release-check.test.ts"   # T009, durante US1
Task: "Implementar scripts/release-check.mjs"                       # T011, após T009
```

---

## Implementation Strategy

### MVP First

1. Fases 1 e 2.
2. US1 → validar com `qa`.
3. US2 → validar com `qa`. **MVP**: tag válida gera rascunho completo; tag inválida não gera
   nada.

### Incremental Delivery

4. US3 (checksums e notas).
5. US4 (segurança: scan, permissões, SHA).
6. Polish: README, CI verde, quickstart sem licença (T024) e, após a licença, com caminho feliz
   (T025); veredito do `qa` na #35.

---

## Notes

- Cada tarefa vira issue `T0xx: …` ligada à #35 via `/speckit-github-taskstoissues`. Esse hook
  `after_tasks` é obrigatório, mas fica pendente até a revisão do coordenador.
- Enquanto não houver arquivo de licença, toda tag é recusada no `preflight` por desenho
  (FR-016); isso não é defeito.
- Não ampliar escopo: sem assinatura, notarização, atualização automática, pré-releases ou
  changelog automático.
