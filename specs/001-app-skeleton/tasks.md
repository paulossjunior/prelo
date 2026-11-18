---

description: "Tasks for 001-app-skeleton"
---

# Tasks: Esqueleto do app desktop Prelo

**Input**: Design documents from `/specs/001-app-skeleton/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: obrigatórios (Princípio VI, TDD). Em cada história, as tarefas de teste vêm antes da
implementação, MUST falhar pelo motivo certo antes de a implementação começar e MUST ser
commitadas antes (ou junto) do código que cobrem. O agente `qa` (modo projetar) pode executar
as tarefas de teste; o modo validar roda ao fim de cada história.

**Organization**: por história de usuário (spec.md), em ordem de prioridade.

**Issue-mãe**: [#1](https://github.com/paulossjunior/prelo/issues/1)

## Format: `[ID] [P?] [Story] Description`

- **[P]**: pode rodar em paralelo (arquivos diferentes, sem dependência pendente)
- **[Story]**: história do spec.md (US1…US5)

## Path Conventions

Layout Tauri 2 do [plan.md](plan.md#source-code-repository-root): interface em `src/`, núcleo em
`src-tauri/`, ponta a ponta em `e2e/`, scripts em `scripts/`, auditoria em `security/`.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: projeto compilando e abrindo uma janela vazia, sem comportamento.

- [X] T001 Criar configuração raiz: `package.json` (nome `prelo`, versão `0.1.0`, `engines.node >=24`, script `prepare` com `git config core.hooksPath .githooks`), `.nvmrc` (24), `rust-toolchain.toml` (1.95, componentes `rustfmt` e `clippy`), `tsconfig.json` (strict) e entradas `node_modules/`, `dist/`, `src-tauri/target/` em `.gitignore`
- [X] T002 Criar o núcleo Tauri 2 mínimo: `src-tauri/Cargo.toml` (`tauri` 2.12, `tauri-build`, feature opcional `self-test`, `tauri` com feature `test` em dev-dependencies, versão `0.1.0`), `src-tauri/build.rs`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs` com `run()` sem comandos, e `src-tauri/tauri.conf.json` (`productName` "Prelo", `version` "0.1.0", identifier `org.prelo.app`)
- [X] T003 Criar a interface mínima: `index.html`, `vite.config.ts` e `src/main.ts` vazio montando um elemento raiz; adicionar `@tauri-apps/api`, `@tauri-apps/cli`, `vite`, `typescript` em `package.json` e scripts `dev` e `build`
- [X] T004 [P] Configurar Vitest com jsdom em `vitest.config.ts` e adicionar `vitest`, `jsdom` em `package.json`

**Checkpoint**: `npm run dev` abre uma janela em branco.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: o comando único `npm run verify` ([contracts/dev-workflow.md](contracts/dev-workflow.md)), do qual todas as histórias dependem para provar TDD.

- [X] T005 Escrever testes do orquestrador em `scripts/verify.test.ts`: roda todas as etapas mesmo após falha; código de saída ≠ 0 se qualquer etapa falhar; resumo lista etapas falhas; detecta `#[ignore]`, `.skip`, `.only` sem comentário `// justificativa:` e falha. Confirmar que falham
- [X] T006 Implementar `scripts/verify.mjs` conforme [contracts/dev-workflow.md](contracts/dev-workflow.md) até T005 passar, com etapas `fmt` (`cargo fmt --check`), `clippy` (`-D warnings`), `tsc` (`--noEmit`), `test:rust`, `test:ui`, `test:e2e`, `audit`; etapa ainda inexistente conta como falha
- [X] T007 Ligar scripts em `package.json`: `test:rust` (`cargo test --manifest-path src-tauri/Cargo.toml`), `test:ui` (`vitest run`), `test:e2e` (WebDriver em Linux/Windows, `scripts/self-test.mjs` em todos), `audit` (`node scripts/audit.mjs`), `verify` (`node scripts/verify.mjs`)

**Checkpoint**: `npm run verify` executa e falha listando as etapas ainda sem implementação.

---

## Phase 3: User Story 1 - Abrir o Prelo em qualquer sistema (Priority: P1) 🎯 MVP

**Goal**: janela "Prelo" abre nos três sistemas, só com as permissões necessárias, e fecha sem deixar processo.

