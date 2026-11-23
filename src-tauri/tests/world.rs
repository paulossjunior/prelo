//! Testes do World do Prelo (T020; research R2, R6; D-15, D-17; data-model.md "Diagnostic").
//!
//! World do `tinymist-world` montado com o `PathAccessModel` do Prelo, só com as fontes
//! embutidas de `src-tauri/fonts/` (nenhuma fonte do sistema) e com os padrões de fonte de
//! matemática e de `raw` definidos no World, sem escrever nada no `.typ` (Princípio I).

mod support;

use prelo_lib::diagnostic::{Kind, Severity};
use prelo_lib::world::{fonts, ProjectWorld};
use support::*;
use typst::layout::{Frame, FrameItem};
use typst::World;

/// Famílias de fonte usadas no documento, na ordem em que aparecem, sem repetição.
fn families_in(frame: &Frame, out: &mut Vec<String>) {
    for (_, item) in frame.items() {
        match item {
            FrameItem::Group(group) => families_in(&group.frame, out),
            FrameItem::Text(text) => {
                let family = text.font.font().info().family.clone();
                if !out.contains(&family) {
                    out.push(family);
                }
            }
            _ => {}
        }
    }
}

fn families(world: &mut ProjectWorld) -> Vec<String> {
    let outcome = world.compile();
    assert!(
        outcome.diagnostics.is_empty(),
        "diagnósticos: {:?}",
        outcome.diagnostics
    );
    let doc = outcome.output.expect("compilou");
    let mut out = Vec::new();
    for page in doc.pages() {
        families_in(&page.frame, &mut out);
    }
    out
}

fn single_file(text: &str) -> (TempProject, ProjectWorld) {
    let project = TempProject::empty();
    project.write("main.typ", text);
    let world = ProjectWorld::open(project.root(), "main.typ").expect("World");
    (project, world)
}

#[test]
fn compila_projeto_com_varios_arquivos() {
    let project = TempProject::from_fixture("artigo-basico");
    let mut world = ProjectWorld::open(project.root(), "main.typ").expect("World");
    let outcome = world.compile();
    assert_eq!(outcome.diagnostics, vec![]);
    assert!(outcome.output.expect("compilou").pages().len() >= 2);
}

#[test]
fn so_fontes_embutidas_e_nenhuma_do_sistema() {
    let (_project, world) = single_file("= Oi");
    let snapshot = world.snapshot();
    let book = snapshot.book();
    let mut count = 0;
    while let Some(info) = book.info(count) {
        assert!(
            info.family.starts_with("Libertinus"),
            "fonte fora das embutidas: {}",
            info.family
        );
        count += 1;
    }
    assert_eq!(count, fonts::EMBEDDED.len());

    // Os arquivos embutidos são exatamente os de `src-tauri/fonts/`.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");
    let mut on_disk: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.ends_with(".otf") || n.ends_with(".ttf"))
        .collect();
    on_disk.sort();
    let mut embedded: Vec<String> = fonts::EMBEDDED.iter().map(|(n, _)| n.to_string()).collect();
    embedded.sort();
    assert_eq!(embedded, on_disk);
}

#[test]
fn fonte_do_sistema_pedida_no_typ_nao_e_usada() {
    let (_project, mut world) = single_file("#set text(font: \"Arial\")\nTexto.");
    let outcome = world.compile();
    let warning = outcome
        .diagnostics
        .iter()
        .find(|d| d.severity == Severity::Warning)
        .expect("aviso de fonte desconhecida");
    assert!(
        warning.message.contains("unknown font family"),
        "{warning:?}"
    );
    assert_eq!(warning.kind, Kind::Typst);
    let doc = outcome.output.expect("compilou com fonte de reserva");
    let mut out = Vec::new();
    families_in(&doc.pages()[0].frame, &mut out);
    assert_eq!(out, vec!["Libertinus Serif".to_string()]);
}

#[test]
fn matematica_e_raw_usam_os_padroes_do_world() {
    let (_project, mut world) = single_file("Texto $x^2 + y$ e `codigo`.");
    let used = families(&mut world);
    assert!(used.contains(&"Libertinus Serif".to_string()), "{used:?}");
    assert!(used.contains(&fonts::MATH_FAMILY.to_string()), "{used:?}");
    assert!(used.contains(&fonts::RAW_FAMILY.to_string()), "{used:?}");
}

