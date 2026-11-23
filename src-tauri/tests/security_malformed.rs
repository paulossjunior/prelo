//! Testes de entrada malformada (T040, T082; FR-024, FR-030, SC-004; research R9, R14;
//! contracts/file-access.md).
//!
//! Cada caso põe no projeto um arquivo malformado (`.typ` inválido ou não UTF-8, `.bib` com
//! chave não fechada, YAML inválido, CSL com XML inválido, imagem corrompida, fonte inválida
//! em `fonts/`, `.prelo/project.json` malformado, recursão profunda, laço infinito, plugin
//! que não retorna, CSL feito para os avisos RUSTSEC-2026-0194/0195 do `quick-xml`) e o
//! compila pela agenda de produção ([`Scheduler`] + [`ProjectCompiler`]). Confere que:
//!
//! - o resultado chega dentro de um limite (nenhum teste pode travar a CI);
//! - há diagnóstico com `kind`, arquivo e, quando o erro tem posição, linha;
//! - logo depois, uma edição válida compila em até 1 s (o núcleo continua respondendo).

mod support;

use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::{Duration, Instant};

use prelo_lib::compile::project::{CompileInput, Compiled, ProjectCompiler};
use prelo_lib::compile::scheduler::{Compilation, Scheduler, Status, COMPILE_TIMEOUT};
use prelo_lib::diagnostic::{Diagnostic, Kind};
use prelo_lib::fs::access_model::AccessModel;
use prelo_lib::fs::scope::{Scope, MAX_CSL_BYTES};
use prelo_lib::project::record::{self, RecordState, RECORD_PATH};
use prelo_lib::world::plugins::{sha256_hex, Decision, PluginApprovals};
use prelo_lib::world::WorldEnv;
use support::{fixtures_dir, TempProject};
use tinymist_vfs::PathAccessModel;

/// Prazo para a resposta a uma edição depois de entrada malformada (SC-004).
const RESPONSIVE: Duration = Duration::from_secs(1);
/// Prazo para o resultado da compilação malformada: o limite de produção mais folga. Nenhum
/// caso espera mais que isso.
const DELIVERY: Duration = Duration::from_secs(COMPILE_TIMEOUT.as_secs() + 20);
/// Limite de tempo da compilação nos casos que não terminam sozinhos (plugin que não
/// retorna). O de produção (10 s) é conferido em `limite_de_producao_e_10_s`; aqui é menor só
/// para a suíte não esperar 10 s.
const SHORT_TIMEOUT: Duration = Duration::from_secs(2);

const EDIT: &str = "= Editado\n\nTexto válido.\n";

/// Projeto aberto na agenda de compilação de produção.
struct Live {
    project: TempProject,
    scheduler: Scheduler<ProjectCompiler>,
    results: Receiver<Compilation<Compiled>>,
}

impl Live {
    fn new(project: TempProject, timeout: Duration) -> Self {
        Self::with_env(project, timeout, WorldEnv::default())
    }

    /// Com decisões de plugin (e cache de pacotes) dados.
    fn with_env(project: TempProject, timeout: Duration, env: WorldEnv) -> Self {
        let root: PathBuf = project.root();
        let (scheduler, results) = Scheduler::with_timeout(
            move || ProjectCompiler::with_env(root.clone(), "main.typ", env.clone()),
            timeout,
        );
        Self {
            project,
            scheduler,
            results,
        }
    }

    /// Projeto com `files` (caminho relativo → bytes) e limite de produção.
    fn with(files: &[(&str, &[u8])]) -> Self {
        Self::with_timeout(files, COMPILE_TIMEOUT)
    }

    fn with_timeout(files: &[(&str, &[u8])], timeout: Duration) -> Self {
        let project = TempProject::empty();
        for (path, content) in files {
            project.write(path, content);
        }
        Self::new(project, timeout)
    }

    /// Compila o que está no disco e devolve o resultado, dentro de [`DELIVERY`].
    fn compile_disk(&self) -> Compilation<Compiled> {
        let id = self.scheduler.submit(CompileInput {
            main: "main.typ".into(),
            files: Vec::new(),
        });
        self.wait(id, DELIVERY)
    }

