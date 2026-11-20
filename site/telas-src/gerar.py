"""Gera as 7 páginas HTML de alta fidelidade das telas propostas do Prelo.

Uso: python3 -I gerar.py  (gera tela-N-nome.html nesta pasta)
"""
import html
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = open(os.path.join(HERE, "main-display.typ"), encoding="utf-8").read().split("\n")

TOK = re.compile(
    r'(?P<c>//.*$)|(?P<s>"[^"]*")|(?P<m>\$[^$]*\$)|(?P<fn>#[A-Za-z][\w.-]*)|(?P<ref>@[\w-]+)|(?P<lbl><[\w-]+>)'
)


def hl(text):
    """Realce simples de sintaxe Typst."""
    if re.match(r"^=+ ", text):
        m = re.search(r"\s<[\w-]+>$", text)
        if m:
            return f'<span class="t-h">{html.escape(text[:m.start()])}</span> <span class="t-lbl">{html.escape(m.group().strip())}</span>'
        return f'<span class="t-h">{html.escape(text)}</span>'
    out, pos = [], 0
    for m in TOK.finditer(text):
        out.append(html.escape(text[pos:m.start()]))
        cls = {"c": "t-c", "s": "t-s", "m": "t-m", "fn": "t-fn", "ref": "t-ref", "lbl": "t-lbl"}[m.lastgroup]
        out.append(f'<span class="{cls}">{html.escape(m.group())}</span>')
        pos = m.end()
    out.append(html.escape(text[pos:]))
    return "".join(out)


def code(first, last, over=None, cls=None, lines=None):
    """Linhas first..last (1-based) do código; over: n -> HTML; cls: n -> classe."""
    over, cls = over or {}, cls or {}
    src = lines or SRC
    rows = []
    for n in range(first, last + 1):
        body = over.get(n, hl(src[n - 1]) if n - 1 < len(src) else "")
        rows.append(f'<div class="ln {cls.get(n, "")}"><span class="g">{n}</span><span class="c">{body or " "}</span></div>')
    return '<div class="code">' + "".join(rows) + "</div>"


ICON = {
    "file": '<svg class="ico" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3"><path d="M4 1.5h5l3 3v10H4z"/><path d="M9 1.5v3h3"/></svg>',
    "folder": '<svg class="ico" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3"><path d="M1.5 4V13h13V5.5H7.5L6 4z"/></svg>',
    "img": '<svg class="ico" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3"><rect x="2" y="3" width="12" height="10" rx="1"/><path d="M2 11l3.5-3.5 3 3 2-2L14 12"/></svg>',
    "book": '<svg class="ico" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3"><path d="M3 2.5h8.5v11H3z"/><path d="M5 2.5v11"/></svg>',
    "save": '<svg class="ico" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4"><path d="M8 2v8M4.5 6.5 8 10l3.5-3.5M3 13.5h10"/></svg>',
    "share": '<svg class="ico" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4"><circle cx="6" cy="6" r="2.5"/><path d="M1.8 13.5c.6-2.3 2.3-3.5 4.2-3.5s3.6 1.2 4.2 3.5M11.5 5v4M9.5 7h4"/></svg>',
    "pdf": '<svg class="ico" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4"><path d="M4 1.5h5l3 3v10H4z"/><path d="M6 10.5h4M8 6.5v4"/></svg>',
    "check": '<svg class="ico" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8"><path d="M3 8.5l3 3 7-7"/></svg>',
    "x": '<svg class="ico" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8"><path d="M4 4l8 8M12 4l-8 8"/></svg>',
    "warn": '<svg class="ico" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4"><path d="M8 2l6.5 11.5h-13z"/><path d="M8 6.5v3.5M8 11.8v.2"/></svg>',
    "err": '<svg class="ico" viewBox="0 0 16 16" fill="none" stroke="#a3271f" stroke-width="1.5"><circle cx="8" cy="8" r="6"/><path d="M5.8 5.8l4.4 4.4M10.2 5.8l-4.4 4.4"/></svg>',
    "wait": '<svg class="ico" viewBox="0 0 16 16" fill="none" stroke="#8a6000" stroke-width="1.5"><circle cx="8" cy="8" r="6"/><path d="M8 4.5V8l2.5 1.5"/></svg>',
    "chat": '<svg class="ico" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4"><path d="M2 3h12v8H7l-3 2.5V11H2z"/></svg>',
    "pen": '<svg class="ico" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4"><path d="M11 2.5l2.5 2.5L6 12.5H3.5V10z"/></svg>',
    "eye": '<svg class="ico" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4"><path d="M1.5 8S4 3.5 8 3.5 14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z"/><circle cx="8" cy="8" r="2"/></svg>',
    "sugg": '<svg class="ico" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4"><path d="M10.5 2.5l3 3L7 12H4V9z"/><path d="M2 14.5h12" stroke-dasharray="2 2"/></svg>',
    "clock": '<svg class="ico" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4"><circle cx="8" cy="8" r="6"/><path d="M8 4.5V8l2.5 1.5"/></svg>',
    "plus": '<svg class="ico" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6"><path d="M8 3v10M3 8h10"/></svg>',
    "open": '<svg class="ico" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4"><path d="M1.5 4V13h11l2-6H4.5l-2 6M1.5 4H6l1.5 1.5h5V7"/></svg>',
    "key": '<svg class="ico" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4"><circle cx="5" cy="8" r="3"/><path d="M8 8h6.5M12 8v2.5M14 8v2"/></svg>',
    "tag": '<svg class="ico" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4"><path d="M2 2.5h5.5l6.5 6.5-5 5L2.5 7.5z"/><circle cx="5.5" cy="5.5" r="1"/></svg>',
    "chev": '<svg class="ico" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M4.5 6.5 8 10l3.5-3.5"/></svg>',
    "commit": '<svg class="ico" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4"><circle cx="8" cy="8" r="2.5"/><path d="M1 8h4.5M10.5 8H15"/></svg>',
}