#[test]
fn escolha_de_fonte_no_typ_vence_o_padrao_do_world() {
    let (_project, mut world) =
        single_file("#show raw: set text(font: \"Libertinus Sans\")\n`codigo` $x$");
    let used = families(&mut world);
    assert!(!used.contains(&fonts::RAW_FAMILY.to_string()), "{used:?}");
    assert!(used.contains(&"Libertinus Sans".to_string()), "{used:?}");

    // Matemática numa fonte sem tabela MATH: vale a escolha do documento, e o Typst avisa.
    let (_project, mut world) =
        single_file("#show math.equation: set text(font: \"Libertinus Serif\")\n$x$");
    let outcome = world.compile();
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|d| d.message == "current font is not designed for math"),
        "{:?}",
        outcome.diagnostics
    );
}

#[test]
fn texto_aberto_no_editor_compila_sem_salvar() {
    let (project, mut world) = single_file("Uma página.");
    assert_eq!(world.compile().output.unwrap().pages().len(), 1);

    world
        .set_open_file("main.typ", "Um.\n#pagebreak()\nDois.")
        .unwrap();
    assert_eq!(world.compile().output.unwrap().pages().len(), 2);
    // Nada foi gravado.
    assert_eq!(
        std::fs::read_to_string(project.path("main.typ")).unwrap(),
        "Uma página."
    );

    // Nova edição invalida a anterior.
    world
        .set_open_file("main.typ", "Um.\n#pagebreak()\nDois.\n#pagebreak()\nTrês.")
        .unwrap();
    assert_eq!(world.compile().output.unwrap().pages().len(), 3);

    // Fechar sem salvar volta ao disco.
    world.close_file("main.typ");
    assert_eq!(world.compile().output.unwrap().pages().len(), 1);
}

#[test]
fn arquivo_incluido_aberto_no_editor() {
    let project = TempProject::from_fixture("artigo-basico");
    let mut world = ProjectWorld::open(project.root(), "main.typ").expect("World");
    let before = world.compile().output.unwrap().pages().len();
    world
        .set_open_file(
            "capitulos/metodo.typ",
            "= Método\n#pagebreak()\n#pagebreak()\n#pagebreak()",
        )
        .unwrap();
    let after = world.compile().output.unwrap().pages().len();
    assert!(after > before, "{before} → {after}");
}

#[test]
fn erro_de_sintaxe_vira_diagnostico_typst_com_posicao() {
    let project = TempProject::empty();
    project.write("main.typ", "= Título\n#include \"cap.typ\"\n");
    project.write("cap.typ", "Linha 1\n#let x = \n");
    let mut world = ProjectWorld::open(project.root(), "main.typ").expect("World");
    let outcome = world.compile();
    assert!(outcome.output.is_none());
    let diag = &outcome.diagnostics[0];
    assert_eq!(diag.kind, Kind::Typst);
    assert_eq!(diag.severity, Severity::Error);
    assert_eq!(diag.path.as_deref(), Some("cap.typ"));
    assert_eq!(diag.line, Some(2));
    assert!(diag.range.is_some());
}

#[test]
fn principal_fora_da_raiz_e_recusado() {
    let project = TempProject::empty();
    let err = ProjectWorld::open(project.root(), "../fora.typ").expect_err("fora da raiz");
    assert_eq!(err.kind, Kind::Access);
    assert_eq!(
        err.message,
        "acesso recusado: `../fora.typ` fica fora da pasta do projeto"
    );
}

#[test]
fn principal_inexistente_vira_diagnostico_de_acesso() {
    let project = TempProject::empty();
    let mut world = ProjectWorld::open(project.root(), "main.typ").expect("World");
    let outcome = world.compile();
    assert!(outcome.output.is_none());
    assert_eq!(outcome.diagnostics[0].kind, Kind::Access);
    assert_eq!(
        outcome.diagnostics[0].message,
        "arquivo não encontrado: `main.typ`"
    );
}

#[test]
fn pacote_nao_e_baixado_nesta_etapa() {
    let (_project, mut world) = single_file("#import \"@preview/exemplo:1.0.0\": *\n");
    let outcome = world.compile();
    assert!(outcome.output.is_none());
    assert!(!outcome.diagnostics.is_empty());
}
