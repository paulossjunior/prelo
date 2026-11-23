//! Fontes na compilação (T052; FR-016, D-15, research R6; contracts/file-access.md "Fontes").
//!
//! - fonte em `fonts/` (e subpastas) do projeto é usada; arquivo novo entra na compilação
//!   seguinte;
//! - fonte só instalada no sistema não é usada: o texto sai numa fonte embutida e há aviso
//!   com o nome pedido;
//! - arquivo de fonte malformado vira diagnóstico, sem derrubar a compilação;
//! - `fonts/` segue a regra de escopo (FR-021): link para fora não é lido;
//! - `compiler_info.fonts` lista as Libertinus embutidas com OFL-1.1.

mod support;

use std::collections::BTreeSet;

use prelo_lib::commands::AppState;
use prelo_lib::diagnostic::{Diagnostic, Kind, Severity};
use prelo_lib::world::ProjectWorld;
use support::{symlink, TempProject};
use typst::layout::{Frame, FrameItem};
use typst_layout::PagedDocument;

const PROJECT_FONT: &str = "Libertinus Keyboard";
const FONT_FILE: &str = "LibertinusKeyboard-Regular.otf";

fn font_bytes() -> Vec<u8> {
    std::fs::read(support::fixtures_dir().join("fontes/fonts").join(FONT_FILE)).unwrap()
}

fn families(frame: &Frame, out: &mut BTreeSet<String>) {
    for (_, item) in frame.items() {
        match item {
            FrameItem::Group(group) => families(&group.frame, out),
            FrameItem::Text(text) => {
                out.insert(text.font.info().family.clone());
            }
            _ => {}
        }
    }
}

fn used_families(doc: &PagedDocument) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for page in doc.pages() {
        families(&page.frame, &mut out);
    }
    out
}

fn compile(root: &std::path::Path) -> (PagedDocument, Vec<Diagnostic>) {
    let mut world = ProjectWorld::open(root, "main.typ").expect("World");
    let outcome = world.compile();
    let doc = outcome
        .output
        .unwrap_or_else(|| panic!("não compilou: {:?}", outcome.diagnostics));
    (doc, outcome.diagnostics)
}

fn project_with_font_text(font: &str) -> TempProject {
    let project = TempProject::empty();
    project.write(
        "main.typ",
        format!("#text(font: \"{font}\")[Texto na fonte pedida.]\n\nTexto padrão.\n"),
    );
    project
}

