# D-07 — Edição liberada só com arquivos sincronizados

- **Status:** aceita · novembro de 2026
- **Contexto:** editar sobre uma versão antiga gera trabalho em cima de texto que já mudou.
- **Decisão:** ao abrir, reconectar ou salvar, o app recebe primeiro as mudanças e mantém o editor em leitura até terminar (RF-31, RF-32).
- **Alternativas consideradas:** não registradas na especificação.
- **Consequências:**
  - (+) Evita editar sobre versão antiga.
  - (−) Sincronização lenta atrasa a edição. Mitigação: o bloqueio vale só para o texto; figuras terminam depois sem travar.

---

Índice de decisões: [adrs.md](../adrs.md)
