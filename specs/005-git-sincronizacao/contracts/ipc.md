# Contrato IPC: Sincronização pelo git (005)

Acrescenta ao contrato da 003 e da 004, com as mesmas convenções. **Nenhuma resposta, evento
ou erro contém token** (FR-002, SC-004). A interface não ganha capability `http` nem `shell`.

## GitHub

| Comando | Argumentos | Resposta | Erros |
|---|---|---|---|
| `github_status` | — | `{ state: "signed_out" \| "signed_in" \| "auth_required", login?: string }` | — |
| `github_sign_in_start` | — | `{ userCode, verificationUri, expiresIn }` (o núcleo abre o navegador) | `network`, `rate_limited` |
| `github_sign_in_poll` | — | `{ state: "pending" \| "signed_in" \| "denied" \| "expired", login? }` | `network` |
| `github_sign_out` | `{ confirm: true }` | `{}` | `confirmation_required` |
| `github_list_repos` | — | `{ owner, repo, private, defaultBranch }[]` | `auth_required`, `network` |
| `clone_dialog` | `{ owner, repo }` | `ProjectInfo` (o da 003) \| `null` | `auth_required`, `not_authorized_repo` (com `detail.installUrl`), `folder_not_empty`, `repo_too_large`, `unsafe_path` (com `detail.path`), `network` |

O núcleo controla o intervalo de *polling* (FR-006): `github_sign_in_poll` chamado antes do
intervalo devolve `pending` sem pedir ao GitHub.

## Sincronização

| Comando / evento | Conteúdo |
|---|---|
| `sync_status` | → `SyncState` ([data-model.md](../data-model.md#syncstate-memória-do-núcleo)) |
| `sync_now` | Salvar completo (FR-016) → `{ summary: SyncSummary \| null, state: SyncState }`. Erros: `offline` (commit local feito; resposta traz `state`), `auth_required`, `diverged` (force-push alheio; nada enviado), `push_rejected` (após 3 tentativas), `choices_pending` (com `ConflictChoice[]`), `write_failed` |
| `resolve_choice` | `{ id, keep: "mine" \| "theirs" }` → `{ remaining: number }`; com 0 restantes, o núcleo continua o Salvar |
| `subscribe_sync(channel)` | recebe `SyncUpdate` |
| evento `SyncUpdate` | `{ state: SyncState, progress?: { step: "commit" \| "fetch" \| "merge" \| "push", done?, total? }, summary?: SyncSummary, warnings: Warning[] }` |
| `close_requested` (003) | passa a rodar `sync_now`; a interface recebe `SyncUpdate` e pode chamar `force_close` |
| `force_close` | `{ confirm: true }` → grava commit local e fecha (FR-028) |

`edit_file` (003) passa a responder `edit_blocked` quando `SyncState.editable` é falso
(Princípio IV). Comandos de comentário da 004 continuam aceitos.

`Warning.code` novos: `remote_not_github`, `submodules_unsupported`, `lfs_unsupported`,
`detached_head`, `second_window_read_only`, `keyring_unavailable`, `versions_json_invalid`,
`unsent_commits` (com `detail.count`), `owner_conflict`.

## Versões nomeadas

| Comando | Argumentos | Resposta | Erros |
|---|---|---|---|
| `mark_version` | `{ name }` | `Version` | `name_taken`, `invalid_name`, `offline`, `auth_required`, `review_read_only` (versions.json mais novo) |
| `list_versions` | — | `{ id, name, author, at, commit, commitsSincePrevious: number, missing: boolean }[]` | — |
