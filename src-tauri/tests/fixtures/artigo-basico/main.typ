#import "utils.typ": destaque

#set document(title: "Artigo básico", author: "Prelo")
#set page(paper: "a4")
#set heading(numbering: "1.1")

#align(center, text(17pt)[*Artigo básico de teste*])

= Introdução

Este parágrafo de abertura fica no arquivo principal. #destaque[Texto destacado.]

#lorem(160)

#include "capitulos/metodo.typ"

= Conclusão <conclusao>

Parágrafo final do arquivo principal, que cita a @conclusao.

#lorem(320)
