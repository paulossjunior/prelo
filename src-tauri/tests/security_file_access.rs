//! Testes de segurança de escape da raiz (T012; FR-021, FR-022, FR-030, SC-003;
//! contracts/file-access.md).
//!
//! Cada caso usa o caminho como aparece no `.typ` (`#image("../../fora.png")`,
//! `#read("/../fora.txt")`, `#include`, `bibliography`, `.csl`, `json`, `csv`) e o pede à
//! regra de escopo, que é por onde todo byte do projeto passa. Cada recusa confere o
//! diagnóstico do contrato citando o caminho pedido, que nenhum byte de fora foi devolvido e
//! que houve **zero** aberturas fora da raiz (observador de aberturas da regra de escopo).
//!
//! Links simbólicos e FIFO são criados no teste, não versionados (tasks.md, Notes).

mod support;

use std::path::Path;
#[cfg(unix)]
use std::sync::mpsc;
use std::sync::Arc;
#[cfg(unix)]
use std::time::Duration;

use prelo_lib::fs::scope::{Refusal, Scope, MAX_CSL_BYTES, MAX_FILE_BYTES};
use support::*;

/// Projeto mínimo com `main.typ` e `capitulos/cap1.typ`, e a regra de escopo instrumentada.
struct Case {
    project: TempProject,
    outside: Outside,
    scope: Scope,
    opens: AccessLog,
}

impl Case {
    fn new() -> Self {
        let project = TempProject::empty();
        project.write("main.typ", "= Artigo");
        project.write("capitulos/cap1.typ", "== Capítulo 1");
        let outside = Outside::new();
        for name in [
            "fora.png",
            "fora.txt",
            "refs.bib",
            "abnt.csl",
            "dados.json",
            "dados.csv",
        ] {
            std::fs::write(outside.root().join(name), SECRET).unwrap();
        }
        let opens = AccessLog::default();
        let log = opens.clone();
        let scope = Scope::new(project.root())
            .unwrap()
            .with_open_observer(Arc::new(move |p: &Path| log.record(p)));
        Self {
            project,
            outside,
            scope,
            opens,
        }
    }

    /// Caminho relativo à raiz do projeto que leva à pasta de fora (irmã da raiz).
    fn to_outside(&self, name: &str) -> String {
        let dir = self.outside.root();
        let dir_name = dir.file_name().unwrap().to_str().unwrap();
        let parent = self.project.root().parent().unwrap().to_path_buf();
        assert_eq!(dir.parent().unwrap(), parent, "pastas temporárias irmãs");
        format!("../{dir_name}/{name}")
    }

    fn assert_no_open_outside(&self) {
        assert_eq!(
            self.opens.outside(&self.project.root()),
            Vec::<std::path::PathBuf>::new(),
            "aberturas fora da raiz"
        );
    }

    /// Confere a recusa, o diagnóstico citando só o caminho pedido e zero aberturas fora.
    fn assert_refused(&self, result: Result<Vec<u8>, Refusal>, expected: Refusal, message: &str) {
        let refusal = result.expect_err("acesso deveria ser recusado");
        assert_eq!(refusal, expected);
        let text = refusal.to_string();
        assert_eq!(text, message);
        let outside_dir = self.outside.root();
        assert!(
            !text.contains(outside_dir.to_str().unwrap()),
            "diagnóstico revela caminho real: {text}"
        );
        self.assert_no_open_outside();
    }

    fn assert_outside(&self, current: &str, requested: &str) {
        self.assert_refused(
            self.scope.read_from(current, requested),
            Refusal::OutsideProject {
                requested: requested.into(),
            },
            &format!("acesso recusado: `{requested}` fica fora da pasta do projeto"),
        );
    }
}

#[test]
fn image_subindo_alem_da_raiz() {
    let case = Case::new();
    case.assert_outside("main.typ", "../../fora.png");
    let requested = case.to_outside("fora.png");
    case.assert_outside("main.typ", &requested);
    assert!(case.opens.all().is_empty(), "recusa antes de abrir");
}

#[test]
fn read_com_barra_seguida_de_ponto_ponto() {
    let case = Case::new();
    case.assert_outside("main.typ", "/../fora.txt");
    let requested = format!("/{}", case.to_outside("fora.txt"));
    case.assert_outside("capitulos/cap1.typ", &requested);
    assert!(case.opens.all().is_empty());
}

