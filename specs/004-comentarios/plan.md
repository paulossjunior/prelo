# Implementation Plan: Comentários (fase 2, parte 1)

**Branch**: `004-comentarios` | **Date**: 2026-11-26 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/004-comentarios/spec.md`

## Summary

Comentários ancorados num trecho de qualquer `.typ` do projeto, com respostas, menções,
resolver, reabrir, apagar e excluir, aba "Resolvidos", contagem no outline e PDF com
comentários na margem (RF-04 parcial, RF-16, RF-17, RF-43). O núcleo Rust passa a manter um
documento Yjs (`yrs`) por `.typ`, alimentado pelos mesmos deltas de `edit_file` da 003; as
âncoras são `StickyIndex` (Q-07). Conversas ficam em `.prelo/review.json` e o estado CRDT em
`.prelo/crdt/`, ambos gravados de forma atômica antes de confirmar a ação e lidos como entrada
não confiável. O `.typ` nunca recebe marcação (Princípio I). O PDF com comentários é uma
segunda compilação em memória que embute as páginas do PDF normal e acrescenta a coluna de
notas, sem mudar o leiaute do documento.

## Technical Context

**Language/Version**: Rust (edição e toolchain da 003) no núcleo; TypeScript 7 na interface.

**Primary Dependencies**: existentes da 003 (Tauri 2.12, typst/typst-pdf/typst-svg/typst-ide
0.15.1, CodeMirror 6). Novas: `yrs =0.28.2` (MIT), `similar =3.2.1` (Apache-2.0). `getrandom`
já está na árvore.

**Storage**: arquivos no projeto: `.prelo/review.json`, `.prelo/crdt/<id>.yjs`, campo
`owner` em `.prelo/project.json`; fora do projeto: `<config>/Prelo/settings.json`.

**Testing**: `cargo test` (unidade + integração em `src-tauri/tests/`), vitest/jsdom,
wdio + tauri-driver (e2e), `npm run verify`.

**Target Platform**: macOS, Windows, Linux (RNF-01).

**Project Type**: app desktop (Tauri: núcleo Rust + interface web).

**Performance Goals**: ação de comentário visível em ≤ 200 ms com 500 conversas; prévia
< 100 ms após edição em 40 páginas não pode piorar (SC-005, RF-02).

**Constraints**: offline (FR-026); gravação atômica antes de confirmar (FR-023/024); entrada
não confiável com limites (research R5); nada no `.typ`.

**Scale/Scope**: até 5.000 conversas por projeto, 500 mensagens por conversa, 10.000
caracteres por mensagem.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Princípio | Situação | Como |
|---|---|---|
| I. Typst puro | ✅ | Comentários e estado CRDT só em `.prelo/` (FR-022, FR-007); PDF com comentários compila documento sintético em memória; teste `review_typ_pure` compara com o `typst` CLI (SC-001) |
| II. Privacidade sem servidor | ✅ | Nada sai do computador nesta spec. RNF-02 vale para servidores do Prelo (Q-05). Identidade por chave fica na R3; limitação documentada na spec |
| III. Nenhum trabalho perdido | ✅ | Gravação atômica antes de confirmar; teste de encerramento forçado (SC-002); trecho removido nunca descartado em silêncio (FR-009) |
| IV. Edição só sobre versão sincronizada | ✅ n/a | Sem sincronização nesta spec (005). Comentar offline é permitido (FR-026) |
| V. Revisão explícita e reversível | ✅ | Âncora por posição relativa do Yjs (FR-006); resolver, não aceitar (D-03, FR-016); excluir conversa pede confirmação, inclusive no núcleo (`confirm: true`); regras por autor de requisitos.md §1 |
| VI. Testes primeiro e segurança | ✅ | TDD; testes de segurança: `review.json`/`.yjs` malformados, adulterados, gigantes, com caminho fora da raiz; recusa de cada ação proibida no núcleo (SC-006); mensagens como texto literal (FR-014); auditoria de `yrs` e `similar` |
| Restrições técnicas | ✅ | Yjs/yrs já está na stack; `similar` é dependência auxiliar, não troca de componente (sem D-xx novo) |
| Fluxo: ordem das fases | ✅ | Fase 2 depois da fase 1 |
| Fluxo: transição entre features | ⚠️ exceção | Ver Complexity Tracking |

**Re-check pós-design (Fase 1)**: data-model, contrato IPC e quickstart mantêm todos os itens
acima; nenhuma violação nova.

## Project Structure

### Documentation (this feature)

```text
specs/004-comentarios/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/ipc.md
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
src-tauri/src/
├── review/                 # novo
│   ├── mod.rs              # estado de revisão do projeto aberto, permissões
│   ├── file.rs             # leitura/validação/gravação atômica de .prelo/review.json
│   ├── crdt.rs             # Doc yrs por .typ, deltas de edit_file, StickyIndex, .prelo/crdt/
│   ├── reconcile.rs        # diff disco ↔ Doc (similar)
│   ├── identity.rs         # settings.json (displayName) e owner do project.json
│   ├── outline.rs          # contagem por seção
│   └── pdf.rs              # PDF com comentários (2 passagens)
├── project/record.rs       # + campo owner
├── compile/export.rs       # reaproveitado por review/pdf.rs
└── commands.rs             # + comandos de contracts/ipc.md

src-tauri/tests/
├── review_anchor.rs  review_json_security.rs  review_crash.rs
├── review_permissions.rs  review_typ_pure.rs  review_pdf.rs

src/
├── review/                 # novo
│   ├── panel.ts            # painel Abertos/Resolvidos, cartões de conversa
│   ├── marks.ts            # decorações CodeMirror das âncoras
│   ├── composer.ts         # caixa de mensagem com @menção
│   └── identity.ts         # pedido do nome, preferências
├── project/outline.ts      # + contagem
├── export.ts               # + PDF com comentários
└── ipc.ts                  # + comandos

fixtures/review/            # projetos de fixture (artigo, adulterados)
```

**Structure Decision**: mesmo layout da 003 (núcleo em `src-tauri/src`, interface em `src/`),
com um módulo `review` em cada lado.

## Complexity Tracking

| Violação | Por que é preciso | Alternativa mais simples rejeitada porque |
|---|---|---|
| Spec e plano da 004 começam antes de a `1.0.0` estar "em produção" (constituição, Fluxo: transição entre features). Faltavam, em 2026-11-26: publicar o rascunho do release `v1.0.0`, validar o `.dmg` no macOS com evidência no épico #38, fechar #145 e #38 e veredito APROVADO do QA | O dono pediu as próximas features enquanto valida a `1.0.0` depois; planejar não toca em código nem na versão publicada. **Aprovado pelo dono em 2026-11-26, só para especificação e planejamento.** A implementação (qualquer tarefa de código do tasks.md) MUST NOT começar até a `1.0.0` cumprir a regra; defeito encontrado na validação vira `1.0.x` antes | Esperar a validação deixaria o time parado; não há risco para a `1.0.0` em escrever documentos numa branch separada |
| Yjs adotado na fase 2, antes do tempo real (fase 3) | Princípio V exige âncora por posição relativa do Yjs; decisão do dono Q-07 | Âncora por texto + contexto exigiria migração na R3 e quebra o Princípio V |
