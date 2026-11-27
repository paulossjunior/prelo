# Quickstart: Sincronização pelo git (005)

Roteiro ponta a ponta. Formatos em [data-model.md](data-model.md), comandos em
[contracts/ipc.md](contracts/ipc.md). Validação de produção no `.dmg` publicado no macOS.

## Pré-requisitos

- GitHub App do Prelo registrado (conta `paulossjunior`) e instalado no repositório de teste
  `prelo-sync-fixture` (privado), com o projeto de fixture `fixtures/review/artigo` da 004.
- Duas contas do GitHub (A e B) com acesso ao repositório, ou dois computadores com a mesma
  conta (o resumo mostra o nome de exibição).

## Passos

1. **Login (A)**: "Entrar com GitHub". Esperado: código na tela, página do GitHub abre;
   autorizar → "conectado como @A".
2. **Clonar (A)**: "Clonar do GitHub" → `prelo-sync-fixture` → pasta vazia. Esperado: projeto
   abre em "Verificando" e chega a "Sincronizado".
3. **Clone existente (B)**: `git clone` por fora e abrir a pasta no Prelo. Esperado:
   reconhece o remoto e sincroniza.
4. **Salvar (A)**: editar a Introdução, comentar e ⌘S. Esperado: editor em leitura por um
   instante; commit `Prelo sync · A · …` no GitHub; e-mail `noreply`.
5. **Receber e mesclar (B)**: sem salvar antes, editar a Conclusão e a mesma frase que A editou;
   ⌘S. Esperado: texto com as mudanças dos dois, sem marca de conflito; resumo "A fez 1 commit,
   deixou 1 comentário" e aviso de edição no mesmo trecho; `typst compile` do resultado OK.
6. **Offline (A)**: desligar a rede, reabrir o projeto. Esperado: "offline", edição
   bloqueada, comentar funciona; ⌘S → "guardado no computador". Matar o app. Ligar a rede e
   reabrir. Esperado: envia sozinho e libera a edição.
7. **Abrir recebe (A)**: depois de B salvar, abrir o projeto em A. Esperado: "Recebendo…",
   depois libera com o resumo.
8. **Fechar envia (B)**: editar e fechar o app. Esperado: progresso de Salvar e commit no
   GitHub.
9. **Versão nomeada (A)**: "Marcar versão" → "Submissão 1". Esperado: aparece na lista; em B,
   após Salvar, também.
10. **Edição fora do Prelo**: editar o `.typ` pelo site do GitHub no mesmo trecho que A editou
    localmente; ⌘S em A. Esperado: pergunta "manter o meu" / "manter o recebido"; o descartado
    está no histórico do git.
11. **Sair**: "Sair do GitHub". Esperado: credencial removida do Keychain (conferir no Acesso
    às Chaves); projeto vira "login necessário".
12. **Token**: `grep -r` pelo prefixo do token (`ghu_`) em `~/Library/Logs`, na pasta do projeto
    e em `.prelo/`. Esperado: nada.

## Testes automatizados

- `npm run verify -- --quick`.
- `cargo test --release --test sync_merge --test sync_crash --test sync_security
  --test sync_auth --test sync_versions` em `src-tauri/`.
- Desempenho com rede real (manual/noturno): `cargo test --release --test perf_sync -- --ignored`.
