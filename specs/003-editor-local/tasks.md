---

description: "Tasks for 003-editor-local"
---

# Tasks: Editor local (fase 1)

**Input**: Design documents from `/specs/003-editor-local/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: obrigatórios (Princípio VI, TDD). Em cada fase, as tarefas de teste vêm antes da
implementação, MUST falhar pelo motivo certo antes de a implementação começar e MUST ser
commitadas antes (ou junto) do código que cobrem. Testes de segurança (escape da raiz, entrada
malformada, plugin sem confirmação, pacote malicioso ou adulterado, IPC com caminho fora do
projeto) seguem [contracts/file-access.md](contracts/file-access.md). O agente `qa` (modo
projetar) pode executar as tarefas de teste; o modo validar roda ao fim de cada história.

**Organization**: por história de usuário (spec.md), em ordem de prioridade.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: pode rodar em paralelo (arquivos diferentes, sem dependência pendente)
- **[Story]**: história do spec.md (US1…US6)

## Path Conventions

Layout do [plan.md](plan.md#source-code-repository-root): interface em `src/`, núcleo em
`src-tauri/src/`, testes Rust de integração em `src-tauri/tests/`, ponta a ponta em `e2e/`,
scripts em `scripts/`, auditoria em `security/`.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: dependências, fontes e apoio de testes; nenhum comportamento novo.

- [X] T001 Escrever testes em `scripts/audit.test.ts`, no padrão do teste "MPL-2.0 não está na lista geral (D-11)": `src-tauri/deny.toml` tem `[[licenses.exceptions]]` para a crate `typst-assets` com exatamente `CC-BY-4.0` e `W3C-20150513` (D-16); nenhuma outra crate tem exceção com essas licenças; `CC-BY-4.0` e `W3C-20150513` não estão em `security/licenses-allow.txt` nem em `[licenses].allow`. Confirmar que falham
- [X] T002 Criar a exceção em `src-tauri/deny.toml` (`[[licenses.exceptions]]`, crate `typst-assets`, `allow = ["CC-BY-4.0", "W3C-20150513"]`, comentário citando D-16) até T001 passar; nenhuma tarefa que adicione o `typst` roda antes desta
- [X] T003 Adicionar dependências Rust de [research.md R12](research.md#r12-dependências-novas-e-licenças) em `src-tauri/Cargo.toml`: `typst`, `typst-svg`, `typst-pdf`, `typst-ide` `=0.15.1`; `tinymist-world`, `tinymist-vfs`, `tinymist-package` `=0.15.0` (versões exatas; atualizar só com a suíte verde; sem `tinymist-query` nem `typlite`, D-17) sem as features `system`, `fonts`, `http-registry`; `ureq` 3.5 com `rustls` (provedor `ring`) e `rustls-native-certs` (certificados do sistema passados ao `ureq`), **sem** raízes `webpki-roots`/`webpki-root-certs` nem `rustls-platform-verifier`; `flate2`, `tar`, `sha2`, `tempfile`, `notify` 8, `dirs` 6, `tauri-plugin-dialog` 2; `tempfile` também em dev-dependencies; feature `equivalence` vazia
- [X] T004 [P] Adicionar `@codemirror/state`, `@codemirror/view`, `@codemirror/commands`, `@codemirror/language`, `@codemirror/autocomplete`, `@codemirror/lint`, `@codemirror/search` (6.x, versões exatas) em `package.json`
- [X] T005 [P] Adicionar as fontes Libertinus 7.051 (Serif, Sans, Math, Mono) em `src-tauri/fonts/`, com `OFL.txt`, aviso de copyright e `src-tauri/fonts/licenses.toml` (arquivo → família, licença, copyright) conforme [data-model.md](data-model.md#fontes) `FontLicense`
- [X] T006 Escrever testes em `scripts/audit.test.ts` para a verificação de licenças de arquivos embutidos: arquivo de fonte em `src-tauri/fonts/` sem entrada em `licenses.toml` falha; licença fora de `security/licenses-allow.txt` + `OFL-1.1` falha; `OFL-1.1` em crate (não fonte) continua proibida; os dados embutidos do `typst-assets` (ICC, HTML, MathML, Foxit) aparecem listados com suas licenças (D-16). Confirmar que falham
- [X] T007 Implementar a verificação em `scripts/audit.mjs` até T006 passar (D-15)
- [X] T008 Rodar `npm run audit` com as dependências de T003/T004; se `cargo-deny` acusar licença fora da lista, ajustar features (ex.: excluir `webpki-roots`) em `src-tauri/Cargo.toml` ou registrar exceção nomeada em `src-tauri/deny.toml` só se coberta por D-11; registrar as exceções de vulnerabilidade RUSTSEC-2026-0194 e RUSTSEC-2026-0195 (`quick-xml` 0.38.4) aprovadas pelo dono em `security/vuln-exceptions.toml` e `[advisories].ignore` (research R14); registrar o resultado em `specs/003-editor-local/research.md` R12
- [X] T009 [P] Criar apoio de testes em `src-tauri/tests/support/mod.rs`: projeto temporário a partir de fixture, criação de link simbólico (Unix e Windows), FIFO (Unix), gerador de `.tar.gz` com entradas arbitrárias (`..`, absoluto, link, link físico, dispositivo) e um `PathAccessModel` instrumentado que registra toda abertura de arquivo
- [X] T010 [P] Criar fixtures versionadas em `src-tauri/tests/fixtures/`: `artigo-basico/` (vários `.typ` com `#include`, seções, ~3 páginas), `bibliografia/` (`.bib`, `.yml`, `abnt.csl`), `imagens/` (PNG, JPEG, GIF, SVG, PDF, WebP), `artigo-40p/` (40 páginas, gerado por script em `src-tauri/tests/fixtures/gen_40p.typ`), `fontes/` (fonte OFL em `fonts/`), `plugin/` (`.wasm` mínimo de teste)