def page(name, title, body, plain=False):
    doc = f"""<!doctype html>
<html lang="pt-BR"><head><meta charset="utf-8"><title>{title}</title>
<link rel="stylesheet" href="app.css"></head>
<body><div class="window{' plain' if plain else ''}">
<div class="titlebar"><div class="lights"><i></i><i></i><i></i></div><div class="wtitle">{title}</div><div></div></div>
{body}
</div></body></html>"""
    with open(os.path.join(HERE, name), "w", encoding="utf-8") as f:
        f.write(doc)


def toolbar(mode="edit", status='<span class="status-chip"><i></i>Sincronizado</span>', export='<span class="btn pri">' + ICON["pdf"] + 'Exportar PDF</span>', note=""):
    seg = {
        "edit": ("on", "", ""),
        "sugg": ("", "on", ""),
        "read": ("", "", "on"),
    }[mode]
    return f"""<div class="toolbar">
<span class="seg"><span class="{seg[0]}">{ICON['pen']}Editando</span><span class="{seg[1]}">{ICON['sugg']}Sugerindo</span><span class="{seg[2]}">{ICON['eye']}Lendo</span></span>
<span class="mode-note">{note}</span><span class="sp"></span>
<span class="presence"><span class="av ana" title="Ana">A</span><span class="av bruno">B</span><span class="av me">V</span></span>
{status}
<span class="btn">{ICON['save']}Salvar <span class="kbd">⌘S</span></span>
<span class="btn">{ICON['share']}Compartilhar</span>
{export}
</div>"""


def side(cur="main.typ", counts=(2, 0, 1, 1, 0), pending=False):
    files = [("main.typ", "file"), ("refs.bib", "book"), ("figures/", "folder")]
    fl = "".join(
        f'<li class="{"cur" if f == cur else ""}">{ICON[i]}{f}</li>' for f, i in files
    )
    fl += f'<li class="ind">{ICON["img"]}fluxo.png{" <span class=n>aguardando</span>" if pending else ""}</li>'
    fl += f'<li class="ind">{ICON["img"]}mapa.png</li>'
    secs = ["Introdução", "Trabalhos relacionados", "Método", "Resultados", "Discussão"]
    ol = "".join(
        f'<li>{i + 1}. {s}{f"<span class=n>{c}</span>" if c else ""}</li>' for i, (s, c) in enumerate(zip(secs, counts))
    )
    return f'<aside class="side"><div><h4>Arquivos</h4><ul>{fl}</ul></div><div><h4>Estrutura</h4><ul>{ol}</ul></div></aside>'


