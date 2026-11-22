# D-14 — Edição concorrente de bibliografia

- **Status:** aceita · 2026-11-18
- **Contexto:** `.bib` e `.yml` (RF-09) são texto e podem ser editados por dois coautores ao mesmo tempo, como o `.typ`. O documento Yjs hoje cobre o texto do artigo (D-02); a arquitetura não diz se a bibliografia entra nele. Ferramentas externas, como exportação automática de gerenciadores de referência, costumam reescrever o arquivo inteiro.
- **Decisão:** `.bib` e `.yml` como `Y.Text` no documento Yjs, com as mesmas regras do `.typ` (edição direta só sincronizado, sugestões, comentários). `.csl` e binários como arquivo inteiro, pelo transporte de D-12.
- **Alternativas consideradas:**
  - Arquivo inteiro, última gravação vence: simples, mas uma das duas edições simultâneas some, o que contraria o Princípio III.
  - Arquivo inteiro com detecção de conflito: guarda o hash da versão base e, se as duas versões mudaram, pede ao usuário que escolha ou junte. Não perde trabalho, mas exige uma tela de conflito e interrompe quem edita referências com frequência.
  - Fusão estruturada por entrada (pela chave de citação): o resultado é sempre BibLaTeX válido entrada a entrada, mas exige analisar e regravar o arquivo, o que pode perder comentários e formatação, e não serve para `.yml` sem um segundo analisador.
- **Consequências:**
  - (+) Um único modelo para todo texto do projeto: CRDT, fila offline, sugestões e comentários já especificados (D-02, D-06, D-07).
  - (+) Edições em entradas diferentes se fundem sem conflito.
  - (+) Mudança de revisor em referência vira sugestão, como no texto (Princípio V).
  - (−) Duas pessoas editando a mesma entrada ao mesmo tempo podem gerar BibLaTeX inválido; o erro aparece como diagnóstico (RF-09), não é evitado.
  - (−) Reescrita do arquivo inteiro por ferramenta externa chega como diff e cai no mesmo problema em aberto da edição fora do app ([arquitetura.md §7](../arquitetura.md#7-riscos-técnicos-e-questões-em-aberto)).
  - (−) TODO(bib-grande): medir o custo de um `.bib` com milhares de entradas como `Y.Text` no carregamento e na sincronização.

---

Índice de decisões: [adrs.md](../adrs.md)
