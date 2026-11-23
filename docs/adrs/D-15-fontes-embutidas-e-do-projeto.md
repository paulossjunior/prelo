# D-15 — Fontes: embutidas + pasta do projeto; licença OFL-1.1 aceita só para arquivos de fonte

- **Status:** aceita · 2026-11-19 · decisor: dono do projeto (clarificação Q1 da feature 003) · **emenda aceita · 2026-11-23** (embutir as fontes do `typst` CLI; aprovada pelo dono do projeto, ver [Emenda de 2026-11-23](#emenda-de-2026-11-23-aceita-paridade-de-fontes-com-o-typst-cli))
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

## Emenda de 2026-11-23 (aceita): paridade de fontes com o `typst` CLI

- **Status:** aceita · 2026-11-23 · decisor: dono do projeto (aprovada para a release 1.0.0). Substitui a lista de fontes e de licenças do texto original acima.
- **Contexto:** modelos acadêmicos do Typst Universe (por exemplo `@preview/stellar-springer-nature`) pedem "New Computer Modern" para o texto e deixam matemática e código nos padrões do Typst. No Prelo, sem essas fontes, o texto saía na fonte de reserva com o aviso `unknown font family: new computer modern`, matemática em Libertinus Math e código em Libertinus Mono; no `typst` CLI o mesmo `.typ` sai em New Computer Modern, New Computer Modern Math e DejaVu Sans Mono. O artigo ficava diferente do que o autor vê fora do Prelo, o que contraria o Princípio I e a equivalência de FR-031.
- **Decisão (emenda):**
  - O Prelo embute, além das Libertinus, exatamente as fontes que o `typst` CLI 0.15.1 embute (crate `typst-assets` 0.15.1, feature `fonts`), com os mesmos bytes: New Computer Modern 10 (NewCM10-Regular, -Bold, -Italic, -BoldItalic), New Computer Modern Math (NewCMMath-Regular, -Book, -Bold) e DejaVu Sans Mono (Regular, Bold, Oblique, BoldOblique). Os arquivos ficam em `src-tauri/fonts/` com o texto de cada licença e uma entrada por arquivo no `licenses.toml`.
  - Além de OFL-1.1, passam a ser aceitas **só para arquivos de fonte embutidos**: GUST Font License (`LicenseRef-GUST-Font-License`, LPPL-1.3c; New Computer Modern), GPL-3.0 ou posterior com Font Exception e Distribution Exception (`LicenseRef-NewCM-GPL-3.0-or-later-FE-DE`; só NewCM10-Regular) e Bitstream Vera (`Bitstream-Vera`; DejaVu Sans Mono). Dependências continuam presas a RNF-06.
  - O World deixa de trocar a fonte de matemática e de `raw`: com as fontes do CLI embutidas, valem os padrões do próprio Typst (Libertinus Serif, New Computer Modern Math, DejaVu Sans Mono).
  - O resto da decisão não muda: nenhuma fonte do sistema é lida, e `fonts/` do projeto continua valendo.
- **Por que aceitar GUST e GPL+FE+DE agora:** a alternativa rejeitada em 2026-11-19 temia termos copyleft. Na prática: a GUST (LPPL-1.3c) permite distribuir os arquivos sem alteração junto de qualquer programa; a Font Exception garante que o PDF gerado não fica sob a GPL; a Distribution Exception do NewCM10-Regular diz que a fonte distribuída sem alteração não põe o programa sob a GPL, desde que o programa tenha licença compatível com a GPL-3.0 — MIT e Apache-2.0, as do Prelo, são. O Prelo distribui os arquivos sem nenhuma alteração (mesmos bytes do `typst-assets`), e o Typst faz o mesmo no CLI oficial, sob Apache-2.0.
- **Custo:** +11 arquivos, +7 825 752 bytes (≈ 7,8 MB) embutidos no binário; `src-tauri/fonts/` vai de 3,4 MB para 11,2 MB de fontes.
- **Consequências:**
  - (+) Templates acadêmicos que pedem New Computer Modern compilam sem aviso e saem iguais ao `typst` CLI; um `.typ` sem fonte definida também sai igual (matemática e código nos padrões do Typst).
  - (+) O teste de equivalência compila o `main.typ` direto, sem arquivo de entrada com regras de fonte.
  - (−) Binário cerca de 7,8 MB maior.
  - (−) Documentos já escritos no Prelo sem fonte definida para matemática ou código mudam de aparência (Libertinus Math/Mono → New Computer Modern Math/DejaVu Sans Mono). Quem quiser o visual antigo escreve `#show math.equation: set text(font: "Libertinus Math")` e `#show raw: set text(font: "Libertinus Mono")` no `.typ`.
  - (−) A lista de licenças só para fontes cresce de uma para quatro, incluindo uma GPL com exceções; cada atualização do `typst-assets` exige conferir o NOTICE de novo.

---

Índice de decisões: [adrs.md](../adrs.md)