def statusbar(extra="", state='<span class="ok">● Sincronizado</span>'):
    return f'<div class="statusbar">{state}<span>Compilador Typst embutido</span>{extra}<span class="sp"></span><span>Typst</span><span>UTF-8</span><span>Ln 17, Col 42</span></div>'


TITLE = "Prelo — artigo-exemplo"

# ---------------------------------------------------------------- 1 início
qr = "".join("<i></i>" if (x * 7 + y * 13 + x * y) % 3 == 0 or (x < 7 and y < 7 and (x in (0, 6) or y in (0, 6) or (1 < x < 5 and 1 < y < 5))) or (x > 13 and y < 7 and (x in (14, 20) or y in (0, 6) or (15 < x < 19 and 1 < y < 5))) or (x < 7 and y > 13 and (x in (0, 6) or y in (14, 20) or (1 < x < 5 and 15 < y < 19))) else "<b></b>" for y in range(21) for x in range(21))
page("tela-1-inicio.html", "Prelo", f"""
<div class="main" style="grid-template-columns: 380px 1fr; background:#fff">
  <aside class="side" style="padding:28px 0; background:#f7f6f3">
    <div style="padding:0 26px">
      <div style="font: 700 34px/1 Georgia, 'Iowan Old Style', serif; letter-spacing:.01em">Prelo<span style="color:var(--accent)">.</span></div>
      <p class="hint" style="margin:8px 0 0; font-size:12.5px">Artigos em Typst, a várias mãos.</p>
    </div>
    <div style="padding:6px 16px; display:flex; flex-direction:column; gap:6px">
      <div class="card" style="flex-direction:row; align-items:center; border-left-width:1px">{ICON['plus']}<div style="flex:1"><b>Novo projeto Typst</b><div class="hint">Cria uma pasta de projeto</div></div><span class="kbd">⌘N</span></div>
      <div class="card" style="flex-direction:row; align-items:center; border-left-width:1px">{ICON['open']}<div style="flex:1"><b>Abrir pasta ou repositório git</b><div class="hint">Projeto Typst existente</div></div><span class="kbd">⌘O</span></div>
      <div class="card focus" style="flex-direction:row; align-items:center; border-left-width:1px; border-color:var(--accent); background:var(--accent-soft)">{ICON['key']}<div style="flex:1"><b>Entrar com convite</b><div class="hint">Código recebido de um coautor</div></div><span class="kbd">⌘J</span></div>
    </div>
    <div style="margin-top:auto; padding:0 26px" class="hint">Sem conta e sem senha. Sua identidade é uma chave criada neste computador.</div>
  </aside>
  <div style="padding:34px 44px; display:grid; grid-template-columns: 1fr 330px; gap:34px; align-content:start">
    <div style="display:flex; flex-direction:column; gap:14px">
      <h2 style="margin:0; font-size:22px">Entrar com convite</h2>
      <p class="hint" style="font-size:13px; margin:0">Cole o código que o dono do projeto enviou, ou clique no link <code>prelo://join/…</code> e o app abre aqui com ele preenchido.</p>
      <div class="lbl">Código do convite</div>
      <div class="input v focus" style="font-family:var(--mono)">prelo://join/artigo-exemplo#K7QF-2MXD-9PLA-RW4C</div>
      <div class="lbl">Salvar em</div>
      <div class="input v" style="display:flex; justify-content:space-between">~/Documentos/artigo-exemplo <span class="hint">Alterar…</span></div>
      <div class="card" style="border-left-color:var(--accent)">
        <div class="hd">{ICON['check']} Convite válido</div>
        <p>Projeto <b>artigo-exemplo</b> · papel <b>revisora</b> · expira em 7 dias · uso único</p>
      </div>
      <div style="display:flex; gap:8px; justify-content:flex-end"><span class="btn">Cancelar</span><span class="btn pri">Entrar no projeto</span></div>
    </div>
    <div class="card" style="border-left-width:1px; gap:12px; padding:16px">
      <h3 style="margin:0; font-size:12px; text-transform:uppercase; letter-spacing:.06em; color:var(--faint)">Sua identidade</h3>
      <div style="display:flex; align-items:center; gap:12px"><span class="av l ana">AR</span><div><b style="font-size:14px">Ana Reis</b><div class="hint">aparece nos cursores, sugestões e comentários</div></div></div>
      <div class="lbl">Nome</div><div class="input v">Ana Reis</div>
      <div class="lbl">Cor</div>
      <div style="display:flex; gap:8px">{''.join(f'<span style="width:22px;height:22px;border-radius:50%;background:{c};{"box-shadow:0 0 0 2px #fff,0 0 0 4px #1d2124" if i == 0 else ""}"></span>' for i, c in enumerate(['#7b3fe4', '#c25a14', '#2266cc', '#0d6a73', '#b0306a', '#4d7a1f']))}</div>
      <div class="lbl">Código da sua chave</div>
      <div class="fp">5F2A · 9B07 · C91E</div>
      <p class="hint" style="margin:0">Quem aprovar sua entrada confere este código com você antes de aprovar.</p>
    </div>
  </div>
</div>""", plain=True)

