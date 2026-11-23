//! Testes de versão do compilador e do registro do projeto (T026; FR-003, FR-012; Restrição
//! "Compilação embutida"; data-model.md "ProjectRecord" e estado `VersaoDiferente`;
//! contracts/ipc.md `compiler_info` e `update_project_compiler_version`).
//!
//! - `compiler_info.embedded` é a versão de `typst` no `Cargo.lock` (research R1);
//! - registro ausente é criado com a versão embutida ao abrir, sem tocar outros arquivos;
//! - versão registrada diferente avisa, não compila (nenhum `PreviewUpdate`, mesmo editando) e
//!   não regrava o registro até `update_project_compiler_version`, que regrava e libera a
//!   compilação. Exportação (`export_pdf_dialog`) entra com US5 e herda o mesmo bloqueio.

mod support;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant, SystemTime};

use prelo_lib::commands::{AppState, Change, ProjectInfo};
use prelo_lib::compile::preview::PreviewUpdate;
use serde_json::{json, Value};
use support::TempProject;
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{
    get_ipc_response, mock_builder, mock_context, noop_assets, MockRuntime, INVOKE_KEY,
};
use tauri::webview::InvokeRequest;
use tauri::{App, WebviewWindow, WebviewWindowBuilder};

const RECORD: &str = ".prelo/project.json";
/// Folga para uma compilação em build de depuração (a primeira carrega as fontes).
const WAIT: Duration = Duration::from_secs(60);
/// Quanto esperar para concluir que nenhuma compilação aconteceu.
const QUIET: Duration = Duration::from_secs(3);

/// Versão de `typst` no `Cargo.lock` do núcleo.
fn typst_in_cargo_lock() -> String {
    let lock = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock"))
        .expect("ler Cargo.lock");
    let mut lines = lock.lines();
    while let Some(line) = lines.next() {
        if line.trim() == r#"name = "typst""# {
            let version = lines.next().expect("linha de versão");
            return version
                .trim()
                .strip_prefix("version = \"")
                .and_then(|v| v.strip_suffix('"'))
                .expect("formato de versão do Cargo.lock")
                .to_owned();
        }
    }
    panic!("pacote `typst` ausente do Cargo.lock");
}

fn app(state: AppState) -> (App<MockRuntime>, WebviewWindow<MockRuntime>) {
    let app = prelo_lib::configure_with_state(mock_builder(), state)
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

/// Todos os arquivos sob `root` (relativo → bytes e data de modificação).
fn snapshot(root: &Path) -> BTreeMap<PathBuf, (Vec<u8>, SystemTime)> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, (Vec<u8>, SystemTime)>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            let meta = std::fs::symlink_metadata(&path).unwrap();
            if meta.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    (std::fs::read(&path).unwrap(), meta.modified().unwrap()),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn record_json(project: &TempProject) -> Value {
    serde_json::from_slice(&std::fs::read(project.path(RECORD)).expect("registro gravado"))
        .expect("registro é JSON")
}

/// Estado com a prévia assinada.
fn subscribed() -> (AppState, Receiver<PreviewUpdate>) {
    let state = AppState::default();
    let (tx, rx) = mpsc::channel();
    state.subscribe_preview(move |update: &PreviewUpdate| {
        let _ = tx.send(update.clone());
    });
    (state, rx)
}

fn next_ok(updates: &Receiver<PreviewUpdate>) -> PreviewUpdate {
    let deadline = Instant::now() + WAIT;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let update = updates
            .recv_timeout(left)
            .expect("nenhum PreviewUpdate ok no prazo");
        if update.ok {
            return update;
        }
    }
}

fn project_with_version(version: &str) -> TempProject {
    let project = TempProject::from_fixture("artigo-basico");
    project.write(
        RECORD,
        json!({
            "schema": 1,
            "compiler": { "typst": version, "flags": ["x"] },
            "main": "main.typ",
            "futuro": { "aninhado": true }
        })
        .to_string(),
    );
    project
}

