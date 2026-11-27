---

description: "Tasks for 004-comentarios"
---

# Tasks: Comentários (fase 2, parte 1)

**Input**: Design documents from `/specs/004-comentarios/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/ipc.md](contracts/ipc.md), [quickstart.md](quickstart.md)

**Portão de início (constituição, transição entre features)**: nenhuma tarefa deste arquivo
começa antes de a `1.0.0` estar "em produção" (release publicada, `.dmg` validado no macOS
com evidência no épico #38, #145 e #38 fechadas, QA APROVADO). Exceção aprovada pelo dono em
2026-11-26 vale só para spec/plano/tarefas ([plan.md, Complexity Tracking](plan.md#complexity-tracking)).

**Tests**: obrigatórios (Princípio VI, TDD). Em cada fase, as tarefas de teste vêm antes da
implementação, MUST falhar pelo motivo certo antes de a implementação começar e MUST ser
commitadas antes (ou junto) do código que cobrem. O agente `qa` (modo projetar) pode executar
as tarefas de teste; o modo validar roda ao fim de cada história.

**Organization**: por história de usuário (spec.md), em ordem de prioridade.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: pode rodar em paralelo (arquivos diferentes, sem dependência pendente)
- **[Story]**: história do spec.md (US1…US5)

## Path Conventions

Layout do [plan.md](plan.md#source-code-repository-root): interface em `src/`, núcleo em
`src-tauri/src/`, testes Rust de integração em `src-tauri/tests/`, ponta a ponta em `e2e/`,
fixtures em `fixtures/review/`, auditoria em `security/` e `src-tauri/deny.toml`.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: dependências, spike de risco e fixtures; nenhum comportamento novo.

- [ ] T001 Spike ([research.md R9](research.md#r9-pdf-com-comentários-rf-43--spike)): em `src-tauri/tests/spike_pdf_image.rs`, compilar com o typst 0.15.1 embutido um documento sintético que embute a página 2 de um PDF em memória via `image(bytes, format: "pdf", page: 2)`; registrar resultado (funciona/fidelidade vetorial) em research.md R9. Se falhar, marcar o fallback SVG como decisão e ajustar T060
- [ ] T002 Escrever testes em `scripts/audit.test.ts`: `yrs` (MIT) e `similar` (Apache-2.0) aparecem em `src-tauri/Cargo.toml` com versão exata (`=0.28.2`, `=3.2.1`) e nenhuma licença nova entra em `security/licenses-allow.txt` nem em `[licenses].allow` de `src-tauri/deny.toml`. Confirmar que falham
- [ ] T003 Adicionar `yrs = "=0.28.2"` e `similar = "=3.2.1"` em `src-tauri/Cargo.toml` ([research.md R1, R4](research.md)); rodar `cargo deny check` e `cargo audit` até T002 passar
- [ ] T004 [P] Criar fixtures em `fixtures/review/artigo/` (`main.typ` com 3 seções, `cap/metodo.typ` incluído, `.prelo/project.json` com `owner.name = "Ana"`) e `fixtures/review/adulterados/` (um `review.json` por caso de [data-model.md ReviewFile/Thread/Message](data-model.md): JSON inválido, truncado, > 16 MiB gerado no teste, `schema: 2`, `schema` ausente, id fora do padrão, `file` com `..`/absoluto/`\`/nulo/link simbólico para fora, `anchor` base64 inválido e > 256 bytes, mensagem > 10.000 caracteres, nome > 100, 5.001 conversas, conversa válida misturada com inválida, `body` com `<script>`, `#import`, caracteres de controle)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: documento CRDT, arquivo de revisão, identidade e evento; nenhuma história começa
antes desta fase.

### Testes (escrever primeiro, confirmar que falham)