**Checkpoint**: `npm run verify` verde com as dependências novas, sem uso delas.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: regra de escopo de arquivo, registro do projeto, World e agenda de compilação; toda
história depende disso. A regra de escopo vem primeiro porque nenhuma leitura de projeto pode
existir sem ela (RF-08).

### Testes (escrever e ver falhar antes da implementação)

- [X] T011 [P] Testes unitários de caminho em `src-tauri/src/fs/scope.rs` (módulo `tests`): `/` inicial relativo à raiz; `../utils.typ` a partir de `capitulos/` aceito; `..` além da raiz recusado; caminho vazio e com `\0` recusados
- [X] T012 [P] Testes de segurança de escape em `src-tauri/tests/security_file_access.rs` (FR-021, FR-022, FR-030, SC-003): `#image("../../fora.png")`, `#read("/../fora.txt")`, `#include` além da raiz, `bibliography`/`.csl`/`json`/`csv` fora da raiz, link simbólico versionado para fora, link simbólico interno (aceito), FIFO na raiz (recusado sem bloquear), junção no Windows (`cfg(windows)`), troca de link entre conferência e leitura, arquivo > 256 MB; cada recusa com o diagnóstico de [file-access.md](contracts/file-access.md#recusas-e-diagnóstico) citando o caminho pedido e **zero** aberturas fora da raiz no modelo instrumentado; parte de documento (compilação pelo World) entregue com T020
- [X] T013 [P] Testes do registro em `src-tauri/src/project/record.rs` (módulo `tests`) cobrindo [data-model.md](data-model.md#projectrecord-preloprojectjson) `ProjectRecord.schema`, `ProjectRecord.compiler.typst`, `ProjectRecord.main` e o limite de tamanho: JSON inválido, > 1 MB, `schema` desconhecido, `main` absoluto/com `..` → aviso `project_json_invalid`, padrões e nenhuma regravação; campos desconhecidos preservados ao regravar
- [X] T014 [P] Testes de agenda em `src-tauri/src/compile/scheduler.rs` (módulo `tests`) com World falso: fila de 1 (edição nova substitui a pendente); resultado mais antigo que o último entregue é descartado ([data-model.md](data-model.md#compilation) `Compilation.compileId`); compilação > 10 s vira `Compilation.status = Abandonada` com diagnóstico e a próxima edição compila numa thread nova (FR-015)
- [X] T015 [P] Testes de segurança do IPC em `src-tauri/tests/security_ipc.rs` (mock runtime): todo comando de [contracts/ipc.md](contracts/ipc.md) que recebe `path` recusa absoluto, `..` além da raiz e link para fora com `path_outside_project`; argumentos malformados recusados sem derrubar o núcleo; comando desconhecido recusado
- [X] T016 [P] Atualizar `src-tauri/tests/security_config.rs`: CSP exatamente a de [contracts/ipc.md](contracts/ipc.md#permissões-da-janela); capabilities sem `fs`, `dialog`, `http`, `shell`, `opener`

### Implementação

- [X] T017 Implementar `src-tauri/src/fs/scope.rs` conforme [file-access.md](contracts/file-access.md#passos-nesta-ordem) passos 2–7 (canonicalização, conferência de prefixo, abertura `O_NONBLOCK`/`O_NOFOLLOW`, conferência no descritor, limite de 256 MB) até T011 e T012 (parte de escape) passarem
- [X] T018 Implementar `src-tauri/src/fs/access_model.rs`: `PathAccessModel` do Prelo usando `scope.rs`, overlay de arquivos abertos (passo 1) e cache por conteúdo sha256 (passo 9, RF-51); erros mapeados para `Diagnostic.kind = access` ([data-model.md](data-model.md#diagnostic))
- [X] T019 Implementar `src-tauri/src/project/record.rs` com [data-model.md](data-model.md#projectrecord-preloprojectjson) `ProjectRecord.schema`, `ProjectRecord.compiler.typst`, `ProjectRecord.main` e o limite de 1 MB, até T013 passar
- [X] T020 Implementar `src-tauri/src/world/mod.rs` e `src-tauri/src/world/fonts.rs` (só fontes embutidas de `src-tauri/fonts/` nesta etapa; nenhuma fonte do sistema): World do `tinymist-world` montado com o `PathAccessModel`; padrões de fonte de matemática e `raw` definidos no World (research R6) até T012 passar por inteiro (T012 completa: casos de escape também no nível de documento, compilando `.typ` pelo World com `RecordingAccess`)
- [X] T021 Implementar `src-tauri/src/compile/scheduler.rs` com [data-model.md](data-model.md#compilation) `Compilation.compileId` e `Compilation.status`, até T014 passar
- [X] T022 Implementar `src-tauri/src/commands.rs` e registro em `src-tauri/src/lib.rs` (validação de `path` comum a todos os comandos), atualizar `src-tauri/tauri.conf.json` (CSP) e `src-tauri/capabilities/default.json` até T015 e T016 passarem (CSP já entregue com T016 por decisão do dono em 2026-11-21)
- [X] T023 [P] Criar `src/ipc.ts` com wrappers tipados dos comandos e eventos de [contracts/ipc.md](contracts/ipc.md) e testes em `src/ipc.test.ts` (códigos de erro → mensagens em português)

**Checkpoint**: o núcleo compila um projeto de fixture em teste, recusa todo acesso fora da raiz
e nenhum comando aceita caminho fora do projeto.

---

## Phase 3: User Story 1 - Escrever com prévia ao vivo (Priority: P1) 🎯 MVP

**Goal**: abrir um projeto existente, editar com realce, autocompletar e diagnósticos, ver a
prévia ao lado sem `typst` instalado, ver a versão do compilador e não perder trabalho.

**Independent Test**: sem `typst` no PATH, abrir `fixtures/artigo-basico/`, editar, ver a
prévia mudar, introduzir erro e ver o diagnóstico na linha certa; ver a versão em "Sobre".

### Testes para User Story 1

- [X] T024 [P] [US1] Testes de compilação em `src-tauri/tests/compile_basic.rs`: abrir projeto compila o arquivo principal; edição muda a prévia; erro gera diagnóstico com arquivo e intervalo e a prévia mantém a última `Ok` (FR-013, FR-014); vários `.typ` sem `main.typ` pedem escolha e gravam `ProjectRecord.main` ([data-model.md](data-model.md#projectrecord-preloprojectjson))
- [X] T025 [P] [US1] Testes de prévia em `src-tauri/src/compile/preview.rs` (módulo `tests`): hash por página; `PreviewUpdate.changed` só com páginas alteradas; `page_svg` com hash velho → `stale_page`
- [X] T026 [P] [US1] Testes de versão em `src-tauri/tests/compile_version.rs`: `compiler_info.embedded` igual à versão de `typst` no `Cargo.lock`; registro ausente é criado com a versão embutida sem tocar outros arquivos (FR-003); versão registrada diferente gera aviso, **não compila** (sem `PreviewUpdate`, diagnósticos de compilação nem exportação) e não regrava até `update_project_compiler_version`, que regrava e libera a compilação (FR-012, Restrição "Compilação embutida")
- [X] T027 [P] [US1] Testes de gravação em `src-tauri/src/fs/save.rs` (módulo `tests`) para [data-model.md](data-model.md#openfile) `OpenFile.dirty`, `OpenFile.version` e `OpenFile.text`: gravação atômica (falha simulada deixa o original intacto e mantém `dirty`), salvamento automático 2 s após a última edição, gravação ao fechar, `edit_file` com versão antiga → `version_mismatch`, arquivo não UTF-8 → `not_utf8` (FR-007, FR-009)
- [X] T028 [P] [US1] Testes de mudança externa em `src-tauri/src/fs/watch.rs` (módulo `tests`) para `OpenFile.lastSavedHash`: própria gravação ignorada; mudança externa sem alteração local recarrega e recompila; com alteração local emite `external_change_conflict` e nada é descartado até `resolve_external_change` (FR-008)
- [X] T029 [P] [US1] Testes do serviço de linguagem em `src-tauri/tests/lang_service.rs` (D-17): autocompletar de função, variável e rótulo pelo `typst-ide`, incluindo texto ainda não salvo (mesmo World da prévia, FR-011); *hover* de função; *tokens* de realce gerados por `typst-syntax` (`LinkedNode` + `highlight`) para markup, código e matemática (FR-006)
- [X] T030 [P] [US1] Testes de interface em `src/editor/editor.test.ts`, `src/editor/diagnostics.test.ts`, `src/preview/preview.test.ts`, `src/project/tree.test.ts`: mudanças enviadas como `{from,to,insert}` com versão; ⌘S/Ctrl+S chama `save_file`; diagnósticos marcados no intervalo; páginas como `<img>` com blob URL (nunca SVG no DOM); virtualização (visíveis ± 2); faixa de erro; árvore abre `.typ`, `.bib`, `.yml`
- [X] T090 [P] [US1] Testes de interface do zoom da prévia em `src/preview/zoom.test.ts` e `src/preview/preview.test.ts` (FR-013a, emenda de 2026-11-22): barra com "−", "+", porcentagem e "Ajustar à largura" (padrão); degraus 50–300 % com "−"/"+" desativados nos limites; ⌘/Ctrl + `+`/`-`/`0` só com o foco na prévia; ⌘/Ctrl + roda e pinça; ponto sob o centro (ou o ponteiro) mantido; nenhum `page_svg` novo só por zoom; visíveis ± 2 em qualquer zoom; nível lembrado por projeto (`localStorage`, falha ignorada); `aria-label` e região viva. Confirmar que falham
- [X] T092 [P] [US1] Testes de interface dos painéis ajustáveis em `src/panes.test.ts`, `src/workspace.test.ts` e `src/layout-check.test.ts` (FR-013b, emenda de 2026-11-22): botão de recolher em cada painel com `aria-label`/`aria-expanded`; trilho de ~28 px com expandir e o nome; editor e prévia nunca os dois recolhidos; divisórias por ponteiro e teclado (`role="separator"`, `aria-valuenow`, setas), mínimos e clique duplo para 200 px | 1,25fr | 1fr; Editor · Ambos · Prévia e botão da árvore; ⌘/Ctrl + 1/2/3 e ⌘/Ctrl + `\`; rolagem preservada; layout por projeto; autoteste depois de recolher e expandir. Confirmar que falham
- [X] T031 [US1] Teste ponta a ponta em `e2e/editor.e2e.ts` e no autoteste (`src/self-test.ts`, `src-tauri/src/self_test.rs`): com `typst` fora do PATH, abrir `fixtures/artigo-basico/`, editar, ver a prévia mudar e um diagnóstico aparecer (SC-006)
- [X] T088 [P] [US1] Testes de criação de arquivo e pasta no núcleo (emenda de 2026-11-22; FR-034, FR-035, cenário 10 de US1): em `src-tauri/src/fs/create.rs` (módulo `tests`) — arquivo vazio e pasta criados na raiz e em subpasta existente, `..` dentro da raiz aceito, nome existente (arquivo, pasta, link, link quebrado) → `already_exists` sem truncar, nomes inválidos (vazio, `.`, `..`, terminado em `/`, `\0`, controle, `\ : * ? " < > |`, espaço/ponto no fim, `CON`/`nul.typ`, > 255 bytes, oculto `.x` e dentro de `.prelo/`) → `invalid_name`, mãe inexistente ou arquivo → `not_found`, mãe que é link para fora → `path_outside_project` sem criar nada fora; em `src-tauri/src/commands.rs` (módulo `tests`) — o novo arquivo aparece em `list_tree`, abre vazio por `open_file` e o aviso do observador para ele é `Ignored` (sem `external_change_conflict`); em `src-tauri/tests/security_ipc.rs` — `create_file` e `create_dir` entram em `path_commands` (absoluto, `..` além da raiz e link para fora → `path_outside_project`; malformados recusados; sem projeto → `no_project`)
- [X] T085 [P] [US1] Testes de interface da criação (FR-034, cenário 10 de US1) em `src/project/tree.test.ts` e `src/ipc.test.ts`: botões "Novo arquivo" e "Nova pasta" na árvore abrem um campo de nome na própria página (sem `window.prompt`), Enter chama `create_file`/`create_dir` com o caminho na pasta escolhida, Esc cancela sem chamar o núcleo, recusa do núcleo vira mensagem em português no campo, a árvore se atualiza e um `.typ` novo abre no editor; wrappers `createFile`/`createDir` e mensagens de `already_exists`/`invalid_name`

### Implementação para User Story 1

- [X] T032 [US1] Implementar `src-tauri/src/project/mod.rs`: `open_project_dialog` (diálogo nativo pelo núcleo), `list_tree`, `choose_main_file`, criação do registro ausente, `update_project_compiler_version` e o estado `VersaoDiferente` de [data-model.md](data-model.md#projectrecord-preloprojectjson) `ProjectRecord.compiler.typst`, com `ProjectRecord.main` validado, até T024 (parte de projeto) e T026 passarem
- [X] T033 [US1] Implementar `src-tauri/src/fs/save.rs` com [data-model.md](data-model.md#openfile) `OpenFile.text`, `OpenFile.version` e `OpenFile.dirty`, até T027 passar e ligar `save_file`, `open_file`, `edit_file` em `src-tauri/src/commands.rs`; fechar a janela espera a última gravação
- [X] T034 [US1] Implementar `src-tauri/src/fs/watch.rs` com `notify` e [data-model.md](data-model.md#openfile) `OpenFile.lastSavedHash`, até T028 passar; comando `resolve_external_change`
- [X] T035 [US1] Implementar `src-tauri/src/compile/preview.rs` (SVG por página com `typst-svg`, hash, `subscribe_preview` por `tauri::ipc::Channel`, `page_svg`) até T024 e T025 passarem
- [X] T036 [US1] Implementar `src-tauri/src/lang_service.rs` sobre `typst-ide` (`autocomplete`, `tooltip`) e `typst-syntax` (realce), sem `tinymist-query` (D-17), com os comandos `complete` e `semantic_tokens`, até T029 passar
- [X] T037 [P] [US1] Implementar `src/editor/editor.ts`, `src/editor/highlight.ts`, `src/editor/completion.ts`, `src/editor/diagnostics.ts` até a parte de editor de T030 passar
- [X] T038 [P] [US1] Implementar `src/preview/preview.ts` e `src/project/tree.ts` até a parte de prévia e árvore de T030 passar
- [X] T039 [US1] Implementar `src/about.ts` (versão embutida e registrada, aviso de diferença, licenças das fontes e avisos de atribuição do `typst-assets`, D-15, D-16) e o diálogo de conflito externo em `src/project/prompts.ts`; montar o layout em `src/main.ts` até T031 passar
- [X] T086 [US1] Implementar `src-tauri/src/fs/create.rs` (regras de [file-access.md](contracts/file-access.md#criação-de-arquivo-e-pasta-fr-034-fr-035): mãe pela regra de escopo e conferida no descritor, `openat`/`mkdirat` com `O_EXCL`/`O_NOFOLLOW`, nunca sobrescreve) e os comandos `create_file` e `create_dir` em `src-tauri/src/commands.rs` e `src-tauri/src/lib.rs`, até T088 passar
- [X] T087 [US1] Implementar em `src/project/tree.ts` e `src/ipc.ts` os botões "Novo arquivo"/"Nova pasta", o campo de nome na árvore e as mensagens de erro, até T085 passar
- [X] T091 [US1] Implementar o zoom da prévia em `src/preview/zoom.ts` e `src/preview/preview.ts` (largura das páginas pelo nível, âncora de leitura, sem novo pedido de SVG), barra no estilo de `prototype/index.html` (`.pvbar` + `.seg`) em `src/workspace.css`, chave do projeto passada por `src/workspace.ts`, até T090 passar (FR-013a)
- [X] T093 [US1] Implementar os painéis ajustáveis em `src/panes.ts`, montar em `src/workspace.ts` (cabeçalho com botão em cada painel, controle na barra de cima) com o estilo de `prototype/index.html` em `src/workspace.css`, e estender `src/layout-check.ts`/`src/self-test.ts`, até T092 passar (FR-013b)
- [X] T084 [US1] Mudança externa sem alteração local chega ao editor (FR-008; bug visto no app real, #189): o núcleo emite `external_change_reloaded { path, version }` depois de recarregar (`src-tauri/src/lib.rs`, `src-tauri/src/fs/watch.rs`); `src/ipc.ts` expõe `onExternalChangeReloaded`; `src/workspace.ts` busca o texto com `open_file` e recarrega o editor aberto com esse caminho (`src/editor/editor.ts` `reload` troca só o trecho que mudou, fora do histórico); testes em `src-tauri/src/lib.rs` (evento só no recarregamento limpo; com alteração local, só `external_change_conflict`), `src/workspace.test.ts`, `src/editor/editor.test.ts` e no autoteste (`self_test_write_external`)

**Checkpoint**: US1 funciona sozinha; QA valida.

---

## Phase 4: User Story 2 - Projeto confinado à própria pasta (Priority: P1)

**Goal**: entrada malformada vira diagnóstico sem travar o app; autocompletar não lista nada
fora da raiz; plugins só rodam com confirmação; pacotes `@preview` são validados e o cache
adulterado é recusado.

**Independent Test**: abrir os projetos de segurança (escape, malformados, plugin, pacotes
maliciosos) e conferir diagnóstico, zero leitura fora da raiz, nenhuma execução não aprovada e
app respondendo.

### Testes para User Story 2

- [x] T040 [P] [US2] Testes de entrada malformada em `src-tauri/tests/security_malformed.rs` (FR-024, FR-030, SC-004): `.typ` inválido e não UTF-8, `.bib` com chave não fechada, `.yml` inválido, `.csl` com XML inválido, imagem com conteúdo inválido, fonte inválida em `fonts/`, `.prelo/project.json` malformado, recursão profunda, laço infinito e plugin aprovado que não retorna; cada um gera diagnóstico com arquivo e linha quando houver, e o núcleo responde a uma edição em até 1 s depois
- [x] T041 [P] [US2] Teste de autocompletar de caminhos em `src-tauri/tests/security_file_access.rs`: com link simbólico para fora e `..` digitado, nenhuma sugestão fora da raiz (RF-08)
- [x] T042 [P] [US2] Testes de plugins em `src-tauri/tests/security_plugins.rs` para [data-model.md](data-model.md#pluginapproval--aprovação-de-plugin-dados-locais-do-app) `PluginApproval.projectRoot`, `PluginApproval.sha256` e `PluginApproval.decision` (FR-019): sem aprovação o plugin não roda (contador de execução zero) e há diagnóstico `kind = plugin` + evento `plugin_approval_needed`; recusado não roda; aprovado roda; conteúdo alterado pede de novo; mesma aprovação não vale em outro projeto; arquivo de aprovação dentro de `.prelo/` é ignorado; `plugin-approvals.json` corrompido é tratado como vazio; plugin de pacote segue a mesma regra
- [x] T043 [P] [US2] Testes de pacotes em `src-tauri/tests/security_packages.rs` com *fetcher* injetado (sem rede real), para [data-model.md](data-model.md#packagecacheentry--cache-de-pacotes-dados-locais-do-app) `PackageCacheEntry.spec`, `PackageCacheEntry.sha256` e `PackageCacheEntry.state` (FR-017, FR-018): `.tar.gz` malformado, entrada com `..`, caminho absoluto, link simbólico, link físico, dispositivo, acima de 50 MB/200 MB/5 000 arquivos → pacote inteiro recusado e nada gravado fora de `<cache>/packages/`; pacote alterado no cache depois do download → `PacoteAdulterado` e `redownload_package` resolve; ausente sem rede → `PacoteIndisponivel`; `@local/x:1.0.0` → `NamespaceNaoSuportado`; arquivo de pacote fora da pasta do pacote → `ForaDoProjeto`; URL pedida contém só nome e versão
- [x] T082 [P] [US2] Testes de segurança em `src-tauri/tests/security_malformed.rs` para RUSTSEC-2026-0194/0195 (`quick-xml` 0.38.4, research R14): `.csl` acima de 1 MiB é recusado com diagnóstico "estilo grande demais" sem ser entregue ao compilador; `.csl` dentro do limite com uma *tag* de milhares de atributos distintos e outro com milhares de declarações `xmlns:*` terminam em diagnóstico ou abandono por tempo (10 s), e o núcleo responde a uma edição em até 1 s depois
- [x] T044 [P] [US2] Testes de interface em `src/project/prompts.test.ts`: pedido de confirmação de plugin mostra caminho, origem e tamanho; aprovar/recusar chama `decide_plugin`; ação "Baixar de novo" chama `redownload_package`

### Implementação para User Story 2

- [x] T045 [US2] Mapear erros do compilador e de leitura para `Diagnostic` com `kind` e linha em `src-tauri/src/compile/scheduler.rs` e `src-tauri/src/fs/access_model.rs` até T040 passar
- [x] T046 [US2] Filtrar sugestões de caminho por `scope.rs` em `src-tauri/src/lang_service.rs` até T041 passar
- [x] T047 [US2] Implementar `src-tauri/src/world/plugins.rs` com [data-model.md](data-model.md#pluginapproval--aprovação-de-plugin-dados-locais-do-app) `PluginApproval.projectRoot`, `PluginApproval.sha256` e `PluginApproval.decision` (armazenamento em dados locais do app) e o bloqueio por assinatura `\0asm` em `src-tauri/src/fs/access_model.rs`; comando `decide_plugin` até T042 passar
- [x] T048 [US2] Implementar `src-tauri/src/world/packages.rs` com [data-model.md](data-model.md#packagecacheentry--cache-de-pacotes-dados-locais-do-app) `PackageCacheEntry.spec`, `PackageCacheEntry.sha256` e `PackageCacheEntry.state`: `PackageRegistry`, download com `ureq` (TLS, raízes do sistema), extração validada em pasta temporária, índice sha256 e `rename` atômico, verificação por sessão; comando `redownload_package` até T043 passar
- [x] T083 [US2] Implementar o limite de 1 MiB para `.csl` em `src-tauri/src/fs/scope.rs`/`src-tauri/src/fs/access_model.rs` (recusa `GrandeDemais` com a mensagem de [file-access.md](contracts/file-access.md#recusas-e-diagnóstico)) até T082 passar
- [x] T049 [US2] Implementar o pedido de confirmação de plugin e a ação "Baixar de novo" em `src/project/prompts.ts` até T044 passar

**Checkpoint**: todas as categorias de [file-access.md](contracts/file-access.md#recusas-e-diagnóstico) têm teste verde; QA valida US2.

---

## Phase 5: User Story 3 - Bibliografia e figuras do projeto (Priority: P2)

**Goal**: `.bib`, `.yml`, `.csl`, os formatos de imagem, fontes de `fonts/` e pacotes em cache
funcionam na prévia; imagens que não mudaram não são decodificadas de novo.

**Independent Test**: abrir `fixtures/bibliografia/`, `fixtures/imagens/` e `fixtures/fontes/`;
conferir citações, figuras e fonte na prévia; contar decodificações ao editar texto.

### Testes para User Story 3

- [ ] T050 [P] [US3] Testes em `src-tauri/tests/compile_bibliography.rs`: `.bib`, `.yml`, vários arquivos, `.csl` do projeto e estilo embutido produzem citações e lista; editar o `.bib` e salvar recompila (FR-025)
- [ ] T051 [P] [US3] Testes em `src-tauri/tests/compile_images.rs`: PNG, JPEG, GIF (estática), SVG, PDF e WebP aparecem; contador de decodificações = 1 por imagem ao longo de 20 edições de texto (FR-026, FR-027, FR-032)
- [ ] T052 [P] [US3] Testes em `src-tauri/tests/compile_fonts.rs`: fonte de `fonts/` é usada; fonte só instalada no sistema não é usada (aviso com o nome); `compiler_info.fonts` lista as Libertinus com OFL-1.1 (FR-016)
- [ ] T053 [P] [US3] Teste em `src-tauri/tests/compile_packages.rs` com *fetcher* local: primeira compilação baixa o pacote; segunda, sem rede, usa o cache e gera a mesma prévia (US3 cenário 9)
- [ ] T054 [P] [US3] Testes de cópia para o projeto em `src-tauri/tests/compile_copy_into_project.rs` (D-13, FR-023): `.typ` apontando para fora emite `copy_into_project_offer` só com o caminho pedido; `copy_into_project` copia só após escolha e a compilação passa; sem confirmação nada é copiado
- [ ] T055 [P] [US3] Teste de protocolo `prelo-asset` em `src-tauri/src/compile/preview.rs` (módulo `tests`): `data:` URLs de imagem trocadas por `prelo-asset://<sha256>`; hash desconhecido → 404; editar texto na página da imagem não reenvia os bytes

### Implementação para User Story 3

- [ ] T056 [US3] Completar `src-tauri/src/world/fonts.rs` com a leitura de `fonts/**/*.{ttf,otf,ttc,otc}` pelo `PathAccessModel` até T052 passar
- [ ] T057 [US3] Garantir cache de bytes e imagens decodificadas por conteúdo em `src-tauri/src/fs/access_model.rs` até T051 passar
- [ ] T058 [US3] Implementar o protocolo `prelo-asset` em `src-tauri/src/compile/preview.rs` e `src-tauri/src/lib.rs` até T055 passar
- [ ] T059 [US3] Implementar `copy_into_project` e a oferta em `src-tauri/src/commands.rs` e `src/project/prompts.ts` até T054 passar
- [ ] T060 [US3] Ligar à compilação os estilos `.csl` do projeto, a recompilação ao salvar `.bib`/`.yml` (`src-tauri/src/fs/save.rs`) e a resolução offline pelo cache (`src-tauri/src/world/packages.rs`) até T050 e T053 passarem

**Checkpoint**: US3 funciona com US1; QA valida.

---

## Phase 6: User Story 4 - Navegar entre código e prévia (Priority: P2)

**Goal**: clique na prévia leva ao código; "localizar na prévia" leva ao trecho; estrutura de
seções navega o artigo.

**Independent Test**: em `fixtures/artigo-basico/`, clicar em pontos conhecidos e conferir a
linha; localizar a partir de linhas conhecidas; clicar em cada seção.

### Testes para User Story 4

- [ ] T061 [P] [US4] Testes em `src-tauri/tests/compile_sync.rs`: `jump_from_click` em parágrafo do principal e de `.typ` incluído devolve arquivo e linha corretos; ponto sem origem no projeto (numeração, pacote) devolve `null`; `jump_from_cursor` devolve página e posição; `outline` em ordem e com nível, incluindo títulos de arquivo incluído ([data-model.md](data-model.md#outlineitem) `OutlineItem.level`) (FR-005, FR-020, SC-007)
- [ ] T062 [P] [US4] Testes de interface em `src/preview/sync.test.ts` e `src/project/outline.test.ts`: clique converte pixels em pontos e abre o arquivo na linha; `null` mostra aviso e não move o cursor; localizar rola e destaca; clicar na seção move editor e prévia

### Implementação para User Story 4

- [ ] T063 [US4] Implementar `src-tauri/src/compile/sync.rs` (`typst_ide::jump_from_click`, `jump_from_cursor`, outline pelo introspector) e os comandos `jump_from_click`, `jump_from_cursor`, `outline` com [data-model.md](data-model.md#outlineitem) `OutlineItem.level`, até T061 passar
- [ ] T064 [US4] Implementar `src/preview/sync.ts` e `src/project/outline.ts` até T062 passar

**Checkpoint**: US4 funciona com US1; QA valida.

---

## Phase 7: User Story 5 - Exportar o PDF (Priority: P2)

**Goal**: PDF da mesma compilação da prévia, idêntico ao do `typst` CLI com o mesmo conjunto de
fontes; recusa com erro de compilação.

**Independent Test**: exportar `fixtures/artigo-basico/` e `fixtures/bibliografia/`; comparar
com o `typst` CLI; tentar exportar com erro.

### Testes para User Story 5

- [x] T065 [P] [US5] Testes em `src-tauri/tests/compile_export.rs`: PDF com o mesmo número de páginas e tamanhos da prévia e gerado do mesmo `compileId` da última prévia `Ok` (FR-011); com erro de compilação → `compile_errors` com a lista; falha de gravação não deixa arquivo parcial (FR-028, FR-029)
- [x] T066 [P] [US5] Teste de equivalência em `src-tauri/tests/equivalence.rs` (feature `equivalence`, `PRELO_TYPST_CLI`): para `artigo-basico`, `bibliografia`, `imagens`, `fontes`, PDF do Prelo e do `typst` CLI 0.15.1 com `--ignore-system-fonts --ignore-embedded-fonts --font-path src-tauri/fonts --font-path <projeto>/fonts`, renderizados com `typst-render` e comparados página a página (FR-031, SC-005)

### Implementação para User Story 5

- [x] T067 [US5] Implementar `src-tauri/src/compile/export.rs` (`typst-pdf`, gravação atômica) e `export_pdf_dialog` até T065 passar
- [x] T068 [P] [US5] Escrever teste em `src/export.test.ts`: ação "Exportar PDF" chama `export_pdf_dialog`; `compile_errors` mostra a lista; desativada com versão diferente. Confirmar que falha
- [x] T069 [US5] Adicionar a ação "Exportar PDF" com lista de erros em `src/main.ts` até T068 passar
- [ ] T070 [US5] Adicionar job de equivalência ao CI em `.github/workflows/ci.yml` (instala `typst` 0.15.1 só nesse job) até T066 passar nos três sistemas

**Checkpoint**: US5 funciona com US1; QA valida.

---

## Phase 8: User Story 6 - Criar um projeto novo (Priority: P3)

**Goal**: projeto mínimo numa pasta vazia, que compila no Prelo e fora dele.

**Independent Test**: criar em pasta vazia, conferir arquivos e prévia; tentar em pasta com
arquivos.

### Testes para User Story 6

- [x] T071 [P] [US6] Testes em `src-tauri/tests/project_create.rs`: pasta vazia → `main.typ` mínimo e `.prelo/project.json` com `ProjectRecord.compiler.typst` e `ProjectRecord.main` ([data-model.md](data-model.md#projectrecord-preloprojectjson)); pasta com arquivos → `folder_not_empty` e nada escrito; o `main.typ` criado compila no Prelo e, com a feature `equivalence`, no `typst` CLI (FR-002, Princípio I)

### Implementação para User Story 6

- [x] T072 [P] [US6] Escrever teste em `src/project/tree.test.ts`: ação "Novo projeto" chama `create_project_dialog`; `folder_not_empty` mostra aviso. Confirmar que falha (feito em `src/workspace.test.ts`, onde mora a barra de ações)
- [x] T073 [US6] Implementar `create_project_dialog` em `src-tauri/src/project/mod.rs` gravando [data-model.md](data-model.md#projectrecord-preloprojectjson) `ProjectRecord.compiler.typst` e `ProjectRecord.main`, até T071 passar
- [x] T074 [US6] Adicionar a ação "Novo projeto" em `src/main.ts` até T072 passar (feito em `src/workspace.ts`; `main.ts` só monta a área de trabalho)

**Checkpoint**: todas as histórias funcionam; QA valida US6.

---

## Phase 9: Polish & Cross-Cutting Concerns

- [ ] T075 Escrever `src-tauri/tests/perf_preview.rs` (`--release`): p95 edição→`PreviewUpdate` em `artigo-40p` e em `artigo-40p` com imagens de 4000×3000 px e ≥ 10 MB; falha acima de 250 ms no CI e imprime o p95 (SC-001, SC-002)
- [ ] T076 Otimizar o necessário (compilação incremental, diff de páginas, `prelo-asset`) em `src-tauri/src/compile/` até T075 passar no CI e o p95 ficar < 100 ms no computador de referência, registrando a medição em `specs/003-editor-local/research.md`
- [ ] T077 [P] Teste de encerramento forçado em `scripts/self-test.mjs`: matar o processo 2 s após a última edição e reabrir; no máximo os últimos 2 s se perdem (SC-009)
- [ ] T078 [P] Teste de threads abandonadas em `src-tauri/tests/security_malformed.rs`: após 3 compilações abandonadas, o número de threads de compilação vivas fica limitado e o editor responde (research R9)
- [ ] T079 [P] Atualizar `README.md`: fontes embutidas e licenças, pasta `fonts/` e `typst compile --font-path fonts`, cache de pacotes e aprovações de plugin, limites (D-15, Princípio I)
- [ ] T080 Rodar o roteiro de [quickstart.md](quickstart.md) e `npm run verify` nos três sistemas; corrigir o que falhar
- [ ] T081 Validação do QA: cobertura de FR-001 a FR-029 por ao menos um teste que falhou antes (SC-010) e de todas as categorias de FR-030, conferida pelo histórico de commits

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: sem dependências.
- **Foundational (Phase 2)**: depende do Setup; bloqueia todas as histórias.
- **US1 (Phase 3)**: depende da Foundational. MVP.
- **US2 (Phase 4)**: depende da Foundational; usa a interface de US1 só em T044/T049 (prompts).
- **US3 (Phase 5)**: depende de US1 (prévia) e de US2 para T053 (pacotes) e T054 (oferta de cópia usa as recusas).
- **US4 (Phase 6)**: depende de US1.
- **US5 (Phase 7)**: depende de US1; T066 usa as fixtures de US3.
- **US6 (Phase 8)**: depende de US1.
- **Polish (Phase 9)**: depende das histórias desejadas.

### Within Each User Story

- Testes primeiro, falhando pelo motivo certo; depois núcleo; depois interface.
- Commit do teste antes ou junto da implementação que o faz passar.

### Parallel Opportunities

- Setup: T004, T005, T009, T010 juntos.
- Foundational: T011–T016 juntos (arquivos diferentes); T023 em paralelo com T017–T022.
- US1: T024–T030 juntos; T037 e T038 juntos; T088 e T085 juntos, depois T086 e T087.
- US2: T040–T044 e T082 juntos.
- US3: T050–T055 juntos.
- US4, US5 e US6 podem andar em paralelo depois de US1.

## Parallel Example: User Story 2

```text
T040 security_malformed.rs   T041 autocompletar de caminhos
T042 security_plugins.rs     T043 security_packages.rs
T044 prompts.test.ts
```

## Implementation Strategy

### MVP First (User Story 1 + 2)

1. Setup e Foundational (a regra de escopo já entra aqui).
2. US1: editor e prévia.
3. US2: completa o Princípio VI (malformados, plugins, pacotes). As duas são P1; nenhuma
   versão para outras pessoas sai sem US2.
4. Parar e validar com o QA.

### Incremental Delivery

US3 (bibliografia, figuras, fontes) → US4 (navegação) → US5 (PDF) → US6 (novo projeto) →
Polish (desempenho medido, README). Cada história é validada pelo QA ao fim.

## Notes

- O hook `after_tasks` (`speckit.github.taskstoissues`) está pendente: o coordenador roda
  depois da revisão.
- Fixtures com links simbólicos e FIFO são geradas em teste, não versionadas (git e Windows).
