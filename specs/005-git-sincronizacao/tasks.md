---

description: "Tasks for 005-git-sincronizacao"
---

# Tasks: Sincronização pelo git (fase 2, parte 2)

**Input**: Design documents from `/specs/005-git-sincronizacao/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/ipc.md](contracts/ipc.md), [quickstart.md](quickstart.md)

**Portão de início (constituição, transição entre features)**: nenhuma tarefa de código deste
arquivo começa antes de (1) a `1.0.0` estar "em produção" (release publicada, `.dmg` validado
no macOS com evidência no épico #38, #145 e #38 fechadas, QA APROVADO) e (2) a 004
(`specs/004-comentarios`, épico #247) estar pronta, porque a mesclagem usa os estados CRDT e
o `review.json` com lápides da 004. Exceção aprovada pelo dono em 2026-11-26 vale só para
spec/plano/tarefas ([plan.md, Complexity Tracking](plan.md#complexity-tracking)). T002
(registrar o GitHub App) é ação do dono e pode ocorrer antes.

**Tests**: obrigatórios (Princípio VI, TDD). Em cada fase, as tarefas de teste vêm antes da
implementação, MUST falhar pelo motivo certo antes de a implementação começar e MUST ser
commitadas antes (ou junto) do código que cobrem. Rede real só em `perf_sync` (`#[ignore]`);
o resto usa remoto *bare* `file://` e o servidor GitHub falso em `127.0.0.1` ([research.md R11](research.md#r11-testes)).

**Organization**: por história de usuário (spec.md), em ordem de prioridade.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: pode rodar em paralelo (arquivos diferentes, sem dependência pendente)
- **[Story]**: história do spec.md (US1…US5)

## Path Conventions

Layout do [plan.md](plan.md#source-code-repository-root): interface em `src/`, núcleo em
`src-tauri/src/` (módulos novos `github/` e `sync/`), testes Rust de integração em
`src-tauri/tests/` (ajudantes em `src-tauri/tests/common/`), ponta a ponta em `e2e/`,
fixtures em `fixtures/sync/`, auditoria em `security/` e `src-tauri/deny.toml`.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: spike de risco, registro do App, dependências e ajudantes de teste; nenhum
comportamento novo.

- [ ] T001 Spike ([research.md R1](research.md#r1-biblioteca-git-gitoxide-gix--spike)): em `src-tauri/tests/spike_gix_push.rs`, com `gix =0.91.0` sem features que executam processos, (a) clonar e buscar de um repositório HTTPS público e de um *bare* `file://`; (b) tentar push por HTTPS com token (`receive-pack`) para um repositório de teste; (c) no plano B, criar o mesmo commit pela API Git Data (blobs → árvore → commit → `PATCH /git/refs/heads/<branch>` com `force: false`) e comparar o SHA com o commit local; (d) confirmar que `PATCH` não *fast-forward* é recusado. Registrar o resultado em research.md R1. Se cair no plano B, registrar D-19 em `docs/adrs.md` (via agente `doc-tecnica`) antes de T043
- [ ] T002 Ação do dono (`agente:humano`): registrar o GitHub App "Prelo" na conta `paulossjunior` com device flow habilitado, permissões por repositório `Contents: read & write` e `Metadata: read`, tokens com expiração ([research.md R2](research.md#r2-login-github-app--device-flow-d-18)); instalar no repositório privado de teste `prelo-sync-fixture` com o projeto `fixtures/review/artigo` da 004; informar o *client ID* (público) para `src-tauri/src/github/config.rs`
- [ ] T003 Escrever testes em `scripts/audit.test.ts`: `gix` (`=0.91.0`, ≥ 0.82 pela CVE-2026-82253) e `keyring` (`=4.2.0`) aparecem em `src-tauri/Cargo.toml` com versão exata; `gix` sem features que executam processos externos nem transporte `curl`/OpenSSL; nenhuma licença nova em `security/licenses-allow.txt` nem em `[licenses].allow` de `src-tauri/deny.toml`; `git2`/`libgit2-sys` ausentes da árvore de dependências. Confirmar que falham
- [ ] T004 Adicionar `gix = "=0.91.0"` (HTTP em Rust puro, TLS `rustls`) e `keyring = "=4.2.0"` (stores nativos: Keychain, Credential Manager, Secret Service) em `src-tauri/Cargo.toml` ([research.md R1, R3](research.md)); rodar `cargo deny check` e `cargo audit` até T003 passar
- [ ] T005 [P] Criar gerador de fixtures em `src-tauri/tests/common/sync_fixtures.rs` e dados em `fixtures/sync/`: remoto *bare* `file://` a partir de `fixtures/review/artigo`; dois clones A/B; repositórios maliciosos ([research.md R7](research.md#r7-segurança-do-repositório)) — árvore com `..`, caminho absoluto, `.git` em componente, nome reservado do Windows (`CON`, `aux.typ`), colisão por maiúsculas e por normalização Unicode, link simbólico para fora da raiz, hook `post-checkout`/`post-merge` que cria arquivo-sentinela, `.git/config` com `credential.helper`, `core.sshCommand`, `core.fsmonitor`, `include.path`, submódulo, ponteiro LFS, arquivo > 200 MiB gerado no teste, `.prelo/review.json` e `.prelo/versions.json` adulterados; clone com remoto não GitHub (SSH, GitLab) e com HEAD destacado
- [ ] T006 [P] Criar servidor GitHub falso em `src-tauri/tests/common/fake_github.rs` (`127.0.0.1`, porta aleatória, URL base injetável no núcleo só em `cfg(test)`/feature de teste): `/login/device/code`, `/login/oauth/access_token` (`authorization_pending`, `slow_down`, `expired_token`, `access_denied`, refresh válido e inválido), `/user`, `/user/installations`, `/user/installations/{id}/repositories` (com paginação), rotas Git Data do plano B com recusa não *fast-forward*, e registro de todas as requisições recebidas para os testes de intervalo (FR-006) e de vazamento de token

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: credenciais, abertura isolada do repositório, validação de caminhos, máquina de
estados e bloqueio de edição no núcleo; nenhuma história começa antes desta fase.

### Testes (escrever primeiro, confirmar que falham)

- [ ] T007 [P] Testes unitários em `src-tauri/src/github/credentials.rs` com o *mock store* do `keyring`: gravar/ler/apagar no serviço `app.prelo.github`; campos de [data-model.md Credential](data-model.md#credential-cofre-do-sistema-nunca-em-disco-do-projeto) — `Credential.accessToken`, `Credential.accessExpiresAt`, `Credential.refreshToken`, `Credential.refreshExpiresAt`, `Credential.login`, `Credential.userId`; `Debug`/`Display`/erros com tokens redigidos; `Credential` não implementa `Serialize`; store indisponível → token só em memória e aviso `keyring_unavailable`
- [ ] T008 [P] Testes unitários em `src-tauri/src/sync/repo.rs`: abrir com `gix::open::Options` isolado (sem config global/sistema; `credential.*`, `core.sshCommand`, `core.fsmonitor`, `include.*` ignorados); derivar [data-model.md RemoteInfo](data-model.md#remoteinfo-derivado-do-gitconfig-do-clone) — `RemoteInfo.url`, `RemoteInfo.owner`, `RemoteInfo.repo`, `RemoteInfo.branch` (remoto não GitHub → sem remoto + `remote_not_github`; HEAD destacado → `detached_head`); contagem `ahead`; criar commit local de [data-model.md SyncCommit](data-model.md#synccommit-commit-criado-pelo-app) — `SyncCommit.mensagem` (resumo ≤ 72 caracteres), `SyncCommit.autor/committer`, `SyncCommit.pais`
- [ ] T009 [P] Testes unitários em `src-tauri/src/sync/safety.rs`: cada caminho malicioso de T005 recusado com `unsafe_path`; limites de [data-model.md Limites do repositório](data-model.md#limites-do-repositório-r7) (clone 2 GiB → `repo_too_large`, arquivo 200 MiB, caminho 4096 bytes, componente 255); links simbólicos que ficam dentro da raiz aceitos; regra de escopo da 003 reaproveitada
- [ ] T010 [P] Testes unitários em `src-tauri/src/sync/mod.rs`: transições de [data-model.md SyncState](data-model.md#syncstate-memória-do-núcleo) — `SyncState.phase`, `SyncState.ahead`, `SyncState.lastSyncAt`, `SyncState.nextRetryAt` (30 s → ×2 → 5 min; só em `offline`), `SyncState.editable` (verdadeiro só em `local`/`synced`); `SyncUpdate` emitido a cada transição; trava `.git/prelo.lock` com PID (segunda instância → só leitura, `second_window_read_only`; PID morto → trava retomada)
- [ ] T011 [P] Teste de integração em `src-tauri/tests/sync_state.rs`: com `SyncState.editable` falso, `edit_file` responde `edit_blocked` e o texto do World não muda; comandos de comentário da 004 continuam aceitos (FR-013, Princípio IV); projeto sem remoto (`local`) edita como na `1.0.0` (FR-009)
- [ ] T012 [P] Teste de segurança em `src-tauri/tests/sync_security.rs` (abrir clone existente com as fixtures de T005): nenhum hook executado (sentinela ausente), config perigosa ignorada, nada escrito fora da pasta, submódulo → `submodules_unsupported`, LFS → `lfs_unsupported` (SC-005)

### Implementação

- [ ] T013 Implementar `src-tauri/src/github/credentials.rs` e `src-tauri/src/github/config.rs` (client ID, URLs base) até T007 passar
- [ ] T014 Implementar cliente HTTP base em `src-tauri/src/github/api.rs` (`ureq` + `rustls` da 003, *timeouts*, `Authorization` só dentro do núcleo, erros mapeados para `network`/`rate_limited`/`auth_required` sem corpo nem cabeçalhos)
- [ ] T015 Implementar `src-tauri/src/sync/repo.rs` (abrir isolado, RemoteInfo, ahead, commit local com `SyncCommit`) até T008 passar
- [ ] T016 Implementar `src-tauri/src/sync/safety.rs` até T009 passar
- [ ] T017 Implementar `src-tauri/src/sync/mod.rs`: máquina de estados, trava, `sync_status`, `subscribe_sync` e evento `SyncUpdate` de [contracts/ipc.md](contracts/ipc.md#sincronização), até T010 e T012 passarem
- [ ] T018 Ligar `edit_file` em `src-tauri/src/commands.rs` a `SyncState.editable` (`edit_blocked`), até T011 passar; sem regressão em `src-tauri/tests/perf_preview.rs`
- [ ] T019 [P] Testes em `src/ipc.test.ts` e depois tipos/funções em `src/ipc.ts` para todos os comandos, erros, `Warning.code` e o evento de [contracts/ipc.md](contracts/ipc.md)
- [ ] T020 [P] Testes em `src/sync/status.test.ts`, depois `src/sync/status.ts`: indicador de estado (Verificando, Recebendo, Sincronizado, Offline, Login necessário, contagem `ahead`) e editor CodeMirror em `readOnly` quando `SyncState.editable` é falso, ainda legível e com comentários habilitados

**Checkpoint**: núcleo abre clones com segurança, guarda credenciais e bloqueia edição fora de
`local`/`synced`; histórias podem começar.

---

## Phase 3: User Story 1 - Conectar ao GitHub e abrir o projeto do grupo (Priority: P1) 🎯 MVP

**Goal**: login por device flow, lista de repositórios autorizados, clonar pelo app e abrir
clone existente (FR-001 a FR-011).

**Independent Test**: com o servidor falso (automático) e com `prelo-sync-fixture` (manual):
login, clonar, abrir pasta clonada; ver "conectado como @login" e o remoto; token ausente de
arquivos, logs e respostas (quickstart passos 1–3, 11, 12).

### Testes (escrever primeiro, confirmar que falham)

- [ ] T021 [P] [US1] Teste em `src-tauri/tests/sync_auth.rs` com T006: device flow completo; `authorization_pending`; `slow_down` aumenta o intervalo; `expired_token` e `access_denied` → `expired`/`denied` sem travar; `github_sign_in_poll` antes do intervalo → `pending` sem requisição ao servidor (FR-006); `Credential.login` e `Credential.userId` de [data-model.md](data-model.md#credential-cofre-do-sistema-nunca-em-disco-do-projeto) validados na resposta de `/user` (valor fora do padrão → erro, nada gravado); renovação 5 min antes de `Credential.accessExpiresAt`; `Credential.refreshExpiresAt` vencido ou refresh recusado → `auth_required`; `github_sign_out { confirm: true }` apaga do cofre, sem `confirm` → `confirmation_required`
- [ ] T022 [P] [US1] Testes unitários em `src-tauri/src/github/api.rs`: `/user/installations` e repositórios com paginação → `{ owner, repo, private, defaultBranch }[]` com `RemoteInfo.owner`/`RemoteInfo.repo` de [data-model.md](data-model.md#remoteinfo-derivado-do-gitconfig-do-clone) validados; repositório fora da instalação → `not_authorized_repo` com `detail.installUrl`; respostas hostis (JSON enorme, campos com tipo errado, 5xx) sem pânico
- [ ] T023 [P] [US1] Teste de segurança em `src-tauri/tests/sync_security.rs` (clonar): cada repositório malicioso de T005 recusado (`unsafe_path` com `detail.path`) ou isolado, nada escrito fora da pasta, hooks não executados (SC-005); pasta não vazia → `folder_not_empty`; acima do limite → `repo_too_large`; clone interrompido não deixa pasta meio escrita como projeto válido
- [ ] T024 [P] [US1] Teste em `src-tauri/tests/sync_state.rs`: abrir pasta já clonada com remoto `https://github.com/<owner>/<repo>(.git)` → `checking`; remoto SSH/GitLab → `local` + `remote_not_github`; HEAD destacado → `local` + `detached_head`
- [ ] T025 [P] [US1] Testes em `src/sync/signin.test.ts`: mostra `userCode` e link, *polling* pelo núcleo, estados `denied`/`expired` com motivo e volta a "sem login", "conectado como @login", "Sair do GitHub" com confirmação
- [ ] T026 [P] [US1] Testes em `src/sync/clone.test.ts`: lista de repositórios, escolha de pasta, erros `not_authorized_repo` (link de instalação), `folder_not_empty`, `repo_too_large`, `unsafe_path`, `network`

### Implementação

- [ ] T027 [US1] Implementar `src-tauri/src/github/device_flow.rs` (início, *polling* com intervalo controlado pelo núcleo, refresh, abertura do navegador pelo núcleo) até T021 passar
- [ ] T028 [US1] Completar `src-tauri/src/github/api.rs` (`/user`, instalações, repositórios) até T022 passar
- [ ] T029 [US1] Implementar clone em `src-tauri/src/sync/repo.rs` com validação de `src-tauri/src/sync/safety.rs` antes de cada escrita e token só no transporte, até T023 passar
- [ ] T030 [US1] Ligar a abertura de projeto (`src-tauri/src/commands.rs`, `src-tauri/src/project/`) à detecção de remoto e ao estado `checking`/`local`, até T024 passar
- [ ] T031 [US1] Registrar `github_status`, `github_sign_in_start`, `github_sign_in_poll`, `github_sign_out`, `github_list_repos`, `clone_dialog` em `src-tauri/src/commands.rs` e `src-tauri/src/lib.rs`; sem capability `http` nem `shell` na interface (`src-tauri/capabilities/`)
- [ ] T032 [P] [US1] Implementar `src/sync/signin.ts` até T025 passar
- [ ] T033 [P] [US1] Implementar `src/sync/clone.ts` e entradas "Entrar com GitHub" / "Clonar do GitHub" na tela inicial (`src/workspace.ts`), até T026 passar
- [ ] T034 [US1] e2e em `e2e/sync-login.e2e.ts` com o servidor falso: login, listar, clonar de remoto `file://` mapeado, abrir; token ausente do DOM e dos logs

**Checkpoint**: US1 validável sozinha; projeto clonado abre e mostra estado.

---

## Phase 4: User Story 2 - Salvar sincroniza (Priority: P1)

**Goal**: ⌘S = bloquear → commit local → receber → mesclar → enviar → liberar, com mesclagem
própria sem marcas de conflito e resumo por pessoa (FR-016 a FR-027, FR-029).

**Independent Test**: dois clones A/B do remoto `file://`; editar e comentar nos dois; Salvar
em A e depois em B; B termina com tudo, remoto com os commits, `.typ` compila (quickstart
passos 4, 5, 10).

### Testes (escrever primeiro, confirmar que falham)

- [ ] T035 [P] [US2] Testes em `src-tauri/tests/sync_merge.rs` (`.typ`): edições em trechos diferentes → ambas; mesmo trecho → as duas inserções ficam e `SyncSummary.sameSpanEdits` de [data-model.md](data-model.md#syncsummary) traz `{ path, quote }`; `.typ` remoto ≠ `Doc` remoto (edição fora do Prelo) reconciliado como autor "externo"; toque em trecho editado localmente → `ConflictChoice` `external_typ`; SC-002: 100 rodadas com semente fixa, 0 marcas de conflito (`<<<<<<<`, `=======`, `>>>>>>>`), `.typ` compila com o typst embutido e com o `typst` CLI quando disponível
- [ ] T036 [P] [US2] Testes unitários em `src-tauri/src/review/merge.rs`: `review.json` por chave — união de conversas e mensagens por id, estado pelo `statusAt` mais recente, lápides de conversa e de mensagem vencem respostas e mudanças de estado concorrentes (emenda de `specs/004-comentarios/data-model.md`); entrada inválida de um lado tratada como na 004 (FR-030 a FR-034 da 004), sem abortar
- [ ] T037 [P] [US2] Testes unitários em `src-tauri/src/sync/merge.rs`: `.prelo/project.json` por chave, `owner` divergente → mais antigo fica + `owner_conflict`; outros arquivos alterados dos dois lados → `ConflictChoice` com campos de [data-model.md ConflictChoice](data-model.md#conflictchoice) — `ConflictChoice.id`, `ConflictChoice.path`, `ConflictChoice.kind`, `ConflictChoice.mine`/`ConflictChoice.theirs` (texto ≤ 200 KiB, `null` para binário); `resolve_choice` `mine`/`theirs` e o descartado alcançável no histórico; arquivos alterados de um lado só aplicados sem pergunta
- [ ] T038 [P] [US2] Testes do fluxo de Salvar em `src-tauri/tests/sync_merge.rs`: ordem de FR-016 (commit local antes de qualquer rede; edição bloqueada até o fim); nada mudou → sem commit e "tudo sincronizado" (FR-018); commit de mesclagem com 2 pais e mensagem/autor de `SyncCommit` ([data-model.md](data-model.md#synccommit-commit-criado-pelo-app)); push recusado por avanço → 3 tentativas e depois `push_rejected`; histórico divergente (force-push alheio) → `diverged`, nada enviado, nada perdido; com `choices_pending` o Salvar para e continua após o último `resolve_choice`; mesmo cenário pelo plano B contra T006 se T001 decidir pela API Git Data
- [ ] T039 [P] [US2] Testes unitários em `src-tauri/src/sync/summary.rs`: `SyncSummary.byPerson` de [data-model.md](data-model.md#syncsummary) agrupado por autor (commits, `newThreads`, `resolvedThreads`) entre o ref antigo e o novo; nada recebido → sem resumo
- [ ] T040 [P] [US2] Testes em `src/sync/summary.test.ts`: resumo por pessoa fechável, aviso de mesmo trecho, diálogo "manter o meu" / "manter o recebido" (texto lado a lado; binário com tamanho e data)

### Implementação

- [ ] T041 [US2] Implementar `src-tauri/src/review/merge.rs` até T036 passar
- [ ] T042 [US2] Implementar `src-tauri/src/sync/merge.rs` ([research.md R4](research.md#r4-mesclagem-própria-não-git-merge)): base = *merge-base* via `gix`; `.typ` pela união dos updates `.prelo/crdt/<id>.yjs` e `.typ` gerado do `Doc`; `.prelo/*.json` por chave; escolhas; tudo validado por `src-tauri/src/sync/safety.rs` e pelas regras da 004 antes de gravar (FR-027), até T035 e T037 passarem
- [ ] T043 [US2] Implementar `src-tauri/src/sync/push.rs` conforme T001 (nativo `gix` ou API Git Data com `force: false`; no plano B, adotar o commit do GitHub como local se o SHA diferir)
- [ ] T044 [US2] Implementar `src-tauri/src/sync/summary.rs` até T039 passar
- [ ] T045 [US2] Implementar `sync_now` e `resolve_choice` em `src-tauri/src/sync/mod.rs` e registrá-los em `src-tauri/src/commands.rs`, com `SyncUpdate.progress` (`commit`, `fetch`, `merge`, `push`), até T038 passar
- [ ] T046 [US2] Ligar ⌘S a `sync_now` em projeto com remoto em `src/workspace.ts` (sem remoto continua só gravando)
- [ ] T047 [P] [US2] Implementar `src/sync/summary.ts` até T040 passar
- [ ] T048 [US2] e2e em `e2e/sync-save.e2e.ts`: dois clones de remoto `file://`, editar e comentar nos dois, Salvar em sequência, conferir texto e comentários mesclados e resumo

**Checkpoint**: US1 + US2 entregam o ciclo assíncrono básico.

---

## Phase 5: User Story 3 - Sem conexão, nada se perde (Priority: P1)

**Goal**: offline lê e comenta, edição bloqueada, Salvar grava commit local e agenda,
reconexão automática, token renovado sem incomodar (FR-004, FR-013, FR-015, FR-019, FR-021).

**Independent Test**: remoto inacessível; abrir, comentar, salvar, matar o app; restaurar e
reabrir; comentários enviados e edição só liberada após receber (quickstart passo 6).

### Testes (escrever primeiro, confirmar que falham)

- [ ] T049 [P] [US3] Testes em `src-tauri/tests/sync_crash.rs`: processo filho morto em cada passo de [research.md R6](research.md#r6-salvar-e-encerramento-forçado) (gravar, commit, fetch, merge, push); ao reabrir, nenhum comentário, edição salva ou commit local perdido, `SyncState.ahead` correto e envio na próxima sincronização (SC-003, FR-021)
- [ ] T050 [P] [US3] Testes em `src-tauri/tests/sync_offline.rs`: remoto inacessível ao abrir → `offline`, `edit_blocked`, comentar/responder/resolver aceitos; `sync_now` → erro `offline` com commit local feito e `state`; `SyncState.nextRetryAt` segue 30 s → 5 min (relógio simulado); reconexão sincroniza sem ação e só então `editable` (SC-006); token expirado no meio → renovado sem evento visível; refresh recusado → `auth_required`, pendências intactas, login volta a `checking`
- [ ] T051 [P] [US3] Testes em `src/sync/status.test.ts`: mensagens "offline", "guardado no computador; será enviado quando a conexão voltar", "Login necessário" com ação de entrar

### Implementação

- [ ] T052 [US3] Implementar agendador de reconexão e envio de pendências em `src-tauri/src/sync/mod.rs` (também ao voltar de suspensão e a cada Salvar), até T050 passar
- [ ] T053 [US3] Integrar renovação de token e `auth_required` ao fluxo de `src-tauri/src/sync/mod.rs` e `src-tauri/src/github/device_flow.rs`, até T049 e T050 passarem
- [ ] T054 [P] [US3] Completar `src/sync/status.ts` até T051 passar
- [ ] T055 [US3] e2e em `e2e/sync-offline.e2e.ts`: remoto `file://` indisponível (pasta renomeada), comentar e salvar, restaurar, ver envio automático e liberação da edição

---

## Phase 6: User Story 4 - Abrir e fechar sincronizam (Priority: P2)

**Goal**: abrir recebe antes de liberar (FR-014); fechar faz último Salvar com "Fechar mesmo
assim" após 15 s (FR-028).

**Independent Test**: salvar em A; abrir B e ver edição liberada só após receber; editar em B
e fechar; commit de B no remoto (quickstart passos 7–8).

### Testes (escrever primeiro, confirmar que falham)

- [ ] T056 [P] [US4] Testes em `src-tauri/tests/sync_state.rs`: abrir com commits novos no remoto → `checking` → `receiving` → `synced`, `edit_blocked` até o fim, resumo emitido (SC-006); reabrir com `ahead > 0` → aviso `unsent_commits` com `detail.count`
- [ ] T057 [P] [US4] Testes em `src-tauri/tests/sync_close.rs`: `close_requested` roda `sync_now` e fecha ao terminar; sem rede → commit local e fecha; `force_close { confirm: true }` grava commit local e fecha; sem `confirm` → `confirmation_required`
- [ ] T058 [P] [US4] Testes em `src/workspace.test.ts`: progresso de Salvar ao fechar e botão "Fechar mesmo assim" só depois de 15 s (relógio falso)

### Implementação

- [ ] T059 [US4] Implementar o fluxo de abertura em `src-tauri/src/sync/mod.rs` (verificar, receber, mesclar, liberar, resumo) até T056 passar
- [ ] T060 [US4] Interceptar `CloseRequested` em `src-tauri/src/lib.rs` para rodar `sync_now`; implementar `force_close` em `src-tauri/src/commands.rs`, até T057 passar
- [ ] T061 [US4] Implementar a UI de fechamento em `src/workspace.ts` até T058 passar
- [ ] T062 [US4] e2e em `e2e/sync-open-close.e2e.ts`: abrir recebe antes de liberar; fechar envia

---

## Phase 7: User Story 5 - Versões nomeadas (Priority: P3)

**Goal**: marcar e listar versões em `.prelo/versions.json` (FR-025, FR-030 a FR-032).

**Independent Test**: marcar "Submissão 1", sincronizar, abrir em outro clone e ver nome,
autor, data e commit (quickstart passo 9).

### Testes (escrever primeiro, confirmar que falham)

- [ ] T063 [P] [US5] Testes em `src-tauri/tests/sync_versions.rs`: validação de cada campo de [data-model.md VersionsFile](data-model.md#versionsfile--prelocversionsjson) — `VersionsFile.schema` (maior → `review_read_only`), `VersionsFile.versions` (≤ 1.000), `Version.name` (único sem diferença de maiúsculas e espaços → `name_taken`; inválido → `invalid_name`), `Version.author`, `Version.at`, `Version.commit` (inexistente → `missing: true`); arquivo > 1 MiB, JSON inválido ou tipo errado → `versions_json_invalid`, original preservado e `.invalid-<ts>` antes de regravar; `mark_version` sincroniza antes e grava segundo commit só com `versions.json` apontando para o commit resultante; offline → `offline`; mesclagem por união de id, nome repetido entre lados → mais antigo fica, outro ganha sufixo ` (2)` e aviso; `list_versions` com `commitsSincePrevious`
- [ ] T064 [P] [US5] Testes em `src/sync/versions.test.ts`: diálogo "Marcar versão", erros `name_taken`/`invalid_name`/`offline`, lista com nome, autor, data e resumo de commits desde a anterior, versão `missing` com aviso

### Implementação

- [ ] T065 [US5] Implementar `src-tauri/src/sync/versions.rs` (leitura defensiva no padrão de `src-tauri/src/review/file.rs`, gravação atômica, mesclagem) e ligar a mesclagem de `versions.json` em `src-tauri/src/sync/merge.rs`, até T063 passar
- [ ] T066 [US5] Registrar `mark_version` e `list_versions` em `src-tauri/src/commands.rs`
- [ ] T067 [P] [US5] Implementar `src/sync/versions.ts` e a entrada "Marcar versão" no menu, até T064 passar
- [ ] T068 [US5] e2e em `e2e/sync-versions.e2e.ts`: marcar em A, Salvar em B, versão aparece em B

---

## Phase 8: Polish & Cross-Cutting Concerns

- [ ] T069 [P] Teste de vazamento em `src-tauri/tests/sync_security.rs` (SC-004): rodar login, clone, Salvar, offline e erros com token marcador conhecido; varrer logs do app, pasta do projeto, `.git/`, `.prelo/`, respostas IPC, eventos e mensagens de erro; 0 ocorrências
- [ ] T070 [P] Teste de desempenho em `src-tauri/tests/perf_sync.rs` (`#[ignore]`, rede real, `prelo-sync-fixture`, [research.md R10](research.md#r10-rede-de-referência-todorede-comum)): Salvar com 1 edição local e 1 commit remoto ~1 s p50, ≤ 3 s p95 (SC-001); prévia continua < 100 ms
- [ ] T071 [P] CI em `.github/workflows/ci.yml`: rodar `sync_*` em macOS, Windows e Linux (keyring nativo; Linux sem Secret Service → `keyring_unavailable`), sem rede externa
- [ ] T072 [P] Acessibilidade em `src/sync/signin.test.ts`, `src/sync/summary.test.ts` e `src/sync/versions.test.ts`: navegação por teclado, foco, nomes acessíveis, estado anunciado por `aria-live`
- [ ] T073 [P] Atualizar `docs/requisitos.md` (RF-32, RF-33, RF-34, RF-36, RF-38, RF-40, RF-41 com issue/US da 005), `docs/release-plan.md` (R2: épico da 005, TODO(rede-comum) resolvido), `docs/arquitetura.md` §5 (estados e mesclagem própria) e `docs/adrs.md` (D-18 pendências resolvidas; D-19 se plano B) via agente `doc-tecnica`
- [ ] T074 Rodar `npm run verify` completo, os testes de [quickstart.md](quickstart.md#testes-automatizados) e o roteiro manual no `.dmg` no macOS, incluindo o tempo de SC-007; registrar evidência no épico
- [ ] T075 Revisão do agente `qa` (modo validar): princípios I–VI, cobertura de SC-001 a SC-007; veredito APROVADO/REPROVADO no épico

---

## Dependencies & Execution Order

### Phase Dependencies

- **Portão**: `1.0.0` em produção **e** 004 pronta → qualquer tarefa de código (T002 pode
  antes)
- **T001 (spike)** → T043 (push) e T038 (cenário do plano B); D-19 registrado antes de T043 se
  plano B
- **T002 (App registrado)** → T027 em diante só para teste manual; automáticos usam T006
- **Setup (1)** → **Foundational (2)** → histórias
- **US1 (3)** é base de US2–US5 (credencial, clone, comandos)
- **US2 (4)** é base de US3, US4 e US5 (`sync_now`, mesclagem)
- **US3 (5)** e **US4 (6)** podem correr em paralelo depois de US2
- **US5 (7)** depende de US2
- **Polish (8)** depois das histórias desejadas

### Within Each Story

Testes (falhando) → núcleo → registro IPC → interface → e2e.

### Parallel Opportunities

- T005 e T006 em paralelo com T003/T004; T001 e T002 independentes
- T007–T012 (testes da fundação) em paralelo; T019 e T020 em paralelo com T013–T018
- Em US1: T021–T026 em paralelo; T032 e T033 em paralelo
- Em US2: T035–T040 em paralelo
- Depois de US2: US3 e US4 em paralelo
- T069–T073 em paralelo

## Parallel Example: User Story 1

```text
Testes juntos: T021 (sync_auth.rs), T022 (api.rs), T023 (sync_security.rs),
T024 (sync_state.rs), T025 (signin.test.ts), T026 (clone.test.ts)
Depois: T027–T031 (núcleo) em paralelo com T032/T033 (interface, contra o contrato)
```

## Parallel Example: User Story 2

```text
Testes juntos: T035 (sync_merge.rs .typ), T036 (review/merge.rs), T037 (sync/merge.rs),
T039 (summary.rs), T040 (summary.test.ts); T038 depois de T035 (mesmo arquivo)
```

## Implementation Strategy

### MVP First (US1 + US2)

1. Portão cumprido (1.0.0 em produção, 004 pronta); T001 e T002 resolvidos
2. Setup + Foundational
3. US1 → validar sozinha (quickstart passos 1–3, 11, 12)
4. US2 → ciclo de Salvar (quickstart passos 4, 5, 10). US1 sozinha não entrega valor de
   sincronização; o MVP útil é US1 + US2

### Incremental Delivery

US1 → US2 → US3 (offline) → US4 (abrir/fechar) → US5 (versões). A `1.1.0` sai com a 004 e a
005; US5 pode ficar para uma `1.1.x` por decisão do dono.

## Notes

- Bloqueio de edição vale no núcleo (`edit_blocked`), não só na interface (Princípio IV).
- Nenhuma marca de conflito é escrita em arquivo algum (T035, T037).
- Nenhum token sai do núcleo (T007, T021, T069).
- Nenhum hook é executado e nenhuma credencial do git do sistema é lida (T012, T023).
- Commits por tarefa ou grupo lógico; push só com `npm run verify -- --quick` verde.
