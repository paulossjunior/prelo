# D-17 — Análise do editor via `typst-ide` oficial em vez de `tinymist-query`

- **Status:** aceita · 2026-11-19 · decisor: dono do projeto
- **Contexto:** o plano da feature 003 previa autocompletar, realce e *hover* pelo `tinymist-query` (research R2), sobre o World do `tinymist-world`. Ao adicionar as dependências (tarefa T003), o `tinymist-query` 0.15.x publicado no crates.io não compilou com o `typst` 0.15.1 oficial: a crate `tinymist-analysis`, de que ele depende, usa `typst::foundations::FuncInner` e `Func::inner()`, que só existem no *fork* do Typst mantido pelo Tinymist; o `typlite` 0.15.12 também não compila contra `tinymist-world` 0.15.0. Já `tinymist-world`, `tinymist-vfs` e `tinymist-package` 0.15.0 compilam com o `typst` oficial. D-10 exige o compilador embutido, e SC-005 exige PDF idêntico ao do `typst` de linha de comando na mesma versão, o que um *fork* não garante.
- **Decisão:** autocompletar, diagnósticos, *hover* e a sincronização código ↔ prévia vêm do `typst-ide` oficial (`=0.15.1`), na mesma versão do compilador. O realce de sintaxe vem do próprio Typst (`typst-syntax`: `LinkedNode` e `highlight`). O Tinymist continua em uso só para `tinymist-world`, `tinymist-vfs` e `tinymist-package` (World, VFS com o `PathAccessModel` do Prelo e a trait de registro de pacotes). `tinymist-query` e `typlite` não entram.
- **Alternativas consideradas:**
  - `[patch.crates-io]` com o *fork* git do Typst usado pelo Tinymist: o compilador deixaria de ser o oficial (D-10, SC-005), a fonte git é proibida pela auditoria (`sources.unknown-git = "deny"`) e cada versão exigiria acompanhar o *fork*.
  - Servidor LSP `tinymist` como processo separado: binário extra por sistema, World diferente do da prévia e política de acesso a arquivos fora do núcleo do Prelo (research R2).
  - Esperar uma versão do `tinymist-query` compatível com o `typst` oficial: sem data, bloqueia a fase 1.
- **Consequências:**
  - (+) Compilador, prévia, diagnósticos, autocompletar e PDF saem do mesmo `typst` oficial, na versão registrada (D-10, SC-005).
  - (+) Menos dependências: sem `tinymist-analysis`, `tinymist-lint`, `tinymist-project` e `typlite`.
  - (−) Mais código nosso: a camada `lang_service.rs` converte as respostas do `typst-ide` e gera os *tokens* de realce a partir de `typst-syntax`; recursos avançados do Tinymist (análise de tipos, *lint*, *inlay hints*) ficam de fora até nova decisão.
  - (−) O Tinymist fica restrito a World, VFS e pacotes; a linha "typst + Tinymist" das Restrições Técnicas da constituição continua valendo para essas partes.

---

Índice de decisões: [adrs.md](../adrs.md)