#[test]
fn fonte_da_pasta_fonts_do_projeto_e_usada() {
    let project = TempProject::from_fixture("fontes");
    let (doc, diagnostics) = compile(&project.root());
    assert!(
        used_families(&doc).contains(PROJECT_FONT),
        "{PROJECT_FONT} de fonts/ não foi usada: {:?}",
        used_families(&doc)
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn fonte_em_subpasta_de_fonts_e_usada() {
    let project = project_with_font_text(PROJECT_FONT);
    project.write(&format!("fonts/teclado/otf/{FONT_FILE}"), font_bytes());
    let (doc, diagnostics) = compile(&project.root());
    assert!(used_families(&doc).contains(PROJECT_FONT));
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn fonte_nova_em_fonts_entra_na_compilacao_seguinte() {
    let project = project_with_font_text(PROJECT_FONT);
    let mut world = ProjectWorld::open(project.root(), "main.typ").expect("World");
    let before = world.compile();
    assert!(!used_families(before.output.as_ref().unwrap()).contains(PROJECT_FONT));

    project.write(&format!("fonts/{FONT_FILE}"), font_bytes());
    let after = world.compile();
    assert!(
        used_families(after.output.as_ref().unwrap()).contains(PROJECT_FONT),
        "fonte acrescentada em fonts/ não entrou"
    );
}

#[test]
fn fonte_so_do_sistema_nao_e_usada_e_gera_aviso_com_o_nome() {
    // Fontes comuns em macOS, Windows e Linux: nenhuma pode ser lida (D-15).
    for system_font in ["Helvetica", "Arial", "Times New Roman", "DejaVu Sans"] {
        let project = project_with_font_text(system_font);
        let (doc, diagnostics) = compile(&project.root());
        let used = used_families(&doc);
        assert!(
            used.iter().all(|f| f.starts_with("Libertinus")),
            "{system_font}: fonte fora das embutidas e de fonts/ usada: {used:?}"
        );
        let warning = diagnostics
            .iter()
            .find(|d| d.severity == Severity::Warning)
            .unwrap_or_else(|| panic!("{system_font}: sem aviso: {diagnostics:?}"));
        assert!(
            warning
                .message
                .to_lowercase()
                .contains(&system_font.to_lowercase()),
            "aviso sem o nome da fonte: {warning:?}"
        );
    }
}

#[test]
fn arquivo_de_fonte_malformado_gera_diagnostico_sem_derrubar() {
    let project = project_with_font_text(PROJECT_FONT);
    project.write("fonts/quebrada.ttf", b"isto nao e uma fonte");
    project.write(&format!("fonts/{FONT_FILE}"), font_bytes());
    let (doc, diagnostics) = compile(&project.root());
    assert!(
        used_families(&doc).contains(PROJECT_FONT),
        "a fonte boa continua valendo"
    );
    let bad: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.kind == Kind::Font)
        .collect();
    assert_eq!(bad.len(), 1, "{diagnostics:?}");
    assert_eq!(bad[0].path.as_deref(), Some("fonts/quebrada.ttf"));
    assert!(
        bad[0].message.contains("fonts/quebrada.ttf"),
        "{:?}",
        bad[0]
    );
}

#[test]
fn so_extensoes_de_fonte_sao_lidas() {
    let project = project_with_font_text(PROJECT_FONT);
    // Mesmo conteúdo de fonte com extensão que não é de fonte: ignorado, sem diagnóstico.
    project.write("fonts/teclado.bin", font_bytes());
    project.write("fonts/LEIAME.txt", "fontes do projeto");
    let (doc, diagnostics) = compile(&project.root());
    assert!(!used_families(&doc).contains(PROJECT_FONT));
    assert!(
        diagnostics.iter().all(|d| d.kind != Kind::Font),
        "{diagnostics:?}"
    );
    // Extensão em maiúsculas vale.
    project.write("fonts/TECLADO.OTF", font_bytes());
    let (doc, _) = compile(&project.root());
    assert!(used_families(&doc).contains(PROJECT_FONT));
}

#[test]
fn link_em_fonts_para_fora_do_projeto_nao_e_lido() {
    let outside = TempProject::empty();
    let font = outside.write(FONT_FILE, font_bytes());

    // Arquivo de fonte que é link para fora.
    let project = project_with_font_text(PROJECT_FONT);
    if symlink(&font, &project.path(&format!("fonts/{FONT_FILE}"))).is_err() {
        eprintln!("sem permissão para criar link; teste pulado");
        return;
    }
    let (doc, diagnostics) = compile(&project.root());
    assert!(
        !used_families(&doc).contains(PROJECT_FONT),
        "fonte de fora do projeto foi lida"
    );
    let refused: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.kind == Kind::Access)
        .collect();
    assert_eq!(refused.len(), 1, "{diagnostics:?}");
    assert!(
        !refused[0]
            .message
            .contains(&outside.root().display().to_string()),
        "diagnóstico revela o caminho real: {:?}",
        refused[0]
    );

    // A própria pasta `fonts` como link para uma pasta de fora.
    let project = project_with_font_text(PROJECT_FONT);
    symlink(&outside.root(), &project.path("fonts")).unwrap();
    let (doc, _) = compile(&project.root());
    assert!(
        !used_families(&doc).contains(PROJECT_FONT),
        "pasta de fontes de fora do projeto foi lida"
    );
}

#[test]
fn compiler_info_lista_as_libertinus_embutidas_com_ofl() {
    let info = serde_json::to_value(AppState::default().compiler_info()).unwrap();
    let fonts = info["fonts"].as_array().expect("fonts é lista");
    let files: BTreeSet<&str> = fonts.iter().map(|f| f["file"].as_str().unwrap()).collect();
    let embedded: BTreeSet<&str> = prelo_lib::world::fonts::EMBEDDED
        .iter()
        .map(|(file, _)| *file)
        .collect();
    assert_eq!(files, embedded, "uma entrada por arquivo embutido");
    for font in fonts {
        let mut keys: Vec<_> = font.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, ["copyright", "family", "file", "license"], "{font}");
        assert_eq!(font["license"], "OFL-1.1", "{font}");
        assert!(
            font["family"].as_str().unwrap().starts_with("Libertinus "),
            "{font}"
        );
        assert!(
            font["copyright"]
                .as_str()
                .unwrap()
                .contains("Libertinus Project Authors"),
            "{font}"
        );
    }
}
