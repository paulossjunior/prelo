# Prelo · Plano de releases

Versão 0.1 · novembro de 2026 · rascunho para discussão · dados do GitHub consultados em 2026-11-20

Este documento é de **referência**: diz o que entra em cada release do Prelo, em que ordem, com
que critério de saída e em que estado está. O porquê do produto está em
[especificacao.md](especificacao.md); os requisitos citados (RF-xx, RNF-xx) estão em
[requisitos.md](requisitos.md); as decisões (D-xx), em [adrs.md](adrs.md). O estado de cada
issue está em [backlog.md](backlog.md).

Nenhuma data de entrega foi definida pelo time. Onde a fonte não traz data, o plano diz
**a definir**.

---

## Sumário

1. [Critério de corte](#1-critério-de-corte)
2. [Resumo](#2-resumo)
3. [Releases](#3-releases)
4. [Definition of Done comum](#4-definition-of-done-comum)
5. [Rastreabilidade RF/RNF → release](#5-rastreabilidade-rfrnf--release)
6. [Riscos e premissas](#6-riscos-e-premissas)
7. [Questões em aberto](#7-questões-em-aberto)
8. [Manutenção deste plano](#8-manutenção-deste-plano)

---

## 1. Critério de corte

- **Uma release por fase do plano de construção.** As fases e sua ordem vêm de
  [especificacao.md §4](especificacao.md#4-plano-de-construção), e a constituição obriga a
  segui-la: "editor local → comentários e git → tempo real → modo sugestão → papéis e convites.
  Pular fase exige justificativa no plan.md" (Fluxo de Desenvolvimento). A R0 reúne o que veio
  antes da fase 1 (esqueleto e pipeline de release).
- **Cada release é usável por si.** Ela só sai quando alguém do público-alvo consegue fazer o
  trabalho prometido sem as releases seguintes.
- **Nenhuma release viola os princípios I–VI**, mesmo parcial. Exemplo: a R2 não pode gravar
  comentário dentro do `.typ` "por enquanto" (Princípio I).
- **Versão SemVer.** A `0.1.0` (R0) foi a única versão `0.x`. A `1.0.0` é a primeira versão
  estável e corresponde à R1: editar e compilar um arquivo Typst e gerar o PDF (decisão do dono,
  2026-11-21, [Q-01](#7-questões-em-aberto)). Depois dela, cada release com funcionalidade nova
  sobe o MINOR (`1.1.0`, `1.2.0`…); correções sobem o PATCH (`1.0.x`). As versões da R2 em
  diante são **propostas**.
- **Publicação.** Toda release sai por tag `vX.Y.Z` na `main`, pelo pipeline da feature 002, e
  só fica pública quando o dono publica o rascunho
  ([README, "Publicar uma versão"](../README.md#publicar-uma-versão)).

## 2. Resumo

| Release | Fase (§4) | Objetivo em uma linha | Quem usa | Versão | Status |
|---|---|---|---|---|---|
| [R0 Fundação](#r0-fundação) | antes da fase 1 | App vazio que abre nos três sistemas, com suíte, auditoria e instaladores publicados por tag | Quem constrói o Prelo | `0.1.0` | **Entregue** (2026-11-19) |
| [R1 Editor local](#r1-editor-local) | 1 | Escrever um artigo Typst sozinho, com prévia ao vivo e PDF, sem `typst` instalado | Autor individual | `1.0.0` | **Em andamento** |
| [R2 Comentários e histórico](#r2-comentários-e-histórico) | 2 | Comentar e versionar o artigo, colaborando em horários diferentes pelo git | Dono e coautores, assíncrono | `1.1.0` (proposta) | Planejado |
| [R3 Tempo real sem servidor](#r3-tempo-real-sem-servidor) | 3 | Editar ao mesmo tempo, sem servidor, com fila offline e edição só sobre versão sincronizada | Dono e coautores, simultâneo | `1.2.0` (proposta) | Planejado |
| [R4 Modo sugestão](#r4-modo-sugestão) | 4 | Sugerir, aceitar e rejeitar mudanças, com prévia marcada e PDF de diferenças | Coautores revisando entre si | `1.3.0` (proposta) | Planejado |
| [R5 Papéis e convites](#r5-papéis-e-convites) | 5 | Convidar revisor e leitor com aprovação do dono, remover pessoas e mudar papéis | Dono, revisor, leitor | `1.4.0` (proposta) | Planejado |

A `1.0.0` exige só o núcleo da R1: editar um `.typ`, compilar com o compilador embutido
(prévia ao vivo) e exportar o PDF, ou seja, [#154](https://github.com/paulossjunior/prelo/issues/154),
[#155](https://github.com/paulossjunior/prelo/issues/155) e
[#158](https://github.com/paulossjunior/prelo/issues/158). As histórias
[#156](https://github.com/paulossjunior/prelo/issues/156),
[#157](https://github.com/paulossjunior/prelo/issues/157) e
[#159](https://github.com/paulossjunior/prelo/issues/159) seguem planejadas na R1, mas não
bloqueiam a `1.0.0` (ver [R1](#r1-editor-local)).

Funcionalidades por release:

| Release | Funcionalidades principais | RFs | Épico |
|---|---|---|---|
| R0 | Janela Tauri nos três sistemas; ponte interface ↔ núcleo; `npm run verify`; auditoria de licenças e vulnerabilidades; trava de push; instaladores no GitHub Releases | — (RNF-01, RNF-06; D-01) | [#1](https://github.com/paulossjunior/prelo/issues/1), [#35](https://github.com/paulossjunior/prelo/issues/35) |
| R1 | **Bloqueiam a `1.0.0`:** editor com realce, autocompletar e diagnósticos; prévia ao vivo; compilador embutido; escopo de arquivos; exportar PDF. **Não bloqueiam:** código ↔ prévia; bibliografia e figuras; criar projeto novo | RF-01 a RF-09, RF-42 (parcial), RF-50, RF-51 | [#38](https://github.com/paulossjunior/prelo/issues/38) |
| R2 | Comentários com respostas e menções; aba Resolvidos; commit e push pelo app; versões nomeadas; Salvar e fechar sincronizam pelo git | RF-16, RF-17, RF-33, RF-34, RF-36, RF-40, RF-41; parciais: RF-04, RF-32, RF-38 | (épico a criar) |
| R3 | Documento Yjs; mDNS e iroh; convite por código; identidade por chave; cursores e presença; estados de sincronização; fila offline; arquivos pendentes | RF-20, RF-23, RF-30, RF-31, RF-35, RF-37, RF-52, RF-53; parciais: RF-21, RF-22, RF-32, RF-38 | (épico a criar) |
| R4 | Modos Editando, Sugerindo e Lendo; sugestões; aceitar e rejeitar, em lote e com desfazer; teste de compilação por sugestão; prévia com marcações; commit proposto; PDF de diferenças | RF-10 a RF-15, RF-18, RF-19, RF-44, RF-54 | (épico a criar) |
| R5 | Convite com papel, validade e aprovação; conferência do código da chave; remover pessoa e mudar papel; permissões por papel; notificações | RF-21, RF-22, RF-24; tabela de papéis ([requisitos.md §1](requisitos.md#1-papéis-e-permissões)) | (épico a criar) |

RF-42 (`.typ` limpo), RF-43 e RF-45 não aparecem em nenhuma fase do §4; a alocação abaixo é
proposta ([Q-03](#7-questões-em-aberto)).

---

## 3. Releases

### R0 Fundação

| Campo | Valor |
|---|---|
| Status | **Entregue**. Tag `v0.1.0`, release publicada em 2026-11-19 com `.dmg` universal, `.exe` x64, `.AppImage` e `.deb` x64 e `SHA256SUMS.txt` |
| Versão | `0.1.0` |
| Milestone | nenhum. As issues da 001 estão no milestone "F1 Editor local" ([Q-06](#7-questões-em-aberto)) |
| Quem usa | Quem constrói o Prelo (pessoas e agentes); quem quer conferir que o app instala |

**Objetivo.** Ter onde as fases do plano existam: um app que abre em macOS, Windows e Linux
(RNF-01), com testes em TDD, auditoria de dependências (RNF-06) e uma forma reprodutível de
publicar instaladores.

**Funcionalidades**

| Funcionalidade | Origem | Spec | Issue |
|---|---|---|---|
| Janela "Prelo" com versão e sistema vindos do núcleo | D-01, RNF-01 | [001](../specs/001-app-skeleton/spec.md) | [#1](https://github.com/paulossjunior/prelo/issues/1) (fechada) |
| Suíte `npm run verify`, auditoria de licenças e vulnerabilidades, hook de pre-push, CI e proteção da `main` | RNF-06, Princípio VI | [001](../specs/001-app-skeleton/spec.md) | [#1](https://github.com/paulossjunior/prelo/issues/1) |
| Tag `vX.Y.Z` na `main` gera instaladores dos três sistemas em release rascunho, com checksums e verify antes do build | RNF-01, D-01 | [002](../specs/002-release-pipeline/spec.md) | Épico [#35](https://github.com/paulossjunior/prelo/issues/35) (fechado); US [#149](https://github.com/paulossjunior/prelo/issues/149), [#150](https://github.com/paulossjunior/prelo/issues/150), [#152](https://github.com/paulossjunior/prelo/issues/152), [#153](https://github.com/paulossjunior/prelo/issues/153) |

**Pendências herdadas.** Duas tarefas da 001 seguem abertas:
[#32](https://github.com/paulossjunior/prelo/issues/32) (T031, validar cenários 8–11 e registrar
a proteção da `main`) e [#33](https://github.com/paulossjunior/prelo/issues/33) (T032, rodar o
quickstart em cada sistema). Elas não impediram a `v0.1.0`, mas a DoD comum
([§4](#4-definition-of-done-comum)) pede o quickstart nos três sistemas.

### R1 Editor local

| Campo | Valor |
|---|---|
| Status | **Em andamento**. Spec, plano e tarefas na branch `003-editor-local`; tarefas T001–T010 fechadas no GitHub, as demais abertas |
| Versão | `1.0.0` (decisão do dono, 2026-11-21) |
| Milestone | [F1 Editor local](https://github.com/paulossjunior/prelo/milestone/1) |
| Épico | [#38](https://github.com/paulossjunior/prelo/issues/38) |
| Spec | `specs/003-editor-local/spec.md` (branch `origin/003-editor-local`, ainda não está na `dev`) |
| Depende de | R0 |
| Quem usa | Autor individual que já escreve em Typst |

**Objetivo.** Primeiro uso real do Prelo: abrir ou criar um projeto Typst local, escrever com
prévia ao vivo e exportar o PDF, sem `typst` instalado. Sem colaboração
([especificacao.md §4](especificacao.md#4-plano-de-construção), item 1).

**Funcionalidades**

| História de usuário | Prioridade | RFs | Bloqueia a `1.0.0`? | Issue |
|---|---|---|---|---|
| Escrever com prévia ao vivo | P1 | RF-01, RF-02, RF-06, RF-07, RNF-03 | **Sim** | [#154](https://github.com/paulossjunior/prelo/issues/154) |
| Projeto confinado à própria pasta | P1 | RF-08, D-13 | **Sim** (Princípio VI, segurança) | [#155](https://github.com/paulossjunior/prelo/issues/155) |
| Bibliografia e figuras do projeto | P2 | RF-05, RF-09, RF-50, RF-51, D-14 | Não | [#156](https://github.com/paulossjunior/prelo/issues/156) |
| Navegar entre código e prévia | P2 | RF-03, RF-04 (sem contagem de pendências) | Não | [#157](https://github.com/paulossjunior/prelo/issues/157) |
| Exportar o PDF | P2 | RF-42 (só PDF), RF-06 | **Sim** | [#158](https://github.com/paulossjunior/prelo/issues/158) |
| Criar um projeto novo | P3 | RF-07 | Não | [#159](https://github.com/paulossjunior/prelo/issues/159) |

**Escopo da `1.0.0`.** Decisão do dono (2026-11-21): "a release 1.0.0 deve permitir editar e
compilar um arquivo typst e gerar o pdf". Por isso a `1.0.0` exige #154 (editar e compilar com
o compilador embutido, prévia ao vivo), #158 (gerar o PDF) e #155 (confinamento à pasta do
projeto, obrigatório pelo Princípio VI). #156, #157 e #159 continuam planejadas na R1, mas não
bloqueiam a `1.0.0`; se atrasarem, saem numa `1.0.x` ou na `1.1.0`.

Decisões do dono que entram nesta release (epic #38, spec 003 "Clarifications"): fontes
embutidas mais a pasta `fonts/` do projeto, sem fontes do sistema (D-15); pacotes `@preview`
baixados sob demanda e guardados em cache; plugin WASM pede confirmação na primeira vez por
projeto; versão do compilador registrada diferente da embutida bloqueia a compilação até
atualizar o projeto; Typst 0.15.1 embutido, portanto PDF e WebP entram em RF-50.

**Fora desta release** (spec 003, FR-033): comentários, git, tempo real, sugestões, papéis,
arquivos pendentes (RF-52 a RF-54), PDF com comentários e de diferenças, `.typ` limpo e
"Preparar submissão".

**Critérios de saída** (além da [DoD comum](#4-definition-of-done-comum); fonte: epic #38 e
spec 003 SC-001 a SC-010)

**Critério obrigatório da `1.0.0`:** editar um arquivo `.typ`, compilá-lo com o compilador
embutido (prévia ao vivo) e exportar o PDF, com #154, #155 e #158 concluídas. Os critérios
numerados abaixo valem para as histórias que entrarem na `1.0.0`; os de histórias que não
bloqueiam (critério 6, de #157) passam a valer na versão em que elas saírem.

1. Prévia atualiza em menos de 100 ms em 95% das edições num artigo de fixture de 40 páginas
   (SC-001), também com imagens grandes que a edição não toca (SC-002).
2. 100% dos casos de fixture de acesso fora da raiz recusados, sem leitura fora da raiz
   (SC-003).
3. 100% das entradas malformadas geram diagnóstico e o app responde em até 1 s (SC-004).
4. PDF idêntico, página a página, ao do `typst` de linha de comando na mesma versão e com as
   mesmas fontes (SC-005).
5. Funciona nos três sistemas sem `typst` instalado (SC-006).
6. 95% dos cliques na prévia levam à linha certa (SC-007).
7. Encerramento forçado perde no máximo os últimos 2 s de digitação não salvos (SC-009).

### R2 Comentários e histórico

| Campo | Valor |
|---|---|
| Status | Planejado. Sem spec |
| Versão | `1.1.0` (proposta) |
| Milestone | (milestone a criar; sugestão: "F2 Comentários e histórico") |
| Épico | (épico a criar) |
| Depende de | R1 |
| Quem usa | Dono e coautores que trabalham em horários diferentes, com remoto git do grupo |

**Objetivo.** Discutir o texto em comentários e guardar o histórico no git, sem sair do app.
A colaboração é assíncrona: o git é o canal (D-05) e Salvar sincroniza por ele
([especificacao.md §4](especificacao.md#4-plano-de-construção), item 2).

**Funcionalidades**

| Funcionalidade | RFs / decisões |
|---|---|
| Comentários ancorados num trecho, com respostas, menções, resolver, reabrir e excluir, guardados em `.prelo/review.json` | RF-16, D-03, Princípios I e V |
| Aba "Resolvidos" | RF-17 |
| Regras de comentário por autor (apagar a própria mensagem, excluir conversa com confirmação) | [requisitos.md §1](requisitos.md#1-papéis-e-permissões) |
| Commit e push pelo app | RF-40, D-05 |
| Versões nomeadas em `.prelo/versions.json` | RF-41 |
| Salvar (⌘S) recebe do git, envia e grava commit; sem conexão, guarda e agenda | RF-33, RF-34, D-08 |
| Fechar faz um último Salvar | RF-36, Princípio III |
| Ao abrir, verificar e receber do git remoto antes de liberar a edição | RF-32 (só git) |
| Resumo do que mudou depois de sincronizar ("Bruno fez 2 commits…") | RF-38 (só git) |
| Contagem de comentários abertos por seção no outline | RF-04 (parcial) |
| PDF com comentários na margem | RF-43 (proposta, [Q-03](#7-questões-em-aberto)) |

**Critérios de saída**

1. O `.typ` versionado continua Typst puro: nenhum comentário dentro dele (Princípio I,
   RNF-05). Teste: compilar o repositório com o `typst` de linha de comando.
2. Salvar sincroniza em cerca de 1 s em rede comum (RNF-03). TODO(rede-comum): definir a rede
   e o tamanho de projeto de referência para medir.
3. Fechar o app ou perder a conexão não perde comentário nem commit (RNF-04): testes de
   encerramento forçado e de remoto indisponível.
4. Testes de segurança para `.prelo/review.json` e `.prelo/versions.json` malformados ou
   adulterados (Princípio VI, entrada não confiável).

### R3 Tempo real sem servidor

| Campo | Valor |
|---|---|
| Status | Planejado. Sem spec |
| Versão | `1.2.0` (proposta) |
| Milestone | (milestone a criar; sugestão: "F3 Tempo real sem servidor") |
| Épico | (épico a criar) |
| Depende de | R2 (o git continua sendo o canal para quem não está online, D-05) |
| Quem usa | Dono e coautores editando ao mesmo tempo, na mesma rede ou pela internet |

**Objetivo.** Editar ao mesmo tempo sem servidor do Prelo, com tráfego criptografado de ponta
a ponta, e nunca editar sobre versão antiga
([especificacao.md §4](especificacao.md#4-plano-de-construção), item 3; Princípios II e IV).

**Funcionalidades**

| Funcionalidade | RFs / decisões |
|---|---|
| Documento Yjs para `.typ`, `.bib` e `.yml` | D-02, D-14 |
| Conexão por mDNS na rede local e iroh pela internet, relay só para conectar | RF-23, D-04, RNF-02 |
| Identidade local por chave, sem conta | RF-22 (sem a conferência pelo dono, que fica na R5) |
| Convite por código de uso único | RF-21 (sem papel, validade e aprovação, que ficam na R5) |
| Edição simultânea com cursores e presença | RF-20 |
| Estados Verificando, Recebendo, Sincronizado e Offline; edição direta só em Sincronizado | RF-31, RF-32, D-07, [requisitos.md §5](requisitos.md#5-regras-de-offline-e-sincronização) |
| Fila offline persistida para comentar, responder e resolver | RF-30, D-06, RNF-04 |
| Reconexão na ordem receber, enviar a fila, gravar no git, liberar | RF-35, Princípio IV |
| Item offline sobre trecho que mudou: perguntar onde manter | RF-37, Princípio III |
| Resumo do que mudou, incluindo o que veio dos pares | RF-38 |
| Manifesto de arquivos e transporte de binários por iroh; arquivos pendentes na prévia; PDF final bloqueado com pendência | RF-52, RF-53, D-12 |

**Critérios de saída**

1. Testes de integração com dois pares e de reconexão para sincronização, merge CRDT e fila
   offline (constituição, Fluxo de Desenvolvimento).
2. Testes de segurança: tráfego criptografado direto e via relay; convite reutilizado,
   expirado ou adulterado recusado; chave trocada recusada; atualização CRDT malformada
   recusada (Princípio VI).
3. Nenhum servidor do Prelo envolvido; relay só conecta (Princípio II).
4. Salvar sincroniza em cerca de 1 s em rede comum (RNF-03).

### R4 Modo sugestão

| Campo | Valor |
|---|---|
| Status | Planejado. Sem spec |
| Versão | `1.3.0` (proposta) |
| Milestone | (milestone a criar; sugestão: "F4 Modo sugestão") |
| Épico | (épico a criar) |
| Depende de | R3 (sugestões são atributos do CRDT, D-02) |
| Quem usa | Coautores revisando o texto uns dos outros; dono preparando resubmissão |

**Objetivo.** Revisão no estilo do Overleaf: toda mudança sugerida tem autor, hora e id, e
pode ser aceita, rejeitada e desfeita
([especificacao.md §4](especificacao.md#4-plano-de-construção), item 4; Princípio V).

**Funcionalidades**

| Funcionalidade | RFs / decisões |
|---|---|
| Modos Editando, Sugerindo e Lendo | RF-10 |
| Sugestões de inserção, exclusão e substituição com a cor do autor | RF-11, D-02 |
| Aceitar e rejeitar pelo cartão ou pelo trecho, com atalhos | RF-12 |
| Aceitar e rejeitar em lote, com filtro e confirmação | RF-13 |
| Desfazer aceite ou rejeição | RF-14 |
| Teste de compilação por sugestão; Aceitar desativado se quebrar | RF-15, RF-54 |
| Prévia com marcações ou só com o texto aceito | RF-18 |
| Commit proposto com mensagem pronta ao aceitar | RF-19 |
| Sugerir offline (entra na fila); aceitar e rejeitar só sincronizado | RF-30, RF-31, D-09 |
| Contagem de sugestões pendentes por seção no outline | RF-04 |
| PDF de diferenças entre duas versões nomeadas | RF-44, [requisitos.md §7](requisitos.md#7-pdf-de-diferenças) |
| Exportar `.typ` limpo | RF-42 (parte `.typ`; proposta, [Q-03](#7-questões-em-aberto)) |
| Tela "Preparar submissão" | RF-45 (proposta, [Q-03](#7-questões-em-aberto)) |

**Critérios de saída**

1. O git e o PDF final recebem só o texto aceito: inserções pendentes ignoradas, exclusões
   pendentes mantidas (Princípio I). Teste com fixture de sugestões pendentes.
2. Teste de recusa para aceitar ou rejeitar offline (D-09).
3. Uma sugestão que apaga um `]` fica com Aceitar desativado; uma sugestão válida com figura
   pendente, não (RF-15, RF-54).

### R5 Papéis e convites

| Campo | Valor |
|---|---|
| Status | Planejado. Sem spec |
| Versão | `1.4.0` (proposta) |
| Milestone | (milestone a criar; sugestão: "F5 Papéis e convites") |
| Épico | (épico a criar) |
| Depende de | R3 (convites e identidade) e R4 (o revisor só sugere) |
| Quem usa | Dono que convida orientador, revisor ou leitor |

**Objetivo.** Controlar quem entra e o que cada pessoa pode fazer, pela tabela de papéis
([especificacao.md §4](especificacao.md#4-plano-de-construção), item 5).

**Funcionalidades**

| Funcionalidade | RFs / decisões |
|---|---|
| Papéis dono, coautor, revisor e leitor, com as permissões da tabela | [requisitos.md §1](requisitos.md#1-papéis-e-permissões), Princípio V |
| Convite com papel, validade e opção de exigir aprovação | RF-21 |
| Dono confere o código da chave antes de aprovar a entrada | RF-22, fluxo [6.1](requisitos.md#61-entrada-no-projeto) |
| Remover pessoa ou mudar papel a qualquer momento | RF-24 |
| Notificações | sem RF ([Q-04](#7-questões-em-aberto)) |

**Critérios de saída**

1. Cada ação proibida na tabela de papéis tem teste de recusa (Princípio VI).
2. Entrada sem chave aprovada é recusada (Princípio VI).

---

## 4. Definition of Done comum

Vale para toda release, além dos critérios de saída próprios. Origem: constituição (Princípio
VI, Fluxo de Desenvolvimento) e critérios do épico [#38](https://github.com/paulossjunior/prelo/issues/38).

1. Todas as tarefas do `tasks.md` da feature concluídas, e cada requisito de comportamento da
   spec com ao menos um teste que falhou antes da implementação (Princípio VI).
2. Testes de segurança aplicáveis à release escritos e verdes (Princípio VI).
3. `npm run verify` completo verde no CI em macOS, Windows e Linux, no pull request da `dev`
   para a `main` (RNF-01).
4. Auditoria de dependências sem achado alto ou crítico não justificado; licenças permissivas
   (RNF-06, D-11).
5. Roteiro do `quickstart.md` da feature executado nos três sistemas.
6. Veredito **APROVADO** do agente `qa` (modo validar).
7. Conformidade com os princípios I–VI verificada no pull request.
8. `README.md` atualizado (linha "Estado" e o que mudou para quem instala).
9. Versão igual em `package.json`, `src-tauri/tauri.conf.json` e `src-tauri/Cargo.toml`; tag
   `vX.Y.Z` na `main`; pipeline gera o rascunho e o dono publica.
10. Épico e milestone da release fechados.

---

## 5. Rastreabilidade RF/RNF → release

"Plano §4" indica que a fase vem de [especificacao.md §4](especificacao.md#4-plano-de-construção).
"Proposta" indica alocação feita neste plano, sem fase explícita na fonte. "R1, não bloqueia
1.0.0" indica RF de história da R1 que pode sair numa `1.0.x` ou na `1.1.0`.

| ID | Resumo | Release | Base da alocação |
|---|---|---|---|
| RF-01 | Editor com realce, autocompletar e diagnósticos | R1 | Plano §4 item 1; spec 003 |
| RF-02 | Prévia ao vivo, < 100 ms | R1 | Plano §4 item 1; spec 003 |
| RF-03 | Código ↔ prévia | R1, não bloqueia 1.0.0 | Plano §4 item 1; spec 003 |
| RF-04 | Árvore e outline | R1, não bloqueia 1.0.0; contagem de pendências em R2 e R4 | Spec 003 (sem contagem); proposta |
| RF-05 | `.bib` e figuras | R1, não bloqueia 1.0.0 | Spec 003 |
| RF-06 | Compilador embutido | R1 | Spec 003; D-10 |
| RF-07 | Versão do compilador em `.prelo/project.json` | R1 (#154 bloqueia 1.0.0; parte de #159 não bloqueia) | Spec 003 |
| RF-08 | Leitura só dentro da raiz | R1 | Spec 003 |
| RF-09 | BibLaTeX, Hayagriva, CSL | R1, não bloqueia 1.0.0 | Spec 003 |
| RF-10 | Modos Editando, Sugerindo, Lendo | R4 | Plano §4 item 4 |
| RF-11 | Sugestões marcadas | R4 | Plano §4 item 4 |
| RF-12 | Aceitar e rejeitar | R4 | Plano §4 item 4 |
| RF-13 | Em lote | R4 | Plano §4 item 4 |
| RF-14 | Desfazer | R4 | Plano §4 item 4 |
| RF-15 | Sugestão que não compila | R4 | Plano §4 item 4 |
| RF-16 | Comentários | R2 | Plano §4 item 2 |
| RF-17 | Aba Resolvidos | R2 | Plano §4 item 2 |
| RF-18 | Prévia com marcações | R4 | Plano §4 item 4 |
| RF-19 | Commit proposto ao aceitar | R4 | Proposta (depende de sugestões) |
| RF-20 | Cursores e presença | R3 | Plano §4 item 3 |
| RF-21 | Convite por código | R3 (código); R5 (papel, validade, aprovação) | Plano §4 itens 3 e 5 |
| RF-22 | Identidade por chave | R3 (chave); R5 (conferência pelo dono) | Plano §4 itens 3 e 5 |
| RF-23 | Conexão direta sem servidor | R3 | Plano §4 item 3 |
| RF-24 | Remover pessoa, mudar papel | R5 | Plano §4 item 5 |
| RF-30 | Fila offline | R3 (comentários); R4 (sugestões) | Plano §4 item 3 |
| RF-31 | Editar e aceitar só sincronizado | R3 (editar); R4 (aceitar) | Plano §4 itens 3 e 4 |
| RF-32 | Verificar versões ao abrir | R2 (git); R3 (pares) | Plano §4 itens 2 e 3 |
| RF-33 | Salvar sincroniza | R2 | Plano §4 item 2 |
| RF-34 | Salvar sem conexão | R2 | Proposta |
| RF-35 | Ordem ao reconectar | R3 | Plano §4 item 3 |
| RF-36 | Último Salvar ao fechar | R2 | Proposta (D-05) |
| RF-37 | Item offline sobre trecho alterado | R3 | Proposta (depende da fila offline) |
| RF-38 | Resumo do que mudou | R2 (git); R3 (pares) | Proposta |
| RF-40 | Commit e push | R2 | Plano §4 item 2 |
| RF-41 | Versões nomeadas | R2 | Plano §4 item 2 |
| RF-42 | PDF final e `.typ` limpos | R1 (PDF); R4 (`.typ`) | Spec 003; proposta |
| RF-43 | PDF com comentários | R2 | Proposta ([Q-03](#7-questões-em-aberto)) |
| RF-44 | PDF de diferenças | R4 | Plano §4 item 4 |
| RF-45 | Preparar submissão | R4 | Proposta ([Q-03](#7-questões-em-aberto)) |
| RF-50 | Formatos de imagem | R1, não bloqueia 1.0.0 | Spec 003 |
| RF-51 | Cache de imagem | R1, não bloqueia 1.0.0 | Spec 003 |
| RF-52 | Arquivo pendente na prévia | R3 | Spec 003 FR-033; D-12 |
| RF-53 | PDF final bloqueado com pendência | R3 | Spec 003 FR-033; D-12 |
| RF-54 | Pendência não desativa Aceitar | R4 | Depende de RF-15 |
| RNF-01 | macOS, Windows, Linux | Todas | DoD comum |
| RNF-02 | Privacidade, E2E | R3 | Plano §4 item 3; ver [Q-05](#7-questões-em-aberto) |
| RNF-03 | Prévia < 100 ms; Salvar ~1 s | R1 (prévia); R2 e R3 (Salvar) | Spec 003; proposta |
| RNF-04 | Nenhum trabalho perdido | R1 (disco), R2, R3 | Princípio III |
| RNF-05 | `.typ` sempre Typst puro | Todas | Princípio I |
| RNF-06 | Licenças permissivas | Todas | DoD comum |

---

## 6. Riscos e premissas

**Premissas**

- A ordem das fases é a da constituição; mudar exige justificativa no `plan.md` da feature.
- Uma release por fase. Se uma fase for grande demais, ela pode sair em mais de uma release
  (MINOR ou PATCH), sem mudar a ordem.
- O pipeline da feature 002 continua sendo o único caminho de publicação.
- Instaladores seguem sem assinatura de código até decisão em contrário (spec 002).

**Riscos por release** (origem: [arquitetura.md §7](arquitetura.md#7-riscos-técnicos-e-questões-em-aberto), salvo indicação)

| Risco | Release afetada | Efeito no plano |
|---|---|---|
| Meta de 100 ms com imagens grandes e primeira compilação (TODO(imagem-grande)) | R1 | Pode atrasar SC-001/SC-002 |
| Atualizar o compilador muda o layout; falta definir quem decide | R1 em diante | Projetos de versões diferentes bloqueiam a compilação até atualizar (decisão da spec 003) |
| Troca do Tinymist por `typst-ide` (D-17, branch 003) | R1 | Ver [Q-02](#7-questões-em-aberto) |
| Edição fora do app (VS Code + push) vira diff a converter | R2 e R4 | Sem definição; pode exigir spec própria |
| Ancoragem de comentário por posição relativa do Yjs antes do Yjs entrar (R3) | R2 | Ver [Q-07](#7-questões-em-aberto) |
| Redes que bloqueiam UDP; relay público ou do grupo | R3 | Decisão pendente antes da spec da R3 |
| Sincronização lenta com muitas figuras; conflito de binário (TODO(conflito-binario)) | R3 | D-12 tem TODOs abertos |
| Conteúdo gerado (`#include`) e comentário pela prévia | R2 e R4 | Sem definição |
| `.bib` com milhares de entradas como `Y.Text` (TODO(bib-grande)) | R3 | Medir antes da spec da R3 |

---

## 7. Questões em aberto

| ID | Questão | Afeta | Fonte da dúvida |
|---|---|---|---|
| Q-01 | **Resolvida** (dono, 2026-11-21): "a release 1.0.0 deve permitir editar e compilar um arquivo typst e gerar o pdf". A R1 é a `1.0.0`; R2–R5 sobem o MINOR (`1.1.0` a `1.4.0`, propostas). Nota: [especificacao.md §3](especificacao.md#3-escopo) usa "primeira versão" para o escopo do produto completo (colaboração, sugestões, git). Este plano lê essa expressão como o conjunto R1–R5, não como a `1.0.0`; o texto da especificação não foi alterado | R1–R5 | Decisão do dono |
| Q-02 | D-17 (branch 003) troca `tinymist-query` por `typst-ide`, mas RF-01 diz "diagnósticos fornecidos pelo Tinymist" e a constituição lista "typst + Tinymist" na stack. RF-01 e a constituição devem ser atualizados quando a 003 entrar na `dev`? | R1 | `docs/adrs.md` na branch 003; RF-01; constituição, Restrições Técnicas |
| Q-03 | RF-42 (parte `.typ`), RF-43 e RF-45 não estão em nenhuma fase do §4. A alocação proposta (RF-43 na R2; RF-42 `.typ` e RF-45 na R4) está certa? | R2, R4 | especificacao.md §4 |
| Q-04 | "Notificações" está na fase 5, mas não há RF para elas. O que notifica, a quem e por qual canal? | R5 | especificacao.md §4 item 5 |
| Q-05 | RNF-02 diz que nenhum servidor guarda o conteúdo do artigo, mas D-05 usa o remoto git do grupo (GitHub ou GitLab). O RNF vale só para servidores do Prelo? Isso define o critério de saída da R2 | R2, R3 | RNF-02, D-05, arquitetura.md §3 |
| Q-06 | As issues da 001 (#1 a #34) estão no milestone "F1 Editor local", e #32 e #33 seguem abertas. Mover para um milestone da R0 ou fechar com a evidência da `v0.1.0`? | R0, R1 | GitHub |
| Q-07 | Comentários ancoram por posição relativa do Yjs (Princípio V), mas o documento Yjs está na fase 3. A R2 já adota o Yjs, ou ancora de outro jeito até a R3? | R2 | arquitetura.md §4; especificacao.md §4 |
| Q-08 | RF-52 e arquitetura.md §7 dizem que D-12 está "proposta", mas adrs.md registra D-12 como aceita | R3 | requisitos.md RF-52; arquitetura.md §7; adrs.md |
| Q-09 | D-15, D-16 e D-17 existem só na branch `003-editor-local`. Este plano as cita como se valessem; confirmar que entram na `dev` com a 003 | R1 | adrs.md na `dev` vs. branch 003 |

---

## 8. Manutenção deste plano

- **Quem mantém:** o dono, com apoio da skill `gestor-projeto`, que gera
  [backlog.md](backlog.md) a partir de `docs/` e das issues. Este plano define o escopo de cada
  release; o backlog mostra o estado de cada issue. Em caso de divergência sobre o estado, vale
  o GitHub.
- **Quando atualizar:**
  1. Ao abrir o épico de uma release: trocar "(épico a criar)" pelo link e preencher o
     milestone.
  2. Ao aprovar a spec de uma release: trocar "Sem spec" pelo caminho e ajustar RFs e critérios
     de saída ao que a spec decidiu.
  3. Ao publicar uma release: status **Entregue**, data e versão da tag.
  4. Ao mudar requisito ou decisão: primeiro em `requisitos.md` ou `adrs.md`; depois aqui.
  5. Ao responder uma questão da §7: remover a linha e aplicar a resposta.
- **O que não fica aqui:** datas inventadas, requisitos novos (vão para `requisitos.md`) e
  tarefas (ficam em `specs/*/tasks.md` e nas issues).