fn has_warning(info: &ProjectInfo, code: &str) -> bool {
    info.warnings.iter().any(|w| w == code)
}

// ---------------------------------------------------------------- versão embutida

#[test]
fn compiler_info_embedded_e_a_versao_do_cargo_lock() {
    let lock = typst_in_cargo_lock();
    assert_eq!(prelo_lib::project::EMBEDDED_TYPST, lock);

    let (_app, window) = app(AppState::default());
    let info = invoke(&window, "compiler_info", json!({})).expect("compiler_info");
    assert_eq!(info["embedded"], json!(lock));
    assert_eq!(info["registered"], Value::Null, "sem projeto aberto");
    assert!(info["fonts"].is_array(), "{info}");
}

#[test]
fn compiler_info_mostra_a_versao_registrada_do_projeto_aberto() {
    let project = project_with_version("0.14.0");
    let state = AppState::default();
    state.open_project(&project.root()).expect("abrir projeto");
    let (_app, window) = app(state);
    let info = invoke(&window, "compiler_info", json!({})).expect("compiler_info");
    assert_eq!(info["embedded"], json!(typst_in_cargo_lock()));
    assert_eq!(info["registered"], json!("0.14.0"));
}

#[test]
fn comandos_de_versao_recusam_argumentos() {
    let (_app, window) = app(AppState::default());
    for cmd in ["compiler_info", "update_project_compiler_version"] {
        assert_eq!(
            invoke(&window, cmd, json!({ "path": "/etc" })),
            Err(json!("invalid_arguments")),
            "{cmd}"
        );
    }
    assert_eq!(
        invoke(&window, "update_project_compiler_version", json!({})),
        Err(json!("no_project"))
    );
}

// ---------------------------------------------------------------- registro ausente (FR-003)

#[test]
fn registro_ausente_e_criado_com_a_versao_embutida_sem_tocar_outros_arquivos() {
    let project = TempProject::from_fixture("artigo-basico");
    let before = snapshot(&project.root());
    assert!(!project.path(".prelo").exists());

    let state = AppState::default();
    let info = state.open_project(&project.root()).expect("abrir projeto");
    assert!(info.warnings.is_empty(), "{:?}", info.warnings);
    assert_eq!(info.compiler.embedded, typst_in_cargo_lock());
    assert_eq!(
        info.compiler.registered.as_deref(),
        Some(info.compiler.embedded.as_str())
    );
    assert_eq!(info.main_file, "main.typ");

    let record = record_json(&project);
    assert_eq!(record["schema"], 1);
    assert_eq!(record["compiler"]["typst"], json!(typst_in_cargo_lock()));
    assert_eq!(record["main"], "main.typ");

    let mut after = snapshot(&project.root());
    assert!(after.remove(Path::new(RECORD)).is_some());
    assert_eq!(before, after, "só .prelo/project.json pode mudar");
    let prelo: Vec<_> = std::fs::read_dir(project.path(".prelo")).unwrap().collect();
    assert_eq!(prelo.len(), 1, "nada além do registro em .prelo/");
}

#[test]
fn registro_valido_nao_e_regravado_ao_abrir() {
    let project = project_with_version(&typst_in_cargo_lock());
    let before = snapshot(&project.root());
    let state = AppState::default();
    let info = state.open_project(&project.root()).expect("abrir projeto");
    assert!(info.warnings.is_empty(), "{:?}", info.warnings);
    assert_eq!(before, snapshot(&project.root()));
}

#[test]
fn registro_invalido_nao_e_recriado_ao_abrir() {
    let project = TempProject::from_fixture("artigo-basico");
    project.write(RECORD, "{ quebrado");
    let state = AppState::default();
    let info = state.open_project(&project.root()).expect("abrir projeto");
    assert_eq!(info.warnings, ["project_json_invalid"]);
    assert_eq!(
        std::fs::read_to_string(project.path(RECORD)).unwrap(),
        "{ quebrado"
    );
}

// ---------------------------------------------------------------- VersaoDiferente (FR-012)