# ---------------------------------------------------------------- 2 editor
over2 = {17: hl("Coautores revisam o mesmo texto em horários diferentes @silva2024. O orientador lê uma versão, o coautor edita outra e as mudanças se cruzam no e-mail."),
         24: 'Acompanhamos a escrita de artigos em grupos de pesquisa durante um semestre.<span class="caret bruno"><b>Bruno Lima</b></span> A taxa de aceite de uma seção <span class="t-m">$s$</span> é'}
page("tela-2-editor.html", TITLE, f"""
{toolbar("edit")}
<div class="main" style="grid-template-columns: 236px 1fr 560px">
  {side()}
  <div class="pane">
    <div class="tabs"><span class="tab on">{ICON['file']}main.typ <span class="x">✕</span></span><span class="tab">{ICON['book']}refs.bib</span><span class="right">Editando · alterações entram direto</span></div>
    {code(1, 40, over2, {17: "hl"})}
  </div>
  <div class="pane">
    <div class="tabs"><span class="tab on">{ICON['eye']}Prévia</span><span class="right"><span class="pill">Com marcações {ICON['chev']}</span><span>100%</span><span>Página 1 de 1</span></span></div>
    <div class="preview rel"><img src="prev-clean-1.png" alt=""><span class="pv-hl" style="left:58px; top:316px; width:226px; height:52px"></span></div>
  </div>
</div>
{statusbar('<span>Prévia atualizada</span>')}
""")

# ---------------------------------------------------------------- 3 comentários
over3 = {17: 'Coautores revisam o mesmo texto em horários diferentes <span class="t-ref">@silva2024</span>. <span class="c-anchor on">O orientador lê uma versão, o coautor edita outra</span> e as mudanças se cruzam no e-mail.',
         29: '  image(<span class="c-anchor">"figures/fluxo.png"</span>, width: 100%),'}
page("tela-3-comentarios.html", TITLE, f"""
{toolbar("read", note="Lendo: só leitura e comentários")}
<div class="main" style="grid-template-columns: 214px 1fr 400px 380px">
  {side(counts=(2, 0, 1, 1, 0))}
  <div class="pane">
    <div class="tabs"><span class="tab on">{ICON['file']}main.typ</span><span class="right">Selecione um trecho para comentar</span></div>
    {code(14, 46, over3, {17: "hl"})}
  </div>
  <div class="pane">
    <div class="tabs"><span class="tab on">{ICON['eye']}Prévia</span></div>
    <div class="preview" style="padding:12px"><img src="prev-comments-1.png" alt=""></div>
  </div>
  <div class="panel">
    <div class="ptabs"><span class="on">Abertos<span class="cnt">3</span></span><span>Resolvidos<span class="cnt">5</span></span><span>Sugestões<span class="cnt">4</span></span></div>
    <div class="pbody">
      <div class="card bruno focus">
        <div class="hd"><span class="av s bruno">B</span>Bruno Lima <small>revisor · há 2 h</small><span class="more">⋯</span></div>
        <div class="q">“O orientador lê uma versão, o coautor edita outra”</div>
        <p>Dá para trazer um exemplo concreto aqui? Algo do estudo, com quantas versões circularam.</p>
        <div class="reply">
          <div class="hd"><span class="av s ana">A</span>Ana Reis <small>coautora · há 1 h</small></div>
          <p><span class="mention">@Bruno</span> boa. Uso o caso do grupo 2, que está na seção 4.</p>
        </div>
        <div class="reply">
          <div class="hd"><span class="av s me">V</span>Você <small>dono · agora</small></div>
          <p>Combinado. <span class="mention">@Carla</span> você tem os números desse grupo?</p>
        </div>
        <div class="input">Responder… use @ para mencionar</div>
        <div class="acts"><span class="btn sm">Responder</span><span class="btn sm ok">{ICON['check']}Resolver</span></div>
      </div>
      <div class="card ana">
        <div class="hd"><span class="av s ana">A</span>Ana Reis <small>coautora · ontem</small><span class="more">⋯</span></div>
        <div class="q">“figures/fluxo.png”</div>
        <p>A figura precisa mostrar o caminho de rejeição com mais destaque.</p>
        <div class="hint">2 respostas</div>
      </div>
      <div class="card carla">
        <div class="hd"><span class="av s carla">C</span>Carla Dias <small>coautora · segunda</small></div>
        <p>Revisar a tabela 1 antes de submeter.</p>
      </div>
    </div>
  </div>
</div>
{statusbar('<span>3 comentários abertos</span>')}
""")

