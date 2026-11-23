# D-15 — Fontes: embutidas + pasta do projeto; licença OFL-1.1 aceita só para arquivos de fonte

- **Status:** aceita · 2026-11-19 · decisor: dono do projeto (clarificação Q1 da feature 003)
- **Contexto:** o `.typ` pede fontes pelo nome; quem decide quais existem é o World do núcleo ([arquitetura.md §6.5](../arquitetura.md#65-fontes-e-pacotes)). D-10 quer que todos os coautores compilem igual, o que fontes do sistema impedem. Quase todas as fontes livres têm licença própria, em geral SIL OFL-1.1, que RNF-06 não cita. As fontes que o Typst traz embutidas (crate `typst-assets` 0.15.1) têm licenças variadas: Libertinus Serif sob OFL-1.1; New Computer Modern sob GUST Font License (LPPL) e, no caso de NewCM10-Regular, GPL-3.0 com Font Exception; DejaVu Sans Mono sob a licença Bitstream Vera.
- **Decisão:**
  - O World entrega só (a) as fontes embutidas no app e (b) os arquivos de fonte da pasta `fonts/` do projeto, que segue RF-08. Fontes instaladas no sistema nunca são lidas.
  - Arquivos de fonte embutidos no app podem ter licença OFL-1.1, além das licenças permitidas por RNF-06. A exceção vale só para arquivos de fonte: código e demais dependências sob OFL-1.1 continuam proibidos.
  - Fontes da pasta `fonts/` do projeto são conteúdo do artigo, como figuras: o app não as distribui nem as audita; a licença de cada uma é responsabilidade de quem a adiciona.
- **Alternativas consideradas:**
  - Só fontes embutidas: reprodutível, mas o artigo fica limitado ao que o app traz, e modelos de revista com fonte própria não funcionam.
  - Embutidas + fontes do sistema: cômodo, mas prévia e PDF mudam de um computador para outro, o que anula parte de D-10.
  - Aceitar também GUST/LPPL e GPL com Font Exception para fontes embutidas: usaria o conjunto do Typst sem mudança, mas amplia a regra de licenças para termos copyleft que o projeto evita (RNF-06, D-11).
- **Consequências:**
  - (+) Todos os coautores, na mesma versão do Prelo, compilam com o mesmo conjunto de fontes.
  - (+) Modelos com fonte própria funcionam com a fonte copiada para `fonts/`, e o `.typ` compila fora do Prelo com `typst compile --font-path fonts` (Princípio I).
  - (−) As fontes do `typst-assets` sob GUST, GPL e Bitstream Vera não são embutidas, porque a exceção é só para OFL-1.1. O Prelo embute substitutas sob OFL-1.1, e o padrão do Typst para matemática (New Computer Modern Math) e para código (DejaVu Sans Mono) fica diferente do `typst` de linha de comando; quem precisar delas as coloca em `fonts/`.
  - (−) A comparação de PDF com o `typst` de linha de comando precisa usar o mesmo conjunto de fontes (`--ignore-system-fonts --ignore-embedded-fonts --font-path`).
  - (−) A OFL-1.1 exige distribuir o aviso de copyright e o texto da licença junto com as fontes; o app precisa exibi-los.
  - (−) A auditoria de licenças por crate não enxerga arquivos de fonte embutidos; é preciso uma verificação própria desses arquivos.

---

Índice de decisões: [adrs.md](../adrs.md)
