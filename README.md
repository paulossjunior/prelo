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
| `npm run audit` | Só a auditoria de licenças e vulnerabilidades (Rust e npm) |
| `npm run test:rust` · `test:ui` · `test:e2e` | Uma camada de testes |

No Linux sem tela, rode os testes ponta a ponta sob `xvfb-run`.

### Regras de contribuição

- **TDD** (constituição, Princípio VI): teste primeiro, vendo falhar, depois o código. Teste pulado precisa de comentário `// justificativa:` ao lado.
- **Push só com `npm run verify` verde.** O hook `.githooks/pre-push` roda a suíte; não use `--no-verify`.
- **`main` só por pull request** com o CI verde em macOS, Windows e Linux, inclusive para o dono do repositório.
- Licenças permitidas: [`security/licenses-allow.txt`](security/licenses-allow.txt). Exceções de vulnerabilidade: [`security/vuln-exceptions.toml`](security/vuln-exceptions.toml), sempre com motivo e data de revisão.

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
