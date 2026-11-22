//! Equivalência com o `typst` de linha de comando (T066; FR-031, SC-005; US5 cenário 2).
//!
//! Para cada fixture, o PDF exportado pelo Prelo e o do `typst` CLI na versão embutida, com o
//! mesmo conjunto de fontes (embutidas no Prelo + `fonts/` do projeto, D-15):
//!
//! ```text
//! typst compile --ignore-system-fonts --ignore-embedded-fonts \
//!   --font-path src-tauri/fonts --font-path <projeto>/fonts --root <projeto> \
//!   .prelo-equivalence.typ out.pdf
//! ```
//!
//! (`.prelo-equivalence.typ` só põe os padrões de fonte de matemática e `raw` do Prelo antes de
//! incluir o `main.typ`; ver [`cli_entry`]) são abertos pelo mesmo leitor (o PDF entra como imagem num documento Typst, `support/pdf.rs`),
//! renderizados com `typst-render` e comparados página a página: mesmo número de páginas, mesmo
//! tamanho e os mesmos pixels.
//!
//! Só roda com a feature `equivalence` (job "equivalence" do CI, que instala o `typst` 0.15.1,
//! aponta `PRELO_TYPST_CLI` para ele e define `PRELO_EQUIVALENCE_REQUIRED`). Fora do CI, sem
//! `PRELO_TYPST_CLI` o teste avisa e não compara: a
//! suíte normal (`npm run verify`) não compila este arquivo e não depende de `typst` instalado
//! (FR-010, SC-006).
#![cfg(feature = "equivalence")]

#[path = "support/pdf.rs"]
mod pdf;
mod support;

use std::path::{Path, PathBuf};
use std::process::Command;

use prelo_lib::project::EMBEDDED_TYPST;
use prelo_lib::world::ProjectWorld;
use support::TempProject;

/// Fixtures da equivalência (tasks.md T066): `.bib`/`.yml`/`.csl`, uma figura de cada formato e
/// fonte do projeto.
const FIXTURES: [&str; 4] = ["artigo-basico", "bibliografia", "imagens", "fontes"];

/// Fixtures que ainda diferem por uma tarefa aberta, com o motivo. O teste exige que elas
/// continuem diferentes: quando a tarefa entregar, ele falha e pede para tirar a fixture
/// daqui, e a comparação passa a valer para ela também.
const PENDING: [(&str, &str); 1] = [(
    "fontes",
    "T056 (#121): o Prelo ainda não lê as fontes de fonts/ do projeto",
)];

fn typst_cli() -> Option<PathBuf> {
    std::env::var_os("PRELO_TYPST_CLI")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

fn embedded_fonts() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts")
}

/// PDF do Prelo: o mesmo World da prévia e a mesma exportação de `export_pdf_dialog`.
fn prelo_pdf(root: &Path) -> Vec<u8> {
    let mut world = ProjectWorld::open(root, "main.typ").expect("World");
    let outcome = world.compile();
    let document = outcome
        .output
        .unwrap_or_else(|| panic!("Prelo não compilou: {:?}", outcome.diagnostics));
    prelo_lib::compile::export::pdf(&document).expect("PDF do Prelo")
}

/// Ponto de entrada do CLI: os padrões de fonte do Prelo para matemática e `raw`
/// (`world::fonts::default_styles`, research R6) e o `main.typ` do projeto. O Prelo aplica essas
/// regras como estilos da biblioteca, por fora do documento; o CLI não tem opção para isso, então
/// elas vêm de um arquivo de entrada que inclui o principal sem alterá-lo. Qualquer regra do
/// próprio documento continua vencendo, nos dois lados.
fn cli_entry() -> String {
    format!(
        "#show math.equation: set text(font: \"{}\")\n\
         #show raw: set text(font: \"{}\")\n\
         #include \"main.typ\"\n",
        prelo_lib::world::fonts::MATH_FAMILY,
        prelo_lib::world::fonts::RAW_FAMILY
    )
}

