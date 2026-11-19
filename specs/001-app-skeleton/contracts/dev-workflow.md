# Contrato: comandos de desenvolvimento e trava de push

## Comandos

| Comando | Faz | Saída |
|---|---|---|
| `npm install` | Instala dependências e ativa `.githooks` (`core.hooksPath`) | — |
| `npm run dev` | Abre o app em modo desenvolvimento | Janela |
| `npm run build` | Gera o pacote do sistema atual | Pacote em `src-tauri/target/release/bundle/` |
| `npm run verify` | fmt, clippy, tsc, testes do núcleo, ponte, interface, ponta a ponta, auditoria | Código 0 só se tudo passar; resumo por etapa |
| `npm run audit` | Só a auditoria (licenças e vulnerabilidades, Rust e npm) | Código 0/1; nomeia dependência e licença ou aviso |

`npm run verify` MUST:

- rodar todas as etapas mesmo após uma falha e, no fim, listar as que falharam;
- terminar com código diferente de 0 se qualquer etapa falhar, inclusive teste pulado sem
  justificativa (busca por `#[ignore]`, `.skip`, `.only` sem comentário `// justificativa:`);
- no macOS, rodar o autoteste no lugar do WebDriver.

## Trava local (`.githooks/pre-push`)

- Roda `npm run verify`; código ≠ 0 bloqueia o push e mostra a etapa que falhou.
- Não há opção para pular. `--no-verify` é proibido pela constituição e coberto pela trava
  remota.

## Trava remota

- Workflow `CI`, job `verify`, matriz `ubuntu-22.04`, `windows-latest`, `macos-latest`; roda
  em todo `pull_request` e em `push` na `main`.
- Proteção da `main`: PR obrigatório; checks `verify (ubuntu-22.04)`,
  `verify (windows-latest)`, `verify (macos-latest)` obrigatórios e atualizados com a `main`;
  `enforce_admins: true`; sem force-push nem exclusão.
