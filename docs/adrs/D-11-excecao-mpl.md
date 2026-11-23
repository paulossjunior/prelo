# D-11 — MPL-2.0 só como exceção por dependência

- **Status:** aceita · 2026-11-18
- **Contexto:** RNF-06 exige dependências com licenças permissivas (Apache-2.0, MIT). A árvore de dependências do Tauri (D-01) costuma incluir crates sob MPL-2.0, como `cssparser` e `selectors`. MPL-2.0 é copyleft por arquivo: usar sem modificar não impõe obrigações ao código do Prelo.
- **Decisão:** MPL-2.0 não entra na lista geral de licenças permitidas. Cada crate MPL-2.0 só é aceita como exceção nomeada em `src-tauri/deny.toml`, com motivo, e apenas se usada sem modificação. Nova exceção exige PR.
- **Alternativas consideradas:**
  - Recusar MPL-2.0: bloquearia o Tauri e contradiria D-01.
  - Incluir MPL-2.0 na lista geral: permitiria novas dependências copyleft sem revisão.
- **Consequências:**
  - (+) O Tauri pode ser usado sem relaxar a regra para o resto do projeto.
  - (−) Cada atualização que traga nova crate MPL-2.0 falha na auditoria até alguém registrar a exceção.
  - (−) Modificar uma dessas crates (fork ou patch) obrigaria publicar os arquivos alterados sob MPL-2.0.

---

Índice de decisões: [adrs.md](../adrs.md)
