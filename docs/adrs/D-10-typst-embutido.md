# D-10 — Compilador Typst embutido no núcleo

- **Status:** aceita · novembro de 2026
- **Contexto:** prévia, diagnósticos e PDFs precisam sair do mesmo compilador e na mesma versão para todos os coautores, sem exigir instalação separada (RF-06, RF-07).
- **Decisão:** typst e Tinymist entram como bibliotecas Rust no núcleo. A versão do compilador fica registrada em `.prelo/project.json`.
- **Alternativas consideradas:** chamar o `typst` instalado na máquina (descartada: versões diferentes entre coautores e dependência de instalação manual).
- **Consequências:**
  - (+) Ninguém precisa instalar `typst`; prévia e PDF final batem.
  - (−) Atualizar o Typst exige nova versão do Prelo, e uma versão nova pode mudar o layout. Falta definir quem decide a troca de versão no projeto.
  - (−) Pacotes `@preview` dependem da internet. Falta definir cache local para compilar offline e quais fontes vêm embutidas.

---

Índice de decisões: [adrs.md](../adrs.md)
