# Contrato: scripts de release

Scripts Node (`.mjs`) com funções exportadas e tipos em `.d.mts`, no padrão de
`scripts/verify.mjs`. Saem com código 0 em sucesso e 1 em falha, com mensagem em português
começando por `release:` no stderr.

## `scripts/release-check.mjs <tag>`

Rodado no job `preflight`, na raiz do repositório, com o commit do `HEAD`. Com a tag fora do
formato, para aí; senão confere todas as regras e lista cada falha (ex.: os três arquivos
divergentes de uma vez).

| Função exportada | Entrada | Retorno / erro |
|---|---|---|
| `parseTag(tag)` | `string` | `{ version }` ou erro `tag "<tag>" fora do formato vX.Y.Z` |
| `isOnMain(commit, cwd)` | SHA, diretório | `boolean` (via `git merge-base --is-ancestor <commit> origin/main`) |
| `readVersions(root)` | raiz | `[{ file, version }]` para os 3 arquivos |
| `versionMismatches(version, versions)` | versão, lista | lista de `{ file, version }` divergentes |
| `findLicense(root)` | raiz | caminho do arquivo ou `null` (vazio conta como ausente) |
| `checkRelease({ tag, root, commit })` | — | lista de erros (vazia = ok) |

Mensagens (uma por falha):

- `release: tag "<tag>" fora do formato vX.Y.Z`
- `release: o commit <sha> da tag não pertence à main`
- `release: <arquivo> declara versão <x>, mas a tag é <tag>`
- `release: falta arquivo de licença na raiz (LICENSE); escolha a licença antes de publicar`

## `scripts/release-artifacts.mjs <comando>`

| Comando | Uso | Faz | Falha quando |
|---|---|---|---|
| `scan <binário>` | job `build` | procura a marca `self_test_report` no binário | marca encontrada |
| `collect <dir> <versão> <saída>` | job `publish` | busca recursiva em `<dir>`, copia para `<saída>` os instaladores esperados | falta ou sobra instalador (ver data-model) |
| `sums <dir>` | job `publish` | grava `<dir>/SHA256SUMS.txt` | diretório vazio |

| Função exportada | Retorno |
|---|---|
| `expectedInstallers(version)` | os 4 nomes de [data-model.md](../data-model.md) |
| `matchInstallers(files, version)` | `{ found, missing, extra }` |
| `sha256File(path)` | hex minúsculo |
| `formatSums(entries)` | texto no formato `sha256sum`, ordem alfabética, `\n` final |
| `verifySums(text, dir)` | lista de arquivos cujo hash não confere |
| `containsSelfTestMarker(buffer)` | `boolean` |
