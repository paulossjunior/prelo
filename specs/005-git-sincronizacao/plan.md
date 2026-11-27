# Implementation Plan: Sincronização pelo git (fase 2, parte 2)

**Branch**: `005-git-sincronizacao` | **Date**: 2026-11-27 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/005-git-sincronizacao/spec.md`

## Summary

Salvar (⌘S) passa a receber, mesclar, gravar commit e enviar ao repositório do grupo no GitHub;
abrir recebe antes de liberar a edição; fechar faz um último Salvar; offline, só ler e
comentar, com commit local e envio automático ao reconectar; versões nomeadas em
`.prelo/versions.json`. Login por GitHub App com device flow, tokens só no cofre do sistema
(D-18). O núcleo usa o gitoxide (`gix`) para clone, fetch e commits locais e faz a **própria
mesclagem de três vias**: `.typ` pela união dos estados Yjs da 004, `.prelo/*.json` por chave,
demais arquivos por escolha da pessoa. O push é nativo do `gix` se o spike confirmar; senão,
pela API Git Data do GitHub com `force: false` ([research.md R1](research.md#r1-biblioteca-git-gitoxide-gix--spike)).

## Technical Context

**Language/Version**: Rust (toolchain da 003) no núcleo; TypeScript 7 na interface.

**Primary Dependencies**: existentes (Tauri 2.12, `ureq` + `rustls`, `yrs` e `similar` da
004). Novas: `gix =0.91.0` (MIT OR Apache-2.0; ≥ 0.82 pela CVE-2026-82253), `keyring =4.2.0`
(MIT OR Apache-2.0) com *stores* nativos.

**Storage**: `.git/` do clone; `.prelo/versions.json`; credenciais no cofre do sistema;
`.git/prelo.lock`.

**Testing**: `cargo test` com remoto *bare* `file://` e servidor HTTP falso em 127.0.0.1
(device flow e API Git Data); vitest/jsdom; wdio e2e; desempenho com rede real fora do verify.

**Target Platform**: macOS, Windows, Linux (RNF-01).

**Project Type**: app desktop (Tauri).

**Performance Goals**: Salvar ~1 s p50, ≤ 3 s p95 na rede de referência
([research.md R10](research.md#r10-rede-de-referência-todorede-comum)); prévia < 100 ms
inalterada.

**Constraints**: nenhuma credencial fora do cofre (FR-002); sem hooks nem config de
credenciais do sistema (FR-010); confinamento de caminhos (FR-011); edição bloqueada fora de
`synced`/`local` no núcleo (Princípio IV); commit local antes da rede (Princípio III).

**Scale/Scope**: clone ≤ 2 GiB, arquivo ≤ 200 MiB, até 1.000 versões nomeadas.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Princípio | Situação | Como |
|---|---|---|
| I. Typst puro | ✅ | `.typ` gravado do `Doc` mesclado, sem marcas de conflito (FR-022, FR-026); SC-002 compila com `typst` CLI |
| II. Privacidade sem servidor | ✅ | Só o repositório do grupo (Q-05); GitHub App com menor privilégio (`Contents`, `Metadata`); tokens só no cofre; TLS em todo tráfego; nenhum servidor do Prelo |
| III. Nenhum trabalho perdido | ✅ | Commit local antes de qualquer rede (R6); fechar faz Salvar; offline guarda e agenda; teste de encerramento forçado em cada passo (SC-003) |
| IV. Edição só sobre versão sincronizada | ✅ | Estados de arquitetura.md §5; `edit_file` → `edit_blocked` no núcleo; ordem receber → enviar → gravar → liberar (FR-016) |
| V. Revisão explícita e reversível | ✅ | Mesclagem de `review.json` respeita lápides e estados; versões não restauram nada nesta fase |
| VI. Testes primeiro e segurança | ✅ | TDD; segurança: token fora de logs/IPC/disco, repositório malicioso (caminhos, links, hooks, config), `.prelo/` adulterado, device flow com respostas hostis, auditoria de `gix` e `keyring` |
| Restrições técnicas | ✅ / ⚠️ | gitoxide mantido. Se o spike cair no plano B (push pela API Git Data), registrar D-19 em docs/adrs.md antes da tarefa de push |
| Fluxo: ordem das fases | ✅ | Fase 2, depois da 004 |
| Fluxo: transição entre features | ⚠️ exceção | Ver Complexity Tracking |

**Re-check pós-design**: data-model, contrato e quickstart mantêm os itens acima.

## Project Structure

### Documentation (this feature)

```text
specs/005-git-sincronizacao/
├── plan.md  research.md  data-model.md  quickstart.md
├── contracts/ipc.md
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
src-tauri/src/
├── github/                 # novo
│   ├── device_flow.rs      # login, polling, refresh
│   ├── credentials.rs      # keyring; redação de segredos
│   └── api.rs              # /user, instalações, repositórios, Git Data (plano B)
├── sync/                   # novo
│   ├── mod.rs              # máquina de estados, SyncUpdate, trava de janela
│   ├── repo.rs             # gix: abrir isolado, clone, fetch, commit, refs, ahead
│   ├── safety.rs           # validação de caminhos/árvores, limites
│   ├── merge.rs            # três vias: .typ via CRDT, .prelo/*.json, escolhas
│   ├── push.rs             # gix nativo ou API Git Data
│   ├── summary.rs          # resumo por pessoa
│   └── versions.rs         # .prelo/versions.json
├── review/                 # (004) + merge por chave e lápides
├── commands.rs             # + comandos do contrato; edit_file → edit_blocked
└── lib.rs                  # CloseRequested → sync_now

src-tauri/tests/
├── sync_merge.rs  sync_crash.rs  sync_security.rs  sync_auth.rs  sync_versions.rs
└── perf_sync.rs            # #[ignore], rede real

src/
├── sync/                   # novo
│   ├── status.ts           # indicador de estado e bloqueio do editor
│   ├── signin.ts           # device flow (código, link)
│   ├── clone.ts            # lista de repositórios e pasta
│   ├── summary.ts          # resumo e escolhas de conflito
│   └── versions.ts         # marcar e listar versões
└── workspace.ts            # ⌘S → sync_now; fechar

fixtures/sync/              # repositórios maliciosos, projetos de dois lados
```

**Structure Decision**: mesmo layout da 003/004, com módulos `github` e `sync` no núcleo e
`sync` na interface.

## Complexity Tracking

| Violação | Por que é preciso | Alternativa mais simples rejeitada porque |
|---|---|---|
| Spec e plano da 005 começam antes de a `1.0.0` estar em produção | Pedido do dono em 2026-11-27 sob a exceção de 2026-11-26, **só para especificação e planejamento**. Nenhuma tarefa de código começa antes de a `1.0.0` cumprir a regra e de a 004 estar pronta | Esperar deixaria o planejamento da R2 parado; documentos numa branch não afetam a `1.0.0` |
| Mesclagem própria em vez de `git merge` | `.typ` precisa mesclar pelo CRDT (decisão do dono) e `.prelo/*.json` por chave, sem marcas de conflito (Princípio I) | `git merge` de texto escreve marcas de conflito e não conhece Yjs nem as lápides |
| Possível push pela API Git Data (plano B) | gitoxide pode não ter push; libgit2 é GPL e `git` do sistema executa hooks e lê credenciais | Ver research R1; decidido pelo spike T001 |
