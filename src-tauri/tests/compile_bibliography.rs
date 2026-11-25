//! Testes de bibliografia (T050; FR-025, US3 cenários 1, 2 e 7; RF-05, RF-09).
//!
//! - BibLaTeX `.bib` e Hayagriva `.yml`/`.yaml`, um ou vários arquivos, produzem citações e
//!   lista de referências;
//! - estilo `.csl` do projeto e estilo embutido do Typst formatam as citações;
//! - editar o `.bib` no Prelo e salvar recompila com as referências novas; o mesmo vale para
//!   um `.bib` gravado por outro programa (gerenciador de referências) sem estar aberto.
//!
//! - bibliografia já processada não é processada de novo numa edição só de texto do `.typ`
//!   (FR-027, RF-51); volta a ser processada quando o `.bib`/`.yml` muda.
//!
//! O texto da prévia sai das páginas do documento compilado (`TextItem.text`).

mod support;

use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use prelo_lib::commands::{AppState, Change};
use prelo_lib::compile::preview::PreviewUpdate;
use prelo_lib::world::ProjectWorld;
use support::TempProject;
use typst::layout::{Frame, FrameItem};
use typst_layout::PagedDocument;

/// Folga para uma compilação em build de depuração (a primeira carrega as fontes).
const WAIT: Duration = Duration::from_secs(60);

const BIB: &str = "@article{silva2020,\n  author = {Silva, Ana and Costa, Bruno},\n  title = {Edição colaborativa de artigos científicos},\n  journal = {Revista de Testes},\n  year = {2020},\n}\n";
const YML: &str = "lima2019:\n  type: article\n  title: Compiladores de texto científico\n  author: [\"Lima, Daniel\"]\n  date: 2019\n";

fn collect(frame: &Frame, out: &mut String) {
    for (_, item) in frame.items() {
        match item {
            FrameItem::Group(group) => collect(&group.frame, out),
            FrameItem::Text(text) => {
                out.push_str(&text.text);
                out.push(' ');
            }
            _ => {}
        }
    }
}

/// Texto de todas as páginas, com os trechos separados por espaço.
fn document_text(doc: &PagedDocument) -> String {
    let mut out = String::new();
    for page in doc.pages() {
        collect(&page.frame, &mut out);
    }
    out
}

/// Compila `project` (principal `main.typ`) e devolve o texto; erro → falha com os
/// diagnósticos.
fn compiled_text(project: &TempProject) -> String {
    let mut world = ProjectWorld::open(project.root(), "main.typ").expect("World");
    let out = world.compile();
    let doc = out
        .output
        .unwrap_or_else(|| panic!("compilação falhou: {:?}", out.diagnostics));
    document_text(&doc)
}

fn project(files: &[(&str, &str)]) -> TempProject {
    let project = TempProject::empty();
    for (path, content) in files {
        project.write(path, content);
    }
    project
}

// ------------------------------------------------------------------ formatos e estilos

#[test]
fn bib_sozinho_gera_citacao_e_lista() {
    let p = project(&[
        ("refs.bib", BIB),
        (
            "main.typ",
            "Segundo @silva2020.\n#bibliography(\"refs.bib\", style: \"ieee\")\n",
        ),
    ]);
    let text = compiled_text(&p);
    assert!(text.contains("[1]"), "citação numérica do IEEE: {text}");
    assert!(text.contains("colaborativa"), "título na lista: {text}");
    assert!(text.contains("Silva"), "autor na lista: {text}");
}

#[test]
fn yml_e_yaml_do_hayagriva_geram_citacao_e_lista() {
    for name in ["refs.yml", "refs.yaml"] {
        let main = format!("Segundo @lima2019.\n#bibliography(\"{name}\", style: \"ieee\")\n");
        let p = project(&[(name, YML), ("main.typ", &main)]);
        let text = compiled_text(&p);
        assert!(text.contains("[1]"), "{name}: {text}");
        assert!(text.contains("Compiladores"), "{name}: {text}");
    }
}

#[test]
fn varios_arquivos_com_csl_do_projeto() {
    // Fixture: `refs.bib` + `refs.yml` com o estilo `abnt.csl` do projeto: citação
    // autor-data entre parênteses e o termo "and" do estilo em maiúsculas (`text-case`).
    let p = TempProject::from_fixture("bibliografia");
    let text = compiled_text(&p);
    for title in ["colaborativa", "Tipografia", "Compiladores"] {
        assert!(text.contains(title), "{title} na lista: {text}");
    }
    assert!(
        text.contains("AND Costa, 2020"),
        "citação pelo .csl: {text}"
    );
    assert!(
        text.contains("Lima, 2019"),
        "entrada do .yml pelo .csl: {text}"
    );
    assert!(
        text.contains("Silva, Ana AND Costa, Bruno"),
        "lista pelo .csl: {text}"
    );
    assert!(
        !text.contains("[1]"),
        "não é o estilo padrão (IEEE): {text}"
    );
}

