//! Testes da exportação do PDF (T065; US5; FR-011, FR-012, FR-028, FR-029; contracts/ipc.md
//! "Exportação").
//!
//! - o PDF vem da mesma compilação da prévia: o `compileId` da exportação é o da última
//!   prévia `Ok`, e o PDF tem o mesmo número de páginas e os mesmos tamanhos (FR-011, FR-028);
//! - com erro de compilação, a exportação é recusada com `compile_errors` e a lista de
//!   diagnósticos da prévia, sem gravar nada (FR-029);
//! - com versão registrada ≠ embutida, `compiler_version_mismatch` (FR-012);
//! - falha de gravação responde `write_failed` e não deixa arquivo parcial nem temporário;
//!   o PDF anterior no mesmo caminho continua intacto.
//!
//! `export_pdf_dialog` recusa antes de mostrar o diálogo; o diálogo nativo em si não roda no
//! runtime de teste, então o caminho feliz é testado por `AppState::export_pdf`, que o comando
//! chama com o caminho escolhido.

#[path = "support/pdf.rs"]
mod pdf;
mod support;

use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use prelo_lib::commands::{AppState, Change};
use prelo_lib::compile::export::ExportError;
use prelo_lib::compile::preview::PreviewUpdate;
use prelo_lib::diagnostic::Severity;
use serde_json::{json, Value};
use support::TempProject;
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{
    get_ipc_response, mock_builder, mock_context, noop_assets, MockRuntime, INVOKE_KEY,
};
use tauri::webview::InvokeRequest;
use tauri::{App, WebviewWindow, WebviewWindowBuilder};

/// Folga para uma compilação em build de depuração (a primeira carrega as fontes).
const WAIT: Duration = Duration::from_secs(60);

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

/// Estado com a prévia assinada.
fn subscribed() -> (AppState, Receiver<PreviewUpdate>) {
    let state = AppState::default();
    let (tx, rx) = mpsc::channel();
    state.subscribe_preview(move |update: &PreviewUpdate| {
        let _ = tx.send(update.clone());
    });
    (state, rx)
}

/// Próximo `PreviewUpdate` com `ok == want`.
fn next(updates: &Receiver<PreviewUpdate>, want: bool) -> PreviewUpdate {
    let deadline = Instant::now() + WAIT;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let update = updates
            .recv_timeout(left)
            .unwrap_or_else(|_| panic!("nenhum PreviewUpdate com ok={want} no prazo"));
        if update.ok == want {
            return update;
        }
    }
}

/// Projeto aberto e compilado: estado, prévia e a primeira prévia `Ok`.
fn opened(
    fixture: &str,
) -> (
    TempProject,
    AppState,
    Receiver<PreviewUpdate>,
    PreviewUpdate,
) {
    let project = TempProject::from_fixture(fixture);
    let (state, updates) = subscribed();
    state.open_project(&project.root()).expect("abrir projeto");
    let first = next(&updates, true);
    (project, state, updates, first)
}

/// Acrescenta `text` ao fim de `main.typ` (texto não salvo) e devolve a nova versão.
fn append(state: &AppState, text: &str) {
    let opened = state.open_file("main.typ").expect("open_file");
    let end = opened.text.encode_utf16().count();
    state
        .edit_file(
            "main.typ",
            opened.version,
            &[Change {
                from: end,
                to: end,
                insert: text.into(),
            }],
        )
        .expect("edit_file");
}

fn assert_same_pages(pdf_bytes: &[u8], update: &PreviewUpdate) {
    let sizes = pdf::open(pdf_bytes).sizes();
    assert_eq!(sizes.len(), update.pages.len(), "número de páginas");
    for (page, (w, h)) in update.pages.iter().zip(sizes) {
        assert!(
            (page.width_pt - w).abs() < 0.01 && (page.height_pt - h).abs() < 0.01,
            "página {}: prévia {}x{} pt, PDF {w}x{h} pt",
            page.index,
            page.width_pt,
            page.height_pt
        );
    }
}

/// Arquivos da pasta, para conferir que nada sobrou.
fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn pdf_vem_da_ultima_previa_ok_com_as_mesmas_paginas() {
    let (_project, state, _updates, first) = opened("artigo-basico");
    let out = tempfile::tempdir().unwrap();
    let path = out.path().join("artigo.pdf");

    let exported = state.export_pdf(&path).expect("export_pdf");
    assert_eq!(exported.compile_id, first.compile_id, "FR-011");
    let bytes = std::fs::read(&path).expect("PDF gravado");
    assert_same_pages(&bytes, &first);
    assert_eq!(names(out.path()), ["artigo.pdf"], "sem temporário");
}

#[test]
fn pdf_acompanha_a_edicao_nao_salva() {
    let (_project, state, updates, first) = opened("artigo-basico");
    append(
        &state,
        "\n#pagebreak()\n#set page(paper: \"a5\")\nPágina nova em A5.\n",
    );
    let second = next(&updates, true);
    assert_eq!(second.pages.len(), first.pages.len() + 1);

    let out = tempfile::tempdir().unwrap();
    let path = out.path().join("artigo.pdf");
    let exported = state.export_pdf(&path).expect("export_pdf");
    assert_eq!(exported.compile_id, second.compile_id, "FR-011");
    assert_same_pages(&std::fs::read(&path).unwrap(), &second);
}

