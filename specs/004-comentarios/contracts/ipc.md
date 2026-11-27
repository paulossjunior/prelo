# Contrato IPC: Comentários (004)

Acrescenta comandos e um evento ao contrato da 003 (`specs/003-editor-local/contracts/ipc.md`),
com as mesmas convenções: deslocamentos em **unidades UTF-16**, erros como
`{ code, message }`, argumentos extras ou malformados → `invalid_arguments`. O autor de toda
ação vem do `settings.json` lido pelo núcleo, **nunca** de um argumento (research R6).

## Identidade e dono

| Comando | Argumentos | Resposta | Erros |
|---|---|---|---|
| `get_identity` | — | `{ displayName: string \| null, owner: string \| null, isOwner: boolean }` | `no_project` (só `owner`/`isOwner` ficam `null`/`false`) |
| `set_display_name` | `{ name }` | `{ displayName }` | `invalid_name` |
| `claim_ownership` | — | `{ owner }` | `owner_already_set`, `no_display_name`, `no_project` |

## Conversas

Toda resposta de sucesso só volta depois de `.prelo/review.json` (e, se a âncora é nova, o
estado CRDT) estar gravado com rename atômico (FR-023, FR-024).

| Comando | Argumentos | Resposta | Erros |
|---|---|---|---|
| `create_thread` | `{ path, from, to, body }` | `Thread` | `no_display_name`, `empty_selection`, `bad_range`, `body_empty`, `body_too_long`, `out_of_scope`, `review_read_only`, `write_failed` |
| `reply` | `{ threadId, body }` | `Thread` | `not_found`, `body_empty`, `body_too_long`, `no_display_name`, `review_read_only`, `write_failed` |
| `set_thread_status` | `{ threadId, status: "open" \| "resolved" }` | `Thread` | `not_found`, `no_display_name`, `review_read_only`, `write_failed` |
| `delete_message` | `{ threadId, messageId }` | `Thread` | `not_found`, `forbidden`, `review_read_only`, `write_failed` |
| `delete_thread` | `{ threadId, confirm: true }` | `{}` | `not_found`, `forbidden`, `confirmation_required`, `review_read_only`, `write_failed` |
| `delete_resolved` | `{ confirm: true }` | `{ deleted: number }` | `forbidden`, `confirmation_required`, `review_read_only`, `write_failed` |
| `reanchor_thread` | `{ threadId, path, from, to }` | `Thread` | `not_found`, `empty_selection`, `bad_range`, `out_of_scope`, `review_read_only`, `write_failed` |
| `list_threads` | `{ status: "open" \| "resolved" }` | `Thread[]` | — |
| `known_names` | — | `string[]` | — |
| `subscribe_review(channel)` | canal Tauri | recebe `ReviewUpdate` | — |

`confirm` ausente ou `false` → `confirmation_required` sem mudança (Princípio V). `forbidden`
não muda nada (FR-020).

### `Thread` (visão para a interface)

```ts
{ id, path, quote, status: "open" | "resolved", detached: boolean,
  range: { from, to } | null,          // UTF-16 no texto atual; null se detached
  openedBy, openedAt, statusBy?, statusAt?,
  messages: { id, author, at, body, mentions: string[], deleted: boolean }[],
  can: { delete: boolean } }           // calculado pelo núcleo para a pessoa local
```

Cada `Message` também traz `can: { delete: boolean }`.

### Evento `ReviewUpdate`

`{ reason: "loaded" | "changed" | "anchors_moved", open: number, resolved: number,
   ranges: { path, threadId, from, to }[], warnings: Warning[] }`

`ranges` só das conversas abertas e não destacadas do arquivo aberto no editor. Emitido após
cada ação, após `edit_file` que mova âncoras (agregado, no máximo 1 por compilação) e ao abrir
projeto.

`Warning` (`{ code, message, detail? }`): `review_json_invalid`, `review_json_newer`,
`review_threads_skipped` (com `detail.count`), `crdt_state_invalid` (com `detail.path`).

## Outline

`OutlineItem` (003) ganha `openComments: number` (0 quando não há; FR-013).

## Exportação

| Comando | Argumentos | Resposta | Erros |
|---|---|---|---|
| `export_pdf_with_comments_dialog` | — | `{ path } \| null` (cancelado) | `compile_errors` (com `Diagnostic[]`, como `export_pdf` da 003), `write_failed`, `no_project` |

Não altera `.typ`, `review.json` nem o PDF final (FR-029).

## Permissões da janela

Os comandos acima entram na mesma capability da janela principal da 003; nenhum novo acesso ao
sistema de arquivos pela interface.
