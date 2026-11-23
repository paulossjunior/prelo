//! Leitura de PDF nos testes de exportação (T065, T066): o PDF entra como imagem num
//! documento Typst de uma página por página do PDF (`image(.., page: k)`, aceito desde a
//! 0.14), compilado pelo mesmo [`ProjectWorld`] do Prelo. Assim os testes conferem número e
//! tamanho das páginas, e a equivalência renderiza cada página com `typst-render`, sem outra
//! biblioteca de PDF.
#![allow(dead_code)]

use prelo_lib::world::ProjectWorld;
use typst_layout::PagedDocument;

/// Teto de páginas procuradas (as fixtures têm poucas).
const MAX_PAGES: usize = 500;

/// PDF aberto como documento Typst: a página `k` do documento é a página `k` do PDF.
pub struct PdfDoc {
    _dir: tempfile::TempDir,
    pub document: PagedDocument,
}

impl PdfDoc {
    /// `(largura, altura)` em pontos de cada página.
    pub fn sizes(&self) -> Vec<(f64, f64)> {
        self.document
            .pages()
            .iter()
            .map(|p| (p.frame.width().to_pt(), p.frame.height().to_pt()))
            .collect()
    }
}

fn wrapper(pages: usize) -> String {
    format!(
        "#set page(width: auto, height: auto, margin: 0pt)\n\
         #for k in range(1, {}) {{ page(block(image(\"doc.pdf\", page: k))) }}\n",
        pages + 1
    )
}

/// Abre `pdf` (bytes) como [`PdfDoc`]. Falha se os bytes não forem um PDF que o Typst leia.
pub fn open(pdf: &[u8]) -> PdfDoc {
    assert!(pdf.starts_with(b"%PDF-"), "não é PDF");
    let dir = tempfile::tempdir().expect("pasta temporária");
    std::fs::write(dir.path().join("doc.pdf"), pdf).expect("gravar PDF");
    std::fs::write(dir.path().join("main.typ"), "").expect("gravar main.typ");
    let mut world = ProjectWorld::open(dir.path(), "main.typ").expect("World");

    // Número de páginas: a primeira página que o Typst recusa.
    let mut count = 0;
    while count < MAX_PAGES {
        let probe = format!("#image(\"doc.pdf\", page: {})\n", count + 1);
        world.set_open_file("main.typ", &probe).expect("overlay");
        if world.compile().output.is_none() {
            break;
        }
        count += 1;
    }
    assert!(count > 0, "PDF sem páginas legíveis");

    world
        .set_open_file("main.typ", &wrapper(count))
        .expect("overlay");
    let outcome = world.compile();
    let document = outcome
        .output
        .unwrap_or_else(|| panic!("PDF não abriu: {:?}", outcome.diagnostics));
    assert_eq!(document.pages().len(), count);
    PdfDoc {
        _dir: dir,
        document,
    }
}
