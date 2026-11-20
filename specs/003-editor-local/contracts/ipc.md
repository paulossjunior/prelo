# Contrato IPC: interface ↔ núcleo (feature 003)

Amplia o contrato de [001](../../001-app-skeleton/contracts/ipc.md). Tudo fora deste contrato
MUST ser recusado. Caminhos vindos da interface são **relativos à raiz do projeto aberto** e
passam pelas regras de [file-access.md](file-access.md); caminho absoluto ou fora da raiz →
`Err("path_outside_project")`. Erros são strings estáveis em inglês (códigos), e a interface
mostra o texto em português.

## Projeto

| Comando | Argumentos | Sucesso | Erros |
|---|---|---|---|
| `open_project_dialog` | — | `ProjectInfo` ou `null` (cancelado) | `project_json_invalid` (abre com aviso, ver data-model) |
| `create_project_dialog` | — | `ProjectInfo` ou `null` | `folder_not_empty` |
| `choose_main_file` | `{ path }` | `ProjectInfo` | `not_a_typ_file`, `path_outside_project` |
| `list_tree` | — | `TreeNode[]` | — |
| `update_project_compiler_version` | — | `ProjectInfo` | — (só por ação explícita, FR-012) |

Com versão registrada ≠ embutida, `ProjectInfo.warnings` traz `compiler_version_mismatch`, nenhum `PreviewUpdate` é emitido e `export_pdf_dialog` responde `compiler_version_mismatch` até `update_project_compiler_version`.

O núcleo mostra o diálogo nativo; a webview não tem permissão de diálogo nem de arquivo.

## Documentos

| Comando | Argumentos | Sucesso | Erros |
|---|---|---|---|
| `open_file` | `{ path }` | `{ text, version }` | `not_found`, `path_outside_project`, `not_utf8`, `too_large` |
| `edit_file` | `{ path, version, changes: {from,to,insert}[] }` | `{ version }` | `version_mismatch`, `bad_range` |
| `save_file` | `{ path }` | `{ savedAt }` | `write_failed` (texto continua em memória) |
| `resolve_external_change` | `{ path, keep: "mine" \| "disk" }` | `{ text, version }` | — |
| `complete` | `{ path, offset }` | `Completion[]` | — |
| `semantic_tokens` | `{ path }` | `Token[]` | — |

## Compilação e prévia

| Comando / evento | Conteúdo |
|---|---|
| `subscribe_preview(channel)` | Canal Tauri; recebe `PreviewUpdate` a cada compilação |
| evento `PreviewUpdate` | `{ compileId, ok, pages: {index, hash, widthPt, heightPt}[], changed: number[], diagnostics: Diagnostic[], durationMs }` |
| `page_svg` | `{ index, hash }` → SVG da página (texto). `hash` velho → `stale_page` |
| `prelo-asset://<sha256>` | Protocolo próprio: bytes de imagem por hash; hash desconhecido → 404 |
| `jump_from_click` | `{ page, xPt, yPt }` → `{ path, line, column }` ou `null` (sem origem) |
| `jump_from_cursor` | `{ path, offset }` → `{ page, xPt, yPt }[]` |
| `outline` | → `OutlineItem[]` |
| `compiler_info` | → `{ embedded: "0.15.1", registered: string \| null, fonts: FontLicense[] }` |

## Exportação

| Comando | Argumentos | Sucesso | Erros |
|---|---|---|---|
| `export_pdf_dialog` | — | `{ path }` ou `null` | `compile_errors` (com `Diagnostic[]`), `compiler_version_mismatch`, `write_failed` |

## Arquivo externo, plugins e pacotes

| Evento / comando | Conteúdo |
|---|---|
| evento `external_change_conflict` | `{ path }` |
| evento `copy_into_project_offer` | `{ requested }` (D-13; só caminho pedido) |
| `copy_into_project` | `{ requested, source }` — `source` escolhido em diálogo nativo do núcleo |
| evento `plugin_approval_needed` | `{ path, origin: "project" \| "package", package?: string, sizeBytes, sha256 }` |
| `decide_plugin` | `{ sha256, approve: boolean }` → recompila |
| `redownload_package` | `{ spec }` → recompila; erros `network_unavailable`, `package_rejected` |

## Permissões da janela

Iguais às de 001 mais a permissão de chamar os comandos acima. Nenhuma permissão de `fs`,
`dialog`, `http`, `shell` ou `opener` na webview. CSP:
`default-src 'self'; connect-src ipc: http://ipc.localhost; img-src 'self' blob: prelo-asset: http://prelo-asset.localhost; style-src 'self'`.
O teste de configuração de segurança é atualizado para essa lista exata.
