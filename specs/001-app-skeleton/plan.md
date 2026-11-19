# Implementation Plan: Esqueleto do app desktop Prelo

**Branch**: `001-app-skeleton` | **Date**: 2026-11-18 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/001-app-skeleton/spec.md` · Issue
[#1](https://github.com/paulossjunior/prelo/issues/1)

## Summary

App Tauri 2 mínimo: uma janela "Prelo" cuja interface (Vite + TypeScript) pede ao núcleo Rust,
pelo comando IPC `get_app_info`, a versão e o sistema, com carregamento, timeout e erro. Em
volta, a infraestrutura que o Princípio VI exige desde o primeiro código: suíte em camadas
(núcleo, contrato IPC, configuração de segurança, interface, ponta a ponta por WebDriver e por
autoteste), auditoria de licenças e vulnerabilidades com fonte única de licenças e exceções com
validade, `npm run verify` como comando único, hook pre-push, CI nos três sistemas e `main`
protegida com PR obrigatório para todos. Detalhes em [research.md](research.md).

## Technical Context

**Language/Version**: Rust 1.95 (fixado em `rust-toolchain.toml`); TypeScript 7 sobre Node 24 LTS

**Primary Dependencies**: Tauri 2.12 (`tauri`, `tauri-build`, `@tauri-apps/api`, `@tauri-apps/cli`), Vite 8

**Storage**: N/A (FR-014)

**Testing**: `cargo test` (incl. `tauri::test` mock runtime), Vitest 5 + jsdom, WebdriverIO 10 + `tauri-driver` 2.1 (Linux/Windows), autoteste `self-test` (3 sistemas); auditoria com `cargo-deny` 0.21, `audit-ci` 7, `license-checker-rseidelsohn` 5

**Target Platform**: macOS (arm64 e x86_64), Windows 10/11 x64, Linux x64 com WebKitGTK 4.1

**Project Type**: desktop-app

**Performance Goals**: versão e sistema visíveis em ≤ 3 s após abrir (SC-002)

**Constraints**: timeout de 5 s na consulta ao núcleo; nenhuma permissão de janela além do IPC do contrato; CSP sem `unsafe-*`; nenhum código de editor, compilador ou rede (FR-014)

**Scale/Scope**: 1 janela, 1 comando IPC (+1 só em build de teste), ~15 arquivos de código e configuração

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Princípio / regra | Situação | Como |
|---|---|---|
| I. Typst puro no repositório | N/A | Sem manipulação de `.typ` |
| II. Privacidade sem servidor | ✅ | Nenhuma permissão de rede ou HTTP na janela; teste de configuração confere |
| III. Nenhum trabalho perdido | N/A | Sem dados do usuário |
| IV. Edição só sincronizada | N/A | Sem edição |
| V. Revisão explícita | N/A | Sem revisão |
| VI. TDD | ✅ | `tasks.md` põe cada teste antes do código; QA confere pelo histórico |
| VI. Testes de segurança | ✅ | Comando desconhecido e argumentos malformados recusados; capabilities mínimas; CSP; `self-test` ausente no release; auditoria |
| VI. Auditoria de dependências | ✅ | `cargo-deny` + `audit-ci` + checker de licenças, alto/crítico bloqueia |
| Restrições: stack | ✅ | Tauri + Rust, conforme D-01 |
| Restrições: plataformas | ✅ | CI nos três sistemas |
| Restrições: licenças (RNF-06) | ✅ | MPL-2.0 transitiva só por exceção por crate (D-11, aprovado); ver Complexity Tracking |
| Fluxo: push só com suíte verde | ✅ | Hook + CI + proteção com `enforce_admins` |
| Fluxo: feature cita requisitos | ✅ | D-01, RNF-01, RNF-06, Princípio VI |
| Fluxo: ordem das fases | ✅ | Primeira entrega da fase 1 |

**Resultado pré-pesquisa**: aprovado; exceção de licenças aprovada pelo dono (D-11).
**Re-checagem pós-design** (após contracts e data model): sem mudança; nenhuma violação nova.

## Project Structure

### Documentation (this feature)

```text
specs/001-app-skeleton/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── ipc.md
│   └── dev-workflow.md
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
package.json            # scripts dev, build, verify, audit, test:*; prepare ativa .githooks
package-lock.json
tsconfig.json
vite.config.ts
vitest.config.ts
index.html
rust-toolchain.toml
.nvmrc
src/                    # interface
├── main.ts             # monta a tela
├── app-info.ts         # chama get_app_info, timeout 5 s, estados Carregando/Pronto/Erro
├── app-info.test.ts
└── self-test.ts        # só no build de teste: reporta o texto renderizado
src-tauri/
├── Cargo.toml          # feature opcional "self-test"
├── build.rs
├── deny.toml
├── tauri.conf.json     # janela "Prelo", CSP
├── capabilities/default.json
├── src/
│   ├── main.rs
│   ├── lib.rs          # builder, registro de comandos
│   ├── app_info.rs     # AppInfo + get_app_info
│   └── self_test.rs    # #[cfg(feature = "self-test")]
└── tests/
    ├── ipc_contract.rs     # mock runtime: sucesso, comando desconhecido, argumentos ruins
    └── security_config.rs  # CSP, capabilities, self-test fora do release, licenças em sincronia
e2e/
├── wdio.conf.ts        # Linux/Windows
└── app.e2e.ts
scripts/
├── verify.mjs          # orquestra etapas, resume falhas, detecta testes pulados
├── audit.mjs           # cargo-deny + audit-ci + licenças + validade das exceções
└── self-test.mjs       # build com self-test, roda o binário, checa código de saída
security/
├── licenses-allow.txt
└── vuln-exceptions.toml
.githooks/pre-push
.github/workflows/ci.yml
```

**Structure Decision**: layout padrão do Tauri 2 (`src/` interface, `src-tauri/` núcleo) para
seguir a documentação oficial e o `tauri-cli` sem configuração extra. Testes Rust de
integração em `src-tauri/tests/`, unitários da interface ao lado do código, ponta a ponta em
`e2e/`. Scripts em Node (`.mjs`) para rodar igual nos três sistemas, sem `make` nem bash
obrigatório. A proteção da `main` é aplicada por `gh api` documentado no README, por ser
configuração do GitHub e não arquivo do repositório.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| Possível dependência transitiva MPL-2.0 (ex.: `cssparser`, `selectors` via Tauri) frente a RNF-06 "permissivas" | Tauri (D-01) as traz; sem elas não há shell | Trocar de shell contradiz D-01. **Aprovado (D-11)**: exceção por crate em `deny.toml` com motivo (uso sem modificação, copyleft só por arquivo). A lista exata sai do primeiro `cargo deny check` |
| Dois caminhos ponta a ponta (WebDriver + autoteste) | `tauri-driver` não suporta macOS | Só WebDriver deixaria macOS sem E2E (SC-001); só autoteste perderia inspeção de interface em Linux/Windows |