#[test]
fn versao_diferente_avisa_nao_compila_e_nao_regrava() {
    let project = project_with_version("0.14.0");
    let before = snapshot(&project.root());
    let (state, updates) = subscribed();
    let info = state.open_project(&project.root()).expect("abrir projeto");
    assert!(has_warning(&info, "compiler_version_mismatch"), "{info:?}");
    assert_eq!(info.compiler.registered.as_deref(), Some("0.14.0"));
    assert_eq!(info.compiler.embedded, typst_in_cargo_lock());

    // O editor continua disponível, mas nada compila.
    let opened = state.open_file("main.typ").expect("open_file");
    state
        .edit_file(
            "main.typ",
            opened.version,
            &[Change {
                from: 0,
                to: 0,
                insert: "#let x = \n".into(),
            }],
        )
        .expect("edit_file");
    assert!(
        updates.recv_timeout(QUIET).is_err(),
        "VersaoDiferente não pode emitir PreviewUpdate (nem diagnóstico de compilação)"
    );
    // Escolher o principal também não libera a compilação nem muda a versão registrada.
    let info = state
        .choose_main_file("main.typ")
        .expect("choose_main_file");
    assert!(has_warning(&info, "compiler_version_mismatch"), "{info:?}");
    assert!(updates.recv_timeout(QUIET).is_err());
    assert_eq!(record_json(&project)["compiler"]["typst"], "0.14.0");
    let mut after = snapshot(&project.root());
    let mut before = before;
    // `choose_main_file` regrava o registro com o mesmo `main`; o resto fica intacto.
    after.remove(Path::new(RECORD));
    before.remove(Path::new(RECORD));
    assert_eq!(before, after);
}

#[test]
fn reabrir_com_versao_diferente_nao_regrava() {
    let project = project_with_version("0.14.0");
    let before = std::fs::read(project.path(RECORD)).unwrap();
    for _ in 0..2 {
        let state = AppState::default();
        let info = state.open_project(&project.root()).expect("abrir projeto");
        assert!(has_warning(&info, "compiler_version_mismatch"));
    }
    assert_eq!(std::fs::read(project.path(RECORD)).unwrap(), before);
}

#[test]
fn atualizar_versao_regrava_o_registro_e_libera_a_compilacao() {
    let project = project_with_version("0.14.0");
    let (state, updates) = subscribed();
    state.open_project(&project.root()).expect("abrir projeto");
    assert!(updates.recv_timeout(QUIET).is_err());

    let info = state
        .update_project_compiler_version()
        .expect("update_project_compiler_version");
    let embedded = typst_in_cargo_lock();
    assert!(info.warnings.is_empty(), "{:?}", info.warnings);
    assert_eq!(info.compiler.registered.as_deref(), Some(embedded.as_str()));

    let record = record_json(&project);
    assert_eq!(record["compiler"]["typst"], json!(embedded));
    // Campos desconhecidos preservados.
    assert_eq!(record["compiler"]["flags"], json!(["x"]));
    assert_eq!(record["futuro"], json!({ "aninhado": true }));
    assert_eq!(record["main"], "main.typ");

    let update = next_ok(&updates);
    assert!(!update.pages.is_empty());

    // Reabrir: sem aviso, compila de imediato.
    let (again, updates) = subscribed();
    let info = again.open_project(&project.root()).expect("reabrir");
    assert!(info.warnings.is_empty(), "{:?}", info.warnings);
    next_ok(&updates);
}

#[test]
fn atualizar_versao_pelo_ipc() {
    let project = project_with_version("0.14.0");
    let state = AppState::default();
    state.open_project(&project.root()).expect("abrir projeto");
    let (_app, window) = app(state);
    let info = invoke(&window, "update_project_compiler_version", json!({}))
        .expect("update_project_compiler_version");
    assert_eq!(info["warnings"], json!([]));
    assert_eq!(info["compiler"]["registered"], json!(typst_in_cargo_lock()));
    let compiler = invoke(&window, "compiler_info", json!({})).unwrap();
    assert_eq!(compiler["registered"], json!(typst_in_cargo_lock()));
}
