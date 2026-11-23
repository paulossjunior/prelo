# Quickstart: validar o editor local

Pré-requisitos: os de [001](../001-app-skeleton/quickstart.md). **Não** instale `typst`; se
estiver instalado, tire-o do `PATH` para validar FR-010. O teste de equivalência precisa do
`typst` 0.15.1 e roda só no CI ou com `PRELO_TYPST_CLI=<caminho>`.

## Suíte

```sh
npm run verify          # inclui os testes desta feature e a auditoria (licenças de fontes inclusas)
npm run test:rust -- --test security_file_access --test security_packages --test security_plugins
npm run test:rust -- --release --features perf --test perf_preview -- --nocapture   # SC-001/SC-002
```

Resultado esperado: tudo verde; `perf_preview` imprime p95 por cenário. Ele só compila com a
feature `perf` (em depuração o número não diz nada), roda no job `perf` do CI com limite de
250 ms e aceita `PRELO_PERF_LIMIT_MS=100` para conferir a meta no computador de referência.
O teste de encerramento forçado (T077, SC-009) roda dentro de `npm run test:e2e`
(`scripts/self-test.mjs`) e precisa de sessão gráfica desbloqueada.

## Roteiro manual (5 minutos, SC-008)

1. `npm run dev`, **Abrir projeto** → `src-tauri/tests/fixtures/artigo-basico/`. Árvore,
   editor e prévia aparecem; "Sobre" mostra Typst 0.15.1 e as fontes Libertinus (OFL-1.1).
2. Digite no parágrafo: a prévia muda sem ação. Apague um `]`: erro na linha, prévia mantém a
   última versão com faixa de erro.
3. Clique num parágrafo da prévia: o editor vai para a linha. Com o cursor numa linha, use
   "Localizar na prévia": destaque aparece.
4. Num projeto novo, escreva `#read("../fora.txt")` e `#image("/etc/hosts")`: diagnósticos
   "acesso recusado … fora da pasta do projeto"; o app continua respondendo. (Não há fixture
   `escape/` versionada: links e FIFO são gerados pelos testes `security_file_access`.)
5. Abra `fixtures/plugin/`: aparece o pedido de confirmação; recusar mantém o diagnóstico;
   aprovar compila.
6. Com rede, num projeto novo, escreva `#import "@preview/cetz:0.4.2"`: o pacote é baixado;
   desligue a rede, feche e reabra: a prévia aparece igual. (Não há fixture `pacote/`
   versionada: os testes de pacotes usam arquivos `.tar.gz` gerados.)
7. **Exportar PDF**: arquivo gravado; com erro no `.typ`, a exportação é recusada com a lista.
8. **Novo projeto** numa pasta vazia: `main.typ` e `.prelo/project.json` criados; prévia
   aparece. Em pasta com arquivos: aviso, nada sobrescrito.

Contratos: [ipc.md](contracts/ipc.md), [file-access.md](contracts/file-access.md). Modelo:
[data-model.md](data-model.md).
