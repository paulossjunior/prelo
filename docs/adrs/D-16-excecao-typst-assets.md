# D-16 — CC-BY-4.0 e W3C só como exceção da crate `typst-assets`

- **Status:** aceita · 2026-11-19 · decisor: dono do projeto
- **Contexto:** o compilador embutido (D-10) depende da crate `typst-assets`, declarada Apache-2.0. Mesmo sem a feature `fonts`, ela compila para dentro do binário dados com licenças próprias, listadas no `NOTICE` da crate (versão 0.15.1):
  - perfis de cor ICC (`files/icc/`), sob CC0-1.0;
  - fontes Foxit (`files/fonts/Foxit*.pfb`), usadas ao embutir PDFs como imagem, sob BSD-3-Clause;
  - tabelas de HTML (`files/html/data.rs`) derivadas das especificações do WHATWG, sob CC-BY-4.0 (as partes incorporadas em código-fonte, sob BSD-3-Clause);
  - tabelas de HTML e MathML (`files/html/data.rs`, `files/mathml/data.rs`) derivadas de especificações do W3C (WAI-ARIA 1.1, Referrer Policy, MathML Core), sob a licença de software e documento do W3C (SPDX `W3C-20150513`).

  CC0-1.0 e BSD-3-Clause já estão na lista de RNF-06; CC-BY-4.0 e W3C não. Sem esses dados não há `typst` e, portanto, não há compilação embutida.
- **Decisão:** CC-BY-4.0 e W3C-20150513 são aceitas só como exceção nomeada da crate `typst-assets` em `src-tauri/deny.toml` (`[[licenses.exceptions]]`), no mesmo padrão de D-11. Não entram na lista geral de licenças permitidas. A exceção é coberta por teste: só `typst-assets` pode tê-la, e as duas licenças não podem aparecer na lista geral. Os avisos de atribuição exigidos pelas duas licenças (texto do `NOTICE` da crate) acompanham o app na tela "Sobre", junto das licenças das fontes (D-15).
- **Alternativas consideradas:**
  - Incluir CC-BY-4.0 e W3C na lista geral: outras dependências poderiam trazer essas licenças sem revisão.
  - *Fork* do `typst-assets` sem esses dados: quebra recursos do Typst (exportação HTML, MathML, atributos ARIA) e obriga a manter um *fork* a cada versão do Typst.
  - Recusar a crate: impossível sem abandonar o compilador embutido (D-10).
- **Consequências:**
  - (+) O compilador embutido fica, sem relaxar a regra para o resto do projeto.
  - (+) Nova crate com CC-BY-4.0 ou W3C falha na auditoria até alguém registrar outra decisão.
  - (−) O `cargo-deny` lê só a licença declarada da crate (Apache-2.0); a exceção documenta e trava o escopo, e a verificação dos arquivos embutidos (D-15) lista os dados e suas licenças.
  - (−) Cada atualização do `typst-assets` exige reler o `NOTICE` e ajustar a exceção se surgir licença nova.
  - (−) O app precisa exibir os avisos de atribuição de CC-BY-4.0 e W3C.

---

Índice de decisões: [adrs.md](../adrs.md)
