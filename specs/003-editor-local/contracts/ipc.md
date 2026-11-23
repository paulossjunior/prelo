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
| `create_project_dialog` | — | `ProjectInfo` ou `null` | `folder_not_empty`, `not_found`, `path_outside_project`, `write_failed` |
| `choose_main_file` | `{ path }` | `ProjectInfo` | `not_a_typ_file`, `path_outside_project` |
| `list_tree` | — | `TreeNode[]` | — |
| `update_project_compiler_version` | — | `ProjectInfo` | — (só por ação explícita, FR-012) |
| `create_file` | `{ path }` | `{ path }` | `already_exists`, `invalid_name`, `path_outside_project`, `not_found`, `write_failed` |
| `create_dir` | `{ path }` | `{ path }` | `already_exists`, `invalid_name`, `path_outside_project`, `not_found`, `write_failed` |

`create_file` cria um arquivo **vazio** (0 bytes, UTF-8 válido) e `create_dir` uma pasta vazia,
ambos em criação exclusiva: nunca sobrescreve nem trunca (FR-034). `path` é relativo à raiz;
a pasta-mãe MUST existir (`not_found` se não existir ou não for pasta) e resolver dentro da
raiz; o último componente segue as regras de nome de FR-035 (`invalid_name`), e nome que já
existe, inclusive como link, é `already_exists`. A resposta traz o caminho normalizado
(relativo, com `/`). Regras completas em
[file-access.md](file-access.md#criação-de-arquivo-e-pasta-fr-034-fr-035).

`create_project_dialog` (US6, FR-002): o núcleo mostra o diálogo de pasta; a pasta MUST estar
vazia (só `.DS_Store` é tolerado e fica intacto), senão `folder_not_empty` e nada é escrito.
Nela nascem, em criação exclusiva e sem seguir link, `main.typ` mínimo e `.prelo/project.json`
com `compiler.typst` = versão embutida e `main: "main.typ"`; depois o projeto abre como em
`open_project_dialog`. Falha no meio desfaz o que foi criado.

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

`offset` de `complete`, e `from`/`to` de `Token`, são **unidades UTF-16** do texto atual do
arquivo (as de `edit_file` e do CodeMirror); o núcleo converte os bytes do Typst. Formatos:
`Completion = { label, kind?, detail?, info?, apply? }` (`kind` no vocabulário do CodeMirror:
`function`, `variable`, `label`, `keyword`, `type`, `property`, `constant`, `namespace`, `text`;
`apply` é texto puro, sem sintaxe de *snippet*) e `Token = { from, to, kind }` (`kind` é a
classe do `typst-syntax` sem o prefixo `typ-`: `heading`, `strong`, `key`, `func`, `math-delim`…;
*tokens* podem se aninhar). Deslocamento fora do texto, ou arquivo que não é `.typ`, responde
lista vazia. Ambos leem o mesmo estado que a prévia compila, inclusive texto não salvo; rótulos
vêm da última prévia `Ok`.

`jump_from_click`, `jump_from_cursor` e `outline` (US4) leem o documento da última prévia `Ok`
**com o World que o compilou**: os *spans* do documento só valem nas fontes daquela compilação.
Depois de uma edição que não compila, os saltos seguem as linhas da prévia mostrada.

## Compilação e prévia

| Comando / evento | Conteúdo |
|---|---|
| `subscribe_preview(channel)` | Canal Tauri; recebe `PreviewUpdate` a cada compilação |
| evento `PreviewUpdate` | `{ compileId, ok, pages: {index, hash, widthPt, heightPt}[], changed: number[], diagnostics: Diagnostic[], durationMs }`. Com `ok: false`, `pages` são as da última `Ok` e `changed` vem vazio; quem assina depois recebe logo a prévia atual com todas as páginas em `changed` |
| `page_svg` | `{ index, hash }` → SVG da página (texto). `hash` velho → `stale_page` |
| `Diagnostic.range` | `{ from, to }` em **unidades UTF-16** do texto atual do arquivo (as mesmas de `edit_file` e do CodeMirror); o núcleo converte os bytes do Typst |
| `prelo-asset://<sha256>` | Protocolo próprio: bytes de imagem por hash; hash desconhecido → 404 |
| `jump_from_click` | `{ page, xPt, yPt }` → `{ path, line, column }` ou `null` (sem origem). `page` a partir de 0 (o `index` de `PreviewUpdate.pages`); `xPt`/`yPt` em pontos a partir do canto superior esquerdo da página; `line` e `column` a partir de 1, `column` em unidades UTF-16. Link interno leva à origem do destino; link externo, conteúdo de pacote e ponto sem *span* (numeração de página, margem) → `null` |
| `jump_from_cursor` | `{ path, offset }` → `{ page, xPt, yPt }[]` (mesmas unidades; `offset` em UTF-16). Cursor fora de texto que aparece na prévia → lista vazia |
| `outline` | → `OutlineItem[]` (vazio antes da primeira prévia `Ok`) |
| `compiler_info` | → `{ embedded: "0.15.1", registered: string \| null, fonts: FontLicense[] }` |

## Exportação

| Comando | Argumentos | Sucesso | Erros |
|---|---|---|---|
| `export_pdf_dialog` | — | `{ path }` ou `null` | `compile_errors` (com `Diagnostic[]`), `compiler_version_mismatch`, `write_failed` |

`compile_errors` chega como `{ code: "compile_errors", diagnostics: Diagnostic[] }` (os da última
compilação); os outros erros, como código em texto. O núcleo recusa antes de mostrar o diálogo
de salvar. O PDF sai do documento da última `PreviewUpdate` com `ok: true` (mesmo `compileId`),
sem recompilar, e é gravado de forma atômica.

## Arquivo externo, plugins e pacotes

| Evento / comando | Conteúdo |
|---|---|
| evento `external_change_conflict` | `{ path }` |
| evento `external_change_reloaded` | `{ path, version }` |
| evento `copy_into_project_offer` | `{ requested }` (D-13; só caminho pedido) |
| `copy_into_project` | `{ requested }` → `{ path, updated: {path, version}[] }` ou `null` (diálogo cancelado); erros `not_offered`, `already_exists`, `invalid_name`, `not_found`, `too_large`, `write_failed`, `no_project` |
| evento `plugin_approval_needed` | `{ path, origin: "project" \| "package", package?: string, sizeBytes, sha256 }` |
| `decide_plugin` | `{ sha256, approve: boolean }` → recompila |
| `redownload_package` | `{ spec }` → recompila; erros `network_unavailable`, `package_rejected` |

Cópia para o projeto (D-13, FR-023): quando uma compilação recusa um caminho que, escrito no
`.typ`, sai da raiz por `..` (`#bibliography("../comum/refs.bib")`), o núcleo emite
`copy_into_project_offer` uma vez por caminho, com **só** o caminho como foi escrito (nunca o
caminho real). Link simbólico dentro do projeto que aponta para fora não gera oferta (a cópia
cairia no lugar do link). `copy_into_project` recebe **só** `{ requested }`: a webview nunca
manda o arquivo de origem (`source` ou qualquer outro campo → `invalid_arguments`), porque
isso permitiria copiar qualquer arquivo do disco para o projeto. O núcleo confere que
`requested` é uma oferta em aberto do projeto (senão `not_offered`, antes de qualquer
diálogo), mostra o diálogo nativo para a pessoa escolher o arquivo e o copia para a pasta do
`.typ` que o pediu, com o nome do último componente de `requested`, em criação exclusiva
(nunca sobrescreve: `already_exists`; regras de nome de FR-035: `invalid_name`). A origem
precisa ser arquivo comum até 256 MB (`not_found`, `too_large`). Depois troca, em cada `.typ`
que pedia `requested`, o literal pelo caminho da cópia (relativo ao `.typ`, ou `/` + caminho
na raiz se o `.typ` estiver em outra pasta), como uma edição do arquivo aberto (versão nova,
gravada pelo salvamento automático), e recompila. `updated` lista esses arquivos com a versão
nova; a interface os recarrega como em `external_change_reloaded`.

Mudança externa (FR-008): com alteração local pendente o núcleo emite
`external_change_conflict` e espera `resolve_external_change`; sem alteração local recarrega o
texto do disco (versão nova), recompila e emite `external_change_reloaded` com a versão nova.
A interface busca o texto com `open_file` e o troca no editor aberto com esse `path`, sem
diálogo e sem enviar edição; evento de arquivo que não está no editor, ou com versão que o
editor já tem, é ignorado. O evento não leva o texto: arquivos vão até 256 MB, eventos são
difundidos a toda a webview, e `open_file` já é a fonte única do estado do arquivo (vários
avisos seguidos resultam numa só leitura do texto mais novo).
Arquivo **não aberto** que muda no disco (um `.bib` exportado pelo gerenciador de referências,
uma figura trocada) não gera evento: o núcleo descarta o conteúdo que a compilação guardava
dele e recompila, relendo pela regra de escopo (FR-025). Itens ocultos (`.prelo/`, `.git/`)
não disparam compilação.

## Permissões da janela

Iguais às de 001 mais a permissão de chamar os comandos acima e, para receber os eventos do
núcleo, exatamente `core:event:allow-listen` e `core:event:allow-unlisten` (sem `emit`, sem
outra permissão `core:`). Nenhuma permissão de `fs`,
`dialog`, `http`, `shell` ou `opener` na webview. CSP:
`default-src 'self'; connect-src ipc: http://ipc.localhost; img-src 'self' blob: prelo-asset: http://prelo-asset.localhost; style-src 'self'`.
O teste de configuração de segurança é atualizado para essa lista exata.
Estilos do CodeMirror: sem `'unsafe-inline'`. O Tauri troca, a cada carga de `index.html`, o
nonce do `<style data-csp-nonce>` vazio por um valor aleatório e acrescenta `'nonce-…'` ao
`style-src` servido; o editor repassa esse nonce ao CodeMirror (`EditorView.cspNonce`,
`src/editor/csp.ts`). Desenhos que o CodeMirror faria com imagens `data:` (marcadores e
sublinhados de diagnóstico) são refeitos em CSS, porque `img-src` não libera `data:`. O
autoteste falha se a webview real registrar violação da CSP ou layout quebrado do editor.

## Autoteste e e2e (só build de teste, feature `self-test`)

| Comando | Argumentos | Efeito |
|---|---|---|
| `self_test_open_project` | — | Copia `src-tauri/tests/fixtures/artigo-basico` para uma pasta temporária, abre a cópia como projeto e devolve `ProjectInfo` (sem diálogo nativo). |
| `self_test_write_external` | `{ text }` | Grava `text` no `main.typ` da cópia temporária da fixture como outro programa faria (temporário oculto + `rename`); nunca escreve fora da cópia. |
| `self_test_report` | `{ rendered, elapsedMs, scenario }` | Além da conferência de 001, confere o roteiro do editor (`scenario = { projectOpened, previewChanged, errorLine, diagnosticLine, editorMarked, compilerShown, externalReloaded, externalErrorLine, externalDiagnosticLine, layoutProblems, failure? }`: a prévia mudou, o erro escrito na linha `errorLine` veio com diagnóstico nessa linha e marcado no editor, o arquivo gravado por fora sem alteração local apareceu no editor com o diagnóstico na linha `externalErrorLine`, a versão embutida aparece) e que não há `typst` no PATH (SC-006). Encerra com 0 ou 1. |

A interface abre a fixture sozinha com `VITE_SELF_TEST` (autoteste) ou `VITE_E2E` (WebDriver).
