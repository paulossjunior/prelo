# Research: Esqueleto do app desktop Prelo

**Feature**: [spec.md](spec.md) · **Date**: 2026-11-18

Versões consultadas em 2026-11-18 (npm e crates.io). Fixar no lockfile; atualizar por PR.

## R1. Shell e versões

- **Decision**: Tauri 2 (`tauri` 2.12.x, `@tauri-apps/cli` e `@tauri-apps/api` 2.12.x), Rust
  fixado em `rust-toolchain.toml` (1.95), Node 24 LTS em `.nvmrc` com `engines` `>=24`.
- **Rationale**: D-01 já decidiu Tauri; v2 é a linha estável atual, com modelo de capabilities
  (permissões explícitas por janela) que serve ao Princípio VI.
- **Alternatives considered**: Tauri 1 (sem capabilities granulares, fim de suporte); Electron
  (descartado em D-01 pelo núcleo Rust).

## R2. Interface

- **Decision**: Vite 8 + TypeScript, sem framework de UI nesta feature.
- **Rationale**: o esqueleto só mostra versão e sistema. A escolha de framework depende do
  editor e do painel de revisão (fase 1 seguinte) e não deve ser travada agora.
- **Alternatives considered**: Svelte, Solid, React, Vue — adiados para a feature do editor,
  que registrará a escolha como D-xx.

## R3. Testes por camada

| Camada | Ferramenta | Plataformas | O que cobre |
|---|---|---|---|
| Núcleo (unidade) | `cargo test` | 3 | Montagem de `AppInfo`, validação |
| Ponte (contrato IPC) | `cargo test` com `tauri::test` (mock runtime) | 3 | Comando `get_app_info`, recusa de comando desconhecido e argumentos malformados (FR-002, FR-003) |
| Configuração de segurança | `cargo test` lendo `tauri.conf.json` e `capabilities/` | 3 | CSP sem `unsafe-*`, só permissões da lista (FR-007) |
| Interface (unidade) | Vitest 5 + jsdom | 3 | Renderização, carregamento, timeout e erro (FR-004) com o `invoke` simulado |
| Ponta a ponta (WebDriver) | WebdriverIO 10 + `tauri-driver` 2.1 | Linux, Windows | Janela "Prelo" com versão e sistema reais (FR-001, FR-002) |
| Ponta a ponta (autoteste) | binário com feature `self-test` | 3 | Abre o app real, a interface reporta o texto renderizado ao núcleo, o processo sai com código 0/1 (FR-001, FR-005, SC-001) |

- **Decision**: as camadas acima, todas chamadas por um único `npm run verify`.
- **Rationale**: `tauri-driver` não suporta macOS (não há driver WebDriver para WKWebView). O
  autoteste cobre os três sistemas com o app real; o WebDriver dá cobertura de interface mais
  rica onde existe.
- **Alternatives considered**: só WebDriver (deixa macOS sem E2E, viola SC-001); Playwright
  (não dirige a webview do Tauri); testar macOS só manualmente (viola Princípio VI).
- **Restrição de segurança**: a feature `self-test` só compila em build de teste; o pacote de
  release não a inclui (teste em `security_config` confere).

## R4. Auditoria de dependências

- **Decision**:
  - Rust: `cargo-deny` 0.21 (`licenses`, `advisories`, `bans`, `sources`), configurado em
    `src-tauri/deny.toml`.
  - npm: `audit-ci` 7 (falha em `high`/`critical`, allowlist por ID com motivo) e
    `license-checker-rseidelsohn` 5 (`--onlyAllow`).
  - Fonte única das licenças: `security/licenses-allow.txt`. Um teste confere que `deny.toml`
    e o checker npm usam exatamente essa lista.
  - Exceções de vulnerabilidade: `security/vuln-exceptions.toml`, cada item com `id`,
    `reason` e `review_by` (data). Exceção vencida faz a auditoria falhar.