#[test]
fn include_alem_da_raiz() {
    let case = Case::new();
    let requested = format!("../{}", case.to_outside("fora.txt"));
    case.assert_outside("capitulos/cap1.typ", &requested);
    case.assert_outside("capitulos/cap1.typ", "../../../fora.typ");
}

#[test]
fn bibliografia_estilo_e_dados_fora_da_raiz() {
    let case = Case::new();
    for name in ["refs.bib", "abnt.csl", "dados.json", "dados.csv"] {
        let requested = case.to_outside(name);
        case.assert_outside("main.typ", &requested);
        let requested = format!("/{requested}");
        case.assert_outside("capitulos/cap1.typ", &requested);
    }
    assert!(case.opens.all().is_empty());
}

#[test]
fn link_simbolico_versionado_para_fora() {
    let case = Case::new();
    symlink(&case.outside.secret(), &case.project.path("dados.csv")).unwrap();
    symlink(&case.outside.root(), &case.project.path("pasta-fora")).unwrap();
    case.assert_outside("main.typ", "dados.csv");
    case.assert_outside("capitulos/cap1.typ", "/pasta-fora/segredo.txt");
    case.assert_outside("capitulos/cap1.typ", "../pasta-fora/refs.bib");
    assert!(case.opens.all().is_empty());
}

#[test]
fn link_simbolico_interno_e_aceito() {
    let case = Case::new();
    symlink(
        &case.project.path("capitulos/cap1.typ"),
        &case.project.path("atalho.typ"),
    )
    .unwrap();
    symlink(&case.project.path("capitulos"), &case.project.path("caps")).unwrap();
    assert_eq!(
        case.scope.read_from("main.typ", "atalho.typ").unwrap(),
        "== Capítulo 1".as_bytes()
    );
    assert_eq!(
        case.scope.read("/caps/cap1.typ").unwrap(),
        "== Capítulo 1".as_bytes()
    );
    assert_eq!(case.opens.all().len(), 2);
    case.assert_no_open_outside();
}

#[cfg(unix)]
#[test]
fn fifo_na_raiz_e_recusado_sem_bloquear() {
    let case = Case::new();
    mkfifo(&case.project.path("fila")).unwrap();
    let scope = case.scope.clone();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        tx.send(scope.read("fila")).ok();
    });
    let result = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("leitura do FIFO bloqueou");
    case.assert_refused(
        result,
        Refusal::SpecialFile {
            requested: "fila".into(),
        },
        "acesso recusado: `fila` não é um arquivo comum",
    );
}

#[cfg(windows)]
#[test]
fn juncao_para_fora_e_recusada() {
    let case = Case::new();
    let link = case.project.path("juncao");
    let status = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(case.outside.root())
        .status()
        .unwrap();
    assert!(status.success(), "mklink /J falhou");
    case.assert_outside("main.typ", "juncao/segredo.txt");
    case.assert_outside("main.typ", "/juncao/fora.png");
}

/// Troca o último componente por link para fora entre a conferência (passos 3–4) e a
/// abertura (passo 5).
#[test]
fn troca_do_arquivo_por_link_entre_conferencia_e_leitura() {
    let case = Case::new();
    case.project.write("figura.png", "imagem de dentro");
    let target = case.project.path("figura.png");
    let secret = case.outside.secret();
    let scope = case
        .scope
        .clone()
        .with_open_observer(Arc::new(move |p: &Path| {
            if p == target {
                std::fs::remove_file(&target).unwrap();
                symlink(&secret, &target).unwrap();
            }
        }));
    let result = scope.read("figura.png");
    assert_eq!(
        result,
        Err(Refusal::OutsideProject {
            requested: "figura.png".into()
        })
    );
}

/// Troca uma pasta intermediária por link para fora entre a conferência e a abertura: a
/// conferência no descritor aberto (passo 6) recusa.
#[test]
fn troca_de_pasta_por_link_entre_conferencia_e_leitura() {
    let case = Case::new();
    case.project.write("dados/segredo.txt", "dado de dentro");
    let dir = case.project.path("dados");
    let outside = case.outside.root();
    let scope = case
        .scope
        .clone()
        .with_open_observer(Arc::new(move |_: &Path| {
            if dir.is_dir() && !dir.is_symlink() {
                std::fs::remove_dir_all(&dir).unwrap();
                symlink(&outside, &dir).unwrap();
            }
        }));
    let result = scope.read("dados/segredo.txt");
    assert_eq!(
        result,
        Err(Refusal::OutsideProject {
            requested: "dados/segredo.txt".into()
        })
    );
}

