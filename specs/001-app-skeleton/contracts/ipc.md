# Contrato IPC: interface → núcleo

Único canal entre interface e núcleo nesta feature. Tudo fora deste contrato MUST ser recusado
(FR-003).

## `get_app_info`

- **Chamada**: `invoke("get_app_info")`, sem argumentos.
- **Sucesso**: `AppInfo` ([data-model.md](../data-model.md#appinfo))

  ```json
  { "version": "0.1.0", "os": "macos", "arch": "aarch64" }
  ```

- **Erros**:

  | Caso | Resultado |
  |---|---|
  | Argumentos extras ou de tipo errado | Rejeição (`Err` string), núcleo segue funcionando |
  | Sistema não reconhecido | Rejeição `"unsupported_os"` |

## Comandos não registrados

`invoke("<qualquer outro>")` MUST ser rejeitado pelo runtime, sem efeito colateral. Teste de
segurança obrigatório.

## `self_test_report` (só build de teste)

- Existe apenas com a feature `self-test`; ausente no binário de release (teste confere).
- **Chamada**: `invoke("self_test_report", { rendered: string, elapsedMs: number })`.
- **Efeito**: o núcleo compara `rendered` com `"Prelo {version} · {os}"` e `elapsedMs` com o
  limite de 3000; encerra o processo com código 0 (ok) ou 1 (falha), registrando o motivo em
  stderr.

## Permissões da janela (capabilities)

A janela principal recebe só o necessário para chamar os comandos acima. Nenhuma permissão de
sistema de arquivos, shell, HTTP, diálogo ou abertura de URL. CSP: `default-src 'self'`, sem
`unsafe-inline` nem `unsafe-eval`. Teste de segurança obrigatório lê os arquivos de
configuração e falha em qualquer permissão fora desta lista.
