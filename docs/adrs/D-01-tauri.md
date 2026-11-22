# D-01 — App desktop com Tauri

- **Status:** aceita · novembro de 2026
- **Contexto:** o Prelo precisa rodar em macOS, Windows e Linux (RNF-01) e funcionar offline. O compilador Typst e o Tinymist são escritos em Rust.
- **Decisão:** shell Tauri, com interface em HTML/TypeScript e núcleo em Rust.
- **Alternativas consideradas:** TODO(D-01): a especificação não registra as alternativas avaliadas (ex.: Electron, app nativo por plataforma).
- **Consequências:**
  - (+) Núcleo na mesma linguagem do Typst e do Tinymist, no mesmo processo.
  - (+) Instaladores leves.
  - (−) A interface depende do WebView de cada sistema, que muda entre plataformas.

---

Índice de decisões: [adrs.md](../adrs.md)