/// Troca o arquivo por FIFO entre a conferência e a abertura: a abertura não bloqueia e a
/// conferência no descritor recusa.
#[cfg(unix)]
#[test]
fn troca_do_arquivo_por_fifo_entre_conferencia_e_leitura() {
    let case = Case::new();
    case.project.write("tabela.csv", "a,b");
    let target = case.project.path("tabela.csv");
    let scope = case
        .scope
        .clone()
        .with_open_observer(Arc::new(move |p: &Path| {
            if p == target {
                std::fs::remove_file(&target).unwrap();
                mkfifo(&target).unwrap();
            }
        }));
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        tx.send(scope.read("tabela.csv")).ok();
    });
    let result = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("abertura do FIFO bloqueou");
    assert_eq!(
        result,
        Err(Refusal::SpecialFile {
            requested: "tabela.csv".into()
        })
    );
}

#[test]
fn arquivo_maior_que_256_mb_e_recusado() {
    let case = Case::new();
    let path = case.project.path("enorme.bin");
    // Arquivo esparso: não ocupa disco nem é lido.
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(MAX_FILE_BYTES + 1).unwrap();
    drop(file);
    case.assert_refused(
        case.scope.read("enorme.bin"),
        Refusal::TooLarge {
            requested: "enorme.bin".into(),
            limit: MAX_FILE_BYTES,
        },
        "arquivo grande demais: `enorme.bin` (limite 256 MB)",
    );
}

#[test]
fn estilo_csl_maior_que_1_mib_e_recusado() {
    let case = Case::new();
    case.project
        .write("estilos/abnt.csl", vec![b' '; MAX_CSL_BYTES as usize + 1]);
    case.assert_refused(
        case.scope.read_from("main.typ", "estilos/abnt.csl"),
        Refusal::TooLarge {
            requested: "estilos/abnt.csl".into(),
            limit: MAX_CSL_BYTES,
        },
        "estilo grande demais: `estilos/abnt.csl` (limite 1 MiB)",
    );
    // No limite exato, é lido.
    case.project
        .write("estilos/ok.CSL", vec![b' '; MAX_CSL_BYTES as usize]);
    assert_eq!(
        case.scope.read("estilos/ok.CSL").unwrap().len(),
        MAX_CSL_BYTES as usize
    );
}

#[test]
fn leitura_dentro_da_raiz_abre_so_dentro() {
    let case = Case::new();
    assert_eq!(
        case.scope
            .read_from("capitulos/cap1.typ", "../main.typ")
            .unwrap(),
        b"= Artigo"
    );
    assert_eq!(case.opens.all(), vec![case.project.root().join("main.typ")]);
}

// ---------------------------------------------------------------------------------------
// Nível de documento (T012 por inteiro, com T020): os mesmos escapes escritos no `.typ` e
// compilados pelo World do Prelo. O modelo de acesso é embrulhado em `RecordingAccess`
// (registra todo caminho que o VFS pede) e a regra de escopo conta toda abertura real.
// ---------------------------------------------------------------------------------------

use prelo_lib::diagnostic::{Diagnostic, Kind};
use prelo_lib::fs::access_model::AccessModel;
use prelo_lib::world::ProjectWorld;

type DocWorld = ProjectWorld<RecordingAccess<AccessModel>>;

/// Projeto compilado pelo World, com os dois registros de acesso.
struct Doc {
    case: Case,
    requested: AccessLog,
}

impl Doc {
    fn new() -> Self {
        let case = Case::new();
        case.project
            .write("refs.bib", "@book{a, title={A}, author={B}, year={2020}}");
        Self {
            case,
            requested: AccessLog::default(),
        }
    }

    fn world(&self) -> DocWorld {
        let access = AccessModel::new(self.case.scope.clone());
        let log = self.requested.clone();
        ProjectWorld::instrumented(access, "main.typ", move |model| RecordingAccess {
            inner: model,
            log,
        })
        .expect("World")
    }

    /// Compila `main` (e `cap1`, que `main` pode incluir) e devolve os diagnósticos.
    fn compile(&self, main: &str, cap1: &str) -> (bool, Vec<Diagnostic>) {
        self.case.project.write("main.typ", main);
        self.case.project.write("capitulos/cap1.typ", cap1);
        let outcome = self.world().compile();
        (outcome.output.is_some(), outcome.diagnostics)
    }

