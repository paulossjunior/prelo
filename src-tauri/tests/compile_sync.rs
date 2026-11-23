//! Sincronização código ↔ prévia e estrutura de seções (T061; FR-005, FR-020, SC-007;
//! research R5; contracts/ipc.md `jump_from_click`, `jump_from_cursor`, `outline`;
//! data-model.md `OutlineItem`).
//!
//! Pelo estado do núcleo (`AppState`), como os comandos IPC o usam. Os pontos clicados saem de
//! `jump_from_cursor` (o ponto de um trecho conhecido na prévia), então o teste confere a ida e
//! a volta sem depender de coordenadas fixas da diagramação. `page` é o índice da página a
//! partir de 0, o mesmo de `PreviewUpdate.pages[].index` e de `page_svg`; `line` e `column`
//! começam em 1 (`column` em unidades UTF-16, como o editor).

mod support;

use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use prelo_lib::commands::{AppState, Change, JumpTarget};
use prelo_lib::compile::preview::PreviewUpdate;
use prelo_lib::compile::sync::{OutlineItem, SourcePosition};
use support::TempProject;

/// Folga para uma compilação em build de depuração (a primeira carrega as fontes).
const WAIT: Duration = Duration::from_secs(60);

struct Session {
    state: AppState,
    updates: Receiver<PreviewUpdate>,
    _project: TempProject,
}

impl Session {
    fn open(project: TempProject) -> Self {
        let state = AppState::default();
        let (tx, updates) = mpsc::channel();
        state.subscribe_preview(move |update: &PreviewUpdate| {
            let _ = tx.send(update.clone());
        });
        state.open_project(&project.root()).expect("abrir projeto");
        Self {
            state,
            updates,
            _project: project,
        }
    }

    fn fixture(name: &str) -> Self {
        Self::open(TempProject::from_fixture(name))
    }

    fn wait_for(&self, what: &str, pred: impl Fn(&PreviewUpdate) -> bool) -> PreviewUpdate {
        let deadline = Instant::now() + WAIT;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let update = self
                .updates
                .recv_timeout(left)
                .unwrap_or_else(|_| panic!("nenhum PreviewUpdate {what} no prazo"));
            if pred(&update) {
                return update;
            }
        }
    }

    fn next_ok(&self) -> PreviewUpdate {
        self.wait_for("ok", |u| u.ok)
    }

    /// Texto atual de `path` (abre no núcleo, como o editor).
    fn text(&self, path: &str) -> String {
        self.state.open_file(path).expect("open_file").text
    }

    /// Deslocamento UTF-16 de `needle` em `path`, mais `skip` unidades.
    fn offset_of(&self, path: &str, needle: &str, skip: usize) -> usize {
        let text = self.text(path);
        let byte = text
            .find(needle)
            .unwrap_or_else(|| panic!("{needle:?} não está em {path}"));
        text[..byte].encode_utf16().count() + skip
    }

    /// Linha (a partir de 1) de `needle` em `path`.
    fn line_of(&self, path: &str, needle: &str) -> u32 {
        let text = self.text(path);
        let byte = text.find(needle).expect("trecho no arquivo");
        text[..byte].matches('\n').count() as u32 + 1
    }

    /// Ponto da prévia do trecho em `offset` de `path`: o primeiro que `jump_from_cursor` dá.
    fn point_of(&self, path: &str, offset: usize) -> JumpTarget {
        let targets = self
            .state
            .jump_from_cursor(path, offset)
            .expect("jump_from_cursor");
        targets
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("nenhum ponto na prévia para {path}@{offset}"))
    }

    /// Clica um pouco à direita e acima da linha de base do ponto (dentro do glifo).
    fn click_at(&self, target: &JumpTarget) -> Option<SourcePosition> {
        self.state
            .jump_from_click(target.page, target.x_pt + 1.0, target.y_pt - 3.0)
            .expect("jump_from_click")
    }
}

#[test]
fn clique_em_paragrafo_do_principal_abre_o_principal_na_linha() {
    let s = Session::fixture("artigo-basico");
    let update = s.next_ok();
    let needle = "Este parágrafo de abertura";
    let offset = s.offset_of("main.typ", needle, 5);
    let point = s.point_of("main.typ", offset);
    assert_eq!(
        point.page, 0,
        "o parágrafo de abertura fica na primeira página"
    );
    assert!(
        point.x_pt > 0.0 && point.x_pt < update.pages[0].width_pt,
        "{point:?}"
    );
    assert!(
        point.y_pt > 0.0 && point.y_pt < update.pages[0].height_pt,
        "{point:?}"
    );

    let pos = s
        .click_at(&point)
        .expect("o parágrafo tem origem no projeto");
    assert_eq!(pos.path, "main.typ");
    assert_eq!(pos.line, s.line_of("main.typ", needle));
    assert!(pos.column >= 1, "{pos:?}");
}

#[test]
fn clique_em_trecho_de_typ_incluido_abre_o_arquivo_incluido() {
    let s = Session::fixture("artigo-basico");
    s.next_ok();
    let path = "capitulos/metodo.typ";
    let needle = "Parágrafo do capítulo incluído";
    let offset = s.offset_of(path, needle, 3);
    let point = s.point_of(path, offset);

    let pos = s.click_at(&point).expect("o trecho incluído tem origem");
    assert_eq!(pos.path, path, "abre o incluído, não o principal");
    assert_eq!(pos.line, s.line_of(path, needle));
    // A coluna aponta para dentro do parágrafo clicado.
    let line_text = s
        .text(path)
        .lines()
        .nth(pos.line as usize - 1)
        .unwrap()
        .to_owned();
    assert!(
        (pos.column as usize) <= line_text.encode_utf16().count() + 1,
        "{pos:?} em {line_text:?}"
    );
}

