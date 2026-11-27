# Data Model: Sincronização pelo git (005)

Arquivos vindos do remoto são **entrada não confiável** (Princípio VI). Formatos da 004
(`review.json`, `crdt/*.yjs`) não mudam aqui, exceto pela emenda das lápides já registrada em
`specs/004-comentarios/data-model.md`.

## Credential (cofre do sistema, nunca em disco do projeto)

| Campo | Tipo | Regra |
|---|---|---|
| `accessToken` | string | só no cofre e na memória do núcleo; nunca serializado para IPC, log ou erro |
| `accessExpiresAt` | data | renovar 5 min antes |
| `refreshToken` | string | idem `accessToken` |
| `refreshExpiresAt` | data | expirado → login necessário |
| `login` | string | `^[A-Za-z0-9-]{1,39}$` |
| `userId` | inteiro | > 0 |

## RemoteInfo (derivado do `.git/config` do clone)

| Campo | Tipo | Regra |
|---|---|---|
| `url` | string | só `https://github.com/<owner>/<repo>(.git)?`; outro formato → sem remoto (FR-008) |
| `owner`, `repo` | string | `^[A-Za-z0-9._-]{1,100}$` |
| `branch` | string | branch atual; HEAD destacado → sem sincronização, aviso |

## SyncState (memória do núcleo)

| Campo | Tipo | Regra |
|---|---|---|
| `phase` | `"local"` \| `"checking"` \| `"receiving"` \| `"synced"` \| `"offline"` \| `"auth_required"` | `local` = projeto sem remoto |
| `ahead` | inteiro | commits locais não enviados |
| `lastSyncAt` | data \| null | |
| `nextRetryAt` | data \| null | só em `offline` (30 s → 5 min) |
| `editable` | bool | `phase ∈ {local, synced}` (Princípio IV) |

### Transições

```text
open(remoto) → checking ─┬─ nada novo ─────────────▶ synced
                         ├─ há novo ─▶ receiving ──▶ synced
                         ├─ sem rede ──────────────▶ offline
                         └─ token inválido ────────▶ auth_required
synced ─ Salvar ─▶ receiving ─▶ synced | offline
offline ─ retry ok ─▶ receiving ─▶ synced
auth_required ─ login ok ─▶ checking
```

## SyncCommit (commit criado pelo app)

| Campo | Regra |
|---|---|
| mensagem | `Prelo sync · <displayName> · <resumo>`; resumo ≤ 72 caracteres ("3 arquivos, 2 comentários") |
| autor/committer | `displayName <userId+login@users.noreply.github.com>` |
| pais | 1 (commit local) ou 2 (mesclagem: local, remoto) |

## SyncSummary

| Campo | Tipo |
|---|---|
| `byPerson` | `{ name, commits, newThreads, resolvedThreads }[]` |
| `sameSpanEdits` | `{ path, quote }[]` — trechos editados dos dois lados (FR aceitação US2.3) |
| `choices` | `ConflictChoice[]` pendentes |

## ConflictChoice

| Campo | Tipo | Regra |
|---|---|---|
| `id` | string | 32 hex |
| `path` | string | relativo, regra de escopo da 003 |
| `kind` | `"external_typ"` \| `"binary"` \| `"other"` | |
| `mine`, `theirs` | string \| null | texto (≤ 200 KiB) para `.typ`/texto; `null` para binário (mostra tamanho e data) |

## VersionsFile — `.prelo/versions.json`

```json
{ "schema": 1, "versions": { "<32 hex>": { "name": "Submissão 1", "author": "Ana",
  "at": "2026-12-01T13:00:00Z", "commit": "<40 hex>" } } }
```

| Campo | Tipo | Regra |
|---|---|---|
| `schema` | inteiro | `1`; maior → só leitura |
| `versions` | objeto id → Version | ≤ 1.000 |
| `Version.name` | string | 1–80 caracteres, sem controle; único (comparação sem diferença de maiúsculas e espaços nas pontas) |
| `Version.author` | string | 1–100 |
| `Version.at` | string | RFC 3339 UTC |
| `Version.commit` | string | `^[0-9a-f]{40}$`; commit inexistente no histórico → versão exibida com aviso |

Arquivo > 1 MiB, JSON inválido ou tipo errado → aviso `versions_json_invalid`, arquivo
preservado, cópia `.invalid-<ts>` antes de regravar (padrão da 004).

## Limites do repositório (R7)

| Limite | Valor |
|---|---|
| tamanho total do clone | 2 GiB |
| tamanho por arquivo | 200 MiB |
| caminho | ≤ 4096 bytes, componentes ≤ 255 |
