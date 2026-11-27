# Data Model: Comentários (004)

Todos os arquivos abaixo são **entrada não confiável** ao ler (Princípio VI; limites em
research.md R5). Nenhum dado desta feature é gravado dentro de um `.typ` (Princípio I).

## ReviewFile — `.prelo/review.json`

```json
{
  "schema": 1,
  "threads": {
    "3f2a…(32 hex)": { "…": "Thread" }
  }
}
```

| Campo | Tipo | Regra |
|---|---|---|
| `schema` | inteiro | `1`. Maior → só leitura (FR-034); ausente/outro → inválido |
| `threads` | objeto id → Thread | ≤ 5.000; chave `^[0-9a-f]{32}$` |
| outros campos | qualquer | preservados ao regravar, nunca interpretados |

Arquivo > 16 MiB, JSON inválido, não UTF-8, link simbólico ou não-arquivo → `review_json_invalid`
(FR-031).

## Thread

| Campo | Tipo | Regra |
|---|---|---|
| `file` | string | caminho relativo à raiz, `/` como separador, sem `..`/absoluto/nulo/`\`; dentro do escopo (FR-033) |
| `anchor.start` / `anchor.end` | string base64 | `StickyIndex` v1 (research R2); ≤ 256 bytes decodificados |
| `quote` | string | ≤ 500 caracteres; texto do trecho na criação |
| `status` | `"open"` \| `"resolved"` | |
| `openedBy` | string | nome, 1–100 caracteres |
| `openedAt` | string | RFC 3339 UTC |
| `statusBy` / `statusAt` | string \| ausente | quem resolveu/reabriu por último e quando (FR-015) |
| `messages` | Message[] | 1–500 itens |

Derivado, não gravado: `detached` (trecho removido; research R2), `range` UTF-16 resolvido,
`section` (outline).

Thread inválida dentro de um arquivo válido → guardada bruta, fora da exibição, regravada
intacta; listada no aviso `review_threads_skipped` (FR-032).

### Estados

```text
        resolve                       (excluir: some do arquivo)
open ───────────▶ resolved
  ▲                  │
  └──── reopen ──────┘
detached = condição sobre open/resolved; "reanchor" troca anchor e limpa detached
```

Responder não muda o estado (FR-003).

## Message

| Campo | Tipo | Regra |
|---|---|---|
| `id` | string | `^[0-9a-f]{32}$`, único na thread |
| `author` | string | 1–100 caracteres |
| `at` | string | RFC 3339 UTC |
| `body` | string | 1–10.000 caracteres após `trim` (FR-021); `""` só se `deleted` |
| `mentions` | string[] | ≤ 50; cada um é nome conhecido no momento da gravação (FR-004) |
| `deleted` | bool | `true` → `body` vazio e `mentions` vazio (FR-017) |

## CrdtState — `.prelo/crdt/<id>.yjs`

| Parte | Regra |
|---|---|
| Linha 1 | JSON `{"schema":1,"path":"<relativo>"}`; `path` igual ao que gera `<id>` (sha256, 16 bytes hex) — senão inválido |
| Resto | update v1 do `yrs::Doc` (`TextRef "content"`, `OffsetKind::Utf16`) |
| Tamanho | ≤ 32 MiB |

Inválido → aviso `crdt_state_invalid`, `Doc` recriado do texto do disco, conversas daquele
arquivo ficam `detached`, arquivo inválido copiado para `<id>.yjs.invalid-<ts>` antes de
regravar.

## ProjectRecord (`.prelo/project.json`) — campo novo

| Campo | Tipo | Regra |
|---|---|---|
| `owner.name` | string | 1–100 caracteres; ausente → "sem dono registrado" (FR-027) |

## Settings (fora do projeto) — `<config>/Prelo/settings.json`

| Campo | Tipo | Regra |
|---|---|---|
| `displayName` | string | 1–100 caracteres, sem caracteres de controle; obrigatório antes de comentar (FR-005) |

## Permissões (FR-017 a FR-020; docs/requisitos.md §1)

| Ação | Permitido se |
|---|---|
| criar, responder, resolver, reabrir, reancorar | `displayName` definido |
| apagar mensagem | `message.author == displayName` |
| excluir conversa | `thread.openedBy == displayName` ou `displayName == owner.name` |
| excluir todos os resolvidos | `displayName == owner.name` |