#[test]
fn clique_sem_origem_no_projeto_devolve_null() {
    let project = TempProject::empty();
    project.write(
        "main.typ",
        "#set page(width: 200pt, height: 200pt, margin: 40pt, numbering: \"1\")\n\
         Texto do corpo.\n",
    );
    let s = Session::open(project);
    s.next_ok();
    // Margem vazia.
    assert_eq!(s.state.jump_from_click(0, 3.0, 3.0), Ok(None));
    // Fora da página e página inexistente.
    assert_eq!(s.state.jump_from_click(0, 900.0, 900.0), Ok(None));
    assert_eq!(s.state.jump_from_click(7, 10.0, 10.0), Ok(None));
    // Número de página gerado pela numeração automática, no rodapé (centro da margem de
    // baixo): nenhum ponto da região tem origem no projeto.
    let mut y = 160.0;
    while y <= 200.0 {
        let mut x = 90.0;
        while x <= 110.0 {
            assert_eq!(
                s.state.jump_from_click(0, x, y),
                Ok(None),
                "numeração em ({x}, {y})"
            );
            x += 1.0;
        }
        y += 1.0;
    }
    // O corpo, por contraste, tem origem.
    let offset = s.offset_of("main.typ", "Texto do corpo", 2);
    let pos = s.click_at(&s.point_of("main.typ", offset)).expect("corpo");
    assert_eq!((pos.path.as_str(), pos.line), ("main.typ", 2));
}

#[test]
fn jump_from_cursor_devolve_pagina_e_posicao() {
    let s = Session::fixture("artigo-basico");
    let update = s.next_ok();
    let first = s.point_of("main.typ", s.offset_of("main.typ", "Este parágrafo", 2));
    let last = s.point_of(
        "main.typ",
        s.offset_of("main.typ", "Parágrafo final do arquivo principal", 2),
    );
    assert_eq!(first.page, 0);
    assert!(
        last.page > first.page,
        "a conclusão vem depois do capítulo incluído: {first:?} {last:?}"
    );
    assert!(last.page < update.pages.len());

    // Cursor em código sem texto na prévia (regra `set`): nenhum ponto.
    let none = s
        .state
        .jump_from_cursor("main.typ", s.offset_of("main.typ", "#set page", 3))
        .expect("jump_from_cursor");
    assert!(none.is_empty(), "{none:?}");

    // Arquivo só com definições (nada dele aparece como texto): nenhum ponto, sem erro.
    let defs = s.state.jump_from_cursor("utils.typ", 0).expect("utils");
    assert!(defs.is_empty(), "{defs:?}");
}

#[test]
fn outline_em_ordem_com_nivel_e_titulos_de_incluido() {
    let s = Session::fixture("artigo-basico");
    let update = s.next_ok();
    let items: Vec<OutlineItem> = s.state.outline().expect("outline");
    let summary: Vec<(u32, &str, &str)> = items
        .iter()
        .map(|i| (i.level, i.title.as_str(), i.path.as_str()))
        .collect();
    assert_eq!(
        summary,
        vec![
            (1, "1 Introdução", "main.typ"),
            (1, "2 Método", "capitulos/metodo.typ"),
            (2, "2.1 Coleta de dados", "capitulos/metodo.typ"),
            (2, "2.2 Análise", "capitulos/metodo.typ"),
            (1, "3 Conclusão", "main.typ"),
        ]
    );
    assert_eq!(items[0].line, s.line_of("main.typ", "= Introdução"));
    assert_eq!(items[1].line, s.line_of("capitulos/metodo.typ", "= Método"));
    assert_eq!(
        items[2].line,
        s.line_of("capitulos/metodo.typ", "== Coleta de dados")
    );
    assert_eq!(items[4].line, s.line_of("main.typ", "= Conclusão"));
    // Em ordem de documento: página e altura não voltam para trás.
    for pair in items.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        assert!(
            (a.page, a.y_pt) < (b.page, b.y_pt),
            "fora de ordem: {a:?} {b:?}"
        );
    }
    for item in &items {
        assert!(item.page < update.pages.len(), "{item:?}");
        assert!(item.y_pt >= 0.0 && item.y_pt <= update.pages[item.page].height_pt);
    }
    // Serializa no formato do contrato.
    let json = serde_json::to_value(&items[0]).unwrap();
    let mut keys: Vec<_> = json.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, ["level", "line", "page", "path", "title", "yPt"]);
}

#[test]
fn sync_usa_o_documento_da_previa_mesmo_com_erro_depois() {
    let s = Session::fixture("artigo-basico");
    s.next_ok();
    let needle = "Este parágrafo de abertura";
    let line = s.line_of("main.typ", needle);
    let point = s.point_of("main.typ", s.offset_of("main.typ", needle, 5));

    // Edição que quebra a compilação e empurra o texto para baixo: a prévia continua a da
    // última compilação boa, e o clique fala das linhas dela (o que a pessoa vê).
    let opened = s.state.open_file("main.typ").unwrap();
    s.state
        .edit_file(
            "main.typ",
            opened.version,
            &[Change {
                from: 0,
                to: 0,
                insert: "#erro_proposital(\n\n\n".into(),
            }],
        )
        .unwrap();
    s.wait_for("com erro", |u| !u.ok);
    let pos = s.click_at(&point).expect("origem pela última prévia boa");
    assert_eq!((pos.path.as_str(), pos.line), ("main.typ", line));
    assert_eq!(s.state.outline().expect("outline").len(), 5);
}

#[test]
fn sem_projeto_aberto_responde_no_project() {
    let state = AppState::default();
    assert_eq!(
        state.jump_from_click(0, 1.0, 1.0),
        Err("no_project".to_string())
    );
    assert_eq!(state.outline(), Err("no_project".to_string()));
}