    /// Nada aberto nem pedido fora da raiz; nenhum diagnóstico revela caminho real ou segredo.
    fn assert_contained(&self, diagnostics: &[Diagnostic]) {
        self.case.assert_no_open_outside();
        let root = self.case.project.root();
        for path in self.requested.all() {
            assert!(path.starts_with(&root), "VFS pediu fora da raiz: {path:?}");
        }
        let outside = self.case.outside.root();
        for diag in diagnostics {
            let text = format!("{diag:?}");
            assert!(!text.contains(outside.to_str().unwrap()), "{text}");
            assert!(!text.contains(root.to_str().unwrap()), "{text}");
            assert!(!text.contains(SECRET), "{text}");
        }
    }

    /// O documento não compila e há diagnóstico de acesso com `message`, no arquivo `path`.
    fn assert_refused(&self, main: &str, cap1: &str, path: &str, message: &str) {
        let (compiled, diagnostics) = self.compile(main, cap1);
        assert!(!compiled, "compilou: {main}");
        let diag = diagnostics
            .iter()
            .find(|d| d.kind == Kind::Access)
            .unwrap_or_else(|| panic!("sem diagnóstico de acesso: {diagnostics:?}"));
        assert_eq!(diag.message, message);
        assert_eq!(diag.path.as_deref(), Some(path));
        assert!(diag.range.is_some() && diag.line.is_some(), "{diag:?}");
        self.assert_contained(&diagnostics);
    }

    fn outside_msg(requested: &str) -> String {
        format!("acesso recusado: `{requested}` fica fora da pasta do projeto")
    }
}

const INCLUI_CAP1: &str = "= Artigo\n#include \"capitulos/cap1.typ\"\n";

#[test]
fn documento_image_subindo_alem_da_raiz() {
    let doc = Doc::new();
    doc.assert_refused(
        "#image(\"../../fora.png\")",
        "",
        "main.typ",
        &Doc::outside_msg("../../fora.png"),
    );
    let requested = doc.case.to_outside("fora.png");
    doc.assert_refused(
        &format!("#image(\"{requested}\")"),
        "",
        "main.typ",
        &Doc::outside_msg(&requested),
    );
}

#[test]
fn documento_read_com_barra_seguida_de_ponto_ponto() {
    let doc = Doc::new();
    doc.assert_refused(
        "#read(\"/../fora.txt\")",
        "",
        "main.typ",
        &Doc::outside_msg("/../fora.txt"),
    );
}

#[test]
fn documento_include_alem_da_raiz() {
    let doc = Doc::new();
    let requested = format!("../{}", doc.case.to_outside("fora.txt"));
    doc.assert_refused(
        INCLUI_CAP1,
        &format!("#include \"{requested}\""),
        "capitulos/cap1.typ",
        &Doc::outside_msg(&requested),
    );
}

#[test]
fn documento_bibliografia_estilo_e_dados_fora_da_raiz() {
    let doc = Doc::new();
    let bib = doc.case.to_outside("refs.bib");
    let csl = doc.case.to_outside("abnt.csl");
    let json = format!("/{}", doc.case.to_outside("dados.json"));
    let csv = doc.case.to_outside("dados.csv");
    for (call, requested) in [
        (format!("#bibliography(\"{bib}\")"), bib.clone()),
        (
            format!("#bibliography(\"refs.bib\", style: \"{csl}\")"),
            csl.clone(),
        ),
        (format!("#json(\"{json}\")"), json.clone()),
        (format!("#csv(\"{csv}\")"), csv.clone()),
    ] {
        doc.assert_refused(&call, "", "main.typ", &Doc::outside_msg(&requested));
    }
}

#[test]
fn documento_link_simbolico_versionado_para_fora() {
    let doc = Doc::new();
    symlink(
        &doc.case.outside.secret(),
        &doc.case.project.path("dados.csv"),
    )
    .unwrap();
    symlink(
        &doc.case.outside.root(),
        &doc.case.project.path("pasta-fora"),
    )
    .unwrap();
    doc.assert_refused(
        "#csv(\"dados.csv\")",
        "",
        "main.typ",
        &Doc::outside_msg("dados.csv"),
    );
    doc.assert_refused(
        INCLUI_CAP1,
        "#read(\"../pasta-fora/segredo.txt\")",
        "capitulos/cap1.typ",
        &Doc::outside_msg("../pasta-fora/segredo.txt"),
    );
}

#[test]
fn documento_link_simbolico_interno_e_aceito() {
    let doc = Doc::new();
    symlink(
        &doc.case.project.path("capitulos/cap1.typ"),
        &doc.case.project.path("atalho.typ"),
    )
    .unwrap();
    let (compiled, diagnostics) = doc.compile("#include \"atalho.typ\"", "== Capítulo 1");
    assert!(compiled, "{diagnostics:?}");
    assert_eq!(diagnostics, vec![]);
    doc.assert_contained(&diagnostics);
}

