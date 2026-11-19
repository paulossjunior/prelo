# Implementation Plan: Pipeline de release dos instaladores

**Branch**: `002-release-pipeline` | **Date**: 2026-11-19 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/002-release-pipeline/spec.md` · Issue
[#35](https://github.com/paulossjunior/prelo/issues/35)

## Summary

Workflow `release.yml` disparado por tag `vX.Y.Z`, em quatro jobs:

1. `preflight`: confere o formato da tag, se o commit está na `main`, se a versão é igual
   nos três arquivos e se há arquivo de licença.
2. `verify`: chama o `ci.yml` da spec 001 por `workflow_call` e roda nos três sistemas.
3. `build` (matriz): gera `.dmg` universal, `.exe` NSIS x86_64, `.AppImage` e `.deb`
   x86_64, sem `self-test`, e varre o binário em busca da marca de autoteste.
4. `publish`: único job com `contents: write`. Confere o conjunto completo, gera
   `SHA256SUMS.txt`, recusa nome já existente e cria o rascunho num comando. Se esta
   execução falhar depois de criar o rascunho, apaga o rascunho.

A lógica fica em dois scripts Node testados com Vitest, e a forma do workflow tem testes
próprios. Detalhes em [research.md](research.md).

## Technical Context

**Language/Version**: YAML do GitHub Actions; Node 24 LTS (scripts `.mjs`); Rust 1.95 e Tauri 2.12 (inalterados, só empacotados)

**Primary Dependencies**: `@tauri-apps/cli` 2.12 (`tauri build --target --bundles`), `gh` CLI dos runners, `actions/checkout`, `actions/setup-node`, `actions/upload-artifact`, `actions/download-artifact`, `Swatinem/rust-cache`; novo devDependency `yaml` (ISC) para os testes do workflow

**Storage**: N/A (artefatos do workflow e assets da release no GitHub)

**Testing**: Vitest 5 (projeto `scripts`, já em `npm run verify`); teste Rust existente `self_test_nao_compila_em_release`; validação real por tags de teste (quickstart)

**Target Platform**: runners `macos-latest` (dmg universal), `windows-latest` (NSIS x64), `ubuntu-22.04` (AppImage e deb amd64)

**Project Type**: desktop-app (infraestrutura de distribuição)

**Performance Goals**: rascunho em ≤ 60 min após a tag (SC-003); preflight falha em < 1 min

**Constraints**: `contents: write` só em `publish`; sem segredos além do `GITHUB_TOKEN`; sem assinatura; sem `--features`; rascunho nunca parcial; nunca sobrescrever release existente

**Scale/Scope**: 1 workflow novo, 1 alteração no `ci.yml`, 2 scripts, 3 arquivos de teste, 1 modelo de notas, seção no README

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Princípio / regra | Situação | Como |
|---|---|---|
| I. Typst puro no repositório | N/A | Sem manipulação de `.typ` |
| II. Privacidade sem servidor | ✅ | Nenhum dado de usuário; GitHub Releases só hospeda instaladores |
| III. Nenhum trabalho perdido | N/A | Sem dados do usuário |
| IV. Edição só sincronizada | N/A | Sem edição |
| V. Revisão explícita | N/A | Sem revisão |
| VI. TDD | ✅ | `tasks.md` põe os testes dos scripts e do workflow antes do código |
| VI. Testes de segurança | ✅ | Permissões mínimas, nenhum segredo extra, `self-test` fora do artefato, ações fixadas por SHA, nome existente não sobrescrito, tag fora da `main` recusada |
| VI. Auditoria de dependências | ✅ | `verify` (com auditoria) roda antes do build; `yaml` é ISC e só de desenvolvimento |
| Restrições: stack | ✅ | Tauri (D-01); nenhum componente trocado |
| Restrições: plataformas (RNF-01) | ✅ | Instaladores dos três sistemas |
| Restrições: licenças (RNF-06) | ✅ | Nenhuma dependência de produção nova |
| Fluxo: push só com suíte verde | ✅ | `verify` repetido na release; testes novos entram no `npm run verify` |
| Fluxo: feature cita requisitos | ✅ | RNF-01, D-01, issue #35 |
| Fluxo: ordem das fases | ✅ | Infraestrutura da fase 1; não pula fase |
| Governance: TODO(LICENSE) | ✅ | FR-016: release recusada sem arquivo de licença |

**Resultado pré-pesquisa**: aprovado, sem violações.
**Re-checagem pós-design**: sem mudança. Observação: enquanto o dono não escolher a licença,
toda release é recusada por desenho (research R10).

## Project Structure

### Documentation (this feature)

```text
specs/002-release-pipeline/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── release-workflow.md
│   └── release-scripts.md
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
.github/
├── workflows/
│   ├── ci.yml                  # + gatilho workflow_call
│   └── release.yml             # novo: preflight → verify → build → publish
└── release-notes.md            # modelo das notas do rascunho ({{VERSION}})
scripts/
├── release-check.mjs           # preflight: tag, main, versões, licença
├── release-check.d.mts
├── release-check.test.ts
├── release-artifacts.mjs       # scan (self-test), collect, sums
├── release-artifacts.d.mts
├── release-artifacts.test.ts
└── release-workflow.test.ts    # forma de release.yml e ci.yml
package.json                    # + devDependency yaml
README.md                       # + seção "Publicar uma versão"
```

**Structure Decision**: segue o padrão da spec 001: scripts Node em `scripts/` com `.d.mts` e
testes Vitest ao lado. Esses testes rodam pelo projeto `scripts` do `vitest.config.ts` e,
portanto, também pelo `npm run verify` e pelo hook pre-push. Nenhum código do app (`src/`,
`src-tauri/src/`) muda.

## Complexity Tracking

Sem violações a justificar.