#[test]
fn erro_de_compilacao_recusa_com_a_lista_e_nao_grava() {
    let (_project, state, updates, _first) = opened("artigo-basico");
    append(&state, "\n#variavel_que_nao_existe\n");
    let bad = next(&updates, false);
    assert!(
        bad.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error),
        "{:?}",
        bad.diagnostics
    );

    let out = tempfile::tempdir().unwrap();
    let path = out.path().join("artigo.pdf");
    match state.export_pdf(&path) {
        Err(ExportError::CompileErrors(diags)) => assert_eq!(diags, bad.diagnostics, "FR-029"),
        other => panic!("esperava compile_errors, veio {other:?}"),
    }
    assert!(names(out.path()).is_empty(), "nada gravado");
}

#[test]
fn versao_diferente_recusa() {
    let project = TempProject::from_fixture("artigo-basico");
    project.write(
        ".prelo/project.json",
        json!({ "schema": 1, "compiler": { "typst": "0.14.0" }, "main": "main.typ" }).to_string(),
    );
    let state = AppState::default();
    state.open_project(&project.root()).expect("abrir projeto");
    let out = tempfile::tempdir().unwrap();
    assert_eq!(
        state.export_pdf(&out.path().join("a.pdf")).unwrap_err(),
        ExportError::VersionMismatch
    );
    assert!(names(out.path()).is_empty());
}

#[test]
fn sem_projeto_recusa() {
    let state = AppState::default();
    let out = tempfile::tempdir().unwrap();
    assert_eq!(
        state.export_pdf(&out.path().join("a.pdf")).unwrap_err(),
        ExportError::NoProject
    );
}

#[test]
fn falha_de_gravacao_nao_deixa_arquivo_parcial() {
    let (_project, state, _updates, _first) = opened("artigo-basico");
    let out = tempfile::tempdir().unwrap();

    // Destino é uma pasta: o `rename` final falha, e o temporário some.
    let dir_target = out.path().join("pasta.pdf");
    std::fs::create_dir(&dir_target).unwrap();
    assert_eq!(
        state.export_pdf(&dir_target).unwrap_err(),
        ExportError::WriteFailed
    );
    assert_eq!(names(out.path()), ["pasta.pdf"], "sem temporário");
    assert!(dir_target.is_dir());

    // Pasta inexistente.
    assert_eq!(
        state
            .export_pdf(&out.path().join("nao-existe").join("a.pdf"))
            .unwrap_err(),
        ExportError::WriteFailed
    );
    assert_eq!(names(out.path()), ["pasta.pdf"]);
}

#[cfg(unix)]
#[test]
fn falha_de_gravacao_mantem_o_pdf_anterior() {
    use std::os::unix::fs::PermissionsExt;

    let (_project, state, _updates, _first) = opened("artigo-basico");
    let out = tempfile::tempdir().unwrap();
    let path = out.path().join("artigo.pdf");
    std::fs::write(&path, b"PDF anterior").unwrap();
    std::fs::set_permissions(out.path(), std::fs::Permissions::from_mode(0o555)).unwrap();
    let result = state.export_pdf(&path);
    std::fs::set_permissions(out.path(), std::fs::Permissions::from_mode(0o755)).unwrap();

    assert_eq!(result.unwrap_err(), ExportError::WriteFailed);
    assert_eq!(std::fs::read(&path).unwrap(), b"PDF anterior");
    assert_eq!(names(out.path()), ["artigo.pdf"]);
}

#[test]
fn erro_serializa_no_formato_do_contrato() {
    assert_eq!(
        serde_json::to_value(ExportError::NoProject).unwrap(),
        json!("no_project")
    );
    assert_eq!(
        serde_json::to_value(ExportError::VersionMismatch).unwrap(),
        json!("compiler_version_mismatch")
    );
    assert_eq!(
        serde_json::to_value(ExportError::WriteFailed).unwrap(),
        json!("write_failed")
    );
    let diag = prelo_lib::diagnostic::Diagnostic::error(prelo_lib::diagnostic::Kind::Typst, "x");
    assert_eq!(
        serde_json::to_value(ExportError::CompileErrors(vec![diag])).unwrap(),
        json!({
            "code": "compile_errors",
            "diagnostics": [{ "severity": "error", "message": "x", "hints": [], "kind": "typst" }]
        })
    );
}

// ---------------------------------------------------------------- IPC

#[test]
fn ipc_recusa_antes_do_dialogo() {
    // Sem projeto.
    let (_app, window) = app(AppState::default());
    assert_eq!(
        invoke(&window, "export_pdf_dialog", json!({})),
        Err(json!("no_project"))
    );
    // Argumentos extras.
    assert_eq!(
        invoke(
            &window,
            "export_pdf_dialog",
            json!({ "path": "/tmp/x.pdf" })
        ),
        Err(json!("invalid_arguments"))
    );
}

#[test]
fn ipc_com_erro_de_compilacao_responde_compile_errors() {
    let (_project, state, updates, _first) = opened("artigo-basico");
    append(&state, "\n#variavel_que_nao_existe\n");
    let bad = next(&updates, false);
    let (_app, window) = app(state);
    let err = invoke(&window, "export_pdf_dialog", json!({})).unwrap_err();
    assert_eq!(err["code"], json!("compile_errors"));
    assert_eq!(
        err["diagnostics"],
        serde_json::to_value(&bad.diagnostics).unwrap()
    );
}

#[test]
fn ipc_com_versao_diferente_responde_compiler_version_mismatch() {
    let project = TempProject::from_fixture("artigo-basico");
    project.write(
        ".prelo/project.json",
        json!({ "schema": 1, "compiler": { "typst": "0.14.0" }, "main": "main.typ" }).to_string(),
    );
    let state = AppState::default();
    state.open_project(&project.root()).expect("abrir projeto");
    let (_app, window) = app(state);
    assert_eq!(
        invoke(&window, "export_pdf_dialog", json!({})),
        Err(json!("compiler_version_mismatch"))
    );
}