#[cfg(unix)]
#[test]
fn documento_fifo_na_raiz_e_recusado_sem_bloquear() {
    let doc = Doc::new();
    mkfifo(&doc.case.project.path("fila")).unwrap();
    doc.case.project.write("main.typ", "#read(\"fila\")");
    let mut world = doc.world();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        tx.send(world.compile().diagnostics).ok();
    });
    let diagnostics = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("compilação com FIFO bloqueou");
    let diag = diagnostics
        .iter()
        .find(|d| d.kind == Kind::Access)
        .expect("diagnóstico de acesso");
    assert_eq!(
        diag.message,
        "acesso recusado: `fila` não é um arquivo comum"
    );
    doc.assert_contained(&diagnostics);
}

#[cfg(windows)]
#[test]
fn documento_juncao_para_fora_e_recusada() {
    let doc = Doc::new();
    let link = doc.case.project.path("juncao");
    let status = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(doc.case.outside.root())
        .status()
        .unwrap();
    assert!(status.success(), "mklink /J falhou");
    doc.assert_refused(
        "#read(\"juncao/segredo.txt\")",
        "",
        "main.typ",
        &Doc::outside_msg("juncao/segredo.txt"),
    );
}

#[test]
fn documento_arquivo_maior_que_256_mb_e_recusado() {
    let doc = Doc::new();
    let file = std::fs::File::create(doc.case.project.path("enorme.bin")).unwrap();
    file.set_len(MAX_FILE_BYTES + 1).unwrap();
    drop(file);
    doc.assert_refused(
        "#read(\"enorme.bin\", encoding: none)",
        "",
        "main.typ",
        "arquivo grande demais: `enorme.bin` (limite 256 MB)",
    );
}

#[test]
fn documento_estilo_csl_maior_que_1_mib_e_recusado() {
    let doc = Doc::new();
    doc.case
        .project
        .write("estilos/abnt.csl", vec![b' '; MAX_CSL_BYTES as usize + 1]);
    doc.assert_refused(
        "#bibliography(\"refs.bib\", style: \"estilos/abnt.csl\")",
        "",
        "main.typ",
        "estilo grande demais: `estilos/abnt.csl` (limite 1 MiB)",
    );
}

#[test]
fn documento_dentro_da_raiz_compila() {
    let doc = Doc::new();
    doc.case.project.write("dados.csv", "a,b\n1,2\n");
    let (compiled, diagnostics) = doc.compile(
        "#include \"capitulos/cap1.typ\"\n#table(columns: 2, ..csv(\"dados.csv\").flatten())",
        "#read(\"../dados.csv\")",
    );
    assert!(compiled, "{diagnostics:?}");
    assert_eq!(diagnostics, vec![]);
    doc.assert_contained(&diagnostics);
    assert!(!doc.case.opens.all().is_empty());
}

// ---------------------------------------------------------------------------------------
// Autocompletar de caminhos (T041; RF-08, FR-021; contracts/file-access.md, "Autocompletar
// de caminhos"): toda sugestão passa pelos passos 2–4 da regra de escopo; link simbólico
// para fora não aparece, nem com `..` digitado.
// ---------------------------------------------------------------------------------------

use prelo_lib::commands::AppState;
use prelo_lib::lang_service::{path_candidates, Completion};

/// Projeto com links simbólicos para fora (arquivo e pasta), um interno e uma imagem.
fn completion_case() -> Case {
    let case = Case::new();
    case.project.write("capitulos/cap2.typ", "== Capítulo 2");
    case.project.write("figuras/grafico.png", "png");
    std::fs::write(case.outside.root().join("vizinho.typ"), SECRET).unwrap();
    std::fs::write(case.outside.root().join("foto.png"), SECRET).unwrap();
    symlink(
        &case.outside.root().join("vizinho.typ"),
        &case.project.path("segredo.typ"),
    )
    .unwrap();
    symlink(
        &case.outside.root().join("foto.png"),
        &case.project.path("foto.png"),
    )
    .unwrap();
    symlink(&case.outside.root(), &case.project.path("pasta-fora")).unwrap();
    symlink(
        &case.project.path("capitulos/cap1.typ"),
        &case.project.path("atalho.typ"),
    )
    .unwrap();
    case
}

