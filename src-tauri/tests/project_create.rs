//! Testes de "Criar um projeto novo" (T071; US6; FR-002, FR-003; Princípio I; contracts/ipc.md
//! `create_project_dialog`; data-model.md "ProjectRecord").
//!
//! O diálogo nativo não roda no runtime de teste, então o caminho feliz é testado por
//! `AppState::create_project`, que o comando chama com a pasta escolhida:
//!
//! - pasta vazia → `main.typ` mínimo e `.prelo/project.json` com `compiler.typst` = versão
//!   embutida e `main` = `main.typ`; o projeto abre sem avisos;
//! - pasta com qualquer coisa (arquivo, pasta oculta, link, `.prelo/`) → `folder_not_empty` e
//!   nada escrito, nem dentro nem fora dela; nunca sobrescreve;
//! - o `main.typ` criado compila no Prelo e, com a feature `equivalence` e `PRELO_TYPST_CLI`,
//!   no `typst` de linha de comando.

mod support;

use std::collections::BTreeSet;
use std::path::Path;

use prelo_lib::commands::AppState;
use prelo_lib::diagnostic::Severity;
use prelo_lib::project::EMBEDDED_TYPST;
use prelo_lib::world::ProjectWorld;
use serde_json::{json, Value};
use support::{symlink, Outside, TempProject};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{
    get_ipc_response, mock_builder, mock_context, noop_assets, MockRuntime, INVOKE_KEY,
};
use tauri::webview::InvokeRequest;
use tauri::{App, WebviewWindow, WebviewWindowBuilder};

const RECORD: &str = ".prelo/project.json";

/// Nomes de primeiro nível da pasta (inclusive ocultos).
fn entries(dir: &Path) -> BTreeSet<String> {
    std::fs::read_dir(dir)
        .expect("listar pasta")
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect()
}

/// Todos os caminhos e conteúdos sob `dir`, sem seguir links (para "nada mudou").
fn snapshot(dir: &Path) -> Vec<(String, Option<Vec<u8>>)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let kind = entry.file_type().unwrap();
            let rel = path
                .strip_prefix(dir)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            if kind.is_dir() {
                stack.push(path);
                out.push((rel, None));
            } else if kind.is_symlink() {
                out.push((
                    rel,
                    Some(
                        std::fs::read_link(&path)
                            .unwrap()
                            .into_os_string()
                            .into_encoded_bytes(),
                    ),
                ));
            } else {
                out.push((rel, Some(std::fs::read(&path).unwrap())));
            }
        }
    }
    out.sort();
    out
}

fn assert_refused_untouched(project: &TempProject, outside: Option<&Outside>) {
    let before = snapshot(&project.root());
    let outside_before = outside.map(|o| snapshot(&o.root()));
    let res = AppState::default().create_project(&project.root());
    assert_eq!(res, Err("folder_not_empty".to_owned()));
    assert_eq!(snapshot(&project.root()), before, "a pasta mudou");
    if let (Some(o), Some(b)) = (outside, outside_before) {
        assert_eq!(
            snapshot(&o.root()),
            b,
            "algo foi escrito fora da pasta escolhida"
        );
    }
}

// ---------------------------------------------------------------- pasta vazia

#[test]
fn pasta_vazia_ganha_main_typ_e_registro_com_a_versao_embutida() {
    let project = TempProject::empty();
    let info = AppState::default()
        .create_project(&project.root())
        .expect("criar projeto");

    assert_eq!(
        entries(&project.root()),
        BTreeSet::from(["main.typ".to_owned(), ".prelo".to_owned()])
    );
    assert_eq!(
        entries(&project.path(".prelo")),
        BTreeSet::from(["project.json".to_owned()])
    );

    let record: Value =
        serde_json::from_slice(&std::fs::read(project.path(RECORD)).unwrap()).expect("JSON");
    assert_eq!(
        record,
        json!({ "schema": 1, "compiler": { "typst": EMBEDDED_TYPST }, "main": "main.typ" })
    );

    let main = std::fs::read_to_string(project.path("main.typ")).expect("main.typ UTF-8");
    assert!(!main.trim().is_empty(), "main.typ vazio");

    // Abre como projeto: principal no registro, versões iguais, nenhum aviso.
    assert_eq!(info.main_file, "main.typ");
    assert_eq!(info.compiler.embedded, EMBEDDED_TYPST);
    assert_eq!(info.compiler.registered.as_deref(), Some(EMBEDDED_TYPST));
    assert!(info.warnings.is_empty(), "avisos: {:?}", info.warnings);
    let expected_name = project
        .root()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert_eq!(info.name, expected_name);
}

#[test]
fn ds_store_do_finder_nao_conta_como_conteudo_e_fica_intacto() {
    let project = TempProject::empty();
    project.write(".DS_Store", b"\x00\x00\x00\x01Bud1");
    AppState::default()
        .create_project(&project.root())
        .expect("criar projeto");
    assert_eq!(
        std::fs::read(project.path(".DS_Store")).unwrap(),
        b"\x00\x00\x00\x01Bud1"
    );
    assert!(project.path("main.typ").is_file());
    assert!(project.path(RECORD).is_file());
}

#[test]
fn main_typ_criado_compila_no_prelo_sem_erros() {
    let project = TempProject::empty();
    AppState::default()
        .create_project(&project.root())
        .expect("criar projeto");
    let mut world = ProjectWorld::open(project.root(), "main.typ").expect("World");
    let outcome = world.compile();
    assert!(
        outcome.output.is_some(),
        "não compilou: {:?}",
        outcome.diagnostics
    );
    let errors: Vec<_> = outcome
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "erros: {errors:?}");
}

