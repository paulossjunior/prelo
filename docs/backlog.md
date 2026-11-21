# Backlog do Prelo

Hierarquia **Epic → User Story → Task** do GitHub, gerada pela skill `gestor-projeto`
(operação **backlog**). Cada linha aponta para a issue no GitHub; o GitHub é a fonte da
verdade, este arquivo é um retrato. Para o mapa de versões (release → epics), veja
[release-plan.md](release-plan.md).

- Gerado em: 2026-11-21, a partir de `gh issue list --state all` e da API de sub-issues.
- Como regenerar: peça `/gestor-projeto backlog` ou siga a seção "Backlog em docs/backlog.md"
  de `.claude/skills/gestor-projeto/SKILL.md`.
- Legenda: `[x]` fechada, `[ ]` aberta · prioridade · agente(s) `agente:*` · semana
  (milestone semanal) nas abertas. A release de cada Epic vem do label `release:*`.

## Resumo

| Nível | Total |
|---|---|
| Epics | 3 (1 abertas, 2 fechadas) |
| User Stories | 11 (6 abertas, 5 fechadas) |
| Tasks | 142 (75 abertas, 67 fechadas) |

## [#1](https://github.com/paulossjunior/prelo/issues/1) Montar esqueleto do app Tauri para macOS, Windows e Linux _(Epic legado, sem label `epic`)_

