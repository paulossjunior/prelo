// Gerador do corpo do artigo de 40 páginas (T010, SC-001). Copiado como
// artigo-40p/corpo.typ; mudar aqui exige recopiar e conferir as 40 páginas.
#let corpo(secoes: 20) = {
  for i in range(1, secoes + 1) {
    heading(level: 1)[Seção #i]
    for j in range(1, 4) {
      heading(level: 2)[Subseção #i.#j]
      par(lorem(130))
      par(lorem(110))
    }
    $ integral_0^#i x^2 dif x = #i^3 / 3 $
  }
}
