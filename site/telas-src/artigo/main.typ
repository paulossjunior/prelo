#let mode = sys.inputs.at("mode", default: "clean")
#let ins(body) = if mode == "marked" { highlight(fill: rgb("#dcf1e2"), text(fill: rgb("#1b6534"), underline(body))) } else { none }
#let del(body) = if mode == "marked" { highlight(fill: rgb("#f8dfdc"), text(fill: rgb("#a3271f"), strike(body))) } else { body }
#let cmt(body) = if mode == "comments" { highlight(fill: rgb("#fbedc6"), body) } else { body }

#set document(title: "Revisão a várias mãos", author: ("Ana Reis", "Bruno Lima", "Carla Dias"))
#set page(paper: "a4", margin: (x: 1.8cm, y: 2cm), columns: 2,
  numbering: "1", header: align(right, text(8pt, fill: luma(90))[Revista Exemplo de Métodos · v. 7, 2026]))
#set text(font: "Libertinus Serif", size: 9.6pt, lang: "pt")
#set par(justify: true, leading: 0.58em)
#set heading(numbering: "1.")
#show heading: set text(size: 10.5pt)
#set math.equation(numbering: "(1)")

#place(top, scope: "parent", float: true)[
  #align(center)[
    #text(17pt, weight: "bold")[Revisão a várias mãos: como grupos de pesquisa \ sugerem, comentam e aceitam mudanças]
    #v(6pt)
    #text(10.5pt)[Ana Reis#super[1] · Bruno Lima#super[2] · Carla Dias#super[1]]
    #v(2pt)
    #text(8pt, fill: luma(80))[#super[1]Universidade Exemplo · #super[2]Instituto Fictício de Pesquisa]
  ]
  #v(6pt)
  #block(inset: (x: 1.2cm))[
    #set par(justify: true)
    #text(8.8pt)[*Resumo.* Artigos com vários autores passam por ciclos de revisão em que alguém sugere uma frase, outro comenta uma figura e o autor principal aceita parte e rejeita o resto. Este estudo descreve esses ciclos em grupos de pesquisa e propõe um modelo de revisão em que cada mudança é uma sugestão explícita, aceita ou rejeitada antes de entrar no texto.]
    #v(2pt)
    #text(8.8pt)[*Palavras-chave:* escrita colaborativa; revisão; controle de alterações.]
  ]
  #v(4pt)
]

= Introdução
Coautores revisam o mesmo texto em #del[horários]#ins[momentos] diferentes @silva2024. #cmt[O orientador lê uma versão, o coautor edita outra] e as mudanças se cruzam no e-mail.

Ferramentas de controle de alterações resolvem parte do problema, mas costumam exigir um servidor central e uma conexão permanente @moura2023. Este trabalho investiga um fluxo em que a revisão é explícita e reversível.

= Trabalhos relacionados
Estudos sobre escrita acadêmica descrevem a revisão como uma conversa assíncrona entre papéis distintos: quem escreve, quem orienta e quem avalia @silva2024. Sistemas de edição em tempo real, por sua vez, privilegiam a fusão automática de mudanças e deixam a decisão editorial para depois @teixeira2022.

Entre esses dois extremos, o controle de alterações registra cada mudança como proposta. A decisão de aceitar ou rejeitar continua humana, mas fica rastreável @moura2023.

= Método
Acompanhamos a escrita de artigos em grupos de pesquisa durante um semestre. Para cada sugestão registramos autor, seção e decisão. A taxa de aceite de uma seção $s$ é
$ a(s) = (sum_(i in S_s) bold(1)[d_i = "aceita"]) / (|S_s|) $ <eq-aceite>
em que $S_s$ é o conjunto de sugestões da seção e $d_i$ a decisão tomada.

#figure(
  if mode == "pending" {
    rect(width: 100%, height: 2.3cm, stroke: (dash: "dashed", paint: luma(120)), fill: luma(245),
      align(center + horizon, text(8pt, fill: luma(70), font: "DejaVu Sans Mono")[Aguardando figures/fluxo.png]))
  } else {
    box(width: 100%, height: 2.3cm, {
      let node(x, label, c) = place(dx: x, dy: 0.2cm, rect(width: 1.9cm, height: 1.1cm, radius: 3pt, fill: c, stroke: 0.6pt + luma(60), align(center + horizon, text(7.5pt, label))))
      node(0cm, [Sugestão], rgb("#dcf1e2"))
      node(2.6cm, [Revisão], rgb("#fbedc6"))
      node(5.2cm, [Texto aceito], rgb("#d6ecee"))
      place(dx: 1.95cm, dy: 0.75cm, line(length: 0.6cm, stroke: 0.8pt))
      place(dx: 4.55cm, dy: 0.75cm, line(length: 0.6cm, stroke: 0.8pt))
      place(dx: 2.9cm, dy: 1.6cm, text(7pt, fill: luma(70))[rejeitar ↺])
    })
  },
  caption: [Fluxo de uma sugestão até o texto aceito.],
) <fig-fluxo>

= Resultados
A @fig-fluxo resume o ciclo observado. A taxa média de aceite, calculada pela @eq-aceite, variou entre as seções (@tab-aceite).

#figure(
  table(columns: 3, stroke: none, align: (left, right, right),
    table.hline(), table.header([*Seção*], [*Sugestões*], [*Aceite*]), table.hline(stroke: 0.4pt),
    [Introdução], [42], [71%], [Método], [35], [63%], [Resultados], [28], [82%], table.hline()),
  caption: [Sugestões por seção (dados ilustrativos).],
) <tab-aceite>

Seções de resultados recebem menos sugestões, mas com aceite maior, o que sugere revisões mais pontuais @teixeira2022.

= Discussão
Três padrões apareceram com frequência. Primeiro, sugestões pequenas e localizadas eram aceitas quase sempre, enquanto reescritas longas geravam conversa antes da decisão. Segundo, comentários sem resposta acumulavam-se perto da submissão. Terceiro, mudanças feitas fora da ferramenta de revisão eram as mais difíceis de reconciliar.

Esses padrões indicam que a revisão funciona melhor quando cada mudança tem autor, lugar e estado visíveis, e quando resolver um comentário é um gesto distinto de aceitar uma sugestão.

= Conclusão
Tornar cada mudança uma sugestão explícita reduz o retrabalho e deixa claro quem decidiu o quê.

#bibliography("refs.bib", title: "Referências", style: "associacao-brasileira-de-normas-tecnicas")