Estado: **fechada** · Prioridade: P1 · Agente: speckit, qa · Release: — (milestone de fase: F1 Editor local)  
Fonte: [`specs/001-app-skeleton/tasks.md`](https://github.com/paulossjunior/prelo/blob/dev/specs/001-app-skeleton/tasks.md) (branch `dev`)

Progresso: 0 US · 31/33 tasks fechadas

### Tasks direto no Epic (Setup, Foundational, Polish)

- [ ] [#32](https://github.com/paulossjunior/prelo/issues/32) T031: Validar cenários 8–11 do quickstart.md e registrar a saída de gh api .../branches/main/p… · — · speckit · sem semana
- [ ] [#33](https://github.com/paulossjunior/prelo/issues/33) T032: Rodar o quickstart.md inteiro em cada sistema disponível e corrigir divergências · — · speckit · sem semana

<details><summary>31 task(s) fechada(s)</summary>

- [x] [#2](https://github.com/paulossjunior/prelo/issues/2) T001: Criar configuração raiz: package.json (nome prelo, versão 0.1.0, engines.node >=24, scri… · — · speckit
- [x] [#3](https://github.com/paulossjunior/prelo/issues/3) T002: Criar o núcleo Tauri 2 mínimo: src-tauri/Cargo.toml (tauri 2.12, tauri-build, feature op… · — · speckit
- [x] [#4](https://github.com/paulossjunior/prelo/issues/4) T003: Criar a interface mínima: index.html, vite.config.ts e src/main.ts vazio montando um ele… · — · speckit
- [x] [#5](https://github.com/paulossjunior/prelo/issues/5) T004: Configurar Vitest com jsdom em vitest.config.ts e adicionar vitest, jsdom em package.json · — · speckit
- [x] [#6](https://github.com/paulossjunior/prelo/issues/6) T005: Escrever testes do orquestrador em scripts/verify.test.ts: roda todas as etapas mesmo ap… · — · qa
- [x] [#7](https://github.com/paulossjunior/prelo/issues/7) T006: Implementar scripts/verify.mjs conforme contracts/dev-workflow.md até T005 passar, com e… · — · speckit
- [x] [#8](https://github.com/paulossjunior/prelo/issues/8) T007: Ligar scripts em package.json: test:rust (cargo test --manifest-path src-tauri/Cargo.tom… · — · speckit
- [x] [#9](https://github.com/paulossjunior/prelo/issues/9) T008: Teste de segurança da configuração em src-tauri/tests/security_config.rs: janela princip… · — · qa
- [x] [#10](https://github.com/paulossjunior/prelo/issues/10) T009: Teste ponta a ponta por autoteste em scripts/self-test.mjs: compila com --features self-… · — · qa
- [x] [#11](https://github.com/paulossjunior/prelo/issues/11) T010: Teste WebDriver em e2e/app.e2e.ts com e2e/wdio.conf.ts (tauri-driver, só Linux e Windows… · — · qa
- [x] [#12](https://github.com/paulossjunior/prelo/issues/12) T011: Configurar janela, CSP e capabilities mínimas em src-tauri/tauri.conf.json e… · — · speckit
- [x] [#13](https://github.com/paulossjunior/prelo/issues/13) T012: Implementar o caminho de autoteste: src-tauri/src/self_test.rs (#[cfg(feature = "self-te… · — · speckit
- [x] [#14](https://github.com/paulossjunior/prelo/issues/14) T013: Testes unitários em src-tauri/src/app_info.rs (módulo tests): AppInfo.version igual a CA… · — · qa
- [x] [#15](https://github.com/paulossjunior/prelo/issues/15) T014: Testes de contrato IPC em src-tauri/tests/ipc_contract.rs com tauri::test mock runtime: … · — · qa
- [x] [#16](https://github.com/paulossjunior/prelo/issues/16) T015: Teste de segurança em src-tauri/tests/ipc_contract.rs: sem a feature self-test, self_tes… · — · qa
- [x] [#17](https://github.com/paulossjunior/prelo/issues/17) T016: Testes da interface em src/app-info.test.ts com invoke simulado e relógio falso: Carrega… · — · qa
- [x] [#18](https://github.com/paulossjunior/prelo/issues/18) T017: Endurecer o autoteste em scripts/self-test.mjs e e2e/app.e2e.ts: o texto renderizado MUS… · — · qa
- [x] [#19](https://github.com/paulossjunior/prelo/issues/19) T018: Implementar AppInfo e o comando get_app_info em src-tauri/src/app_info.rs e registrá-lo … · — · speckit
- [x] [#20](https://github.com/paulossjunior/prelo/issues/20) T019: Implementar src/app-info.ts (consulta com timeout de 5 s, estados Carregando/Pronto/Erro… · — · speckit
- [x] [#21](https://github.com/paulossjunior/prelo/issues/21) T020: Atualizar src-tauri/src/self_test.rs e src/self-test.ts para comparar o texto exato e o … · — · speckit
- [x] [#22](https://github.com/paulossjunior/prelo/issues/22) T021: Escrever o README: pré-requisitos por sistema (incluindo pacotes Linux de research R8), … · — · speckit
- [x] [#23](https://github.com/paulossjunior/prelo/issues/23) T022: Validar em clone limpo (diretório temporário) os cenários 1, 4 e 5 do quickstart.md e co… · — · speckit
- [x] [#24](https://github.com/paulossjunior/prelo/issues/24) T023: Testes em scripts/audit.test.ts: lista de security/licenses-allow.txt idêntica à allow d… · — · qa
- [x] [#25](https://github.com/paulossjunior/prelo/issues/25) T024: Criar security/licenses-allow.txt com a lista de research R4.1 e security/vuln-exception… · — · speckit
- [x] [#26](https://github.com/paulossjunior/prelo/issues/26) T025: Implementar scripts/audit.mjs: valida security/vuln-exceptions.toml, roda cargo deny che… · — · speckit
- [x] [#27](https://github.com/paulossjunior/prelo/issues/27) T026: Criar src-tauri/deny.toml (licenças da lista única, advisories com ignores gerados das e… · — · speckit
- [x] [#28](https://github.com/paulossjunior/prelo/issues/28) T027: Testes em scripts/hooks.test.ts: em repositório git temporário, .githooks/pre-push com n… · — · qa
- [x] [#29](https://github.com/paulossjunior/prelo/issues/29) T028: Criar .githooks/pre-push (POSIX sh, executável, roda npm run verify e repassa o código) … · — · speckit
- [x] [#30](https://github.com/paulossjunior/prelo/issues/30) T029: Criar .github/workflows/ci.yml: gatilhos pull_request e push; job verify com matriz ubun… · — · speckit
- [x] [#31](https://github.com/paulossjunior/prelo/issues/31) T030: Abrir PR da branch 001-app-skeleton, esperar o CI verde nos três sistemas e então aplica… · — · speckit, humano
- [x] [#34](https://github.com/paulossjunior/prelo/issues/34) T033: Validação final pelo agente qa (modo validar): suíte completa, cobertura FR-001–FR-014, … · — · qa

</details>

## [#35](https://github.com/paulossjunior/prelo/issues/35) [Epic] Publicar instaladores no GitHub Releases por tag de versão

Estado: **fechada** · Prioridade: — · Agente: speckit, qa · Release: —  
Fonte: [`specs/002-release-pipeline/tasks.md`](https://github.com/paulossjunior/prelo/blob/dev/specs/002-release-pipeline/tasks.md) (branch `dev`)

Progresso: 5 US · 26/26 tasks fechadas

### Tasks direto no Epic (Setup, Foundational, Polish)


<details><summary>8 task(s) fechada(s)</summary>

- [x] [#40](https://github.com/paulossjunior/prelo/issues/40) T001: Adicionar yaml (ISC, versão exata) em devDependencies de package.json e atualizar packag… · — · speckit, qa
- [x] [#41](https://github.com/paulossjunior/prelo/issues/41) T002: Escrever em scripts/release-workflow.test.ts o carregador de .github/workflows/release.y… · — · speckit, qa
- [x] [#42](https://github.com/paulossjunior/prelo/issues/42) T003: Criar .github/workflows/release.yml com gatilho, concurrency (cancel-in-progress: false)… · — · speckit, qa
- [x] [#61](https://github.com/paulossjunior/prelo/issues/61) T022: Documentar em README.md a seção "Publicar uma versão" (FR-015): atualizar os três arquiv… · — · speckit, qa
- [x] [#62](https://github.com/paulossjunior/prelo/issues/62) T023: Abrir PR e confirmar npm run verify verde nos três sistemas pelo CI (checks verify (<sis… · — · speckit, qa
- [x] [#63](https://github.com/paulossjunior/prelo/issues/63) T024: Rodar o quickstart.md: cenários locais L1–L4 e, no GitHub, 4, 5, 6 e 8 (possíveis sem li… · — · speckit, qa
- [x] [#64](https://github.com/paulossjunior/prelo/issues/64) T025: Rodar os cenários 1, 2, 3, 7 e 9 do quickstart.md **depois** que o dono adicionar o arqu… · — · speckit, qa
- [x] [#65](https://github.com/paulossjunior/prelo/issues/65) T026: Validação final pelo agente qa (modo validar): suíte completa, cobertura FR-001–FR-016, … · — · speckit, qa

</details>

### [#149](https://github.com/paulossjunior/prelo/issues/149) [002/US1] Gerar uma release a partir de uma tag de versão

Estado: **fechada** · P1 · speckit, qa · 5/5 tasks fechadas


<details><summary>5 task(s) fechada(s)</summary>

- [x] [#43](https://github.com/paulossjunior/prelo/issues/43) T004: Testes em scripts/release-artifacts.test.ts para expectedInstallers e matchInstallers (c… · — · speckit, qa
- [x] [#44](https://github.com/paulossjunior/prelo/issues/44) T005: Testes em scripts/release-workflow.test.ts (contracts/release-workflow.md): matriz de bu… · — · speckit, qa
- [x] [#45](https://github.com/paulossjunior/prelo/issues/45) T006: Implementar expectedInstallers, matchInstallers e o comando collect em scripts/release-a… · — · speckit, qa
- [x] [#46](https://github.com/paulossjunior/prelo/issues/46) T007: Adicionar o job build em .github/workflows/release.yml (research.md R4): matriz do contr… · — · speckit, qa
- [x] [#47](https://github.com/paulossjunior/prelo/issues/47) T008: Adicionar o job publish em .github/workflows/release.yml (research.md R5): download-arti… · — · speckit, qa

</details>

### [#150](https://github.com/paulossjunior/prelo/issues/150) [002/US2] Barrar release inválida antes de gerar instaladores

Estado: **fechada** · P1 · speckit, qa · 4/4 tasks fechadas


<details><summary>4 task(s) fechada(s)</summary>

- [x] [#48](https://github.com/paulossjunior/prelo/issues/48) T009: Testes em scripts/release-check.test.ts (contracts/release-scripts.md) · — · speckit, qa
- [x] [#49](https://github.com/paulossjunior/prelo/issues/49) T010: Testes em scripts/release-workflow.test.ts: job preflight com contents: read, checkout f… · — · speckit, qa
- [x] [#50](https://github.com/paulossjunior/prelo/issues/50) T011: Implementar scripts/release-check.mjs (funções exportadas e CLI) e scripts/release-check… · — · speckit, qa
- [x] [#51](https://github.com/paulossjunior/prelo/issues/51) T012: Adicionar os jobs preflight e verify em .github/workflows/release.yml e ligar build.need… · — · speckit, qa

</details>

### [#152](https://github.com/paulossjunior/prelo/issues/152) [002/US3] Baixar e conferir o instalador

Estado: **fechada** · P2 · speckit, qa · 5/5 tasks fechadas


<details><summary>5 task(s) fechada(s)</summary>

- [x] [#52](https://github.com/paulossjunior/prelo/issues/52) T013: Testes em scripts/release-artifacts.test.ts (contracts/release-scripts.md) · — · speckit, qa
- [x] [#53](https://github.com/paulossjunior/prelo/issues/53) T014: Testes em scripts/release-workflow.test.ts e sobre .github/release-notes.md · — · speckit, qa
- [x] [#54](https://github.com/paulossjunior/prelo/issues/54) T015: Implementar sha256File, formatSums, verifySums e o comando sums em scripts/release-artif… · — · speckit, qa
- [x] [#55](https://github.com/paulossjunior/prelo/issues/55) T016: Escrever .github/release-notes.md (research.md R9), conteúdo de data-model.md Release em… · — · speckit, qa
- [x] [#56](https://github.com/paulossjunior/prelo/issues/56) T017: Ligar sums, a renderização das notas e o anexo de SHA256SUMS.txt no job publish de .gith… · — · speckit, qa

</details>

### [#151](https://github.com/paulossjunior/prelo/issues/151) [002/US3] Baixar e conferir o instalador

Estado: **fechada** · P2 · speckit, qa · duplicada · 0/0 tasks fechadas

### [#153](https://github.com/paulossjunior/prelo/issues/153) [002/US4] Release segura por construção

Estado: **fechada** · P2 · speckit, qa · 4/4 tasks fechadas


<details><summary>4 task(s) fechada(s)</summary>

- [x] [#57](https://github.com/paulossjunior/prelo/issues/57) T018: Testes em scripts/release-artifacts.test.ts para containsSelfTestMarker e o comando scan · — · speckit, qa
- [x] [#58](https://github.com/paulossjunior/prelo/issues/58) T019: Testes de segurança em scripts/release-workflow.test.ts (contracts/release-workflow.md) · — · speckit, qa
- [x] [#59](https://github.com/paulossjunior/prelo/issues/59) T020: Implementar containsSelfTestMarker e o comando scan em scripts/release-artifacts.mjs até… · — · speckit, qa
- [x] [#60](https://github.com/paulossjunior/prelo/issues/60) T021: Em .github/workflows/release.yml: adicionar o passo scan no build (binário prelo / prelo… · — · speckit, qa

</details>

## [#38](https://github.com/paulossjunior/prelo/issues/38) [Epic] Construir o editor local com prévia e compilador Typst embutido

Estado: **aberta** · Prioridade: P1 · Agente: speckit, qa · Release: — (milestone de fase: F1 Editor local)  
Fonte: [`specs/003-editor-local/tasks.md`](https://github.com/paulossjunior/prelo/blob/003-editor-local/specs/003-editor-local/tasks.md) (branch `003-editor-local`)

Progresso: 6 US · 10/83 tasks fechadas

### Tasks direto no Epic (Setup, Foundational, Polish)

- [ ] [#77](https://github.com/paulossjunior/prelo/issues/77) T011: Testes unitários de caminho em src-tauri/src/fs/scope.rs (módulo tests): / inicial relat… · P1 · speckit, qa · sem semana
- [ ] [#78](https://github.com/paulossjunior/prelo/issues/78) T012: Testes de segurança de escape em src-tauri/tests/security_file_access.rs (FR-021, FR-022… · P1 · speckit, qa · sem semana
- [ ] [#79](https://github.com/paulossjunior/prelo/issues/79) T013: Testes do registro em src-tauri/src/project/record.rs (módulo tests) cobrindo data-model… · P1 · speckit, qa · sem semana
- [ ] [#80](https://github.com/paulossjunior/prelo/issues/80) T014: Testes de agenda em src-tauri/src/compile/scheduler.rs (módulo tests) com World falso: f… · P1 · speckit, qa · sem semana
- [ ] [#81](https://github.com/paulossjunior/prelo/issues/81) T015: Testes de segurança do IPC em src-tauri/tests/security_ipc.rs (mock runtime): todo coman… · P1 · speckit, qa · sem semana
- [ ] [#82](https://github.com/paulossjunior/prelo/issues/82) T016: Atualizar src-tauri/tests/security_config.rs: CSP exatamente a de contracts/ipc.md; capa… · P1 · speckit, qa · sem semana
- [ ] [#83](https://github.com/paulossjunior/prelo/issues/83) T017: Implementar src-tauri/src/fs/scope.rs conforme file-access.md passos 2–7 (canonicalizaçã… · P1 · speckit, qa · sem semana
- [ ] [#84](https://github.com/paulossjunior/prelo/issues/84) T018: Implementar src-tauri/src/fs/access_model.rs: PathAccessModel do Prelo usando scope.rs, … · P1 · speckit, qa · sem semana
- [ ] [#147](https://github.com/paulossjunior/prelo/issues/147) T019: Implementar src-tauri/src/project/record.rs com data-model.md ProjectRecord.schema, Proj… · P1 · speckit, qa · sem semana
- [ ] [#85](https://github.com/paulossjunior/prelo/issues/85) T020: Implementar src-tauri/src/world/mod.rs e src-tauri/src/world/fonts.rs (só fontes embutid… · P1 · speckit, qa · sem semana
- [ ] [#86](https://github.com/paulossjunior/prelo/issues/86) T021: Implementar src-tauri/src/compile/scheduler.rs com data-model.md Compilation.compileId e… · P1 · speckit, qa · sem semana
- [ ] [#87](https://github.com/paulossjunior/prelo/issues/87) T022: Implementar src-tauri/src/commands.rs e registro em src-tauri/src/lib.rs (validação de p… · P1 · speckit, qa · sem semana
- [ ] [#88](https://github.com/paulossjunior/prelo/issues/88) T023: Criar src/ipc.ts com wrappers tipados dos comandos e eventos de contracts/ipc.md e teste… · P1 · speckit, qa · sem semana
- [ ] [#140](https://github.com/paulossjunior/prelo/issues/140) T075: Escrever src-tauri/tests/perf_preview.rs (--release): p95 edição→PreviewUpdate em artigo… · P1 · speckit, qa · sem semana
- [ ] [#141](https://github.com/paulossjunior/prelo/issues/141) T076: Otimizar o necessário (compilação incremental, diff de páginas, prelo-asset) em src-taur… · P1 · speckit, qa · sem semana
- [ ] [#142](https://github.com/paulossjunior/prelo/issues/142) T077: Teste de encerramento forçado em scripts/self-test.mjs: matar o processo 2 s após a últi… · P1 · speckit, qa · sem semana
- [ ] [#143](https://github.com/paulossjunior/prelo/issues/143) T078: Teste de threads abandonadas em src-tauri/tests/security_malformed.rs: após 3 compilaçõe… · P1 · speckit, qa · sem semana
- [ ] [#144](https://github.com/paulossjunior/prelo/issues/144) T079: Atualizar README.md: fontes embutidas e licenças, pasta fonts/ e typst compile --font-pa… · P1 · speckit, qa · sem semana
- [ ] [#145](https://github.com/paulossjunior/prelo/issues/145) T080: Rodar o roteiro de quickstart.md e npm run verify nos três sistemas; corrigir o que falh… · P1 · speckit, qa · sem semana
- [ ] [#146](https://github.com/paulossjunior/prelo/issues/146) T081: Validação do QA: cobertura de FR-001 a FR-029 por ao menos um teste que falhou antes (SC… · P1 · speckit, qa · sem semana

<details><summary>10 task(s) fechada(s)</summary>

- [x] [#67](https://github.com/paulossjunior/prelo/issues/67) T001: Escrever testes em scripts/audit.test.ts, no padrão do teste "MPL-2.0 não está na lista … · P1 · speckit, qa
- [x] [#68](https://github.com/paulossjunior/prelo/issues/68) T002: Criar a exceção em src-tauri/deny.toml ([[licenses.exceptions]], crate typst-assets, all… · P1 · speckit, qa
- [x] [#69](https://github.com/paulossjunior/prelo/issues/69) T003: Adicionar dependências Rust de research.md R12 em src-tauri/Cargo.toml: typst, typst-svg… · P1 · speckit, qa
- [x] [#70](https://github.com/paulossjunior/prelo/issues/70) T004: Adicionar @codemirror/state, @codemirror/view, @codemirror/commands, @codemirror/languag… · P1 · speckit, qa
- [x] [#71](https://github.com/paulossjunior/prelo/issues/71) T005: Adicionar as fontes Libertinus 7.051 (Serif, Sans, Math, Mono) em src-tauri/fonts/, com … · P1 · speckit, qa
- [x] [#72](https://github.com/paulossjunior/prelo/issues/72) T006: Escrever testes em scripts/audit.test.ts para a verificação de licenças de arquivos embu… · P1 · speckit, qa
- [x] [#73](https://github.com/paulossjunior/prelo/issues/73) T007: Implementar a verificação em scripts/audit.mjs até T006 passar (D-15) · P1 · speckit, qa
- [x] [#74](https://github.com/paulossjunior/prelo/issues/74) T008: Rodar npm run audit com as dependências de T003/T004; se cargo-deny acusar licença fora … · P1 · speckit, qa
- [x] [#75](https://github.com/paulossjunior/prelo/issues/75) T009: Criar apoio de testes em src-tauri/tests/support/mod.rs: projeto temporário a partir de … · P1 · speckit, qa
- [x] [#76](https://github.com/paulossjunior/prelo/issues/76) T010: Criar fixtures versionadas em src-tauri/tests/fixtures/: artigo-basico/ (vários .typ com… · P1 · speckit, qa

</details>

### [#154](https://github.com/paulossjunior/prelo/issues/154) [003/US1] Escrever com prévia ao vivo

Estado: **aberta** · P1 · speckit, qa · 0/16 tasks fechadas

- [ ] [#89](https://github.com/paulossjunior/prelo/issues/89) T024: Testes de compilação em src-tauri/tests/compile_basic.rs: abrir projeto compila o arquiv… · P1 · speckit, qa · sem semana
- [ ] [#90](https://github.com/paulossjunior/prelo/issues/90) T025: Testes de prévia em src-tauri/src/compile/preview.rs (módulo tests): hash por página; Pr… · P1 · speckit, qa · sem semana
- [ ] [#91](https://github.com/paulossjunior/prelo/issues/91) T026: Testes de versão em src-tauri/tests/compile_version.rs: compiler_info.embedded igual à v… · P1 · speckit, qa · sem semana
- [ ] [#92](https://github.com/paulossjunior/prelo/issues/92) T027: Testes de gravação em src-tauri/src/fs/save.rs (módulo tests) para data-model.md OpenFil… · P1 · speckit, qa · sem semana
- [ ] [#93](https://github.com/paulossjunior/prelo/issues/93) T028: Testes de mudança externa em src-tauri/src/fs/watch.rs (módulo tests) para OpenFile.last… · P1 · speckit, qa · sem semana
- [ ] [#94](https://github.com/paulossjunior/prelo/issues/94) T029: Testes do serviço de linguagem em src-tauri/tests/lang_service.rs: autocompletar de funç… · P1 · speckit, qa · sem semana
- [ ] [#95](https://github.com/paulossjunior/prelo/issues/95) T030: Testes de interface em src/editor/editor.test.ts, src/editor/diagnostics.test.ts, src/pr… · P1 · speckit, qa · sem semana
- [ ] [#96](https://github.com/paulossjunior/prelo/issues/96) T031: Teste ponta a ponta em e2e/editor.e2e.ts e no autoteste (src/self-test.ts, src-tauri/src… · P1 · speckit, qa · sem semana
- [ ] [#97](https://github.com/paulossjunior/prelo/issues/97) T032: Implementar src-tauri/src/project/mod.rs: open_project_dialog (diálogo nativo pelo núcle… · P1 · speckit, qa · sem semana
- [ ] [#98](https://github.com/paulossjunior/prelo/issues/98) T033: Implementar src-tauri/src/fs/save.rs com data-model.md OpenFile.text, OpenFile.version e… · P1 · speckit, qa · sem semana
- [ ] [#99](https://github.com/paulossjunior/prelo/issues/99) T034: Implementar src-tauri/src/fs/watch.rs com notify e data-model.md OpenFile.lastSavedHash,… · P1 · speckit, qa · sem semana
- [ ] [#100](https://github.com/paulossjunior/prelo/issues/100) T035: Implementar src-tauri/src/compile/preview.rs (SVG por página com typst-svg, hash, subscr… · P1 · speckit, qa · sem semana
- [ ] [#101](https://github.com/paulossjunior/prelo/issues/101) T036: Implementar src-tauri/src/lang_service.rs sobre typst-ide (autocomplete, tooltip) e typs… · P1 · speckit, qa · sem semana
- [ ] [#102](https://github.com/paulossjunior/prelo/issues/102) T037: Implementar src/editor/editor.ts, src/editor/highlight.ts, src/editor/completion.ts, src… · P1 · speckit, qa · sem semana
- [ ] [#103](https://github.com/paulossjunior/prelo/issues/103) T038: Implementar src/preview/preview.ts e src/project/tree.ts até a parte de prévia e árvore … · P1 · speckit, qa · sem semana
- [ ] [#104](https://github.com/paulossjunior/prelo/issues/104) T039: Implementar src/about.ts (versão embutida e registrada, aviso de diferença, licenças das… · P1 · speckit, qa · sem semana

### [#155](https://github.com/paulossjunior/prelo/issues/155) [003/US2] Projeto confinado à própria pasta

Estado: **aberta** · P1 · speckit, qa · 0/12 tasks fechadas

- [ ] [#105](https://github.com/paulossjunior/prelo/issues/105) T040: Testes de entrada malformada em src-tauri/tests/security_malformed.rs (FR-024, FR-030, S… · P1 · speckit, qa · sem semana
- [ ] [#106](https://github.com/paulossjunior/prelo/issues/106) T041: Teste de autocompletar de caminhos em src-tauri/tests/security_file_access.rs: com link … · P1 · speckit, qa · sem semana
- [ ] [#107](https://github.com/paulossjunior/prelo/issues/107) T042: Testes de plugins em src-tauri/tests/security_plugins.rs para data-model.md PluginApprov… · P1 · speckit, qa · sem semana
- [ ] [#108](https://github.com/paulossjunior/prelo/issues/108) T043: Testes de pacotes em src-tauri/tests/security_packages.rs com *fetcher* injetado (sem re… · P1 · speckit, qa · sem semana
- [ ] [#109](https://github.com/paulossjunior/prelo/issues/109) T044: Testes de interface em src/project/prompts.test.ts: pedido de confirmação de plugin most… · P1 · speckit, qa · sem semana
- [ ] [#110](https://github.com/paulossjunior/prelo/issues/110) T045: Mapear erros do compilador e de leitura para Diagnostic com kind e linha em src-tauri/sr… · P1 · speckit, qa · sem semana
- [ ] [#111](https://github.com/paulossjunior/prelo/issues/111) T046: Filtrar sugestões de caminho por scope.rs em src-tauri/src/lang_service.rs até T041 pass… · P1 · speckit, qa · sem semana
- [ ] [#112](https://github.com/paulossjunior/prelo/issues/112) T047: Implementar src-tauri/src/world/plugins.rs com data-model.md PluginApproval.projectRoot,… · P1 · speckit, qa · sem semana
- [ ] [#113](https://github.com/paulossjunior/prelo/issues/113) T048: Implementar src-tauri/src/world/packages.rs com data-model.md PackageCacheEntry.spec, Pa… · P1 · speckit, qa · sem semana
- [ ] [#114](https://github.com/paulossjunior/prelo/issues/114) T049: Implementar o pedido de confirmação de plugin e a ação "Baixar de novo" em src/project/p… · P1 · speckit, qa · sem semana
- [ ] [#161](https://github.com/paulossjunior/prelo/issues/161) T082: Testes de segurança em src-tauri/tests/security_malformed.rs para RUSTSEC-2026-0194/0195… · P1 · speckit, qa · sem semana
- [ ] [#162](https://github.com/paulossjunior/prelo/issues/162) T083: Implementar o limite de 1 MiB para .csl em src-tauri/src/fs/scope.rs/src-tauri/src/fs/ac… · P1 · speckit, qa · sem semana

### [#156](https://github.com/paulossjunior/prelo/issues/156) [003/US3] Bibliografia e figuras do projeto

Estado: **aberta** · P2 · speckit, qa · 0/11 tasks fechadas

- [ ] [#115](https://github.com/paulossjunior/prelo/issues/115) T050: Testes em src-tauri/tests/compile_bibliography.rs: .bib, .yml, vários arquivos, .csl do … · P1 · speckit, qa · sem semana
- [ ] [#116](https://github.com/paulossjunior/prelo/issues/116) T051: Testes em src-tauri/tests/compile_images.rs: PNG, JPEG, GIF (estática), SVG, PDF e WebP … · P1 · speckit, qa · sem semana
- [ ] [#117](https://github.com/paulossjunior/prelo/issues/117) T052: Testes em src-tauri/tests/compile_fonts.rs: fonte de fonts/ é usada; fonte só instalada … · P1 · speckit, qa · sem semana
- [ ] [#118](https://github.com/paulossjunior/prelo/issues/118) T053: Teste em src-tauri/tests/compile_packages.rs com *fetcher* local: primeira compilação ba… · P1 · speckit, qa · sem semana
- [ ] [#119](https://github.com/paulossjunior/prelo/issues/119) T054: Testes de cópia para o projeto em src-tauri/tests/compile_copy_into_project.rs (D-13, FR… · P1 · speckit, qa · sem semana
- [ ] [#120](https://github.com/paulossjunior/prelo/issues/120) T055: Teste de protocolo prelo-asset em src-tauri/src/compile/preview.rs (módulo tests): data:… · P1 · speckit, qa · sem semana
- [ ] [#121](https://github.com/paulossjunior/prelo/issues/121) T056: Completar src-tauri/src/world/fonts.rs com a leitura de fonts/**/*.{ttf,otf,ttc,otc} pel… · P1 · speckit, qa · sem semana
- [ ] [#122](https://github.com/paulossjunior/prelo/issues/122) T057: Garantir cache de bytes e imagens decodificadas por conteúdo em src-tauri/src/fs/access_… · P1 · speckit, qa · sem semana
- [ ] [#123](https://github.com/paulossjunior/prelo/issues/123) T058: Implementar o protocolo prelo-asset em src-tauri/src/compile/preview.rs e src-tauri/src/… · P1 · speckit, qa · sem semana
- [ ] [#124](https://github.com/paulossjunior/prelo/issues/124) T059: Implementar copy_into_project e a oferta em src-tauri/src/commands.rs e src/project/prom… · P1 · speckit, qa · sem semana
- [ ] [#125](https://github.com/paulossjunior/prelo/issues/125) T060: Ligar à compilação os estilos .csl do projeto, a recompilação ao salvar .bib/.yml (src-t… · P1 · speckit, qa · sem semana

### [#157](https://github.com/paulossjunior/prelo/issues/157) [003/US4] Navegar entre código e prévia

Estado: **aberta** · P2 · speckit, qa · 0/4 tasks fechadas

- [ ] [#126](https://github.com/paulossjunior/prelo/issues/126) T061: Testes em src-tauri/tests/compile_sync.rs: jump_from_click em parágrafo do principal e d… · P1 · speckit, qa · sem semana
- [ ] [#127](https://github.com/paulossjunior/prelo/issues/127) T062: Testes de interface em src/preview/sync.test.ts e src/project/outline.test.ts: clique co… · P1 · speckit, qa · sem semana
- [ ] [#128](https://github.com/paulossjunior/prelo/issues/128) T063: Implementar src-tauri/src/compile/sync.rs (typst_ide::jump_from_click, jump_from_cursor,… · P1 · speckit, qa · sem semana
- [ ] [#129](https://github.com/paulossjunior/prelo/issues/129) T064: Implementar src/preview/sync.ts e src/project/outline.ts até T062 passar · P1 · speckit, qa · sem semana

### [#158](https://github.com/paulossjunior/prelo/issues/158) [003/US5] Exportar o PDF

Estado: **aberta** · P2 · speckit, qa · 0/6 tasks fechadas

- [ ] [#130](https://github.com/paulossjunior/prelo/issues/130) T065: Testes em src-tauri/tests/compile_export.rs: PDF com o mesmo número de páginas e tamanho… · P1 · speckit, qa · sem semana
- [ ] [#131](https://github.com/paulossjunior/prelo/issues/131) T066: Teste de equivalência em src-tauri/tests/equivalence.rs (feature equivalence, PRELO_TYPS… · P1 · speckit, qa · sem semana
- [ ] [#132](https://github.com/paulossjunior/prelo/issues/132) T067: Implementar src-tauri/src/compile/export.rs (typst-pdf, gravação atômica) e export_pdf_d… · P1 · speckit, qa · sem semana
- [ ] [#133](https://github.com/paulossjunior/prelo/issues/133) T068: Escrever teste em src/export.test.ts: ação "Exportar PDF" chama export_pdf_dialog; compi… · P1 · speckit, qa · sem semana
- [ ] [#134](https://github.com/paulossjunior/prelo/issues/134) T069: Adicionar a ação "Exportar PDF" com lista de erros em src/main.ts até T068 passar · P1 · speckit, qa · sem semana
- [ ] [#135](https://github.com/paulossjunior/prelo/issues/135) T070: Adicionar job de equivalência ao CI em .github/workflows/ci.yml (instala typst 0.15.1 só… · P1 · speckit, qa · sem semana

### [#159](https://github.com/paulossjunior/prelo/issues/159) [003/US6] Criar um projeto novo

Estado: **aberta** · P3 · speckit, qa · 0/4 tasks fechadas

- [ ] [#136](https://github.com/paulossjunior/prelo/issues/136) T071: Testes em src-tauri/tests/project_create.rs: pasta vazia → main.typ mínimo e .prelo/proj… · P1 · speckit, qa · sem semana
- [ ] [#137](https://github.com/paulossjunior/prelo/issues/137) T072: Escrever teste em src/project/tree.test.ts: ação "Novo projeto" chama create_project_dia… · P1 · speckit, qa · sem semana
- [ ] [#138](https://github.com/paulossjunior/prelo/issues/138) T073: Implementar create_project_dialog em src-tauri/src/project/mod.rs gravando data-model.md… · P1 · speckit, qa · sem semana
- [ ] [#139](https://github.com/paulossjunior/prelo/issues/139) T074: Adicionar a ação "Novo projeto" em src/main.ts até T072 passar · P1 · speckit, qa · sem semana

## Órfãos / inconsistências

- #1: tem 33 filhas mas não tem o label `epic` (tratada como Epic legado abaixo)
- Citam `Parte de #155` no corpo, mas não são sub-issues de #155 (vincular pela API) (2): [#161](https://github.com/paulossjunior/prelo/issues/161), [#162](https://github.com/paulossjunior/prelo/issues/162)
- Citam `Parte de #35` no corpo, mas não são sub-issues de #35 (vincular pela API) (1): [#151](https://github.com/paulossjunior/prelo/issues/151)
- Citam `Parte de #1` no corpo, mas não são sub-issues de #1 (vincular pela API) (33): [#2](https://github.com/paulossjunior/prelo/issues/2), [#3](https://github.com/paulossjunior/prelo/issues/3), [#4](https://github.com/paulossjunior/prelo/issues/4), [#5](https://github.com/paulossjunior/prelo/issues/5), [#6](https://github.com/paulossjunior/prelo/issues/6), [#7](https://github.com/paulossjunior/prelo/issues/7), [#8](https://github.com/paulossjunior/prelo/issues/8), [#9](https://github.com/paulossjunior/prelo/issues/9), [#10](https://github.com/paulossjunior/prelo/issues/10), [#11](https://github.com/paulossjunior/prelo/issues/11), [#12](https://github.com/paulossjunior/prelo/issues/12), [#13](https://github.com/paulossjunior/prelo/issues/13), [#14](https://github.com/paulossjunior/prelo/issues/14), [#15](https://github.com/paulossjunior/prelo/issues/15), [#16](https://github.com/paulossjunior/prelo/issues/16), [#17](https://github.com/paulossjunior/prelo/issues/17), [#18](https://github.com/paulossjunior/prelo/issues/18), [#19](https://github.com/paulossjunior/prelo/issues/19), [#20](https://github.com/paulossjunior/prelo/issues/20), [#21](https://github.com/paulossjunior/prelo/issues/21), [#22](https://github.com/paulossjunior/prelo/issues/22), [#23](https://github.com/paulossjunior/prelo/issues/23), [#24](https://github.com/paulossjunior/prelo/issues/24), [#25](https://github.com/paulossjunior/prelo/issues/25), [#26](https://github.com/paulossjunior/prelo/issues/26), [#27](https://github.com/paulossjunior/prelo/issues/27), [#28](https://github.com/paulossjunior/prelo/issues/28), [#29](https://github.com/paulossjunior/prelo/issues/29), [#30](https://github.com/paulossjunior/prelo/issues/30), [#31](https://github.com/paulossjunior/prelo/issues/31), [#32](https://github.com/paulossjunior/prelo/issues/32), [#33](https://github.com/paulossjunior/prelo/issues/33), [#34](https://github.com/paulossjunior/prelo/issues/34)
- Epics com milestone de fase e sem label `release:*` (migrar, ver Milestones semanais) (2): [#1](https://github.com/paulossjunior/prelo/issues/1), [#38](https://github.com/paulossjunior/prelo/issues/38)
- Filhas de #1 sem o label `task` (33): [#2](https://github.com/paulossjunior/prelo/issues/2), [#3](https://github.com/paulossjunior/prelo/issues/3), [#4](https://github.com/paulossjunior/prelo/issues/4), [#5](https://github.com/paulossjunior/prelo/issues/5), [#6](https://github.com/paulossjunior/prelo/issues/6), [#7](https://github.com/paulossjunior/prelo/issues/7), [#8](https://github.com/paulossjunior/prelo/issues/8), [#9](https://github.com/paulossjunior/prelo/issues/9), [#10](https://github.com/paulossjunior/prelo/issues/10), [#11](https://github.com/paulossjunior/prelo/issues/11), [#12](https://github.com/paulossjunior/prelo/issues/12), [#13](https://github.com/paulossjunior/prelo/issues/13), [#14](https://github.com/paulossjunior/prelo/issues/14), [#15](https://github.com/paulossjunior/prelo/issues/15), [#16](https://github.com/paulossjunior/prelo/issues/16), [#17](https://github.com/paulossjunior/prelo/issues/17), [#18](https://github.com/paulossjunior/prelo/issues/18), [#19](https://github.com/paulossjunior/prelo/issues/19), [#20](https://github.com/paulossjunior/prelo/issues/20), [#21](https://github.com/paulossjunior/prelo/issues/21), [#22](https://github.com/paulossjunior/prelo/issues/22), [#23](https://github.com/paulossjunior/prelo/issues/23), [#24](https://github.com/paulossjunior/prelo/issues/24), [#25](https://github.com/paulossjunior/prelo/issues/25), [#26](https://github.com/paulossjunior/prelo/issues/26), [#27](https://github.com/paulossjunior/prelo/issues/27), [#28](https://github.com/paulossjunior/prelo/issues/28), [#29](https://github.com/paulossjunior/prelo/issues/29), [#30](https://github.com/paulossjunior/prelo/issues/30), [#31](https://github.com/paulossjunior/prelo/issues/31), [#32](https://github.com/paulossjunior/prelo/issues/32), [#33](https://github.com/paulossjunior/prelo/issues/33), [#34](https://github.com/paulossjunior/prelo/issues/34)
- Abertas, mas o Epic #1 está fechado (2): [#32](https://github.com/paulossjunior/prelo/issues/32), [#33](https://github.com/paulossjunior/prelo/issues/33)
- Epics sem label `release:*` (1): [#35](https://github.com/paulossjunior/prelo/issues/35)
