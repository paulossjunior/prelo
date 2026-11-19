# Prelo

Editor desktop para escrever artigos científicos em [Typst](https://typst.app) a várias mãos: sugestões que se aceitam ou rejeitam, comentários em conversa, edição em tempo real sem servidor, trabalho offline e histórico no git.

> **Estado:** esqueleto do app (feature [001](specs/001-app-skeleton/spec.md)): janela Tauri que mostra versão e sistema vindos do núcleo, suíte de testes, auditoria de dependências e trava de push. Editor, compilador e rede vêm nas próximas features.

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