    fn wait(&self, id: u64, limit: Duration) -> Compilation<Compiled> {
        let deadline = Instant::now() + limit;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let compilation = self
                .results
                .recv_timeout(left)
                .unwrap_or_else(|_| panic!("compilação {id} não chegou em {limit:?}"));
            if compilation.compile_id >= id {
                return compilation;
            }
        }
    }

    /// Uma edição válida, feita logo depois, compila em até 1 s (SC-004).
    fn assert_responds(&self) {
        let started = Instant::now();
        let id = self.scheduler.submit(CompileInput {
            main: "main.typ".into(),
            files: vec![("main.typ".into(), Arc::from(EDIT))],
        });
        let compilation = self.wait(id, RESPONSIVE);
        let elapsed = started.elapsed();
        assert_eq!(
            compilation.status,
            Status::Ok,
            "{:?}",
            compilation.diagnostics
        );
        assert!(elapsed <= RESPONSIVE, "edição levou {elapsed:?}");
    }
}

/// O diagnóstico de erro com `kind`, no arquivo `path`, com linha `line`.
fn find<'a>(diagnostics: &'a [Diagnostic], kind: Kind, path: &str) -> &'a Diagnostic {
    diagnostics
        .iter()
        .find(|d| d.kind == kind && d.path.as_deref() == Some(path))
        .unwrap_or_else(|| panic!("sem diagnóstico {kind:?} em {path}: {diagnostics:?}"))
}

/// Compila o projeto malformado, confere o diagnóstico (arquivo, linha, `kind`) e a resposta
/// à edição seguinte.
fn assert_malformed(files: &[(&str, &[u8])], path: &str, line: u32) -> Diagnostic {
    let live = Live::with(files);
    let compilation = live.compile_disk();
    assert_eq!(compilation.status, Status::ComErros);
    let diag = find(&compilation.diagnostics, Kind::Typst, path).clone();
    assert_eq!(diag.line, Some(line), "{diag:?}");
    assert!(!diag.message.is_empty());
    live.assert_responds();
    diag
}

// ------------------------------------------------------------------------------ `.typ`

#[test]
fn typ_com_sintaxe_invalida() {
    assert_malformed(&[("main.typ", b"= Artigo\n#let x = (\n")], "main.typ", 2);
}

#[test]
fn typ_principal_que_nao_e_utf8() {
    let diag = assert_malformed(
        &[("main.typ", b"= Artigo\nol\xe1 \xff\xfe mundo\n")],
        "main.typ",
        2,
    );
    assert!(
        diag.hints
            .iter()
            .any(|h| h.contains("`main.typ`") && h.contains("linha 2")),
        "{diag:?}"
    );
}

#[test]
fn typ_incluido_que_nao_e_utf8() {
    // O erro aparece no `#include` (linha 3 do principal) e a dica aponta o arquivo e a linha
    // do byte inválido.
    let diag = assert_malformed(
        &[
            ("main.typ", b"= Artigo\n\n#include \"capitulos/cap.typ\"\n"),
            ("capitulos/cap.typ", b"== Um\n\nTexto\n\xc3\x28\n"),
        ],
        "main.typ",
        3,
    );
    assert!(
        diag.hints
            .iter()
            .any(|h| h.contains("`capitulos/cap.typ`") && h.contains("linha 4")),
        "{diag:?}"
    );
}

// ------------------------------------------------------------------------- bibliografia

const BIB_OK: &[u8] = b"@book{silva2020, title={A}, author={Silva, Ana}, year={2020}}\n";

#[test]
fn bib_com_chave_nao_fechada() {
    assert_malformed(
        &[
            ("main.typ", b"= Artigo\n@silva2020\n#bibliography(\"refs.bib\")\n"),
            (
                "refs.bib",
                b"@book{silva2020, title={A}, author={Silva, Ana}, year={2020}}\n\n@book{souza2021, title={B\n",
            ),
        ],
        // O BibLaTeX acusa o fim do arquivo, na linha depois da última quebra.
        "refs.bib",
        4,
    );
}

#[test]
fn yaml_de_bibliografia_invalido() {
    assert_malformed(
        &[
            ("main.typ", b"= Artigo\n#bibliography(\"refs.yml\")\n"),
            (
                "refs.yml",
                b"silva2020:\n  type: book\n  title: [sem fim\n  : : :\n",
            ),
        ],
        "refs.yml",
        3,
    );
}

#[test]
fn yaml_de_dados_invalido() {
    assert_malformed(
        &[
            ("main.typ", b"= Artigo\n#yaml(\"dados.yml\")\n"),
            ("dados.yml", b"a: 1\nb: [\n"),
        ],
        "dados.yml",
        3,
    );
}

#[test]
fn csl_com_xml_invalido() {
    assert_malformed(
        &[
            (
                "main.typ",
                b"= Artigo\n@silva2020\n#bibliography(\"refs.bib\", style: \"estilo.csl\")\n",
            ),
            ("refs.bib", BIB_OK),
            (
                "estilo.csl",
                b"<?xml version=\"1.0\"?>\n<style xmlns=\"http://purl.org/net/xbiblio/csl\">\n<info></style>\n",
            ),
        ],
        "estilo.csl",
        1,
    );
}