- **Rationale**: `cargo-deny` cobre licenças e RustSec numa ferramenta; `npm audit` puro não
  aceita exceções justificadas, `audit-ci` sim.
- **Alternatives considered**: `cargo-audit` (só vulnerabilidades, sem licenças); Dependabot
  sozinho (alerta, não bloqueia).

### R4.1 Lista inicial de licenças (proposta)

Apache-2.0, MIT, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, 0BSD, CC0-1.0 e
Apache-2.0 WITH LLVM-exception (aprovada em 2026-11-18).

Escopo (decidido em 2026-11-18): licenças npm só nas dependências de produção
(`--production`); ferramentas de desenvolvimento passam só pelo `audit-ci`. Motivo: o primeiro
levantamento achou 25 violações, todas em dev tools que não entram no app (BlueOak-1.0.0,
MPL-2.0 do lightningcss, CC-BY, PSF-2.0, Unlicense, metadados malformados).

`cargo-deny` com `unmaintained = "workspace"`: crate sem manutenção só falha se for
dependência direta. Primeiro caso: `proc-macro-error` (RUSTSEC-2024-0370), transitivo via GTK
no Linux.

Crates MPL-2.0 com exceção (D-11): cssparser, cssparser-macros, selectors, dtoa-short,
option-ext.

**Decidido pelo dono em 2026-11-18 (D-11)**: a árvore de dependências do Tauri historicamente inclui
crates sob **MPL-2.0** (ex.: `cssparser`, `selectors`). MPL-2.0 é copyleft por arquivo: usar
sem modificar não impõe nada ao Prelo, mas não é "permissiva" no sentido estrito de RNF-06.
Aprovado: MPL-2.0 permitida **apenas** como exceção por crate em `deny.toml`
(`[[licenses.exceptions]]`), com motivo, nunca na lista geral. Só entram crates usadas sem
modificação; cada nova crate MPL exige PR com o motivo.

## R5. Trava de push

- **Decision**:
  - Local: `.githooks/pre-push` (shell POSIX, roda no Git Bash do Windows) que chama
    `npm run verify`; ativado por `git config core.hooksPath .githooks` no script `prepare` do
    `npm install`.
  - Remoto: workflow `.github/workflows/ci.yml` com matriz `ubuntu-22.04`, `windows-latest`,
    `macos-latest`, job `verify` estável para servir de check obrigatório.
  - `main`: proteção via `gh api` com PR obrigatório, checks `verify (ubuntu-22.04)`,
    `verify (windows-latest)`, `verify (macos-latest)` obrigatórios, `enforce_admins: true`,
    sem force-push nem exclusão (FR-012, clarificação de 2026-11-18).
- **Rationale**: hook local dá retorno rápido; CI + proteção cobrem quem contorna o hook
  (SC-004). Repositório público: proteção de branch disponível sem plano pago.
- **Alternatives considered**: husky/lefthook (mais uma dependência para um script de 3
  linhas); rulesets do GitHub (equivalentes; proteção clássica é mais simples de auditar
  via `gh api`).

## R6. Qualidade estática

- **Decision**: `cargo fmt --check`, `cargo clippy -- -D warnings` e `tsc --noEmit` dentro de
  `npm run verify`.
- **Rationale**: barato, pega erro antes dos testes; não é exigido pela constituição, mas não
  conflita.

## R7. Limites de tempo

- **Decision**: consulta ao núcleo com timeout de 5 s (FR-004); meta de 3 s para mostrar
  versão (SC-002) medida pelo autoteste.
- **Rationale**: padrões da spec (Assumptions); ajustáveis por PR.

## R8. Dependências de sistema no Linux

- **Decision**: CI instala `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`,
  `librsvg2-dev`, `webkit2gtk-driver`, `xvfb`; README lista os mesmos pacotes.
- **Rationale**: requisitos documentados do Tauri 2 em Linux; `webkit2gtk-driver` e `xvfb`
  para o WebDriver sem tela. Cobre o edge case "sistema sem componente web".