/// PDF do `typst` CLI com o mesmo conjunto de fontes e os mesmos padrões de fonte.
fn cli_pdf(cli: &Path, root: &Path) -> Vec<u8> {
    let entry = root.join(".prelo-equivalence.typ");
    std::fs::write(&entry, cli_entry()).expect("gravar entrada do CLI");
    let out = root.join("cli.pdf");
    let output = Command::new(cli)
        .arg("compile")
        .arg("--ignore-system-fonts")
        .arg("--ignore-embedded-fonts")
        .arg("--font-path")
        .arg(embedded_fonts())
        .arg("--font-path")
        .arg(root.join("fonts"))
        .arg("--root")
        .arg(root)
        .arg(&entry)
        .arg(&out)
        .output()
        .expect("rodar o typst CLI");
    assert!(
        output.status.success(),
        "typst CLI falhou: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = std::fs::read(&out).expect("PDF do CLI");
    std::fs::remove_file(&out).unwrap();
    bytes
}

fn assert_cli_version(cli: &Path) {
    let output = Command::new(cli)
        .arg("--version")
        .output()
        .expect("typst --version");
    let version = String::from_utf8_lossy(&output.stdout);
    assert!(
        version.starts_with(&format!("typst {EMBEDDED_TYPST}")),
        "typst CLI {version:?} não é a versão embutida {EMBEDDED_TYPST}"
    );
}

/// Compara os dois PDFs página a página; devolve as diferenças encontradas.
fn compare(fixture: &str, prelo: &[u8], cli: &[u8]) -> Vec<String> {
    let a = pdf::open(prelo);
    let b = pdf::open(cli);
    // Padrão do `typst-render`: 2 pixels por ponto (144 dpi), sem sangria.
    let options = typst_render::RenderOptions::default();
    let mut problems = Vec::new();
    if a.document.pages().len() != b.document.pages().len() {
        problems.push(format!(
            "{fixture}: {} páginas no Prelo, {} no CLI",
            a.document.pages().len(),
            b.document.pages().len()
        ));
        return problems;
    }
    for (i, (pa, pb)) in a
        .document
        .pages()
        .iter()
        .zip(b.document.pages())
        .enumerate()
    {
        let ra = typst_render::render(pa, &options);
        let rb = typst_render::render(pb, &options);
        if (ra.width(), ra.height()) != (rb.width(), rb.height()) {
            problems.push(format!(
                "{fixture} p. {}: {}x{} px no Prelo, {}x{} px no CLI",
                i + 1,
                ra.width(),
                ra.height(),
                rb.width(),
                rb.height()
            ));
            continue;
        }
        let differing = ra
            .data()
            .chunks_exact(4)
            .zip(rb.data().chunks_exact(4))
            .filter(|(x, y)| x != y)
            .count();
        if differing > 0 {
            problems.push(format!(
                "{fixture} p. {}: {differing} pixels diferentes",
                i + 1
            ));
        }
    }
    problems
}

/// Com `PRELO_EQUIVALENCE_OUT`, grava os dois PDFs que diferem (o job do CI os anexa).
fn keep_for_inspection(fixture: &str, prelo: &[u8], cli: &[u8]) {
    let Some(dir) = std::env::var_os("PRELO_EQUIVALENCE_OUT").filter(|v| !v.is_empty()) else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).expect("pasta de saída");
    std::fs::write(dir.join(format!("{fixture}-prelo.pdf")), prelo).unwrap();
    std::fs::write(dir.join(format!("{fixture}-cli.pdf")), cli).unwrap();
}

#[test]
fn pdf_do_prelo_e_igual_ao_do_typst_cli() {
    let Some(cli) = typst_cli() else {
        // No CI o job define `PRELO_EQUIVALENCE_REQUIRED=1`: sem o CLI, falha em vez de passar
        // sem comparar.
        assert!(
            std::env::var_os("PRELO_EQUIVALENCE_REQUIRED").is_none(),
            "PRELO_EQUIVALENCE_REQUIRED definido, mas PRELO_TYPST_CLI não"
        );
        eprintln!(
            "equivalence: PRELO_TYPST_CLI não definido; comparação não executada \
             (o job \"equivalence\" do CI define)"
        );
        return;
    };
    assert_cli_version(&cli);
    let mut problems = Vec::new();
    for fixture in FIXTURES {
        let project = TempProject::from_fixture(fixture);
        let root = project.root();
        let prelo = prelo_pdf(&root);
        let cli = cli_pdf(&cli, &root);
        let found = compare(fixture, &prelo, &cli);
        if !found.is_empty() {
            keep_for_inspection(fixture, &prelo, &cli);
        }
        match PENDING.iter().find(|(name, _)| *name == fixture) {
            Some((_, reason)) if found.is_empty() => problems.push(format!(
                "{fixture}: já coincide com o CLI; tire-a de PENDING ({reason})"
            )),
            Some((_, reason)) => eprintln!("equivalence: {fixture} pendente, {reason}"),
            None => problems.extend(found),
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
