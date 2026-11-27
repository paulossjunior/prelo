# Quickstart: Comentários (004)

Roteiro de validação ponta a ponta. Detalhes de formato em [data-model.md](data-model.md) e
[contracts/ipc.md](contracts/ipc.md). Na validação de produção (constituição), rodar no `.dmg`
publicado no macOS.

## Pré-requisitos

- Build da `1.1.0` (ou `npm run tauri dev` em desenvolvimento).
- Projeto de fixture `fixtures/review/artigo` (3 seções, `main.typ` + `cap/metodo.typ`).
- `typst` 0.15.1 de linha de comando **só** para o passo 9 (comparação).

## Passos

1. **Nome**: abrir o projeto, selecionar uma frase da Introdução, escolher "Comentar".
   Esperado: o app pede o nome de exibição; informar "Ana".
2. **Criar**: escrever "Qual organização?" e confirmar. Esperado: trecho marcado; conversa no
   painel com "Ana" e a hora; outline mostra `1` na Introdução.
3. **Responder e mencionar**: responder "@" e escolher "Ana" na sugestão. Esperado: menção
   destacada.
4. **Âncora**: digitar um parágrafo antes do trecho e editar uma palavra dentro dele.
   Esperado: a marca segue o trecho.
5. **Encerramento forçado**: criar outro comentário e, logo após aparecer, matar o processo
   (`kill -9`). Reabrir. Esperado: os dois comentários estão lá.
6. **Resolver/reabrir**: resolver o primeiro. Esperado: marca some, outline volta a `0`
   (ou a contagem certa), conversa em "Resolvidos" com "Ana" e hora. Reabrir. Esperado: volta.
7. **Permissões**: trocar o nome para "Bruno" (preferências). Esperado: "Apagar mensagem" e
   "Excluir conversa" não aparecem nas mensagens/conversas de Ana; "Excluir todos os
   resolvidos" não aparece (Ana é dona). Voltar a "Ana": excluir uma conversa pede
   confirmação; cancelar não muda nada.
8. **Trecho removido**: apagar o trecho comentado inteiro. Esperado: conversa em "trecho
   removido"; reancorar numa frase nova funciona.
9. **Typst puro**: `typst compile main.typ` na pasta do projeto e comparar com o PDF final do
   app. Esperado: idênticos; `git diff` do `.typ` não mostra marcação de comentário.
10. **PDF com comentários**: "Exportar PDF com comentários". Esperado: conversas abertas na
    margem, na página do trecho; resolvidas ausentes; o PDF final continua igual ao do passo 9.
11. **Arquivo adulterado**: fechar o app, truncar `.prelo/review.json` pela metade, reabrir.
    Esperado: projeto abre e edita; aviso de comentários ilegíveis; arquivo truncado intacto.
    Criar um comentário. Esperado: cópia `.prelo/review.json.invalid-…` criada antes da
    gravação nova.

## Testes automatizados

- `npm run verify -- --quick` (unidade, integração, segurança, auditoria).
- `cargo test --release --test review_anchor --test review_json_security --test review_crash
  --test review_typ_pure --test review_pdf` em `src-tauri/`.
- e2e: `npm run test:e2e` (cenário criar → responder → resolver).
