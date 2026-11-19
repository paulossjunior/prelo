# Research: Pipeline de release dos instaladores

**Feature**: [spec.md](spec.md) · **Date**: 2026-11-19

Versões das ações e do Tauri seguem as já usadas na spec 001 (`.github/workflows/ci.yml`,
`package.json`). Atualizar por PR.

## R1. Estrutura do workflow

- **Decision**: novo workflow `.github/workflows/release.yml`, disparado por
  `push: tags: ['v[0-9]+.[0-9]+.[0-9]+']`, com quatro jobs em sequência:
  `preflight` → `verify` → `build` (matriz de 3 sistemas) → `publish`.
  `publish` tem `needs: [preflight, verify, build]`; qualquer falha anterior impede a
  publicação.
- **Rationale**: separar validação, build e publicação permite dar escrita só ao último job
  (FR-011) e garante, pelo `needs`, que nada é publicado sem todos os instaladores (FR-008).
- **Alternatives considered**: um job por sistema que já cria a release (padrão do
  `tauri-action`): exigiria `contents: write` nos três jobs de build e deixaria rascunho
  parcial se um sistema falhasse. Workflow único com `if:` por etapa: mais difícil de testar.

## R2. Reaproveitar o `verify` da spec 001

- **Decision**: acrescentar `workflow_call:` aos gatilhos de `ci.yml` e chamá-lo em
  `release.yml` (`verify: uses: ./.github/workflows/ci.yml`). O `release.yml` usa grupo de
  concorrência próprio (`release-${{ github.ref }}`), diferente do `ci-…` do `ci.yml`.
- **Rationale**: uma única definição do `verify` nos três sistemas (FR-004); mudança no CI
  vale para a release sem duplicação. Os nomes dos checks de PR (`verify (<sistema>)`) não
  mudam, porque em PR o `ci.yml` continua rodando direto.
- **Alternatives considered**: copiar o job `verify` para `release.yml` (duplicação que
  diverge com o tempo); confiar no CI do merge na `main` (a tag pode apontar para commit
  antigo e as dependências externas e avisos de vulnerabilidade mudam).

## R3. Pré-checagem (`preflight`)

- **Decision**: script `scripts/release-check.mjs <tag>`, rodado num job `ubuntu-22.04` com
  `contents: read` e `fetch-depth: 0`. Checa, nesta ordem, e para no primeiro erro com
  mensagem clara:
  1. formato estrito `^v(\d+)\.(\d+)\.(\d+)$` (FR-001; o filtro de tag do GitHub não é regex
     completa, então a checagem é repetida);
  2. commit da tag no histórico de `origin/main` (`git merge-base --is-ancestor`) (FR-002);
  3. versão igual em `package.json`, `src-tauri/tauri.conf.json` e `src-tauri/Cargo.toml`,
     e igual à tag; nomeia o arquivo divergente (FR-003);
  4. arquivo de licença na raiz (`LICENSE`, `LICENSE.md`, `LICENSE.txt`, `COPYING`), não vazio
     (FR-016).
  As funções são exportadas e testadas com Vitest (padrão de `scripts/verify.mjs`).
- **Rationale**: Node roda igual em qualquer sistema e já é a linguagem dos scripts do
  projeto; falhar em segundos, antes dos ~40 min de verify e build.
- **Alternatives considered**: checagens inline em bash no YAML (não testáveis por unidade);
  regras de proteção de tag do GitHub (desejáveis, mas não substituem a checagem; fora do
  escopo, ver Assumptions da spec).

## R4. Build dos instaladores

- **Decision**: em cada job da matriz, `npx tauri build` direto, sem `tauri-action`:

  | Runner | Alvo | `--bundles` | Saída |
  |---|---|---|---|
  | `macos-latest` | `universal-apple-darwin` | `dmg` | `Prelo_X.Y.Z_universal.dmg` |
  | `windows-latest` | x86_64 (padrão) | `nsis` | `Prelo_X.Y.Z_x64-setup.exe` |
  | `ubuntu-22.04` | x86_64 (padrão) | `appimage,deb` | `Prelo_X.Y.Z_amd64.AppImage`, `Prelo_X.Y.Z_amd64.deb` |

  No macOS, `rustup target add aarch64-apple-darwin x86_64-apple-darwin` antes do build.
  Linux em `ubuntu-22.04` (glibc mais antiga = AppImage roda em mais distribuições), com as
  mesmas dependências de sistema do CI. Nenhum `--features` é passado. Cada job envia os
  arquivos com `actions/upload-artifact@v4` (artefato do workflow, não da release).
- **Rationale**: decisões do dono (Clarifications 2026-11-19: universal, x86_64, só NSIS).
  Construir sem publicar mantém os jobs de build só com leitura.
- **Alternatives considered**: `tauri-action` (publica de dentro do build, exige escrita);
  `targets: "all"` padrão do `tauri.conf.json` (geraria `.msi`, `.app` solto e `.rpm`, fora do
  combinado; por isso `--bundles` explícito na linha de comando, sem mexer no
  `tauri.conf.json` usado no dia a dia).

## R5. Garantir que não sobra rascunho parcial

- **Decision**: três camadas.
  1. `publish` só roda se `preflight`, `verify` e as três pernas de `build` passarem (`needs`).
  2. Antes de criar, `scripts/release-artifacts.mjs` confere o conjunto completo e exato de
     instaladores (1 `.dmg`, 1 `.exe`, 1 `.AppImage`, 1 `.deb`, todos com a versão da tag) e
     gera `SHA256SUMS.txt`; faltou ou sobrou arquivo → falha antes de tocar na release.
  3. `gh release view <tag>` precisa falhar (release inexistente, inclusive rascunho, que só
     é visível com escrita; por isso a checagem fica no `publish`) (FR-013). Depois,
     `gh release create <tag> --draft --verify-tag --title <tag> --notes-file <notas> <arquivos>`
     num só comando. Se esse passo ou a conferência posterior dos assets anexados falhar, um
     passo `if: failure() && steps.create.outputs.created == 'true'` apaga o rascunho criado
     **nesta execução** (`gh release delete <tag> --yes`, sem apagar a tag).
