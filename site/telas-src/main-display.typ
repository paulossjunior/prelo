#set document(title: "Revisão a várias mãos")
#set page(paper: "a4", margin: (x: 1.8cm, y: 2cm), columns: 2, numbering: "1")
#set text(font: "Libertinus Serif", size: 9.6pt, lang: "pt")
#set par(justify: true)
#set heading(numbering: "1.")
#set math.equation(numbering: "(1)")

#place(top, scope: "parent", float: true)[
  #align(center)[
    #text(17pt, weight: "bold")[Revisão a várias mãos: como grupos de pesquisa \ sugerem, comentam e aceitam mudanças]
    #text(10.5pt)[Ana Reis#super[1] · Bruno Lima#super[2] · Carla Dias#super[1]]
  ]
  #text(8.8pt)[*Resumo.* Artigos com vários autores passam por ciclos de revisão em que alguém sugere uma frase, outro comenta uma figura e o autor principal aceita parte e rejeita o resto.]
]

= Introdução <sec-intro>
Coautores revisam o mesmo texto em horários diferentes @silva2024. O orientador lê uma versão, o coautor edita outra e as mudanças se cruzam no e-mail.

Ferramentas de controle de alterações resolvem parte do problema, mas costumam exigir um servidor central e uma conexão permanente @moura2023.

= Trabalhos relacionados
Estudos sobre escrita acadêmica descrevem a revisão como uma conversa assíncrona entre papéis distintos @silva2024.

= Método
Acompanhamos a escrita de artigos em grupos de pesquisa durante um semestre. A taxa de aceite de uma seção $s$ é
$ a(s) = (sum_(i in S_s) bold(1)[d_i = "aceita"]) / (|S_s|) $ <eq-aceite>

#figure(
  image("figures/fluxo.png", width: 100%),
  caption: [Fluxo de uma sugestão até o texto aceito.],
) <fig-fluxo>

= Resultados
A @fig-fluxo resume o ciclo observado. A taxa média de aceite, calculada pela @eq-aceite, variou entre as seções (@tab-aceite).

// TODO: conferir números com a Carla
#figure(
  table(columns: 3, [*Seção*], [*Sugestões*], [*Aceite*], ...),
  caption: [Sugestões por seção (dados ilustrativos).],
) <tab-aceite>

= Discussão
Três padrões apareceram com frequência.

#bibliography("refs.bib", style: "associacao-brasileira-de-normas-tecnicas")