/// Caminhos sugeridos (`kind = text`), sem as aspas que o `typst-ide` põe em volta.
fn suggested_paths(items: &[Completion]) -> Vec<String> {
    items
        .iter()
        .filter(|c| c.kind.as_deref() == Some("text"))
        .map(|c| c.label.trim_matches('"').to_owned())
        .collect()
}

/// Nenhuma sugestão sai da raiz: cada uma, relativa a `current`, passa pela regra de escopo,
/// e nenhuma cita o que está do lado de fora.
fn assert_inside(case: &Case, current: &str, paths: &[String]) {
    let outside_dir = case.outside.root();
    let outside_name = outside_dir.file_name().unwrap().to_str().unwrap();
    for path in paths {
        let resolved = case.scope.resolve_from(current, path);
        assert!(
            resolved.is_ok(),
            "sugestão {path:?} sai da raiz: {resolved:?}"
        );
        for forbidden in ["segredo", "foto.png", "pasta-fora", "vizinho", outside_name] {
            assert!(
                !path.contains(forbidden),
                "sugestão revela {forbidden}: {path}"
            );
        }
    }
}

#[test]
fn candidatos_de_caminho_ficam_dentro_da_raiz() {
    let case = completion_case();
    let candidates = path_candidates(&case.scope);
    for expected in [
        "main.typ",
        "capitulos/cap1.typ",
        "capitulos/cap2.typ",
        "figuras/grafico.png",
        // Link interno é aceito (FR-021).
        "atalho.typ",
    ] {
        assert!(
            candidates.iter().any(|c| c == expected),
            "faltou {expected}: {candidates:?}"
        );
    }
    assert_inside(&case, "", &candidates);
    // Listar não abre arquivo nenhum, muito menos fora da raiz.
    case.assert_no_open_outside();
}

#[cfg(unix)]
#[test]
fn candidatos_de_caminho_ignoram_fifo_e_arquivos_ocultos() {
    let case = completion_case();
    mkfifo(&case.project.path("fila.typ")).unwrap();
    case.project.write(".prelo/project.json", "{}");
    case.project.write(".git/config", "x");
    let candidates = path_candidates(&case.scope);
    assert!(
        !candidates.iter().any(|c| c.contains("fila")),
        "{candidates:?}"
    );
    assert!(
        !candidates.iter().any(|c| c.starts_with('.')),
        "{candidates:?}"
    );
    assert!(candidates.iter().any(|c| c == "main.typ"), "{candidates:?}");
}

#[test]
fn autocompletar_de_include_com_ponto_ponto_nao_sugere_nada_fora() {
    let case = completion_case();
    let state = AppState::default();
    state
        .open_project(&case.project.root())
        .expect("abrir projeto");

    // `..` digitado no arquivo da raiz: o que vier não pode sair dela.
    let main = "= Artigo\n#include \"../\"\n";
    case.project.write("main.typ", main);
    state.open_file("main.typ").expect("abrir main");
    let cursor = main.find("../").unwrap() + 3;
    let items = state.complete("main.typ", cursor).expect("complete");
    let paths = suggested_paths(&items);
    assert!(paths.iter().any(|p| p == "capitulos/cap1.typ"), "{items:?}");
    assert!(paths.iter().any(|p| p == "atalho.typ"), "{items:?}");
    assert_inside(&case, "main.typ", &paths);

    // `../../` num capítulo: sobe até a raiz e não além.
    let cap = "#include \"../../\"\n";
    case.project.write("capitulos/cap1.typ", cap);
    state.open_file("capitulos/cap1.typ").expect("abrir cap1");
    let cursor = cap.find("../../").unwrap() + 6;
    let items = state
        .complete("capitulos/cap1.typ", cursor)
        .expect("complete");
    let paths = suggested_paths(&items);
    assert!(paths.iter().any(|p| p == "../main.typ"), "{items:?}");
    assert_inside(&case, "capitulos/cap1.typ", &paths);
}

#[test]
fn autocompletar_de_imagem_nao_lista_link_para_fora() {
    let case = completion_case();
    let state = AppState::default();
    state
        .open_project(&case.project.root())
        .expect("abrir projeto");
    let main = "#image(\"\")\n";
    case.project.write("main.typ", main);
    state.open_file("main.typ").expect("abrir main");
    let cursor = main.find('"').unwrap() + 1;
    let items = state.complete("main.typ", cursor).expect("complete");
    let paths = suggested_paths(&items);
    assert_eq!(paths, vec!["figuras/grafico.png".to_owned()], "{items:?}");
    assert_inside(&case, "main.typ", &paths);
}
