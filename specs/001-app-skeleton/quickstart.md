# Quickstart: validar o esqueleto

Guia de validação ponta a ponta da feature. Detalhes de contrato em [contracts/](contracts/).

## Pré-requisitos

- Rust (versão em `rust-toolchain.toml`), Node 24+ e npm.
- macOS: Xcode Command Line Tools. Windows: WebView2 (já vem no Windows 11) e Build Tools do
  Visual Studio. Linux: pacotes listados no README (WebKitGTK 4.1 e afins).
- Ferramentas de auditoria: `cargo install cargo-deny tauri-driver --locked`
  (`tauri-driver` só em Linux e Windows).

## Cenários

| # | Passo | Resultado esperado | Cobre |
|---|---|---|---|
| 1 | `npm install` | `git config core.hooksPath` retorna `.githooks` | FR-010 |
| 2 | `npm run dev` | Janela "Prelo" mostra "Prelo 0.1.0 · <sistema>" em até 3 s | US1, US2, SC-002 |
| 3 | Fechar a janela | Processo encerra; nenhum `prelo` em execução | FR-005 |
| 4 | `npm run verify` | Todas as etapas verdes, código 0 | FR-006, US3 |
| 5 | Quebrar um teste e rodar `npm run verify` | Código ≠ 0, etapa e teste nomeados | US3 cenário 2 |
| 6 | Adicionar dependência npm com licença GPL-3.0 e rodar `npm run audit` | Falha nomeando dependência e licença | US4, FR-008 |
| 7 | Adicionar em `vuln-exceptions.toml` uma exceção com `review_by` no passado | `npm run audit` falha | FR-009 |
| 8 | Com teste quebrado, `git push` | Hook bloqueia | US5 cenário 1 |
| 9 | Com teste quebrado, `git push --no-verify` em branch e abrir PR | CI vermelho; botão de merge bloqueado | US5 cenário 2, SC-004 |
| 10 | `git push origin main` direto (inclusive dono) | Recusado pela proteção | FR-012 |
| 11 | `gh api repos/{owner}/{repo}/branches/main/protection` | Mostra checks dos 3 sistemas e `enforce_admins` | FR-012 |

Desfazer os passos 5–9 depois de validar.