/// Com a feature `equivalence` (job do CI que instala o `typst` 0.15.1): o mesmo `main.typ`
/// compila no `typst` de linha de comando (Princípio I; US6 cenário 3).
#[cfg(feature = "equivalence")]
#[test]
fn main_typ_criado_compila_no_typst_cli() {
    let Some(cli) = std::env::var_os("PRELO_TYPST_CLI").filter(|v| !v.is_empty()) else {
        assert!(
            std::env::var_os("PRELO_EQUIVALENCE_REQUIRED").is_none(),
            "PRELO_EQUIVALENCE_REQUIRED definido, mas PRELO_TYPST_CLI não"
        );
        eprintln!("project_create: PRELO_TYPST_CLI não definido; CLI não executado");
        return;
    };
    let project = TempProject::empty();
    AppState::default()
        .create_project(&project.root())
        .expect("criar projeto");
    let out = project.root().join("main.pdf");
    let output = std::process::Command::new(cli)
        .arg("compile")
        .arg("--root")
        .arg(project.root())
        .arg(project.path("main.typ"))
        .arg(&out)
        .output()
        .expect("rodar o typst CLI");
    assert!(
        output.status.success(),
        "typst CLI falhou: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(std::fs::metadata(&out).unwrap().len() > 0);
}

// ---------------------------------------------------------------- pasta com conteúdo

#[test]
fn pasta_com_arquivo_responde_folder_not_empty_e_nada_e_escrito() {
    let project = TempProject::empty();
    project.write("notas.txt", "minhas notas");
    assert_refused_untouched(&project, None);
}

#[test]
fn pasta_com_main_typ_nunca_e_sobrescrita() {
    let project = TempProject::empty();
    project.write("main.typ", "= Meu artigo\n");
    assert_refused_untouched(&project, None);
    assert_eq!(
        std::fs::read_to_string(project.path("main.typ")).unwrap(),
        "= Meu artigo\n"
    );
}

#[test]
fn pasta_so_com_pasta_oculta_ou_registro_tambem_nao_esta_vazia() {
    let git = TempProject::empty();
    std::fs::create_dir(git.path(".git")).unwrap();
    assert_refused_untouched(&git, None);

    let prelo = TempProject::empty();
    prelo.write(
        RECORD,
        r#"{ "schema": 1, "compiler": { "typst": "0.14.0" } }"#,
    );
    assert_refused_untouched(&prelo, None);

    let empty_dir = TempProject::empty();
    std::fs::create_dir(empty_dir.path("capitulos")).unwrap();
    assert_refused_untouched(&empty_dir, None);
}

#[test]
fn link_na_pasta_nao_leva_a_escrita_fora_dela() {
    let outside = Outside::new();
    // `.prelo` e `main.typ` apontando para fora: recusados sem seguir o link.
    let project = TempProject::empty();
    if symlink(&outside.root(), &project.path(".prelo")).is_err() {
        eprintln!("sem permissão para criar link; teste ignorado");
        return;
    }
    assert_refused_untouched(&project, Some(&outside));

    let dangling = TempProject::empty();
    symlink(&outside.root().join("main.typ"), &dangling.path("main.typ")).unwrap();
    assert_refused_untouched(&dangling, Some(&outside));
    assert!(!outside.root().join("main.typ").exists());
}

#[test]
fn pasta_inexistente_ou_arquivo_responde_not_found() {
    let project = TempProject::empty();
    let state = AppState::default();
    assert_eq!(
        state.create_project(&project.path("nao-existe")),
        Err("not_found".to_owned())
    );
    let file = project.write("arquivo.txt", "x");
    assert_eq!(state.create_project(&file), Err("not_found".to_owned()));
    assert!(!project.path("nao-existe").exists());
}

#[test]
fn recusa_nao_abre_nem_fecha_o_projeto_atual() {
    let current = TempProject::from_fixture("artigo-basico");
    let state = AppState::default();
    let before = state.open_project(&current.root()).expect("abrir projeto");
    let full = TempProject::empty();
    full.write("notas.txt", "x");
    assert_eq!(
        state.create_project(&full.root()),
        Err("folder_not_empty".to_owned())
    );
    assert_eq!(
        state.compiler_info().registered.as_deref(),
        before.compiler.registered.as_deref()
    );
    // O projeto aberto continua o mesmo: a árvore lista os arquivos dele.
    let tree = serde_json::to_value(state.list_tree().expect("árvore")).unwrap();
    assert!(tree.to_string().contains("main.typ"));
    assert!(!tree.to_string().contains("notas.txt"));
}

// ---------------------------------------------------------------- IPC

fn app() -> (App<MockRuntime>, WebviewWindow<MockRuntime>) {
    let app = prelo_lib::configure(mock_builder())
        .build(mock_context(noop_assets()))
        .expect("app de teste não montou");
    let window = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("janela de teste não abriu");
    (app, window)
}

fn invoke(window: &WebviewWindow<MockRuntime>, cmd: &str, body: Value) -> Result<Value, Value> {
    let url = if cfg!(windows) {
        "http://tauri.localhost"
    } else {
        "tauri://localhost"
    };
    get_ipc_response(
        window,
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: url.parse().unwrap(),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .map(|b| b.deserialize::<Value>().expect("resposta não é JSON"))
}

#[test]
fn ipc_create_project_dialog_nao_aceita_caminho_da_interface() {
    let (_app, window) = app();
    let outside = Outside::new();
    for body in [
        json!({ "path": outside.root() }),
        json!({ "folder": "/tmp" }),
        json!("x"),
    ] {
        assert_eq!(
            invoke(&window, "create_project_dialog", body),
            Err(json!("invalid_arguments"))
        );
    }
    assert_eq!(
        entries(&outside.root()),
        BTreeSet::from(["segredo.txt".to_owned()])
    );
}
