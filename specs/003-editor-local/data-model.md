# Data Model: Editor local (fase 1)

**Feature**: [spec.md](spec.md) · **Date**: 2026-11-19

## Projeto

| Campo | Tipo | Regra |
|---|---|---|
| `root` | caminho canônico | Resolvido uma vez ao abrir; base de todas as regras de [file-access.md](contracts/file-access.md) |
| `record` | `ProjectRecord` | Lido de `.prelo/project.json` |
| `mainFile` | caminho relativo | `.typ` dentro da raiz |
| `openFiles` | mapa caminho → `OpenFile` | Overlay do VFS |

`ProjectInfo` (IPC) = `{ name, mainFile, compiler: { embedded, registered }, warnings[] }`.

## ProjectRecord (`.prelo/project.json`)

```json
{ "schema": 1, "compiler": { "typst": "0.15.1" }, "main": "main.typ" }
```

- Tamanho máximo 1 MB; JSON inválido, `schema` desconhecido ou tipo errado → projeto abre com
  aviso `project_json_invalid`, usa padrões (`main.typ`, versão embutida) e **não regrava** o
  arquivo até a pessoa escolher "Recriar registro".
- Campos desconhecidos são preservados ao regravar (fases seguintes acrescentam participantes
  e papéis) e nunca interpretados.
- Criado só quando não existe (FR-003); `compiler.typst` só muda por
  `update_project_compiler_version` (FR-012). Enquanto `compiler.typst` ≠ versão embutida, o
  projeto fica em estado `VersaoDiferente`: sem compilação, prévia nem exportação. `main` muda por `choose_main_file`.
- `main` com `..`, absoluto ou fora da raiz → inválido (aviso, usa padrão).

## OpenFile

| Campo | Tipo | Regra |
|---|---|---|
| `text` | string UTF-8 | Arquivo não UTF-8 → `not_utf8`, não abre |
| `version` | inteiro | Incrementa a cada `edit_file`; edição com versão antiga → `version_mismatch` |
| `dirty` | bool | `true` desde a última edição até gravar |
| `lastSavedHash` | sha256 | Para ignorar o próprio evento do observador de arquivos |

Estados: `Limpo → (edição) → Sujo → (1,5 s desde a primeira edição não gravada, ⌘S ou
fechar) → Gravando → Limpo`;
`Gravando → (falha) → Sujo` + aviso. Mudança externa: `Limpo → recarrega`; `Sujo →
Conflito → (escolha) → Limpo/Sujo`.

## Compilation

| Campo | Tipo |
|---|---|
| `compileId` | inteiro crescente |
| `status` | `Ok` \| `ComErros` \| `Abandonada` |
| `pages` | `PageInfo[]` (`index`, `hash`, `widthPt`, `heightPt`) |
| `diagnostics` | `Diagnostic[]` |
| `durationMs` | inteiro |

A prévia mostra a última `Ok`; `ComErros` e `Abandonada` só trazem diagnósticos (FR-014,
FR-015). Resultado de compilação mais antiga que a última entregue é descartado.

## Diagnostic

`{ severity: "error"|"warning", path?, range?: {from,to}, line?, message, hints[], kind }`,
onde `kind` ∈ `typst`, `access` (recusas de file-access), `plugin`, `package`, `font`,
`project`. Diagnóstico de acesso usa o caminho pedido, nunca o real.

## OutlineItem

`{ level, title, path, line, page, yPt }`, em ordem de documento. `level` a partir de 1 (o
nível resolvido do título); `title` com a numeração quando houver ("2.1 Coleta de dados");
`path` relativo à raiz e `line` a partir de 1 (o título pode estar num `.typ` incluído);
`page` a partir de 0 e `yPt` em pontos a partir do topo da página. Títulos `outlined: false` e
os que não vêm de arquivo do projeto (pacote) ficam de fora.

## Fontes

`FontLicense` = `{ family, file, license, copyright }` (`license`: "OFL-1.1" ou, pela emenda proposta de 2026-11-23 da D-15, uma das licenças das fontes do `typst` CLI), de
`src-tauri/fonts/licenses.toml`. Fontes do projeto: lidas de `fonts/`; não auditadas (D-15).

## PackageCacheEntry — cache de pacotes (dados locais do app)

```text
<dados locais>/packages/
├── index.json                    # { "preview/nome/1.0.0": { "sha256": "...", "downloadedAt": "..." } }
└── preview/<nome>/<versão>/      # conteúdo extraído e validado
```

Campos de cada entrada do índice (`PackageCacheEntry`): `spec` (`preview/<nome>/<versão>`,
só namespace `preview`), `sha256` (hash da árvore extraída), `downloadedAt`, `state`
(`Ausente` \| `Baixando` \| `Disponível` \| `Recusado` \| `Adulterado`). Limites do download:
50 MB compactado, 200 MB e 5 000 arquivos extraídos.

Estados por pacote: `Ausente → Baixando → (validação ok) → Disponível`; `(validação falha) →
Recusado`; `Disponível → (hash difere) → Adulterado → (baixar de novo) → Baixando`.

## PluginApproval — aprovação de plugin (dados locais do app)

`plugin-approvals.json`: lista de `{ projectRoot, sha256, decision: "approved"|"rejected",
decidedAt, path }`. Chave = (`projectRoot` canônico, `sha256`). Nunca lida de `.prelo/`.
Arquivo corrompido → tratado como vazio (pede de novo) e reescrito com aviso.

## Cache de conteúdo (RF-51)

Mapa `sha256 → bytes`, mais o cache de imagens decodificadas do próprio Typst (memoizado por
bytes). Invalidação: evento do observador ou gravação muda o hash do arquivo.
