# Data Model: Pipeline de release dos instaladores

**Feature**: [spec.md](spec.md) · **Date**: 2026-11-19

Sem banco de dados. As entidades são arquivos e objetos do GitHub manipulados pelo workflow.

## Tag de versão

| Campo | Regra |
|---|---|
| `name` | `^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$` (FR-001); sem sufixo de pré-release |
| `version` | `name` sem o `v` |
| `commit` | MUST estar no histórico de `origin/main` (FR-002) |

## Versão declarada

| Fonte | Campo |
|---|---|
| `package.json` | `.version` |
| `src-tauri/tauri.conf.json` | `.version` |
| `src-tauri/Cargo.toml` | `[package].version` |

Regra: os três MUST ser iguais a `Tag.version`; a mensagem de erro nomeia cada fonte
divergente com o valor encontrado (FR-003).

## Arquivo de licença

Um de `LICENSE`, `LICENSE.md`, `LICENSE.txt`, `COPYING` na raiz, não vazio (FR-016).

## Instalador

| Sistema | Arquivo esperado | Origem |
|---|---|---|
| macOS (universal) | `Prelo_<v>_universal.dmg` | `src-tauri/target/universal-apple-darwin/release/bundle/dmg/` |
| Windows (x86_64) | `Prelo_<v>_x64-setup.exe` | `src-tauri/target/release/bundle/nsis/` |
| Linux (x86_64) | `Prelo_<v>_amd64.AppImage` | `src-tauri/target/release/bundle/appimage/` |
| Linux (x86_64) | `Prelo_<v>_amd64.deb` | `src-tauri/target/release/bundle/deb/` |

Regras: o conjunto MUST ser exatamente esses quatro, com `<v>` = `Tag.version` (FR-005,
FR-006, FR-008); o binário do app em cada sistema MUST NOT conter a marca `self_test_report`
(FR-010).

## Arquivo de checksums (`SHA256SUMS.txt`)

Uma linha por instalador, ordem alfabética: `<sha256 hex minúsculo>  <nome do arquivo>`
(dois espaços, formato `sha256sum`) (FR-009).

## Release em rascunho

| Campo | Valor |
|---|---|
| `tag_name` / `name` | `Tag.name` |
| `draft` | `true` (FR-007); nunca alterado pelo workflow |
| `body` | `.github/release-notes.md` com `{{VERSION}}` substituído (FR-014) |
| `assets` | os 4 instaladores + `SHA256SUMS.txt` |

### Estados

```text
(inexistente) --publish cria--> rascunho completo --dono publica (manual)--> publicada
(inexistente) --publish cria--> rascunho incompleto --limpeza if: failure()--> (inexistente)
(existente)   --publish--> falha, sem alteração (FR-013)
```
