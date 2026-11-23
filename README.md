# Prelo

Editor desktop para escrever artigos científicos em [Typst](https://typst.app) a várias mãos: sugestões que se aceitam ou rejeitam, comentários em conversa, edição em tempo real sem servidor, trabalho offline e histórico no git.

> **Estado:** editor local (feature [003](specs/003-editor-local/spec.md)): abrir ou criar um projeto, editar com prévia ao vivo, saltar entre código e prévia, exportar PDF, sem `typst` instalado. Colaboração, revisão e git vêm nas próximas features.

## Conteúdo

| Caminho | O que é |
|---|---|
| [`docs/especificacao.md`](docs/especificacao.md) | Visão geral: problema, alternativas, escopo, plano e pendências |
| [`docs/requisitos.md`](docs/requisitos.md) | Requisitos: papéis, RF e RNF, regras de revisão e sincronização, fluxos |
| [`docs/arquitetura.md`](docs/arquitetura.md) | Arquitetura: componentes, conexão sem servidor, modelo de revisão, sincronização, acesso a arquivos, riscos técnicos |
| [`docs/adrs.md`](docs/adrs.md) | Decisões de arquitetura D-01 a D-14 |
| [`specs/`](specs/) | Features no fluxo [spec-kit](https://github.com/github/spec-kit): spec, plano, tarefas |
| [`prototype/index.html`](prototype/index.html) | Protótipo clicável da interface. Abra no navegador |
| `src/` | Interface (Vite + TypeScript) |
| `src-tauri/` | Núcleo (Rust, Tauri 2) |

## Usar o Prelo com um projeto Typst

O Prelo abre uma pasta de projeto e compila com o Typst **0.15.1** embutido no app. Não é preciso instalar o `typst`. O `.typ` continua sendo Typst puro: o mesmo projeto compila fora do Prelo (Princípio I da [constituição](.specify/memory/constitution.md)).

### Fontes

O Prelo só usa duas fontes de fontes (decisão [D-15](docs/adrs/D-15-fontes-embutidas-e-do-projeto.md)). Nenhuma fonte instalada no sistema é lida.

1. **Fontes embutidas:** a família [Libertinus](https://github.com/alerque/libertinus) 7.051 (Serif, Sans, Math e Mono), sob a licença SIL Open Font License 1.1. Os arquivos, o [`OFL.txt`](src-tauri/fonts/OFL.txt) e o aviso de copyright de cada um ([`licenses.toml`](src-tauri/fonts/licenses.toml)) ficam em [`src-tauri/fonts/`](src-tauri/fonts/). A tela **Sobre** lista essas fontes com a licença. A auditoria (`npm run audit`) falha se houver arquivo de fonte sem entrada no `licenses.toml` ou com licença fora da lista permitida.
2. **Fontes do projeto:** todo `.ttf`, `.otf`, `.ttc` e `.otc` dentro de `fonts/` na raiz do projeto, inclusive em subpastas (até 16 níveis e 2 000 arquivos). Fonte malformada vira diagnóstico e não derruba a compilação.

Quando o `.typ` não pede fonte, matemática usa Libertinus Math e blocos de código usam Libertinus Mono. O Prelo define esse padrão sem escrever nada no `.typ`.

### Compilar fora do Prelo

Com o `typst` 0.15.1 instalado, na raiz do projeto:

```sh
typst compile --font-path fonts main.typ
```

O `--font-path fonts` faz o `typst` achar as fontes do projeto. Sem as fontes embutidas do Prelo, o `typst` usa as dele (New Computer Modern Math e DejaVu Sans Mono) em matemática e código, e o PDF pode ficar diferente. Para obter o mesmo PDF do Prelo, aponte também para as fontes embutidas e ignore as outras:

```sh
typst compile --ignore-system-fonts --ignore-embedded-fonts \
  --font-path <prelo>/src-tauri/fonts --font-path fonts main.typ
```

Num documento que escolhe todas as fontes (texto, `math.equation` e `raw`) e as traz em `fonts/`, o primeiro comando já dá o mesmo resultado. O CI confere a equivalência página a página nos três sistemas (job `equivalence`, [`src-tauri/tests/equivalence.rs`](src-tauri/tests/equivalence.rs)).

### Pacotes `@preview`

Um `#import "@preview/nome:versão"` baixa o pacote de `https://packages.typst.org` na primeira vez. Só nome e versão saem na URL. O download usa TLS com os certificados do sistema.

- **Cache:** `<dados locais do app>/packages/preview/<nome>/<versão>/`, separado do cache do `typst` de linha de comando. O índice `packages/index.json` guarda o SHA-256 de cada pacote. Na primeira vez que um pacote é usado em cada sessão, o Prelo confere o hash; se o conteúdo mudou, o pacote é recusado como adulterado e o diagnóstico oferece **Baixar de novo**.
- **Sem rede:** pacote já no cache funciona normalmente. Sem rede e sem cache, o diagnóstico cita o pacote.
- **Recusados na extração:** caminho absoluto, `..`, link simbólico ou físico, dispositivo, FIFO, e arquivos acima dos limites (tabela abaixo). Qualquer recusa descarta o pacote inteiro.
- Pacotes `@local` e outros namespaces não são aceitos nesta versão.

### Plugins WASM

Um arquivo WASM (do projeto ou de um pacote) só chega ao compilador depois que você aprova. Antes disso, a compilação mostra "plugin aguardando confirmação" e o app pergunta, com caminho, origem e tamanho. Aprovado, o plugin roda no sandbox do Typst, sem acesso a arquivo, rede ou relógio.

- A aprovação vale para aquela pasta de projeto e aquele conteúdo (SHA-256). Se o arquivo mudar, o Prelo pergunta de novo.
- As decisões ficam em `<dados locais do app>/plugin-approvals.json`, nunca dentro do projeto: um coautor não consegue pré-aprovar plugin para você. Para desfazer todas as decisões, feche o app e apague esse arquivo.

`<dados locais do app>` é, por sistema:

| Sistema | Pasta |
|---|---|
| macOS | `~/Library/Application Support/org.prelo.app/` |
| Windows | `%LOCALAPPDATA%\org.prelo.app\` |
| Linux | `~/.local/share/org.prelo.app/` |

### Limites

| O quê | Limite | Ao passar |
|---|---|---|
| Arquivos lidos pelo compilador | só dentro da pasta do projeto, sem seguir link para fora | acesso recusado, com diagnóstico |
| Tamanho de arquivo aberto ou lido | 256 MiB | recusado, com diagnóstico |
| Estilo `.csl` | 1 MiB | "estilo grande demais" |
| `.prelo/project.json` | 1 MiB | aviso; o projeto abre com os padrões |
| Tempo de uma compilação | 10 s | compilação abandonada com diagnóstico; a prévia mantém a última versão boa |
| Compilações presas (plugin que não retorna) | 3 threads por projeto aberto | a prévia fica suspensa, com diagnóstico, até uma delas terminar; desfaça a chamada e reabra o Prelo |
| Pacote `@preview` | 50 MB compactado, 200 MB e 5 000 arquivos extraídos | pacote recusado |
| Salvamento automático | até 1,5 s depois da primeira alteração não gravada | um encerramento forçado perde no máximo os últimos 2 s de digitação |

## Desenvolvimento

### Pré-requisitos

| Ferramenta | Versão |
|---|---|
| Rust | a de [`rust-toolchain.toml`](rust-toolchain.toml) (instalada pelo `rustup`) |
| Node.js | 24 ou mais nova ([`.nvmrc`](.nvmrc)), com npm |
| cargo-deny | `cargo install cargo-deny --locked` |
| tauri-driver | Linux e Windows: `cargo install tauri-driver --locked` |

Por sistema:

- **macOS:** Xcode Command Line Tools (`xcode-select --install`).
- **Windows:** Microsoft C++ Build Tools e WebView2 (já vem no Windows 11). Para o WebDriver: `msedgedriver` da mesma versão do Edge no `PATH`.
- **Linux (Debian/Ubuntu):**

  ```sh
  sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev \
    build-essential curl wget file libssl-dev libxdo-dev webkit2gtk-driver xvfb
  ```

  Sem `libwebkit2gtk-4.1` o app não abre.

### Comandos

| Comando | Faz |
|---|---|
| `npm install` | Instala dependências e ativa a trava de push (`git config core.hooksPath .githooks`) |
| `npm run dev` | Abre o app em modo desenvolvimento |
| `npm run build` | Gera o pacote do sistema atual em `src-tauri/target/release/bundle/` |
| `npm run verify` | Tudo: formatação, clippy, tsc, testes do núcleo, da interface e ponta a ponta, auditoria. Código 0 só se tudo passar |
| `npm run verify -- --quick` | Igual, sem o build e os testes ponta a ponta. É o que o hook roda em push de `feature/*` e `dev` |
| `npm run audit` | Só a auditoria de licenças e vulnerabilidades (Rust e npm) |
| `npm run test:rust` · `test:ui` · `test:e2e` | Uma camada de testes |

No Linux sem tela, rode os testes ponta a ponta sob `xvfb-run`.

### Regras de contribuição

- **TDD** (constituição, Princípio VI): teste primeiro, vendo falhar, depois o código. Teste pulado precisa de comentário `// justificativa:` ao lado.
- **Gitflow:** `feature/NNN-nome` sai da `dev` e volta para ela por pull request. A `dev` vai para a `main` por pull request quando houver uma versão para publicar.

  | Branch | Entra por | Hook de pre-push | CI exigido |
  |---|---|---|---|
  | `feature/*` | (trabalho) | `verify --quick` | (nenhum; roda no PR) |
  | `dev` | PR de `feature/*` | `verify --quick` | `verify (macos-latest)` |
  | `main` | PR da `dev` | `verify` completo | `verify` nos 3 sistemas |
  | tag `vX.Y.Z` | (na `main`) | `verify` completo | release nos 3 sistemas |

- **Push só com o verify verde.** O hook `.githooks/pre-push` escolhe o modo pelo destino; não use `--no-verify`.
- **`main` e `dev` só por pull request**, inclusive para o dono do repositório.
  A proteção está em [`.github/branch-protection.json`](.github/branch-protection.json); para reaplicar:
  `gh api -X PUT repos/paulossjunior/prelo/branches/main/protection --input .github/branch-protection.json`.
  A da `dev` está em [`.github/branch-protection-dev.json`](.github/branch-protection-dev.json):
  `gh api -X PUT repos/paulossjunior/prelo/branches/dev/protection --input .github/branch-protection-dev.json`.
- Ao contribuir, você licencia sua contribuição sob a mesma licença do projeto (`MIT OR Apache-2.0`), sem CLA.
- Licenças permitidas: [`security/licenses-allow.txt`](security/licenses-allow.txt). Exceções de vulnerabilidade: [`security/vuln-exceptions.toml`](security/vuln-exceptions.toml), sempre com motivo e data de revisão.

### Publicar uma versão

Uma tag `vX.Y.Z` num commit da `main` dispara [`.github/workflows/release.yml`](.github/workflows/release.yml) (feature [002](specs/002-release-pipeline/spec.md)). Ele gera os instaladores (`.dmg` universal, `.exe` x64, `.AppImage` e `.deb` x64) e cria uma release em **rascunho**. Nada fica público sem você publicar.

1. Num PR, atualize a versão para `X.Y.Z` nos três arquivos: `package.json`, `src-tauri/tauri.conf.json` e `src-tauri/Cargo.toml` (rode `cargo check --manifest-path src-tauri/Cargo.toml` para atualizar o `Cargo.lock`). Mescle o PR na `main`.
2. Crie e envie a tag (só o dono):

   ```sh
   git fetch origin
   git tag vX.Y.Z origin/main && git push origin vX.Y.Z
   ```

3. Acompanhe a execução em **Actions > Release** (até cerca de 60 min). A ordem é `preflight` → `verify` (o mesmo CI dos PRs, nos três sistemas) → `build` → `publish`. O `preflight` recusa tag fora do formato `vX.Y.Z`, commit fora da `main`, versão diferente em algum dos três arquivos e repositório sem arquivo de licença na raiz (`LICENSE-MIT`/`LICENSE-APACHE`). Se já houver release ou rascunho com o mesmo nome, o `publish` falha sem alterá-lo. Se qualquer etapa falhar, nenhum rascunho fica para trás.
4. Revise o rascunho em **Releases**: quatro instaladores, `SHA256SUMS.txt` e as notas, que explicam como abrir o app não assinado e conferir o checksum. Se quiser, baixe e confira com `sha256sum -c SHA256SUMS.txt --ignore-missing` (Linux) ou `shasum -a 256 -c SHA256SUMS.txt --ignore-missing` (macOS).
5. Publique o rascunho pelo botão **Publish release**. O workflow nunca publica sozinho.

Para refazer uma versão que falhou, apague o rascunho (se houver) e a tag (`gh release delete vX.Y.Z --yes; git push origin :refs/tags/vX.Y.Z`), corrija na `main` e crie a tag de novo.

## Ideias principais

- **Typst puro no git.** O `.typ` do repositório é sempre compilável fora do Prelo. Sugestões e comentários ficam em `.prelo/`.
- **Revisão no estilo do Overleaf.** Modos Editando, Sugerindo e Lendo; aceitar e rejeitar por sugestão ou em lote; comentários com respostas, menções, resolver e excluir.
- **Sem servidor obrigatório.** Conexão direta entre os computadores dos coautores (mDNS na rede local, iroh pela internet). O git cobre quem não está online ao mesmo tempo.
- **Offline com regra clara.** Offline se comenta e sugere. Editar direto só com os arquivos sincronizados.
- **Salvar sincroniza.** ⌘S recebe as mudanças dos outros, envia as suas e grava um commit.

## Tecnologia

Tauri · Rust · CodeMirror 6 · Yjs / yrs · Typst + Tinymist · iroh · gitoxide

## Nome

Prelo é a prensa tipográfica. Um texto "no prelo" está prestes a ser publicado, que é a fase do artigo que a ferramenta atende.

## Licença

O Prelo é distribuído sob a licença MIT ou a Apache-2.0, à sua escolha: [`LICENSE-MIT`](LICENSE-MIT) e [`LICENSE-APACHE`](LICENSE-APACHE).