# ---------------------------------------------------------------- 4 sugestão
over4 = {17: 'Coautores revisam o mesmo texto em <span class="s-del">horários</span><span class="s-ins">momentos</span> diferentes <span class="t-ref">@silva2024</span>. O orientador lê uma versão, o coautor edita outra e as mudanças se cruzam no e-mail.',
         19: 'Ferramentas de controle de alterações resolvem parte do problema, mas costumam exigir um servidor central e uma conexão permanente<span class="s-ins b">, o que não serve a quem trabalha offline</span> <span class="t-ref">@moura2023</span>.',
         30: '  caption: [Fluxo de uma sugestão até o texto aceito.<span class="s-del">]</span>,'}
page("tela-4-sugestao.html", TITLE, f"""
{toolbar("sugg", note="Suas alterações viram sugestões para os coautores")}
<div class="main rel" style="grid-template-columns: 214px 1fr 400px 390px">
  {side(counts=(2, 0, 1, 1, 0))}
  <div class="pane">
    <div class="tabs"><span class="tab on">{ICON['file']}main.typ</span><span class="right"><span class="av s ana">A</span><span class="av s bruno">B</span> 4 sugestões</span></div>
    {code(14, 46, over4, {17: "hl"})}
  </div>
  <div class="pane">
    <div class="tabs"><span class="tab on">{ICON['eye']}Prévia</span><span class="right"><span class="pill">Com marcações {ICON['chev']}</span></span></div>
    <div class="preview" style="padding:12px"><img src="prev-marked-1.png" alt=""></div>
  </div>
  <div class="panel">
    <div class="ptabs"><span>Abertos<span class="cnt">3</span></span><span>Resolvidos<span class="cnt">5</span></span><span class="on">Sugestões<span class="cnt">4</span></span></div>
    <div class="pbody">
      <div class="filter">Filtrar: <span class="pill"><span class="av s ana" style="width:14px;height:14px;font-size:8px">A</span>Ana {ICON['chev']}</span><span class="pill">Todas as seções {ICON['chev']}</span><span class="sp"></span><span class="btn sm ok">Aceitar 3 de Ana</span></div>
      <div class="card ana focus">
        <div class="hd"><span class="av s ana">A</span>Ana Reis <small>substituição · Introdução · 14:02</small></div>
        <p><span class="s-del">horários</span> → <span class="s-ins">momentos</span></p>
        <div class="acts"><span class="btn sm no">{ICON['x']}Rejeitar <span class="kbd">⌘⌫</span></span><span class="btn sm ok">{ICON['check']}Aceitar <span class="kbd">⌘↵</span></span></div>
      </div>
      <div class="card bruno">
        <div class="hd"><span class="av s bruno">B</span>Bruno Lima <small>inserção · Introdução · 13:40</small></div>
        <p><span class="s-ins b">, o que não serve a quem trabalha offline</span></p>
        <div class="acts"><span class="btn sm no">{ICON['x']}Rejeitar</span><span class="btn sm ok">{ICON['check']}Aceitar</span></div>
      </div>
      <div class="card bruno bad">
        <div class="hd"><span class="av s bruno">B</span>Bruno Lima <small>exclusão · Método · 13:31</small></div>
        <p>Apaga <code style="font-family:var(--mono);background:var(--del-bg);color:var(--del);padding:0 3px">]</code> na legenda da figura 1.</p>
        <div class="warnline">{ICON['warn']}<span>Se aceita, o artigo deixa de compilar: <i>unclosed delimiter</i> na linha 30.</span></div>
        <div class="acts"><span class="btn sm no">{ICON['x']}Rejeitar</span><span class="btn sm dis">Aceitar</span></div>
      </div>
      <div class="hint">⌥↓ vai para a próxima sugestão</div>
    </div>
  </div>
  <div class="toast"><span>{ICON['check']}</span><span>2 sugestões de Carla aceitas em Resultados</span><span class="k">Commit proposto: “Accept 2 suggestions from Carla in Resultados”</span><span class="u">Desfazer ⌘Z</span></div>
</div>
{statusbar('<span>4 sugestões pendentes · todas testadas</span>')}
""")