#[test]
fn estilo_embutido_e_csl_do_projeto_formatam_diferente() {
    let p = TempProject::from_fixture("bibliografia");
    let main = p.path("main.typ");
    let fixture = std::fs::read_to_string(&main).unwrap();
    std::fs::write(&main, fixture.replace("\"abnt.csl\"", "\"apa\"")).unwrap();
    let text = compiled_text(&p);
    assert!(text.contains("Silva"), "{text}");
    assert!(text.contains("2020"), "{text}");
    assert!(
        !text.contains("AND"),
        "o estilo embutido não usa o .csl: {text}"
    );
}

// ------------------------------------------------------------- editar e salvar (FR-025)

struct Session {
    state: AppState,
    updates: Receiver<PreviewUpdate>,
    project: TempProject,
}

impl Session {
    fn fixture(name: &str) -> Self {
        Self::open(TempProject::from_fixture(name))
    }

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
            project,
        }
    }

    /// Texto da última prévia `Ok`.
    fn preview_text(&self) -> Option<String> {
        let (source, _, _) = self.state.export_source().ok()?;
        Some(document_text(&source.document))
    }

    /// Espera uma prévia `Ok` cujo texto satisfaz `pred`.
    fn wait_text(&self, what: &str, pred: impl Fn(&str) -> bool) -> String {
        let deadline = Instant::now() + WAIT;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let update = self
                .updates
                .recv_timeout(left)
                .unwrap_or_else(|_| panic!("nenhuma prévia {what} no prazo"));
            if !update.ok {
                continue;
            }
            if let Some(text) = self.preview_text().filter(|t| pred(t)) {
                return text;
            }
        }
    }

    /// Troca `from` por `to` no texto aberto de `path`.
    fn replace(&self, path: &str, from: &str, to: &str) {
        let opened = self.state.open_file(path).expect("open_file");
        let start = opened.text.find(from).expect("trecho no arquivo");
        let units = |s: &str| s.encode_utf16().count();
        let at = units(&opened.text[..start]);
        self.state
            .edit_file(
                path,
                opened.version,
                &[Change {
                    from: at,
                    to: at + units(from),
                    insert: to.into(),
                }],
            )
            .expect("edit_file");
    }
}

#[test]
fn editar_o_bib_no_prelo_e_salvar_recompila_com_a_referencia_nova() {
    let s = Session::fixture("bibliografia");
    s.wait_text("inicial", |t| t.contains("colaborativa"));

    s.replace("refs.bib", "Edição colaborativa", "Revisão distribuída");
    s.state.save_file("refs.bib").expect("save_file");
    let text = s.wait_text("com o .bib novo", |t| t.contains("distribuída"));
    assert!(!text.contains("colaborativa"), "{text}");
    let disk = std::fs::read_to_string(s.project.path("refs.bib")).unwrap();
    assert!(disk.contains("Revisão distribuída"), "gravado no disco");
}

#[test]
fn editar_o_yml_no_prelo_e_salvar_recompila() {
    let s = Session::fixture("bibliografia");
    s.wait_text("inicial", |t| t.contains("Compiladores"));

    s.replace("refs.yml", "Compiladores de texto", "Montadores de texto");
    s.state.save_file("refs.yml").expect("save_file");
    s.wait_text("com o .yml novo", |t| {
        t.contains("Montadores") && !t.contains("Compiladores")
    });
}

#[test]
fn bib_gravado_por_outro_programa_recompila_sem_estar_aberto() {
    // Gerenciador de referências (Zotero, JabRef) exporta o `.bib` por cima: o arquivo não
    // está aberto no editor, e a prévia precisa acompanhar.
    let s = Session::fixture("bibliografia");
    s.wait_text("inicial", |t| t.contains("colaborativa"));

    let path = s.project.path("refs.bib");
    let bib = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        bib.replace("Edição colaborativa", "Escrita conjunta"),
    )
    .unwrap();
    // O observador avisa o núcleo do caminho alterado.
    s.state.handle_disk_change("refs.bib");
    s.wait_text("com o .bib do disco", |t| {
        t.contains("conjunta") && !t.contains("colaborativa")
    });
}

fn svg(height: u32) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10\" height=\"{height}\"><rect width=\"10\" height=\"{height}\" fill=\"red\"/></svg>"
    )
}

#[test]
fn imagem_trocada_por_outro_programa_atualiza_a_previa() {
    // Mesmo caminho da releitura do `.bib`: arquivo não aberto que muda no disco é relido
    // (o cache de bytes por conteúdo não segura a versão velha).
    let project = TempProject::empty();
    project.write("fig.svg", svg(10));
    project.write(
        "main.typ",
        "#set page(width: 4cm, height: auto, margin: 0pt)\n#image(\"fig.svg\", width: 2cm)\n",
    );
    let s = Session::open(project);
    let wait_ok = || loop {
        let u = s.updates.recv_timeout(WAIT).expect("prévia no prazo");
        if u.ok {
            return u;
        }
    };
    let before = wait_ok();
    std::fs::write(s.project.path("fig.svg"), svg(40)).unwrap();
    s.state.handle_disk_change("fig.svg");
    let after = wait_ok();
    assert_ne!(
        before.pages[0].height_pt, after.pages[0].height_pt,
        "a página acompanha a imagem nova"
    );
}