- **Rationale**: atende FR-008 e US1 cenário 3 mesmo com falha de rede no upload; nunca apaga
  release preexistente, porque a limpeza só age no rascunho que a própria execução criou.
- **Alternatives considered**: criar o rascunho cedo e anexar a cada build (rascunho parcial
  visível ao dono em caso de falha); `--clobber` para reaproveitar rascunho existente (viola
  FR-013).

## R6. Permissões e segredos

- **Decision**: `permissions: {}` no topo do `release.yml`; `contents: read` em `preflight`,
  `verify` e `build`; `contents: write` só em `publish`. Único segredo: `GITHUB_TOKEN`
  automático (`GH_TOKEN: ${{ github.token }}`). Nenhuma variável de assinatura
  (`APPLE_*`, `TAURI_SIGNING_*`, certificados) (FR-011, FR-012). Ações externas do
  `release.yml` fixadas por SHA de commit, com a versão em comentário.
- **Rationale**: menor privilégio; o job com escrita usa só `checkout`, `download-artifact` e
  o `gh` pré-instalado. Fixar por SHA evita que uma tag movida de terceiro rode com escrita.
- **Alternatives considered**: `contents: write` no topo (mais simples, viola FR-011); fixar
  por tag como no `ci.yml` (aceitável num job só de leitura; aqui há escrita).

## R7. Feature `self-test` fora da release

- **Decision**: três garantias, todas testadas.
  1. O comando de build do `release.yml` não contém `self-test` nem `--features` (teste do
     workflow, R8).
  2. A trava `compile_error!` já existente em `src-tauri/src/lib.rs` impede `self-test` em
     build de release (teste `self_test_nao_compila_em_release` da spec 001).
  3. Após o build, `scripts/release-artifacts.mjs scan` procura no binário do app a marca do
     comando de autoteste (`self_test_report`) e falha se encontrar (FR-010, SC-005); a função
     de busca é testada com binários falsos com e sem a marca.
- **Rationale**: a 1 e a 2 previnem; a 3 confere o artefato real que vai para quem baixa.
- **Alternatives considered**: só a trava de compilação (não confere o artefato final).

## R8. Como testar o workflow

- **Decision**: testes em Vitest (projeto `scripts`, já incluso em `npm run verify`):
  - `scripts/release-workflow.test.ts`: lê `release.yml` e `ci.yml` com o pacote `yaml` e
    confere gatilho só por tag no padrão; `permissions: {}` no topo e escrita só em
    `publish`; `needs` de `publish`; `--draft` presente e `gh release edit --draft=false` /
    `--latest` ausentes; nenhum `secrets.*` além de `GITHUB_TOKEN`; nenhum `--features` /
    `self-test` no build; matriz e `--bundles` iguais a R4; ações externas fixadas por SHA;
    `ci.yml` aceita `workflow_call`.
  - `scripts/release-check.test.ts`: formato da tag, versões divergentes por arquivo, licença
    ausente ou vazia e ancestralidade na `main` com repositórios git temporários (padrão de
    `scripts/hooks.test.ts`).
  - `scripts/release-artifacts.test.ts`: conjunto completo, faltando e sobrando; formato e
    valores do `SHA256SUMS.txt`; arquivo adulterado não confere; busca da marca
    `self_test_report`.
  - Validação real em GitHub (quickstart): tags de teste válidas e inválidas, e exclusão dos
    rascunhos e tags de teste depois.
- **Rationale**: o que dá para testar localmente (lógica dos scripts e forma do workflow)
  fica em TDD; o comportamento do GitHub em si só se prova numa execução real.
- **Alternatives considered**: `act` (não roda runners macOS e Windows nem `gh release` real);
  `actionlint` (útil, mas não confere as regras de negócio; pode entrar depois como etapa
  extra).

## R9. Checksums e notas

- **Decision**: `SHA256SUMS.txt` no formato do `sha256sum` (`<hex>  <arquivo>`, uma linha por
  instalador, ordem alfabética), conferível com `sha256sum -c` (Linux), `shasum -a 256 -c`
  (macOS) e `Get-FileHash` (Windows). Notas em `.github/release-notes.md`, com `{{VERSION}}`
  substituído no `publish`; explicam app não assinado (Gatekeeper: "Abrir" pelo menu de
  contexto ou Ajustes > Privacidade; SmartScreen: "Mais informações" > "Executar assim
  mesmo") e a conferência do checksum nos três sistemas (FR-014).
- **Rationale**: formato padrão que as ferramentas nativas leem.
- **Alternatives considered**: um `.sha256` por instalador (mais arquivos para o usuário
  achar); checksums só no texto das notas (não conferível por ferramenta).

## R10. Pendência que bloqueava a primeira release real (resolvida)

- Resolvida em 2026-11-19 (PR #37): a `main` tem `LICENSE-MIT` e `LICENSE-APACHE`, e
  `package.json` e `Cargo.toml` declaram `MIT OR Apache-2.0`. O `preflight` aceita esses dois
  nomes além de `LICENSE`, `LICENSE.md`, `LICENSE.txt` e `COPYING` (data-model). O caminho
  feliz (quickstart, cenário 1) não depende mais de decisão do dono; o cenário 6 (sem licença)
  só se valida localmente ou num commit de teste sem os arquivos.
