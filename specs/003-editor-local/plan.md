# Implementation Plan: Editor local (fase 1)

**Branch**: `003-editor-local` | **Date**: 2026-11-19 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/003-editor-local/spec.md` · docs/especificacao.md
§4 item 1

## Summary

Editor Typst local sobre o esqueleto de 001: o núcleo Rust embute o `typst` 0.15.1 e o
Tinymist 0.15 como bibliotecas, com um World próprio cujo único ponto de leitura de disco
aplica as regras de escopo de RF-08 (conferência no descritor aberto), as fontes de D-15
(Libertinus OFL embutida + `fonts/` do projeto, sem fontes do sistema), o cache de pacotes
`@preview` validado com hash e a confirmação de plugins WASM. A interface usa CodeMirror 6 com
realce, autocompletar e diagnósticos vindos do núcleo, e uma prévia SVG por página enviada
incrementalmente; clique na prévia e "localizar na prévia" usam `typst-ide`. Exportação de PDF
pela mesma compilação. Gravação atômica, salvamento automático e detecção de mudanças externas
cobrem o Princípio III. Testes de segurança e de entrada malformada vêm antes de cada parte.
Detalhes em [research.md](research.md).

## Technical Context

**Language/Version**: Rust 1.95 (`rust-toolchain.toml`); TypeScript 7 sobre Node 24 LTS

**Primary Dependencies**: Tauri 2.12; `typst`/`typst-svg`/`typst-pdf`/`typst-ide` 0.15.1;
`tinymist-world`/`tinymist-vfs`/`tinymist-package` 0.15.0 e `tinymist-query` 0.15.12;
`ureq` 3.5 + `rustls` (ring) + `rustls-platform-verifier`; `flate2`, `tar`, `sha2`,
`tempfile`, `notify` 8, `dirs` 6, `tauri-plugin-dialog` 2 (só do lado Rust); CodeMirror 6
(`@codemirror/*`); fontes Libertinus 7.051 (OFL-1.1). Licenças em research R12.

**Storage**: arquivos do projeto no disco; `.prelo/project.json`; nos dados locais do app,
cache de pacotes com `index.json` e `plugin-approvals.json` ([data-model.md](data-model.md))

**Testing**: `cargo test` (unidade, integração, segurança com fixtures geradas, equivalência
com `typst` CLI no CI, desempenho em `--release`), `tauri::test` para o contrato IPC, Vitest +
jsdom, WebdriverIO (Linux/Windows) e autoteste (3 sistemas); `cargo-deny`, `audit-ci` e
verificação nova de licenças de arquivos de fonte

**Target Platform**: macOS (arm64, x86_64), Windows 10/11 x64, Linux x64 (WebKitGTK 4.1)

**Project Type**: desktop-app

**Performance Goals**: prévia p95 < 100 ms após edição em artigo de 40 páginas, inclusive com
imagens grandes não tocadas (SC-001, SC-002) no computador de referência (Apple M1 ou x64 de 4
núcleos, 8 GB); guarda de regressão de 250 ms no CI; clique → editor < 50 ms

**Constraints**: nenhuma leitura fora da raiz (RF-08); nenhuma fonte do sistema (D-15); webview
sem permissão de `fs`, `dialog`, `http`; CSP sem `unsafe-*`; compilação abandonada após 10 s;
arquivo lido ≤ 256 MB; pacote ≤ 50 MB compactado / 200 MB e 5 000 arquivos extraídos

**Scale/Scope**: 6 histórias, 33 FR; ~20 módulos Rust novos, ~10 módulos TS; fixtures de
segurança geradas em teste

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Princípio / regra | Situação | Como |
|---|---|---|
| I. Typst puro no repositório | ✅ | O Prelo não escreve nada no `.typ` além do texto digitado; padrões de fonte vêm do World; projeto novo compila com `typst` CLI (US6, teste) |
| II. Privacidade sem servidor | ✅ | Só download de pacote sai do computador, com nome e versão na URL, por TLS; sem servidor do Prelo |
| III. Nenhum trabalho perdido | ✅ | Gravação atômica, automática em 2 s, no fechamento; conflito com mudança externa pergunta (R10) |
| IV. Edição só sincronizada | N/A | Sem pares nem git nesta fase |
| V. Revisão explícita | N/A | Sem sugestões nem comentários |
| VI. TDD | ✅ | `tasks.md` põe testes antes da implementação em cada fase |
| VI. Testes de segurança | ✅ | Escape da raiz, entrada malformada, pacote malicioso/adulterado, plugin sem confirmação, IPC com caminho fora da raiz, configuração da janela ([file-access.md](contracts/file-access.md)) |
| VI. Auditoria de dependências | ✅ | `cargo-deny`/`audit-ci` existentes + licenças de arquivos de fonte (D-15) |
| Restrições: stack | ✅ | CodeMirror 6, typst + Tinymist, Tauri; nenhuma troca de componente |
| Restrições: compilação embutida | ✅ | Prévia, diagnósticos, autocompletar e PDF do mesmo World; com versão registrada ≠ embutida o projeto não compila até a pessoa atualizar o registro (FR-012), então nada sai de versão diferente da registrada; E2E sem `typst` no PATH |
| Restrições: plataformas | ✅ | CI nos três sistemas; regras de link/junção por sistema (R11) |
| Restrições: desempenho | ✅ | Teste `perf_preview` (R13) |
| Restrições: licenças (RNF-06) | ✅ | Fontes só OFL-1.1 por D-15; `webpki-roots` excluído; ver Complexity Tracking sobre dados do `typst-assets` |
| Fluxo: feature cita requisitos | ✅ | Spec cita RF/RNF/D |
| Fluxo: ordem das fases | ✅ | Fase 1 do plano |

**Resultado pré-pesquisa**: aprovado. **Re-checagem pós-design**: sem violação nova; um ponto
de licença de dados (não código) registrado em Complexity Tracking para decisão do dono.

## Project Structure

### Documentation (this feature)

```text
specs/003-editor-local/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── ipc.md
│   └── file-access.md
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
src/                         # interface
├── main.ts                  # monta o layout de quatro painéis
├── ipc.ts                   # wrappers tipados dos comandos de contracts/ipc.md
├── editor/
│   ├── editor.ts            # CodeMirror 6: edição, envio de mudanças, ⌘S
│   ├── highlight.ts         # semantic tokens → decorações
│   ├── completion.ts
│   ├── diagnostics.ts
│   └── *.test.ts
├── preview/
│   ├── preview.ts           # páginas virtualizadas, <img> com blob URL, destaque
│   ├── sync.ts              # clique → jump_from_click; localizar → jump_from_cursor
│   └── *.test.ts
├── project/
│   ├── tree.ts              # árvore de arquivos
│   ├── outline.ts           # estrutura de seções
│   ├── prompts.ts           # confirmação de plugin, conflito externo, cópia para o projeto
│   └── *.test.ts
└── about.ts                 # versão do compilador e licenças das fontes
src-tauri/
├── Cargo.toml
├── fonts/                   # Libertinus *.otf, OFL.txt, licenses.toml
├── src/
│   ├── lib.rs               # registro de comandos e do protocolo prelo-asset
│   ├── project/
│   │   ├── mod.rs           # abrir/criar projeto, arquivo principal
│   │   └── record.rs        # .prelo/project.json
│   ├── fs/
│   │   ├── scope.rs         # regras de RF-08 (contracts/file-access.md)
│   │   ├── access_model.rs  # PathAccessModel do Prelo
│   │   ├── save.rs          # gravação atômica, automática
│   │   └── watch.rs         # mudanças externas
│   ├── world/
│   │   ├── mod.rs           # montagem do World tinymist com as peças abaixo
│   │   ├── fonts.rs         # Libertinus + fonts/ do projeto
│   │   ├── packages.rs      # PackageRegistry, download, extração validada, índice
│   │   └── plugins.rs       # aprovações por (raiz, sha256)
│   ├── compile/
│   │   ├── scheduler.rs     # fila de 1, watchdog de 10 s
│   │   ├── preview.rs       # SVG por página, hash, troca de data: URL
│   │   ├── export.rs        # PDF
│   │   └── sync.rs          # jump_from_click/cursor, outline
│   ├── lang_service.rs      # camada fina sobre tinymist-query
│   └── commands.rs          # comandos IPC
└── tests/
    ├── fixtures/            # projetos pequenos versionados
    ├── support/             # geradores: links, FIFO, .tar.gz maliciosos, contador de leituras
    ├── security_file_access.rs
    ├── security_malformed.rs
    ├── security_packages.rs
    ├── security_plugins.rs
    ├── security_ipc.rs
    ├── security_config.rs   # existente, atualizado (CSP, capabilities)
    ├── compile_*.rs
    ├── equivalence.rs       # feature "equivalence"
    └── perf_preview.rs
e2e/editor.e2e.ts
scripts/audit.mjs            # + licenças de arquivos de fonte
```

**Structure Decision**: mantém o layout de 001 (`src/` interface, `src-tauri/` núcleo) e separa
o núcleo por responsabilidade: `fs/` é a única camada que toca o disco do projeto, `world/`
monta o que o compilador enxerga, `compile/` produz prévia, PDF e saltos. Testes de segurança
ficam em arquivos próprios, para o QA conferir cobertura por categoria do Princípio VI.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| Dados internos do `typst-assets` (perfis ICC CC0, dados HTML com partes CC-BY-4.0/W3C, fontes Foxit BSD-3-Clause usadas ao embutir PDF) entram no binário, embora a crate seja Apache-2.0 | O `typst` depende do `typst-assets`; sem ele não há compilador (D-10) | Remover exigiria *fork* do Typst. A nova verificação de licenças de arquivos embutidos lista esses dados; CC0 e BSD-3-Clause já estão na lista; **CC-BY-4.0/W3C (dados, não código) aguardam confirmação do dono** antes do primeiro release |
| Threads de compilação abandonadas em vez de canceladas (R9) | O Typst não tem cancelamento; FR-015 exige editor responsivo | Processo filho perde o cache incremental e a meta de 100 ms |