**Independent Test**: `node scripts/self-test.mjs` sai com 0 em macOS, Windows e Linux; WebDriver vê o título "Prelo" em Linux e Windows.

### Tests for User Story 1 ⚠️ escrever primeiro, ver falhar

- [ ] T008 [P] [US1] Teste de segurança da configuração em `src-tauri/tests/security_config.rs`: janela principal com título "Prelo"; CSP com `default-src 'self'` e sem `unsafe-inline`/`unsafe-eval`; `src-tauri/capabilities/default.json` sem nenhuma permissão de sistema de arquivos, shell, HTTP, diálogo ou abertura de URL ([contracts/ipc.md](contracts/ipc.md#permissões-da-janela-capabilities))
- [ ] T009 [P] [US1] Teste ponta a ponta por autoteste em `scripts/self-test.mjs`: compila com `--features self-test`, abre o binário, espera código de saída 0 em até 30 s, falha com 1 ou por tempo; depois confirma que nenhum processo `prelo` sobrou (FR-005)
- [ ] T010 [P] [US1] Teste WebDriver em `e2e/app.e2e.ts` com `e2e/wdio.conf.ts` (`tauri-driver`, só Linux e Windows): título da janela é "Prelo"

### Implementation for User Story 1

- [ ] T011 [US1] Configurar janela, CSP e capabilities mínimas em `src-tauri/tauri.conf.json` e `src-tauri/capabilities/default.json` até T008 passar
- [ ] T012 [US1] Implementar o caminho de autoteste: `src-tauri/src/self_test.rs` (`#[cfg(feature = "self-test")]`, comando `self_test_report` que por ora valida que `rendered` contém "Prelo" e encerra com 0/1) registrado em `src-tauri/src/lib.rs` só com a feature, e `src/self-test.ts` que chama o comando quando `import.meta.env.VITE_SELF_TEST` estiver definido, até T009 e T010 passarem

**Checkpoint**: US1 funciona e é testável sozinha.

---

## Phase 4: User Story 2 - Interface conversa com o núcleo (Priority: P1)

**Goal**: janela mostra "Prelo {version} · {os}" vindo do núcleo, com carregamento, timeout e erro; consultas fora do contrato são recusadas.

**Independent Test**: autoteste confere o texto e o tempo ≤ 3 s; testes de contrato IPC passam com mock runtime nos três sistemas.

### Tests for User Story 2 ⚠️ escrever primeiro, ver falhar

- [ ] T013 [P] [US2] Testes unitários em `src-tauri/src/app_info.rs` (módulo `tests`): `AppInfo.version` igual a `CARGO_PKG_VERSION` e à versão de `tauri.conf.json`; `AppInfo.os` mapeado para `macos`/`windows`/`linux` e erro `unsupported_os` para outro valor; `AppInfo.arch` preenchido (data-model.md `AppInfo.version`, `AppInfo.os`, `AppInfo.arch`)
- [ ] T014 [P] [US2] Testes de contrato IPC em `src-tauri/tests/ipc_contract.rs` com `tauri::test` mock runtime: `get_app_info` responde no formato de [contracts/ipc.md](contracts/ipc.md); comando não registrado é rejeitado; argumentos extras ou de tipo errado são rejeitados; após cada rejeição, `get_app_info` continua respondendo (FR-003, segurança)
- [ ] T015 [P] [US2] Teste de segurança em `src-tauri/tests/ipc_contract.rs`: sem a feature `self-test`, `self_test_report` é rejeitado como comando inexistente
- [ ] T016 [P] [US2] Testes da interface em `src/app-info.test.ts` com `invoke` simulado e relógio falso: Carregando → Pronto com texto "Prelo 0.1.0 · macos"; erro do núcleo → Erro com mensagem e botão "Tentar de novo"; sem resposta em 5 s → Erro; "Tentar de novo" volta a Carregando (data-model.md Estado da tela inicial)
- [ ] T017 [US2] Endurecer o autoteste em `scripts/self-test.mjs` e `e2e/app.e2e.ts`: o texto renderizado MUST ser exatamente "Prelo {version} · {os}" e `elapsedMs` ≤ 3000 (SC-002)

### Implementation for User Story 2

- [ ] T018 [US2] Implementar `AppInfo` e o comando `get_app_info` em `src-tauri/src/app_info.rs` e registrá-lo em `src-tauri/src/lib.rs` até T013–T015 passarem (data-model.md `AppInfo.version`, `AppInfo.os`, `AppInfo.arch`)
- [ ] T019 [US2] Implementar `src/app-info.ts` (consulta com timeout de 5 s, estados Carregando/Pronto/Erro, "Tentar de novo") e montar em `src/main.ts` até T016 passar
- [ ] T020 [US2] Atualizar `src-tauri/src/self_test.rs` e `src/self-test.ts` para comparar o texto exato e o tempo, até T017 passar ([contracts/ipc.md](contracts/ipc.md#self_test_report-só-build-de-teste))

**Checkpoint**: US1 e US2 funcionam; MVP pronto.

---

## Phase 5: User Story 3 - Rodar todos os testes com um comando (Priority: P2)

**Goal**: pessoa nova roda a suíte completa seguindo só o README.

**Independent Test**: em clone limpo, seguir o README e `npm run verify` passa (cenários 1 e 4 do [quickstart.md](quickstart.md)).

- [ ] T021 [P] [US3] Escrever o README: pré-requisitos por sistema (incluindo pacotes Linux de research R8), `npm install`, `npm run dev`, `npm run build`, `npm run verify`, `npm run audit`, como instalar `cargo-deny` e `tauri-driver`, e regra de PR obrigatório em `README.md` (FR-013)
- [ ] T022 [US3] Validar em clone limpo (diretório temporário) os cenários 1, 4 e 5 do [quickstart.md](quickstart.md) e corrigir README ou scripts que divergirem

**Checkpoint**: suíte completa roda com um comando.

---

## Phase 6: User Story 4 - Dependências auditadas (Priority: P2)

**Goal**: licença fora da lista ou vulnerabilidade alta/crítica sem justificativa válida falha a auditoria.

**Independent Test**: cenários 6 e 7 do [quickstart.md](quickstart.md).

### Tests for User Story 4 ⚠️ escrever primeiro, ver falhar

- [ ] T023 [P] [US4] Testes em `scripts/audit.test.ts`: lista de `security/licenses-allow.txt` idêntica à `allow` de `src-tauri/deny.toml` e à passada ao checker npm; exceção sem `reason` invalida a auditoria; exceção com `review_by` no passado invalida a auditoria; exceção com `ecosystem` fora de `cargo`/`npm` invalida (data-model.md Exceção de vulnerabilidade `id`, `ecosystem`, `reason`, `review_by`); dependência com licença fora da lista ou sem licença faz o checker falhar nomeando-a

### Implementation for User Story 4

- [ ] T024 [P] [US4] Criar `security/licenses-allow.txt` com a lista de research R4.1 e `security/vuln-exceptions.toml` vazio com comentário de formato (data-model.md Lista de licenças permitidas)
- [ ] T025 [US4] Implementar `scripts/audit.mjs`: valida `security/vuln-exceptions.toml`, roda `cargo deny check` (manifest `src-tauri/Cargo.toml`), `audit-ci --high` com allowlist gerada das exceções `npm`, e `license-checker-rseidelsohn --onlyAllow` com a lista única; adicionar `audit-ci` e `license-checker-rseidelsohn` em `package.json`; até T023 passar
- [ ] T026 [US4] Criar `src-tauri/deny.toml` (licenças da lista única, `advisories` com ignores gerados das exceções `cargo`, `sources` só crates.io) e rodar `cargo deny check`; para cada crate MPL-2.0 encontrada, adicionar `[[licenses.exceptions]]` com motivo citando D-11 (data-model.md Exceção de licença por dependência `name`, `allow`); nenhuma outra licença fora da lista é aceita

**Checkpoint**: `npm run audit` verde com a árvore atual.

---

## Phase 7: User Story 5 - Push só com tudo verde (Priority: P3)

**Goal**: push com teste falhando é barrado localmente e remotamente; `main` só por PR com CI verde nos três sistemas, inclusive para o dono.

**Independent Test**: cenários 8–11 do [quickstart.md](quickstart.md).

### Tests for User Story 5 ⚠️ escrever primeiro, ver falhar

- [ ] T027 [P] [US5] Testes em `scripts/hooks.test.ts`: em repositório git temporário, `.githooks/pre-push` com `npm run verify` simulado falhando sai com código ≠ 0 e com ele passando sai com 0; o script `prepare` deixa `core.hooksPath` igual a `.githooks`

### Implementation for User Story 5

- [ ] T028 [US5] Criar `.githooks/pre-push` (POSIX sh, executável, roda `npm run verify` e repassa o código) até T027 passar
- [ ] T029 [P] [US5] Criar `.github/workflows/ci.yml`: gatilhos `pull_request` e `push`; job `verify` com matriz `ubuntu-22.04`, `windows-latest`, `macos-latest`; instala Rust da `rust-toolchain.toml`, Node da `.nvmrc`, dependências Linux de research R8, `cargo-deny`, `tauri-driver` (Linux/Windows); roda `npm ci` e `npm run verify` (Linux sob `xvfb-run`)
- [ ] T030 [US5] Abrir PR da branch `001-app-skeleton`, esperar o CI verde nos três sistemas e então aplicar a proteção da `main` com `gh api -X PUT repos/paulossjunior/prelo/branches/main/protection` (PR obrigatório, checks `verify (ubuntu-22.04)`, `verify (windows-latest)`, `verify (macos-latest)` com `strict`, `enforce_admins: true`, sem force-push nem exclusão); registrar o comando no README (FR-012). Ação publica configuração no GitHub: confirmar com o dono antes
- [ ] T031 [US5] Validar cenários 8–11 do [quickstart.md](quickstart.md) e registrar a saída de `gh api .../branches/main/protection` na issue #1

**Checkpoint**: trava de push ativa nos dois lados.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [ ] T032 Rodar o [quickstart.md](quickstart.md) inteiro em cada sistema disponível e corrigir divergências
- [ ] T033 Validação final pelo agente `qa` (modo validar): suíte completa, cobertura FR-001–FR-014, checklist de segurança, ordem TDD pelo histórico git; anexar o veredito na issue #1

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (1)** → **Foundational (2)** → histórias.
- **US1 (3)** depende de 2. **US2 (4)** depende de US1 (reaproveita o autoteste de T012).
- **US3 (5)** depende de US1 e US2 (README descreve o que existe).
- **US4 (6)** depende só de 2; pode correr em paralelo a US1/US2.
- **US5 (7)** depende de US3 e US4 (o hook e o CI rodam `verify`, que inclui `audit`). T030 depende de T029 e do primeiro CI verde.
- **Polish (8)** depende de todas.

### Within Each User Story

- Testes primeiro, falhando pelo motivo certo; commit dos testes antes ou junto da implementação.
- Implementação até os testes da história passarem; nada além.
- Checkpoint antes da próxima história; `qa` em modo validar no fim de cada uma.

### Parallel Opportunities

- T004 com T002–T003.
- US1: T008, T009, T010 juntos.
- US2: T013, T014, T015, T016 juntos.
- US4 inteira em paralelo a US1/US2, após a fase 2.
- T029 em paralelo a T027–T028.

---

## Parallel Example: User Story 2

```bash
Task: "Testes unitários de AppInfo em src-tauri/src/app_info.rs"
Task: "Testes de contrato IPC em src-tauri/tests/ipc_contract.rs"
Task: "Teste self_test_report ausente sem feature em src-tauri/tests/ipc_contract.rs"
Task: "Testes da interface em src/app-info.test.ts"
```

(T014 e T015 tocam o mesmo arquivo: escrever em sequência se o mesmo executor fizer os dois.)

---

## Implementation Strategy

### MVP First

1. Fases 1 e 2.
2. US1 → validar com `qa`.
3. US2 → validar com `qa`. **MVP**: app abre nos três sistemas mostrando dados do núcleo.

### Incremental Delivery

4. US4 (pode ter começado em paralelo).
5. US3 (README e clone limpo).
6. US5 (hook, CI, proteção). A partir de T030, todo trabalho vai por PR.
7. Polish e veredito final do `qa` na #1.

---

## Notes

- Cada tarefa vira issue `T0xx: …` ligada à #1 via `/speckit-github-taskstoissues`.
- Até T030, a branch `001-app-skeleton` ainda não tem trava remota: rodar `npm run verify` antes de cada push mesmo assim.
- Não ampliar escopo (FR-014): nada de editor, compilador ou rede.
