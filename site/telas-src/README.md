# Fontes das telas do site

As imagens em `site/img/tela-N-nome.png` mostram a interface **planejada** do Prelo. Não são capturas do app: são páginas HTML desenhadas a partir do protótipo (`prototype/index.html`) e dos requisitos. A página do artigo na prévia é compilada de verdade pelo Typst. Nomes, artigo e referências são fictícios.

| Arquivo | O que é |
|---|---|
| `gerar.py` | Gera `tela-1-inicio.html` … `tela-7-erros.html` nesta pasta |
| `app.css` | Estilo comum das telas |
| `main-display.typ` | Código Typst mostrado no editor das telas |
| `artigo/main.typ`, `artigo/refs.bib` | Artigo de exemplo compilado para a prévia; `--input mode=` escolhe a variante |

## Como regerar

Pré-requisitos: Python 3, `typst` (testado com 0.15.1), Google Chrome e `ffmpeg`.

```sh
cd site/telas-src

# 1. Prévias do artigo (uma por variante), em PNG a 144 ppi
for m in clean marked pending comments; do
  typst compile --input mode=$m artigo/main.typ "prev-$m-{p}.png" --ppi 144
done

# 2. Páginas HTML das telas
python3 -I gerar.py

# 3. Captura a 1440x900 com fator 2 e redução para 256 cores (alvo: menos de 600 KB)
CHROME="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
for f in tela-*.html; do
  n=${f%.html}
  "$CHROME" --headless=new --hide-scrollbars --window-size=1440,900 \
    --force-device-scale-factor=2 --screenshot="/tmp/$n.png" "file://$PWD/$f"
  ffmpeg -loglevel error -y -i "/tmp/$n.png" \
    -vf "split[a][b];[a]palettegen=max_colors=256:stats_mode=full[p];[b][p]paletteuse=dither=none" \
    -pix_fmt pal8 "../img/$n.png"
done

# 4. Apague os arquivos gerados nesta pasta
rm -f tela-*.html prev-*.png
```

O teste `scripts/site.test.ts` recusa imagem acima de 600 KB em `site/`.
