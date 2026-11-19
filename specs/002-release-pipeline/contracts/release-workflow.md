# Contrato: workflow de release

Arquivo `.github/workflows/release.yml`. Os testes de `scripts/release-workflow.test.ts`
conferem cada regra marcada com **(T)**.

## Gatilho

- `on: push: tags: ['v[0-9]+.[0-9]+.[0-9]+']`; nenhum outro gatilho (sem `workflow_dispatch`,
  `pull_request` ou `push` de branch). **(T)**
- `concurrency: group: release-${{ github.ref }}`, `cancel-in-progress: false`.

## Permissões e segredos

- Topo: `permissions: {}`. **(T)**
- `contents: write` só no job `publish`; `preflight`, `verify` e `build` com exatamente
  `contents: read` (o `verify` precisa declarar, porque o workflow chamado só reduz
  permissões). **(T)**
- Nenhuma referência a `secrets.*` além de `secrets.GITHUB_TOKEN` / `github.token`. **(T)**
- Ações externas fixadas por SHA de 40 caracteres, com a versão em comentário. **(T)**

## Jobs

| Job | Runner | Permissões | `needs` | Faz | Falha quando |
|---|---|---|---|---|---|
| `preflight` | `ubuntu-22.04` | `contents: read` | — | checkout com `fetch-depth: 0`; `node scripts/release-check.mjs "$GITHUB_REF_NAME"` | tag fora do formato, fora da `main`, versão divergente, sem licença (FR-001–003, FR-016) |
| `verify` | (de `ci.yml`) | `contents: read` | `preflight` | `uses: ./.github/workflows/ci.yml` | qualquer etapa do `npm run verify` em qualquer sistema (FR-004) |
| `build` | matriz (abaixo) | `contents: read` | `verify` | `npx tauri build --target <alvo> --bundles <lista>`; `release-artifacts.mjs scan`; `upload-artifact` | build falha; marca `self-test` encontrada (FR-010) |
| `publish` | `ubuntu-22.04` | `contents: write` | `preflight`, `verify`, `build` | `download-artifact`; `release-artifacts.mjs collect` e `sums`; release inexistente; `gh release create --draft`; confere assets; limpeza em falha | conjunto incompleto/excedente; release já existe (FR-008, FR-013) |

`build` usa `fail-fast: true`; o comando de build MUST NOT conter `--features` nem
`self-test`. **(T)**

### Matriz de `build` **(T)**

| `os` | `target` | `bundles` |
|---|---|---|
| `macos-latest` | `universal-apple-darwin` | `dmg` |
| `windows-latest` | `x86_64-pc-windows-msvc` | `nsis` |
| `ubuntu-22.04` | `x86_64-unknown-linux-gnu` | `appimage,deb` |

## Publicação

- `gh release create "$TAG" --draft --verify-tag --title "$TAG" --notes-file <notas> <4 instaladores> SHA256SUMS.txt`. **(T)** `--draft` presente.
- MUST NOT aparecer `--draft=false`, `--latest`, `--clobber` nem `gh release upload`. **(T)**
- Passo de limpeza: `if: failure() && steps.create.outputs.created == 'true'` →
  `gh release delete "$TAG" --yes` (sem `--cleanup-tag`). **(T)**

## `ci.yml`

- Acrescenta `workflow_call:` aos gatilhos; demais gatilhos, permissões (`contents: read`) e
  nomes de job inalterados (os checks obrigatórios da `main` continuam
  `verify (<sistema>)`). **(T)**
