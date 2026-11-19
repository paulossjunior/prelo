# Quickstart: validar o pipeline de release

Guia de validação ponta a ponta. Regras em [contracts/](contracts/), dados em
[data-model.md](data-model.md).

## Pré-requisitos

- Ambiente da spec 001 (`npm install`, `npm run verify` verde).
- `gh` autenticado com permissão de escrita no repositório (só o dono cria tags).
- Para o caminho feliz (cenário 1): arquivo de licença na raiz (`LICENSE-MIT` e
  `LICENSE-APACHE` já estão na `main`; research R10).

## Local

| # | Passo | Resultado esperado | Cobre |
|---|---|---|---|
| L1 | `npx vitest run --project scripts` | Testes de `release-check`, `release-artifacts` e `release-workflow` verdes | FR-001–FR-016 (forma e lógica) |
| L2 | `node scripts/release-check.mjs v0.1.0` | Ok se houver licença e as versões forem 0.1.0; senão mensagem `release:` | FR-003, FR-016 |
| L3 | `node scripts/release-check.mjs v0.1` | Falha: fora do formato | FR-001 |
| L4 | `npx tauri build --bundles <do sistema>` e `node scripts/release-artifacts.mjs scan <binário>` | Sem marca `self-test` | FR-010 |

## No GitHub

Usar versões de teste (ex.: `0.0.900`) e apagar depois rascunho e tag:
`gh release delete vX.Y.Z --yes; git push origin :refs/tags/vX.Y.Z`.

| # | Passo | Resultado esperado | Cobre |
|---|---|---|---|
| 1 | PR que muda os 3 arquivos para `0.0.900`, merge, `git tag v0.0.900 origin/main && git push origin v0.0.900` | Rascunho `v0.0.900` com `.dmg`, `.exe`, `.AppImage`, `.deb` e `SHA256SUMS.txt` em ≤ 60 min; invisível sem login | US1, SC-001, SC-003 |
| 2 | Baixar os arquivos e `sha256sum -c SHA256SUMS.txt` (ou `shasum -a 256 -c`) | Todos `OK` | US3, SC-004 |
| 3 | Instalar e abrir em cada sistema seguindo só as notas | App abre após o alerta de não assinado | SC-006 |
| 4 | Tag `v0.0.901` num commit de branch fora da `main` | `preflight` falha: commit não pertence à main; sem rascunho | US2-1, FR-002 |
| 5 | Tag `v0.0.902` na `main` com versões `0.0.900` | `preflight` falha nomeando os 3 arquivos | US2-2, FR-003 |
| 6 | Tag válida sem arquivo de licença | `preflight` falha: falta licença | US2-5, FR-016 |
| 7 | Repetir o push da tag do cenário 1 (apagar e recriar a tag, mantendo o rascunho) | `publish` falha: release já existe; rascunho intacto | FR-013 |
| 8 | Tag `v0.0.903-rc.1` | Nenhum workflow de release roda | US2-4, FR-001 |
| 9 | Ver a execução do cenário 1 | Só `publish` com `contents: write`; jobs na ordem preflight → verify → build → publish | FR-011, US4 |