// ----------------------------------------------------------------------- imagens, fontes

#[test]
fn png_corrompido() {
    assert_malformed(
        &[
            ("main.typ", b"= Artigo\n\n#image(\"figura.png\")\n"),
            (
                "figura.png",
                b"\x89PNG\r\n\x1a\nlixo que n\xc3\xa3o \xc3\xa9 imagem",
            ),
        ],
        "main.typ",
        3,
    );
}

#[test]
fn jpeg_corrompido() {
    assert_malformed(
        &[
            ("main.typ", b"= Artigo\n#image(\"figura.jpg\")\n"),
            ("figura.jpg", b"\xff\xd8\xff\xe0lixo"),
        ],
        "main.typ",
        2,
    );
}

#[test]
fn svg_com_xml_invalido() {
    assert_malformed(
        &[
            ("main.typ", b"= Artigo\n#image(\"figura.svg\")\n"),
            (
                "figura.svg",
                b"<svg xmlns=\"http://www.w3.org/2000/svg\">\n<g>\n</svg>\n",
            ),
        ],
        "figura.svg",
        3,
    );
}

#[test]
fn fonte_invalida_em_fonts_nao_derruba() {
    // As fontes de `fonts/` do projeto ainda não são carregadas (entram com T056, que
    // trará o diagnóstico `kind = font`); até lá, o arquivo inválido não pode derrubar nem
    // travar a compilação.
    let live = Live::with(&[
        ("main.typ", b"#set text(font: \"Fonte Quebrada\")\nOi\n"),
        (
            "fonts/quebrada.ttf",
            b"\x00\x01\x00\x00lixo que n\xc3\xa3o \xc3\xa9 fonte",
        ),
        ("fonts/quebrada.otf", b"OTTO\x00\x00"),
    ]);
    let compilation = live.compile_disk();
    assert_ne!(compilation.status, Status::Abandonada);
    live.assert_responds();
}

// --------------------------------------------------------------------- registro do projeto

#[test]
fn project_json_malformado_vira_diagnostico_de_projeto() {
    let project = TempProject::empty();
    project.write("main.typ", "= Artigo\n");
    project.write(
        RECORD_PATH,
        "{\n  \"schema\": 1,\n  \"compiler\": {\n    \"typst\": \n",
    );
    let loaded = record::load(&project.root(), "0.15.1");
    let RecordState::Invalid(reason) = &loaded.state else {
        panic!("registro malformado aceito: {loaded:?}");
    };
    let diag = reason.diagnostic();
    assert_eq!(diag.kind, Kind::Project);
    assert_eq!(diag.path.as_deref(), Some(RECORD_PATH));
    assert_eq!(diag.line, Some(5), "{diag:?}");
    // O projeto continua compilando com os padrões.
    let live = Live::new(project, COMPILE_TIMEOUT);
    assert_eq!(live.compile_disk().status, Status::Ok);
    live.assert_responds();
}

#[test]
fn project_json_que_nao_e_json_cita_o_arquivo_sem_linha_inventada() {
    let project = TempProject::empty();
    project.write(RECORD_PATH, vec![b'{'; 2 * 1024 * 1024]);
    let loaded = record::load(&project.root(), "0.15.1");
    let RecordState::Invalid(reason) = &loaded.state else {
        panic!("registro grande demais aceito");
    };
    let diag = reason.diagnostic();
    assert_eq!(diag.kind, Kind::Project);
    assert_eq!(diag.path.as_deref(), Some(RECORD_PATH));
    assert_eq!(diag.line, None);
}

// ----------------------------------------------------------------- compilação sem fim

#[test]
fn recursao_profunda() {
    assert_malformed(
        &[("main.typ", b"= Artigo\n#let f(n) = f(n + 1)\n#f(0)\n")],
        "main.typ",
        2,
    );
}

#[test]
fn laco_infinito() {
    assert_malformed(
        &[("main.typ", b"= Artigo\n#while true {}\n")],
        "main.typ",
        2,
    );
    assert_malformed(
        &[(
            "main.typ",
            b"= Artigo\n#let i = 0\n#while i >= 0 { i += 1 }\n",
        )],
        "main.typ",
        3,
    );
}