# ---------------------------------------------------------------- 5 colaboradores
page("tela-5-colaboradores.html", TITLE, f"""
{toolbar("edit")}
<div class="main rel" style="grid-template-columns: 236px 1fr 560px">
  {side()}
  <div class="pane"><div class="tabs"><span class="tab on">{ICON['file']}main.typ</span></div>{code(1, 40)}</div>
  <div class="pane"><div class="tabs"><span class="tab on">{ICON['eye']}Prévia</span></div><div class="preview"><img src="prev-clean-1.png" alt=""></div></div>
  <div class="overlay">
    <div class="sheet">
      <div class="sheet-h">{ICON['share']}<h2>Compartilhar “artigo-exemplo”</h2><span class="sp"></span><span class="btn sm">Fechar</span></div>
      <div class="sheet-b">
        <div>
          <h3>Participantes · 4</h3>
          <div class="person"><span class="av l me">V</span><div class="nm">Você<small><span class="dot on"></span>dono · este computador</small></div><span class="role">Dono</span></div>
          <div class="person"><span class="av l ana">AR</span><div class="nm">Ana Reis<small><span class="dot on"></span>online · rede local</small></div><span class="role">Coautora {ICON['chev']}</span><span class="btn sm no">Remover</span></div>
          <div class="person"><span class="av l bruno">BL</span><div class="nm">Bruno Lima<small><span class="dot on"></span>online · pela internet, direto</small></div><span class="role">Revisor {ICON['chev']}</span><span class="btn sm no">Remover</span></div>
          <div class="person"><span class="av l carla">CD</span><div class="nm">Carla Dias<small><span class="dot off"></span>offline · visto ontem</small></div><span class="role">Leitora {ICON['chev']}</span><span class="btn sm no">Remover</span></div>
          <div class="req">
            <div class="hd" style="display:flex;gap:8px;align-items:center;font-weight:600"><span class="av s dan">DS</span>Daniel Souza pede para entrar como revisor</div>
            <p style="margin:0">Confira com ele o código da chave antes de aprovar:</p>
            <div class="fp">7C1D · 44E0 · A9F3</div>
            <div class="card acts" style="border:0;padding:0;box-shadow:none;background:none;flex-direction:row"><span class="btn sm no">Recusar</span><span class="btn sm pri">Aprovar entrada</span></div>
          </div>
        </div>
        <div>
          <h3>Novo convite</h3>
          <div class="lbl">Papel</div>
          <span class="seg" style="align-self:flex-start"><span>Coautor</span><span class="on">Revisor</span><span>Leitor</span></span>
          <div class="hint">Revisor comenta e sugere; toda edição dele vira sugestão.</div>
          <div class="lbl">Validade</div>
          <div class="input v" style="display:flex;justify-content:space-between">7 dias · uso único {ICON['chev']}</div>
          <div class="toggle"><span>Exigir minha aprovação para entrar</span><span class="sw"></span></div>
          <div style="display:flex; gap:12px; align-items:center">
            <div class="qr">{qr}</div>
            <div style="flex:1; display:flex; flex-direction:column; gap:8px">
              <div class="codebox">prelo://join/artigo-exemplo#K7QF-2MXD-9PLA-RW4C</div>
              <div style="display:flex; gap:6px"><span class="btn sm pri">Copiar código</span><span class="btn sm">Gerar outro</span></div>
            </div>
          </div>
          <div class="hint">Envie por um canal privado. O código contém a chave do documento.</div>
        </div>
      </div>
    </div>
  </div>
</div>
{statusbar()}
""")

