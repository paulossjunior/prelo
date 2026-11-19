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
  `update_project_compiler_version` (FR-012). `main` muda por `choose_main_file`.
- `main` com `..`, absoluto ou fora da raiz → inválido (aviso, usa padrão).

## OpenFile

| Campo | Tipo | Regra |
|---|---|---|
| `text` | string UTF-8 | Arquivo não UTF-8 → `not_utf8`, não abre |
| `version` | inteiro | Incrementa a cada `edit_file`; edição com versão antiga → `version_mismatch` |
| `dirty` | bool | `true` desde a última edição até gravar |
| `lastSavedHash` | sha256 | Para ignorar o próprio evento do observador de arquivos |

Estados: `Limpo → (edição) → Sujo → (2 s ociosos, ⌘S ou fechar) → Gravando → Limpo`;
`Gravando → (falha) → Sujo` + aviso. Mudança externa: `Limpo → recarrega`; `Sujo →
Conflito → (escolha) → Limpo/Sujo`.

## Compilação

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

`{ level, title, path, line, page, yPt }`, em ordem de documento.

## Fontes

`FontLicense` = `{ family, file, license: "OFL-1.1", copyright }`, de
`src-tauri/fonts/licenses.toml`. Fontes do projeto: lidas de `fonts/`; não auditadas (D-15).

## Cache de pacotes (dados locais do app)

```text
<dados locais>/packages/
├── index.json                    # { "preview/nome/1.0.0": { "sha256": "...", "downloadedAt": "..." } }
└── preview/<nome>/<versão>/      # conteúdo extraído e validado
```

Estados por pacote: `Ausente → Baixando → (validação ok) → Disponível`; `(validação falha) →
Recusado`; `Disponível → (hash difere) → Adulterado → (baixar de novo) → Baixando`.

## Aprovação de plugin (dados locais do app)

`plugin-approvals.json`: lista de `{ projectRoot, sha256, decision: "approved"|"rejected",
decidedAt, path }`. Chave = (`projectRoot` canônico, `sha256`). Nunca lida de `.prelo/`.
Arquivo corrompido → tratado como vazio (pede de novo) e reescrito com aviso.

## Cache de conteúdo (RF-51)

Mapa `sha256 → bytes`, mais o cache de imagens decodificadas do próprio Typst (memoizado por
bytes). Invalidação: evento do observador ou gravação muda o hash do arquivo.