// ------------------------------------------------- bibliografia já processada (FR-027)

/// Identidade da bibliografia processada que o documento usa. O Typst guarda o resultado de
/// `Bibliography::decode` (memorizado pelo comemo) num `Arc`: um acerto no cache devolve o
/// mesmo `Arc`; processar de novo cria outro. O tipo é `Bibliography(Arc<..>)` com campo
/// privado, por isso o endereço sai dos bytes do próprio valor (um ponteiro, conferido).
fn bibliography_identity(doc: &PagedDocument) -> usize {
    use typst::foundations::{NativeElement, Selector};
    use typst::introspection::Introspector;
    use typst::model::BibliographyElem;

    let found = doc
        .introspector()
        .query(&Selector::Elem(BibliographyElem::ELEM, None));
    assert_eq!(found.len(), 1, "uma bibliografia no documento");
    let elem = found[0]
        .to_packed::<BibliographyElem>()
        .expect("BibliographyElem");
    let bibliography = &elem.sources.derived;
    assert_eq!(
        std::mem::size_of_val(bibliography),
        std::mem::size_of::<usize>(),
        "Bibliography mudou de forma: rever esta identidade"
    );
    // SAFETY: o valor é um `Arc` (tamanho de um ponteiro, conferido acima); só os bytes do
    // ponteiro são copiados, sem mexer na contagem de referências.
    unsafe { std::mem::transmute_copy::<_, usize>(bibliography) }
}

fn compile_doc(world: &mut ProjectWorld) -> PagedDocument {
    let out = world.compile();
    out.output
        .unwrap_or_else(|| panic!("compilação falhou: {:?}", out.diagnostics))
}

#[test]
fn bibliografia_processada_uma_vez_em_20_edicoes_de_texto() {
    let p = TempProject::from_fixture("bibliografia");
    // Em LF, como o editor entrega ao overlay (no Windows o Git grava a fixture em CRLF).
    let base = std::fs::read_to_string(p.path("main.typ"))
        .unwrap()
        .replace("\r\n", "\n");
    let bib = std::fs::read_to_string(p.path("refs.bib")).unwrap();
    let yml = std::fs::read_to_string(p.path("refs.yml")).unwrap();

    // Documentos guardados: nenhum endereço é reaproveitado entre compilações.
    let mut docs = Vec::new();
    let mut compile = |world: &mut ProjectWorld| {
        let doc = compile_doc(world);
        let id = bibliography_identity(&doc);
        docs.push(doc);
        id
    };

    // Abertura e 20 edições de texto não salvas, como a digitação as produz (cada uma muda um
    // trecho só): as 10 primeiras escrevem um parágrafo antes de `#bibliography(...)`, as 10
    // seguintes um depois. Os bytes do `.bib`/`.yml` não mudam. Limite do Typst: a chave do
    // cache inclui o *span* do caminho em `#bibliography(...)`; uma única troca que mude
    // trechos dos dois lados da chamada a renumera, e a bibliografia é processada de novo.
    let typed = |edit: usize| {
        let before = "Antes da lista "
            .chars()
            .cycle()
            .take(edit.min(10))
            .collect::<String>();
        let after = "Depois da lista"
            .chars()
            .cycle()
            .take(edit.saturating_sub(10))
            .collect::<String>();
        let text = base.replace(
            "= Referências de teste",
            &format!("= Referências de teste\n\n{before}"),
        );
        format!("{text}\n{after}\n")
    };
    // Abertura: o `main.typ` no disco já tem os dois parágrafos, vazios.
    p.write("main.typ", typed(0));
    let mut world = ProjectWorld::open(p.root(), "main.typ").expect("World");
    let opened = compile(&mut world);
    for edit in 1..=20 {
        world
            .set_open_file("main.typ", &typed(edit))
            .expect("overlay");
        let id = compile(&mut world);
        assert_eq!(
            id, opened,
            "edição {edit}: bibliografia processada de novo numa edição só de texto (FR-027)"
        );
    }

    // O `.bib` muda: processa de novo, e as edições de texto seguintes reaproveitam.
    world
        .set_open_file("refs.bib", &bib.replace("colaborativa", "distribuída"))
        .expect("overlay .bib");
    let new_bib = compile(&mut world);
    assert_ne!(new_bib, opened, "o .bib mudou: processa de novo");
    for edit in 21..=25 {
        world
            .set_open_file("main.typ", &typed(edit))
            .expect("overlay");
        assert_eq!(
            compile(&mut world),
            new_bib,
            "edição {edit} depois do .bib novo"
        );
    }

    // O mesmo para o `.yml` do Hayagriva.
    world
        .set_open_file("refs.yml", &yml.replace("Compiladores", "Montadores"))
        .expect("overlay .yml");
    let new_yml = compile(&mut world);
    assert_ne!(new_yml, new_bib, "o .yml mudou: processa de novo");
    world
        .set_open_file("main.typ", &typed(26))
        .expect("overlay");
    assert_eq!(compile(&mut world), new_yml, "edição depois do .yml novo");
}