# ---------------------------------------------------------------- 6 histórico
diff_html = f"""<div class="code" style="font-size:12.5px">
<div class="ln"><span class="g">15</span><span class="c"> </span></div>
<div class="ln"><span class="g">16</span><span class="c">{hl(SRC[15])}</span></div>
<div class="ln" style="background:#fbe9e7"><span class="g" style="color:#a3271f">−</span><span class="c" style="color:#7d1d17">Coautores revisam o mesmo texto em <b style="background:#f1b9b2;font-weight:400">horários</b> diferentes @silva2024. O orientador lê uma versão, o coautor edita outra e as mudanças se cruzam no e-mail.</span></div>
<div class="ln" style="background:#e6f4ea"><span class="g" style="color:#1b6534">+</span><span class="c" style="color:#14522a">Coautores revisam o mesmo texto em <b style="background:#b8e2c4;font-weight:400">momentos</b> diferentes @silva2024. O orientador lê uma versão, o coautor edita outra e as mudanças se cruzam no e-mail.</span></div>
<div class="ln"><span class="g">18</span><span class="c"> </span></div>
<div class="ln"><span class="g">19</span><span class="c">{hl(SRC[18])}</span></div>
</div>"""
hist = [
    ("named", "Revisão R1", "versão nomeada · Você · hoje 15:10", "tag"),
    ("", "Accept 1 suggestion from Ana in Introdução", "Você · hoje 14:58", "commit"),
    ("", "Prelo sync · Bruno · 2 sugestões", "Bruno Lima · hoje 13:42", "commit"),
    ("", "Prelo sync · Ana · 1 comentário", "Ana Reis · hoje 11:05", "commit"),
    ("", "Accept 3 suggestions from Ana in Método", "Você · ontem 18:20", "commit"),
    ("", "Prelo sync · Carla · 1 comentário", "Carla Dias · ontem 09:12", "commit"),
    ("named", "Submissão 1", "versão nomeada · Você · 03/11", "tag"),
    ("", "Prelo sync · Você · primeira versão completa", "Você · 03/11", "commit"),
]
hrows = "".join(
    f'<li class="{"cur" if i == 1 else ""}" style="padding:7px 14px;align-items:flex-start;{"background:var(--accent-soft)" if i == 1 else ""}">'
    f'<span style="color:{"var(--accent)" if k == "named" else "var(--faint)"}">{ICON[ic]}</span>'
    f'<span><b style="{"color:var(--accent)" if k == "named" else "font-weight:500"}">{t}</b><br><span class="hint">{s}</span></span></li>'
    for i, (k, t, s, ic) in enumerate(hist)
)
page("tela-6-historico.html", TITLE, f"""
{toolbar("edit")}
<div class="banner">{ICON['check']}<span><b>Sincronizado.</b> Bruno fez 2 commits, Ana deixou 1 comentário.</span><span class="sp"></span><span class="btn sm">Ver mudanças</span></div>
<div class="main" style="grid-template-columns: 380px 1fr 360px">
  <aside class="side" style="gap:6px"><h4 style="margin-top:4px">{'Histórico'}</h4><ul style="display:flex;flex-direction:column">{hrows}</ul></aside>
  <div class="pane">
    <div class="tabs"><span class="tab on">{ICON['commit']}Accept 1 suggestion from Ana in Introdução</span><span class="right">main.typ · 1 linha alterada</span></div>
    <div style="padding:12px 16px; border-bottom:1px solid var(--line); display:flex; gap:10px; align-items:center; font-size:12.5px"><span class="av s me">V</span><span><b>Você</b> aceitou a sugestão de <b style="color:var(--u-ana)">Ana Reis</b> · hoje 14:58</span><span class="sp"></span><span class="hint" style="font-family:var(--mono)">a41c9e2</span></div>
    {diff_html}
  </div>
  <div class="panel">
    <div class="ptabs"><span class="on">Versões nomeadas</span></div>
    <div class="pbody" style="gap:12px">
      <div class="card" style="border-left-color:var(--accent)"><div class="hd">{ICON['tag']}Revisão R1<small>hoje</small></div><p class="hint">Enviada aos revisores da revista</p></div>
      <div class="card" style="border-left-color:var(--accent)"><div class="hd">{ICON['tag']}Submissão 1<small>03/11</small></div><p class="hint">Primeira submissão</p></div>
      <span class="btn" style="align-self:flex-start">{ICON['plus']}Nomear versão atual</span>
      <div class="card" style="border-left-width:1px; gap:10px">
        <div class="hd">PDF de diferenças</div>
        <div class="lbl">De</div><div class="input v" style="display:flex;justify-content:space-between">Submissão 1 {ICON['chev']}</div>
        <div class="lbl">Para</div><div class="input v" style="display:flex;justify-content:space-between">Revisão R1 {ICON['chev']}</div>
        <p class="hint">As mudanças entre as duas versões saem marcadas, com número de linha, para a resposta aos revisores.</p>
        <span class="btn pri" style="align-self:flex-end">{ICON['pdf']}Gerar PDF de diferenças</span>
      </div>
    </div>
  </div>
</div>
{statusbar()}
""")