#[test]
fn plugin_aprovado_que_nao_retorna_e_abandonado() {
    // Plugin aprovado pelo mecanismo de T047 (decisão nos dados locais do app, FR-019).
    let wasm = std::fs::read(fixtures_dir().join("plugin/teste.wasm")).unwrap();
    let project = TempProject::empty();
    project.write(
        "main.typ",
        b"#let p = plugin(\"teste.wasm\")\n#p.forever()\n",
    );
    project.write("teste.wasm", &wasm);
    let data = tempfile::tempdir().unwrap();
    let approvals = Arc::new(PluginApprovals::load(data.path()));
    approvals
        .decide(
            &project.root(),
            &sha256_hex(&wasm),
            "teste.wasm",
            Decision::Approved,
        )
        .unwrap();
    let env = WorldEnv {
        approvals: Some(approvals),
        ..WorldEnv::default()
    };
    let live = Live::with_env(project, SHORT_TIMEOUT, env);
    let compilation = live.compile_disk();
    assert_eq!(compilation.status, Status::Abandonada);
    let diag = compilation
        .diagnostics
        .iter()
        .find(|d| d.kind == Kind::Typst)
        .expect("diagnóstico de abandono");
    assert!(diag.message.contains("excedeu"), "{diag:?}");
    live.assert_responds();
}

#[test]
fn limite_de_producao_e_10_s() {
    assert_eq!(COMPILE_TIMEOUT, Duration::from_secs(10));
}

// ---------------------------------------- `.csl` e RUSTSEC-2026-0194/0195 (T082, R14)

fn csl_project(csl: Vec<u8>) -> Live {
    Live::with(&[
        (
            "main.typ",
            b"= Artigo\n@silva2020\n#bibliography(\"refs.bib\", style: \"estilo.csl\")\n",
        ),
        ("refs.bib", BIB_OK),
        ("estilo.csl", &csl),
    ])
}

/// Estilo real (fixture) com `extra` dentro da *tag* `<style ...>`.
fn csl_with_style_attributes(extra: &str) -> Vec<u8> {
    let base = std::fs::read_to_string(fixtures_dir().join("bibliografia/abnt.csl")).unwrap();
    let csl = base.replacen("<style ", &format!("<style {extra} "), 1);
    assert_ne!(csl, base);
    csl.into_bytes()
}

#[test]
fn csl_acima_de_1_mib_e_recusado_sem_chegar_ao_compilador() {
    let mut csl = csl_with_style_attributes("");
    csl.resize(MAX_CSL_BYTES as usize + 1, b' ');
    let live = csl_project(csl);

    // Pelo modelo de acesso: o conteúdo não é devolvido nem entra no cache.
    let access = AccessModel::new(Scope::new(live.project.root()).unwrap());
    let error = access
        .content(&live.project.path("estilo.csl"))
        .expect_err("estilo grande entregue");
    assert!(
        error.to_string().contains("estilo grande demais"),
        "{error}"
    );
    assert_eq!(access.cached_entries(), 0);

    // Pela compilação: diagnóstico de acesso na chamada, citando o caminho pedido.
    let compilation = live.compile_disk();
    assert_eq!(compilation.status, Status::ComErros);
    let diag = find(&compilation.diagnostics, Kind::Access, "main.typ");
    assert_eq!(
        diag.message,
        "estilo grande demais: `estilo.csl` (limite 1 MiB)"
    );
    assert_eq!(diag.line, Some(3));
    live.assert_responds();
}

/// Termina em diagnóstico ou em abandono por tempo, dentro do prazo, e o núcleo responde.
fn assert_bounded(live: &Live) {
    let started = Instant::now();
    let compilation = live.compile_disk();
    assert!(started.elapsed() < DELIVERY);
    match compilation.status {
        Status::Ok => {}
        Status::ComErros | Status::Abandonada => {
            assert!(!compilation.diagnostics.is_empty())
        }
    }
    live.assert_responds();
}

#[test]
fn csl_com_milhares_de_atributos_distintos_numa_tag() {
    // RUSTSEC-2026-0194: conferência quadrática de atributos repetidos.
    let attrs: String = (0..20_000).map(|i| format!("a{i}=\"x\" ")).collect();
    let csl = csl_with_style_attributes(&attrs);
    assert!(csl.len() as u64 <= MAX_CSL_BYTES);
    assert_bounded(&csl_project(csl));
}

#[test]
fn csl_com_milhares_de_declaracoes_de_namespace() {
    // RUSTSEC-2026-0195: alocação sem limite para declarações `xmlns:*`.
    let decls: String = (0..20_000)
        .map(|i| format!("xmlns:n{i}=\"urn:n{i}\" "))
        .collect();
    let csl = csl_with_style_attributes(&decls);
    assert!(csl.len() as u64 <= MAX_CSL_BYTES);
    assert_bounded(&csl_project(csl));
}
