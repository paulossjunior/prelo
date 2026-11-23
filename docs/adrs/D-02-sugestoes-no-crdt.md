# D-02 — Sugestões como atributos do CRDT, fora do `.typ`

- **Status:** aceita · novembro de 2026
- **Contexto:** o `.typ` no repositório deve ser Typst puro, compilável fora do Prelo (RNF-05). Sugestões precisam marcar trechos sem alterar esse arquivo.
- **Decisão:** sugestões são atributos de formatação sobre trechos do `Y.Text`; comentários e sugestões persistem em `.prelo/review.json`. Ver [arquitetura.md §4](../arquitetura.md#4-modelo-de-revisão).
- **Alternativas consideradas:** marcações dentro do próprio `.typ` (descartada: suja o código-fonte e quebra quem não usa o Prelo).
- **Consequências:**
  - (+) Quem não usa o Prelo vê um repositório Typst normal.
  - (−) Edições feitas fora do app chegam como diff do git e precisam virar sugestões de um autor "externo"; o tratamento de trechos alterados nos dois lados ainda está em aberto ([arquitetura.md §7](../arquitetura.md#7-riscos-técnicos-e-questões-em-aberto)).

---

Índice de decisões: [adrs.md](../adrs.md)