# ---------------------------------------------------------------- 7 erros
src7 = SRC[:]
src7[32:34] = [
    "A @fig-fluxo resume o ciclo observado. A taxa média de aceite, calculada pela @eq-aceite, variou entre as seções (@tab-aceite).",
    "#image(\"../../fotos/campo.png\", width: 60%)",
]
over7 = {
    34: '<span class="t-fn">#image</span>(<span class="squig"><span class="t-s">"../../fotos/campo.png"</span></span>, width: 60%)',
    29: '  image(<span class="squig w"><span class="t-s">"figures/fluxo.png"</span></span>, width: 100%),',
}
page("tela-7-erros.html", TITLE, f"""
{toolbar("edit", status='<span class="status-chip"><i></i>Sincronizado</span>', export='<span class="btn dis">' + ICON['pdf'] + 'Exportar PDF final ' + ICON['chev'] + '</span>')}
<div class="main rel" style="grid-template-columns: 236px 1fr 560px">
  {side(pending=True)}
  <div class="pane">
    <div class="tabs"><span class="tab on">{ICON['file']}main.typ <span class="x" style="color:var(--del)">●</span></span><span class="tab">{ICON['book']}refs.bib <span class="x" style="color:var(--del)">●</span></span></div>
    <div style="flex:1; display:flex; flex-direction:column; min-height:0">
      {code(26, 44, over7, {34: "err", 29: "warn"}, lines=src7)}
      <div class="diag">
        <div class="dh"><span class="on">Problemas</span><span>2 erros · 1 aviso</span></div>
        <ul>
          <li class="sel">{ICON['err']}<span class="loc">main.typ:34:8</span><span><b>Acesso recusado:</b> <code>../../fotos/campo.png</code> fica fora da pasta do projeto.<small>O Prelo só lê arquivos dentro de artigo-exemplo/. Copie a imagem para o projeto.</small></span></li>
          <li>{ICON['err']}<span class="loc">refs.bib:14:1</span><span><b>Bibliografia malformada:</b> chave não fechada na entrada <code>teixeira2022</code>.</span></li>
          <li>{ICON['wait']}<span class="loc">main.typ:29:9</span><span><b>Aguardando</b> <code>figures/fluxo.png</code>: ainda chegando do computador de Ana.<div class="bar" style="margin-top:5px;width:220px"><i style="width:62%"></i></div></span></li>
        </ul>
      </div>
    </div>
  </div>
  <div class="pane">
    <div class="tabs"><span class="tab on">{ICON['eye']}Prévia</span><span class="right" style="color:var(--cmt)">{ICON['wait']} Mostrando a última compilação bem-sucedida</span></div>
    <div class="preview"><img src="prev-pending-1.png" alt=""></div>
  </div>
  <div class="popover">
    <h3>Exportar PDF final está bloqueado</h3>
    <p style="margin:0">O artigo referencia um arquivo que ainda não chegou a este computador:</p>
    <ul><li><code>figures/fluxo.png</code> · 62%</li></ul>
    <p class="hint" style="margin:0">A exportação é liberada quando o arquivo chegar. Corrija também os 2 erros.</p>
  </div>
</div>
{statusbar('<span class="er">2 erros</span><span class="wa">1 aguardando</span>')}
""")
print("ok")