- [ ] T005 [P] Testes unitários em `src-tauri/src/review/crdt.rs`: aplicar deltas `{from,to,insert}` UTF-16 (inclusive emoji e acentos) mantém `TextRef "content"` igual ao texto do World; `StickyIndex` início (`Assoc::After`) e fim (`Assoc::Before`) seguem edições antes, dentro e depois; trecho apagado → `detached`; serializar/ler estado com cabeçalho de [data-model.md CrdtState](data-model.md#crdtstate--preloccrdtidyjs) (linha 1, `path` ↔ `<id>`, tamanho ≤ 32 MiB)
- [ ] T006 [P] Testes unitários em `src-tauri/src/review/reconcile.rs`: diff disco ↔ `Doc` preserva âncoras de trechos intactos; troca total → âncoras `detached`; limite de 200 ms cai para diff por linhas ([research.md R4](research.md#r4-reconciliação-com-o-disco-fr-008))
- [ ] T007 [P] Testes unitários em `src-tauri/src/review/file.rs`: ida e volta de `ReviewFile` com campos desconhecidos preservados; validação de cada campo restrito de [data-model.md](data-model.md) — `ReviewFile.schema`, `ReviewFile.threads`, `Thread.file`, `Thread.anchor.start`, `Thread.anchor.end`, `Thread.quote`, `Thread.status`, `Thread.openedBy`, `Thread.openedAt`, `Thread.statusBy`, `Thread.statusAt`, `Thread.messages`, `Message.id`, `Message.author`, `Message.at`, `Message.body`, `Message.mentions`, `Message.deleted`; ids gerados casam `^[0-9a-f]{32}$`
- [ ] T008 [P] Teste de segurança em `src-tauri/tests/review_json_security.rs` com as fixtures de T004: cada caso → aviso certo (`review_json_invalid`, `review_json_newer`, `review_threads_skipped`), projeto abre e edita, arquivo original intacto byte a byte, resposta ≤ 1 s (SC-004, FR-030 a FR-034); conversa inválida preservada na regravação (FR-032); cópia `.prelo/review.json.invalid-<ts>` criada antes da primeira gravação nova (FR-031); `schema: 2` recusa toda gravação com `review_read_only`
- [ ] T009 [P] Teste de segurança em `src-tauri/tests/review_json_security.rs` para `.prelo/crdt/*.yjs`: cabeçalho inválido, `path` que não gera o `<id>`, `path` fora da raiz, update v1 corrompido, > 32 MiB → `crdt_state_invalid`, `Doc` recriado do disco, conversas daquele arquivo `detached`, cópia `.invalid-<ts>` antes de regravar ([data-model.md CrdtState](data-model.md))
- [ ] T010 [P] Testes em `src-tauri/src/review/identity.rs`: `settings.json` em diretório temporário (`Settings.displayName` de [data-model.md](data-model.md): 1–100, sem controle); `ProjectRecord.owner.name` lido e regravado em `src-tauri/src/project/record.rs` preservando campos desconhecidos; `settings.json` malformado → sem nome, sem pânico
- [ ] T011 [P] Teste de gravação atômica em `src-tauri/src/review/file.rs`: falha simulada entre escrita do temporário e rename deixa a versão anterior inteira (FR-024)

### Implementação

- [ ] T012 Implementar `src-tauri/src/review/crdt.rs` ([research.md R1–R3](research.md)): `Doc` por `.typ` aberto com `OffsetKind::Utf16`, aplicação dos deltas, `StickyIndex` em base64, leitura/gravação de `.prelo/crdt/<id>.yjs` (tempfile + rename, como `src-tauri/src/fs/save.rs`), até T005 e T009 passarem
- [ ] T013 Ligar `edit_file` (`src-tauri/src/commands.rs`, `src-tauri/src/compile/project.rs`) ao `Doc` de T012: mesma transação do texto do World; gravar estado CRDT no ciclo do salvamento automático; sem regressão da prévia < 100 ms (`src-tauri/tests/perf_preview.rs` da 003 continua verde)
- [ ] T014 Implementar `src-tauri/src/review/reconcile.rs` com `similar` e chamá-lo ao abrir `.typ` e em `resolve_external_change { keep: "disk" }`, até T006 passar
- [ ] T015 Implementar `src-tauri/src/review/file.rs` (leitura defensiva no padrão de `src-tauri/src/project/record.rs`, limites de [research.md R5](research.md#r5-arquivo-prelo-reviewjson), gravação atômica com `fsync`), até T007, T008 e T011 passarem
- [ ] T016 Implementar `src-tauri/src/review/identity.rs` e o campo `owner` em `src-tauri/src/project/record.rs`, até T010 passar
- [ ] T017 Implementar `src-tauri/src/review/mod.rs`: estado de revisão do projeto aberto (carrega `review.json` + estados CRDT ao abrir projeto, descarta ao fechar), evento `ReviewUpdate` e `subscribe_review` de [contracts/ipc.md](contracts/ipc.md#evento-reviewupdate) com agregação de no máximo 1 `anchors_moved` por compilação
- [ ] T018 [P] Testes em `src/ipc.test.ts` e depois tipos/funções em `src/ipc.ts` para todos os comandos e o evento de [contracts/ipc.md](contracts/ipc.md)
- [ ] T019 [P] Testes em `src/review/identity.test.ts`, depois `src/review/identity.ts`: pedido do nome antes do primeiro comentário, edição nas preferências (`get_identity`, `set_display_name`)

**Checkpoint**: núcleo mantém `Doc` e lê/grava `review.json` com segurança; histórias podem
começar.

---

## Phase 3: User Story 1 - Comentar um trecho e conversar sobre ele (Priority: P1) 🎯 MVP

**Goal**: criar conversa ancorada, responder, mencionar, navegar trecho ↔ conversa, tudo fora
do `.typ` e sobrevivendo a encerramento forçado.

**Independent Test**: spec US1 — comentar, responder com menção, editar antes/dentro do
trecho, fechar e reabrir; `.typ` compila no `typst` CLI sem vestígio.

### Testes (escrever primeiro, confirmar que falham)

- [ ] T020 [P] [US1] Testes de comando em `src-tauri/src/review/mod.rs` para `create_thread`, `reply`, `list_threads`, `known_names`: erros `no_display_name`, `empty_selection`, `bad_range`, `body_empty`, `body_too_long` (`Message.body` de [data-model.md](data-model.md)), `out_of_scope` (`Thread.file`), `review_read_only`; autor sempre vem de `settings.json`, nunca de argumento (argumento `author` extra → `invalid_arguments`); `Message.mentions` só com nomes conhecidos (FR-004)
- [ ] T021 [P] [US1] Teste de integração em `src-tauri/tests/review_anchor.rs` (SC-003): 200 edições aleatórias com semente fixa antes, dentro e depois de 10 trechos; 100% das âncoras de trechos vivos apontam para o mesmo texto lógico, 100% das de trechos apagados ficam `detached`, nenhuma conversa some; inclui reconciliação após mudança externa do `.typ`
- [ ] T022 [P] [US1] Teste de encerramento forçado em `src-tauri/tests/review_crash.rs` (SC-002, FR-023): processo filho cria comentário, sinaliza resposta recebida e é morto (`SIGKILL` / `TerminateProcess`) em pontos aleatórios; 100 repetições; todo comentário confirmado está presente ao reabrir; `review.json` sempre JSON válido
- [ ] T023 [P] [US1] Teste em `src-tauri/tests/review_typ_pure.rs` (SC-001, Princípio I): após criar comentários na fixture, os `.typ` são byte a byte iguais aos de antes e o PDF do núcleo é idêntico, página a página, ao do `typst` 0.15.1 CLI (mesmo mecanismo da suíte `equivalence` da 003)
- [ ] T024 [P] [US1] Testes em `src/review/marks.test.ts`: `ReviewUpdate.ranges` viram `Decoration.mark` no CodeMirror; atualização por diff de ids; sobreposição de conversas; clique na marca destaca a conversa
- [ ] T025 [P] [US1] Testes em `src/review/panel.test.ts`: aba "Abertos" em ordem do documento; cartão com autor, hora, `quote`, mensagens; texto inserido como `textContent` (corpo com `<script>`/`<img onerror>` aparece literal, FR-014); clique no cartão abre o arquivo e seleciona o trecho; conversa `detached` mostra "trecho removido"
- [ ] T026 [P] [US1] Testes em `src/review/composer.test.ts`: `@` sugere `known_names`; menção escolhida vira `@{nome}` destacado; vazio/só espaços não envia; > 10.000 caracteres mostra aviso e não corta; erro `write_failed` mantém o texto na caixa

### Implementação

- [ ] T027 [US1] Implementar `create_thread`, `reply`, `list_threads`, `known_names` em `src-tauri/src/review/mod.rs` e registrar em `src-tauri/src/commands.rs` e na capability da janela (`src-tauri/capabilities/`), com `Thread.quote` truncado em 500 e gravação do estado CRDT antes de confirmar âncora nova, até T020–T023 passarem
- [ ] T028 [US1] Implementar `src/review/marks.ts` (StateField + decorações) e ligar ao editor em `src/editor/editor.ts`, até T024 passar
- [ ] T029 [US1] Implementar `src/review/panel.ts` (painel recolhível no padrão de `src/panes.ts`, aba "Abertos") e estilos em `src/workspace.css`, até T025 passar
- [ ] T030 [US1] Implementar `src/review/composer.ts` (CodeMirror mínimo com `@codemirror/autocomplete`) e a ação "Comentar" (menu e atalho; indisponível sem seleção) em `src/workspace.ts`, até T026 passar
- [ ] T031 [US1] Testes em `src/workspace.test.ts`, depois ligação em `src/workspace.ts`: abrir projeto assina `subscribe_review`; trocar de projeto limpa marcas e painel; avisos de `ReviewUpdate.warnings` aparecem na barra de status
- [ ] T032 [US1] e2e em `e2e/review.e2e.ts`: selecionar, comentar, responder com menção, editar antes do trecho, conferir marca no lugar

**Checkpoint**: US1 funcional e testável sozinha (MVP).

---

## Phase 4: User Story 2 - Resolver, reabrir e consultar os resolvidos (Priority: P1)

**Goal**: estados aberto ↔ resolvido com autoria, aba "Resolvidos", sem aceitar/rejeitar.

**Independent Test**: spec US2.

- [ ] T033 [P] [US2] Testes em `src-tauri/src/review/mod.rs` para `set_thread_status`: grava `Thread.statusBy` e `Thread.statusAt` ([data-model.md](data-model.md)); responder não muda `Thread.status`; `not_found`; `review_read_only`
- [ ] T034 [P] [US2] Testes em `src/review/panel.test.ts`: aba "Resolvidos" da mais recente para a mais antiga com quem/quando; "Resolver" tira a marca e a conversa de "Abertos"; "Reabrir" devolve; nenhum botão "Aceitar"/"Rejeitar" em cartão de comentário (D-03, FR-016)
- [ ] T035 [US2] Implementar `set_thread_status` em `src-tauri/src/review/mod.rs` e registrar em `src-tauri/src/commands.rs`, até T033 passar
- [ ] T036 [US2] Implementar aba "Resolvidos" e ações em `src/review/panel.ts`, até T034 passar
- [ ] T037 [US2] Estender `e2e/review.e2e.ts`: resolver, conferir em "Resolvidos", reabrir

**Checkpoint**: US1 + US2 entregam o ciclo básico de revisão.

---

## Phase 5: User Story 3 - Apagar mensagens e excluir conversas com segurança (Priority: P2)

**Goal**: regras por autor e dono, confirmação, recusa no núcleo, reancorar trecho removido.

**Independent Test**: spec US3.

- [ ] T038 [P] [US3] Testes de segurança em `src-tauri/tests/review_permissions.rs` (SC-006, FR-017 a FR-020; tabela de permissões de [data-model.md](data-model.md#permissões-fr-017-a-fr-020-docsrequisitosmd-1)): para cada ação proibida (apagar mensagem alheia, excluir conversa alheia sem ser dono, `delete_resolved` sem ser dono) → `forbidden` e arquivo inalterado byte a byte; `delete_thread`/`delete_resolved` sem `confirm: true` → `confirmation_required`; `review.json` editado à mão trocando `Thread.openedBy` não dá permissão ao nome local diferente
- [ ] T039 [P] [US3] Testes em `src-tauri/src/review/mod.rs`: mensagem apagada mantém `Message.id`, `Message.deleted = true`, `Message.body` e `Message.mentions` vazios; apagar a primeira mensagem com respostas mantém conversa e âncora; `delete_resolved` não toca abertas; `can.delete` calculado certo em `Thread` e `Message`
- [ ] T040 [P] [US3] Testes para `claim_ownership` em `src-tauri/src/review/identity.rs` (FR-027): sem dono → registra `ProjectRecord.owner.name`; com dono → `owner_already_set`; sem nome → `no_display_name`
- [ ] T041 [P] [US3] Testes para `reanchor_thread` em `src-tauri/src/review/mod.rs` (FR-009): só conversas existentes; nova âncora; limpa `detached`; `out_of_scope`/`bad_range`
- [ ] T042 [US3] Implementar `delete_message`, `delete_thread`, `delete_resolved`, `claim_ownership`, `reanchor_thread` em `src-tauri/src/review/` e registrar em `src-tauri/src/commands.rs`, até T038–T041 passarem
- [ ] T043 [P] [US3] Testes em `src/review/panel.test.ts`, depois implementação em `src/review/panel.ts`: ações só aparecem com `can.delete`; diálogo de confirmação (cancelar não chama o núcleo); "Excluir todos os resolvidos" só para o dono; pergunta "Você é o dono?" na primeira ação de dono sem dono registrado; "Reancorar na seleção" em conversa `detached`
- [ ] T044 [US3] Estender `e2e/review.e2e.ts`: trocar o nome, conferir ações ocultas; excluir com confirmação

**Checkpoint**: conversas confiáveis contra engano na interface.

---

## Phase 6: User Story 4 - Ver onde há comentários pelo outline (Priority: P3)

**Goal**: contagem de conversas abertas por seção.

**Independent Test**: spec US4.

- [ ] T045 [P] [US4] Testes em `src-tauri/src/review/outline.rs` ([research.md R10](research.md#r10-outline-com-contagem-fr-013)): conta só abertas não `detached`; conversa que atravessa seções conta onde começa; títulos aninhados; arquivo incluído conta no próprio arquivo
- [ ] T046 [P] [US4] Testes em `src/project/outline.test.ts`: `OutlineItem.openComments` > 0 mostra número; 0 não mostra; atualiza com `ReviewUpdate` sem recarregar
- [ ] T047 [US4] Implementar `src-tauri/src/review/outline.rs` e preencher `openComments` no comando `outline` (`src-tauri/src/compile/sync.rs` ou onde o `outline` da 003 é montado), até T045 passar
- [ ] T048 [US4] Implementar a contagem em `src/project/outline.ts`, até T046 passar

**Checkpoint**: outline mostra onde está a revisão.

---

## Phase 7: User Story 5 - Exportar o PDF com comentários na margem (Priority: P3)

**Goal**: PDF com conversas abertas na margem, sem alterar `.typ`, `review.json` nem PDF
final.

**Independent Test**: spec US5.

- [ ] T049 [P] [US5] Testes em `src-tauri/tests/review_pdf.rs` (SC-007, FR-028, FR-029): com a fixture, o PDF gerado tem uma nota por conversa aberta na página do trecho (extrair texto das notas do documento sintético antes do PDF), nenhuma resolvida ou `detached`; `.typ`, `review.json` e PDF final byte a byte iguais antes/depois; com erro de compilação → `compile_errors`
- [ ] T050 [P] [US5] Teste de segurança em `src-tauri/tests/review_pdf.rs`: mensagens com sintaxe Typst (`#import`, `#read("/etc/passwd")`, `]`, `$`), caracteres de controle e 10.000 caracteres aparecem literais e não executam nem quebram a compilação (FR-014); documento sintético não lê arquivo do disco
- [ ] T051 [US5] Implementar `src-tauri/src/review/pdf.rs` ([research.md R9](research.md), ou fallback SVG conforme T001): passagem 1 reaproveita `src-tauri/src/compile/export.rs` e o mapeamento de `jump_from_cursor`; passagem 2 em World sintético só com bytes em memória; notas como `str` escapado; empilhamento e página de continuação; até T049 e T050 passarem
- [ ] T052 [US5] Registrar `export_pdf_with_comments_dialog` em `src-tauri/src/commands.rs` e capability
- [ ] T053 [P] [US5] Testes em `src/export.test.ts`, depois ação "Exportar PDF com comentários" em `src/export.ts`, com a mesma lista de erros do PDF final

**Checkpoint**: todas as histórias funcionais.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [ ] T054 [P] Teste de desempenho em `src-tauri/tests/perf_review.rs` (`--release`, SC-005): 500 conversas em artigo de 40 páginas; criar/resolver/reabrir p95 ≤ 200 ms até o `ReviewUpdate`; prévia continua < 100 ms após edição
- [ ] T055 [P] Acessibilidade em `src/review/panel.test.ts`: painel e abas navegáveis por teclado, `aria-selected`, foco volta ao editor após enviar, nomes acessíveis dos botões
- [ ] T056 [P] Atualizar `docs/requisitos.md` (RF-16, RF-17, RF-43 e RF-04 com issue/US da 004), `docs/release-plan.md` (R2: épico, specs 004/005, Q-03/Q-05/Q-07 resolvidas) e `docs/arquitetura.md` §4 (arquivos `.prelo/crdt/`, PDF com comentários em 2 passagens) via agente `doc-tecnica`
- [ ] T057 Rodar `npm run verify` completo e o [quickstart.md](quickstart.md) no macOS; registrar evidência no épico
- [ ] T058 Revisão do agente `qa` (modo validar): princípios I–VI, cobertura de SC-001 a SC-008; veredito APROVADO/REPROVADO no épico

---

## Dependencies & Execution Order

### Phase Dependencies

- **Portão**: `1.0.0` em produção → qualquer tarefa
- **Setup (1)** → **Foundational (2)** → histórias
- **US1 (3)** é base de US2–US5 (comandos e painel)
- **US2 (4)**, **US3 (5)**, **US4 (6)** dependem só de US1; podem correr em paralelo
- **US5 (7)** depende de US1 e de T001 (spike); usa estados de US2 (só abertas)
- **Polish (8)** depois das histórias desejadas

### Within Each Story

Testes (falhando) → núcleo → registro IPC → interface → e2e.

### Parallel Opportunities

- T004 com T002/T003
- T005–T011 (testes da fundação) em paralelo; T018 e T019 em paralelo com T012–T017
- Em US1: T020–T026 em paralelo
- Depois de US1: US2, US3, US4 em paralelo por pessoas/agentes diferentes
- T054–T056 em paralelo

## Parallel Example: User Story 1

```text
Testes juntos: T020 (mod.rs), T021 (review_anchor.rs), T022 (review_crash.rs),
T023 (review_typ_pure.rs), T024 (marks.test.ts), T025 (panel.test.ts), T026 (composer.test.ts)
Depois: T027 (núcleo) em paralelo com T028/T029/T030 (interface, contra o contrato)
```

## Implementation Strategy

### MVP First (US1)

1. Portão da `1.0.0` cumprido
2. Setup + Foundational
3. US1 → validar sozinha (quickstart passos 1–5 e 9)

### Incremental Delivery

US1 → US2 (ciclo de revisão completo) → US3 (regras) → US4 (outline) → US5 (PDF). A `1.1.0`
sai com a 004 inteira **e** a 005 (git); se a 005 atrasar, a 004 pode sair numa `1.1.0` só com
comentários, por decisão do dono.

## Notes

- Toda ação proibida tem teste de recusa no núcleo (SC-006), não só na interface.
- Nada desta feature escreve dentro de `.typ` (T023 guarda isso).
- Commits por tarefa ou grupo lógico; push só com `npm run verify -- --quick` verde.
