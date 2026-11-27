# Research: Sincronização pelo git (005)

Fase 0 do plano. Itens **spike** precisam de teste de viabilidade antes das tarefas que
dependem deles (tasks.md, Setup).

## R1 Biblioteca git: gitoxide (`gix`) — **spike**

- **Decisão**: `gix` (MIT OR Apache-2.0), versão exata ≥ 0.82 (correção de *path traversal*
  CVE-2026-82253 em gix 0.82.0 / gix-validate 0.11.1); hoje `=0.91.0`. Sem as features que
  executam processos externos (`gix` com transporte HTTP em Rust puro, TLS por `rustls`, como o
  `ureq` da 003). Usos: clone, fetch, leitura de árvores/blobs, criação de commit local,
  atualização de refs e do índice/worktree.
- **Push**: a lista de recursos do gitoxide marcava push como não implementado na última
  versão verificada ([gitoxide README](https://docs.rs/crate/gitoxide/0.49.0/source/README.md),
  [crate-status](https://github.com/Byron/gitoxide/blob/c8bef2c39d4a6f8f547422405eb1827d520ea96d/crate-status.md)).
  **Spike T001**: confirmar com a versão fixada se `gix` envia *packfile* por HTTPS
  (`receive-pack`).
  - Se sim: push nativo do `gix`.
  - Se não (**plano B, preferido ao libgit2**): enviar pela **API Git Data do GitHub** com o
    mesmo token: criar blobs → árvore → commit (com os mesmos pais e autor do commit local) →
    `PATCH /git/refs/heads/<branch>` com `force: false`. O GitHub recusa avanço não
    *fast-forward*, que é exatamente a semântica de FR-020. A permissão `Contents: write` do
    GitHub App cobre essas rotas. O SHA do commit criado pela API é idêntico ao local quando
    árvore, pais, autor, committer, datas e mensagem são iguais; o spike confirma isso, e, se
    não for, o app adota o commit do GitHub como o local após o envio (fetch + reset do ref).
- **Alternativas**: `git2`/libgit2 — descartado: libgit2 é GPL-2.0 com *linking exception*,
  fora da lista de licenças (RNF-06) e traria OpenSSL; `git` do sistema — descartado: exige git
  instalado, executa hooks e lê credenciais do sistema (contra FR-010).
- **Constituição**: `gix` continua sendo o componente git; o plano B é uso da API do remoto,
  não troca de componente. Mesmo assim, registrar D-19 em docs/adrs.md ("push pela API Git
  Data do GitHub enquanto o gitoxide não tiver push") se o spike cair no plano B.

## R2 Login: GitHub App + Device Flow (D-18)

- **Decisão**: `POST https://github.com/login/device/code` com o *client ID* do GitHub App
  (constante pública no binário; não há segredo), *polling* em
  `/login/oauth/access_token` respeitando `interval` e `slow_down`, tratamento de
  `authorization_pending`, `expired_token`, `access_denied`. Renovação com `refresh_token`
  (`grant_type=refresh_token`). HTTP com o `ureq` + `rustls` já usados na 003.
- **Identidade**: `GET /user` → `login` e `id` para "conectado como @login" e para o e-mail
  `<id>+<login>@users.noreply.github.com` (FR-017).
- **Repositórios autorizados**: `GET /user/installations` e
  `GET /user/installations/{id}/repositories` (lista para "Clonar do GitHub").
- **Registro do App**: ação do dono (conta `paulossjunior`), com device flow habilitado,
  permissões `Contents: RW`, `Metadata: R`, tokens com expiração. Tarefa `agente:humano`.

## R3 Cofre de credenciais

- **Decisão**: `keyring = "=4.2.0"` (MIT OR Apache-2.0) com os *stores* nativos: Keychain
  (macOS), Credential Manager (Windows), Secret Service via `zbus` (Linux). Serviço
  `app.prelo.github`, conta = `login`. Linux sem Secret Service → login funciona só na sessão
  (token em memória) e o app avisa que precisará entrar de novo.
- **Testes**: *mock store* do `keyring` nos testes; teste de segurança que varre logs,
  `.prelo/`, respostas IPC e mensagens de erro procurando o token (SC-004).

## R4 Mesclagem própria, não `git merge`

- **Decisão**: o núcleo faz a mesclagem de três vias por arquivo, com base = *merge-base* entre
  o commit local e o remoto (via `gix`):
  - `.typ`: união dos updates CRDT de `.prelo/crdt/<id>.yjs` dos dois lados (aplicar o
    update remoto no `Doc` local; Yjs é comutativo), depois gerar o `.typ` do `Doc`. Se o
    `.typ` remoto ≠ texto do `Doc` remoto (edição fora do Prelo), reconciliar como autor
    "externo" (004, FR-008) e, se a diferença externa toca trecho alterado localmente desde a
    base, pedir escolha (FR-023).
  - `.prelo/review.json`: união por id; estado por `statusAt` mais recente; lápides vencem
    (004 data-model, emenda de 2026-11-27).
  - `.prelo/versions.json`: união por id; nome repetido entre lados → o mais antigo fica, o
    outro recebe sufixo ` (2)` e aviso.
  - `.prelo/project.json`: por chave; `owner` divergente → mais antigo fica.
  - outros arquivos alterados dos dois lados com conteúdos diferentes: pergunta "manter o meu"
    ou "manter o recebido" (FR-026).
- **Commit**: commit de mesclagem com dois pais (local e remoto) e a mensagem de FR-017; depois
  push *fast-forward*. Nenhuma marca de conflito é escrita.
- **Alternativa**: `gix` merge de árvores — descartado para `.typ` e `.prelo/` (não conhece
  CRDT nem a semântica dos JSON); pode ser usado para detectar arquivos não alterados.

## R5 Estado de sincronização e bloqueio de edição

- **Decisão**: máquina de estados no núcleo (`Verificando → Recebendo → Sincronizado`,
  `Offline`; arquitetura.md §5), evento `SyncUpdate` para a interface. Com estado ≠
  Sincronizado, `edit_file` responde `edit_blocked` (o bloqueio vale no núcleo, não só na
  interface; Princípio IV). Comentários (004) continuam aceitos.
- **Reconexão**: tentativa com espera 30 s → ×2 até 5 min; também ao voltar de suspensão e a
  cada Salvar.
- **Trava entre janelas**: `.git/prelo.lock` com PID; segunda janela abre só leitura.

## R6 Salvar e encerramento forçado

- **Decisão**: Salvar = (1) bloqueio, (2) gravar `.typ`, `.prelo/` (atômico, da 003/004),
  (3) **commit local primeiro** (garante FR-021 mesmo se a rede cair), (4) fetch, (5) mesclar se
  preciso (novo commit de mesclagem), (6) push, (7) liberar. Commits locais sem push são
  detectados ao abrir (`ahead > 0`) e enviados na próxima sincronização.
- **Fechar** (FR-028): o núcleo intercepta `CloseRequested` (já usado na 003 para gravar) e
  roda Salvar com prazo de 15 s antes de oferecer "Fechar mesmo assim".

## R7 Segurança do repositório

- **Decisão**:
  - nunca executar hooks (o `gix` não executa; teste confirma com hook que cria arquivo);
  - ignorar `credential.*`, `core.sshCommand`, `core.fsmonitor`, `include.*` do config do
    repositório e do sistema (abrir com `gix::open::Options` isolado: sem config global/sistema);
  - validar todo caminho de árvore recebida com `gix-validate` + regra de escopo da 003
    (file-access.md) antes de escrever: recusar `..`, absolutos, `.git` em qualquer
    componente, nomes reservados do Windows, links simbólicos que saiam da raiz, colisão por
    maiúsculas/Unicode (macOS/Windows);
  - limite de tamanho do clone (2 GiB) e por arquivo (200 MiB) com aviso;
  - `.prelo/` recebido passa pelas validações da 004.

## R8 Resumo da sincronização (RF-38)

- **Decisão**: commits entre o ref antigo e o novo do remoto, agrupados por autor; diff de
  `review.json` antes/depois da mesclagem → conversas novas e resolvidas por autor. Mostrado
  num aviso que a pessoa fecha.

## R9 Versões nomeadas

- **Decisão**: `.prelo/versions.json` (`schema: 1`, `versions` por id: `name`, `author`,
  `at`, `commit`). "Marcar versão" roda Salvar, grava a versão apontando para o commit
  resultante e roda um segundo commit só com `versions.json` (o commit não pode conter o próprio
  SHA). Sem tags git nesta fase (tags exigiriam push de refs extras; fica para depois).

## R10 Rede de referência (TODO(rede-comum))

- **Decisão**: rede de referência = 50 Mbit/s, RTT 30 ms até o GitHub; projeto de fixture de
  40 páginas com 20 MiB de figuras já sincronizadas; Salvar com 1 edição local e 1 commit remoto
  de texto. Medido em teste de desempenho manual/noturno com repositório de teste
  (`prelo-sync-fixture`), não no `verify` (depende de rede).

## R11 Testes

- Integração local sem rede: remoto como repositório *bare* em pasta temporária via
  `file://`; para o plano B (API Git Data), servidor HTTP falso em `127.0.0.1` que emula as
  rotas usadas, inclusive recusa de não *fast-forward*.
- Dois clones (A, B) para SC-002 (100 rodadas com semente fixa) e SC-006.
- Encerramento forçado (processo filho morto) em cada passo de R6 (SC-003).
- Repositórios maliciosos de fixture para SC-005.
- Device flow com servidor falso (respostas `authorization_pending`, `slow_down`, expiração,
  recusa, refresh inválido).
- e2e: login com servidor falso, Salvar com remoto `file://`.
