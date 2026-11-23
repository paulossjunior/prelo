//! Figuras na compilação (T051; FR-026, FR-027, FR-032; RF-50, RF-51).
//!
//! - PNG, JPEG, GIF (estática), SVG, PDF e WebP aparecem no documento compilado e na prévia;
//! - ao longo de 20 edições de texto, cada imagem é decodificada uma vez só: o objeto
//!   decodificado (pixels, árvore SVG, página PDF) é o mesmo em todas as compilações;
//! - o cache de bytes por conteúdo do modelo de acesso não cresce quando o mesmo arquivo muda
//!   de conteúdo várias vezes (só a versão atual de cada arquivo fica guardada).

mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use prelo_lib::compile::preview::render_svg;
use prelo_lib::fs::access_model::AccessModel;
use prelo_lib::fs::scope::Scope;
use prelo_lib::world::ProjectWorld;
use support::TempProject;
use tinymist_vfs::PathAccessModel;
use typst::layout::{Frame, FrameItem};
use typst::visualize::{ImageFormat, ImageKind, RasterFormat, VectorFormat};
use typst_layout::PagedDocument;

/// Formatos esperados na fixture `imagens`, na ordem do `main.typ`.
const FORMATS: [&str; 6] = ["png", "jpg", "gif", "svg", "pdf", "webp"];

fn format_name(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Raster(RasterFormat::Exchange(f)) => match f {
            typst::visualize::ExchangeFormat::Png => "png",
            typst::visualize::ExchangeFormat::Jpg => "jpg",
            typst::visualize::ExchangeFormat::Gif => "gif",
            typst::visualize::ExchangeFormat::Webp => "webp",
        },
        ImageFormat::Raster(RasterFormat::Pixel(_)) => "pixel",
        ImageFormat::Vector(VectorFormat::Svg) => "svg",
        ImageFormat::Vector(VectorFormat::Pdf) => "pdf",
    }
}

/// Identidade do objeto decodificado de cada imagem: muda só se a imagem for decodificada de
/// novo. Os documentos ficam vivos durante o teste, então um endereço não é reaproveitado.
fn decoded_identity(kind: &ImageKind) -> usize {
    match kind {
        ImageKind::Raster(raster) => Arc::as_ptr(raster.dynamic()) as usize,
        ImageKind::Svg(svg) => svg.tree() as *const _ as usize,
        ImageKind::Pdf(pdf) => pdf.page() as *const _ as usize,
    }
}

fn collect(frame: &Frame, out: &mut Vec<(&'static str, usize)>) {
    for (_, item) in frame.items() {
        match item {
            FrameItem::Group(group) => collect(&group.frame, out),
            FrameItem::Image(image, _, _) => {
                out.push((format_name(image.format()), decoded_identity(image.kind())))
            }
            _ => {}
        }
    }
}

fn images(doc: &PagedDocument) -> Vec<(&'static str, usize)> {
    let mut out = Vec::new();
    for page in doc.pages() {
        collect(&page.frame, &mut out);
    }
    out
}

fn compile(world: &mut ProjectWorld) -> PagedDocument {
    let outcome = world.compile();
    let errors: Vec<_> = outcome
        .diagnostics
        .iter()
        .filter(|d| d.severity == prelo_lib::diagnostic::Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    outcome.output.expect("compilou")
}

#[test]
fn todos_os_formatos_de_figura_aparecem() {
    let project = TempProject::from_fixture("imagens");
    let mut world = ProjectWorld::open(project.root(), "main.typ").expect("World");
    let doc = compile(&mut world);
    let found: Vec<_> = images(&doc).into_iter().map(|(f, _)| f).collect();
    assert_eq!(found, FORMATS, "uma figura de cada formato (FR-026)");

    // Na prévia, cada figura vira um `<image>` no SVG da página.
    let svg: String = doc.pages().iter().map(render_svg).collect();
    assert_eq!(
        svg.matches("<image ").count(),
        FORMATS.len(),
        "uma imagem por figura no SVG da prévia"
    );
}

#[test]
fn cada_imagem_e_decodificada_uma_vez_em_20_edicoes_de_texto() {
    let project = TempProject::from_fixture("imagens");
    let base = std::fs::read_to_string(project.path("main.typ")).unwrap();
    let mut world = ProjectWorld::open(project.root(), "main.typ").expect("World");

    // Documentos guardados: nenhum endereço é reaproveitado entre compilações.
    let mut docs = Vec::new();
    let mut seen: BTreeMap<&'static str, BTreeSet<usize>> = BTreeMap::new();
    for edit in 0..=20 {
        // Edição 0 é a abertura; 1..=20 são edições de texto não salvas, na mesma página das
        // figuras.
        let text = format!("{base}\nParágrafo editado {edit}.\n");
        world.set_open_file("main.typ", &text).expect("overlay");
        let doc = compile(&mut world);
        for (format, identity) in images(&doc) {
            seen.entry(format).or_default().insert(identity);
        }
        docs.push(doc);
    }
    assert_eq!(seen.len(), FORMATS.len(), "{seen:?}");
    for (format, identities) in &seen {
        assert_eq!(
            identities.len(),
            1,
            "{format}: decodificada {} vezes em 20 edições de texto (FR-027, FR-032)",
            identities.len()
        );
    }
}

#[test]
fn cache_de_bytes_guarda_so_a_versao_atual_de_cada_arquivo() {
    let project = TempProject::empty();
    project.write("main.typ", "texto");
    let path = project.write("figura.png", b"versao 0");
    let access = AccessModel::new(Scope::new(project.root()).unwrap());

    let first = access.content(&path).expect("lê");
    let again = access.content(&path).expect("lê de novo");
    assert_eq!(first, again);
    assert_eq!(access.cached_entries(), 1, "mesmo conteúdo, uma entrada");

    for version in 1..=10 {
        std::fs::write(&path, format!("versao {version}")).unwrap();
        let bytes = access.content(&path).expect("lê");
        assert_eq!(bytes.as_slice(), format!("versao {version}").as_bytes());
    }
    assert_eq!(
        access.cached_entries(),
        1,
        "versões antigas do mesmo arquivo saem do cache (RF-51 sem crescer sem limite)"
    );

    // Dois arquivos com o mesmo conteúdo compartilham a entrada; mudar um não derruba o outro.
    let other = project.write("copia.png", b"versao 10");
    let shared = access.content(&other).expect("lê a cópia");
    assert_eq!(access.cached_entries(), 1);
    std::fs::write(&path, "versao 11").unwrap();
    access.content(&path).expect("lê");
    assert_eq!(access.cached_entries(), 2);
    assert_eq!(access.content(&other).expect("cópia"), shared);
    assert_eq!(access.cached_entries(), 2);
}
