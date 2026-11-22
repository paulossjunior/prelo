# D-08 — Salvar (⌘S) sincroniza automaticamente

- **Status:** aceita · novembro de 2026
- **Contexto:** receber, enviar e gravar no git em gestos separados deixa espaço para esquecer um deles.
- **Decisão:** ⌘S recebe, envia a fila, grava commit e faz push. Sem conexão, guarda local e agenda (RF-33, RF-34).
- **Alternativas consideradas:** não registradas na especificação.
- **Consequências:**
  - (+) Um único gesto conhecido garante as três etapas.
  - (−) Cada Salvar gera um commit; o histórico do git fica com muitos commits `Prelo sync`.

---

Índice de decisões: [adrs.md](../adrs.md)
