//! Testes do "Compilar" forçado (T100; FR-014a, cenário 14 de US1; contracts/ipc.md
//! `compile_now`).
//!
//! - figura, `.bib` e `.typ` incluído, não abertos no editor, trocados no disco **sem** aviso
//!   do observador (que pode ter perdido o evento) aparecem na compilação forçada;
//! - o texto não salvo do editor entra na compilação forçada;
//! - sem projeto: `no_project`; versão registrada diferente: `compiler_version_mismatch`, sem
//!   `PreviewUpdate`;
//! - plugin recusado continua recusado, sem novo pedido de confirmação (FR-019);
//! - pelo IPC, o comando só aceita corpo vazio.
//!
//! O abandono de compilação presa e o limite de threads (T078) são testados na agenda
//! (`src/compile/scheduler.rs`, módulo `tests`), com um compilador falso.

mod support;

use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};

use prelo_lib::commands::{AppState, Change};
use prelo_lib::compile::preview::PreviewUpdate;
use prelo_lib::world::plugins::PluginRequest;
use serde_json::{json, Value};
use support::{fixtures_dir, TempProject};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{
    get_ipc_response, mock_builder, mock_context, noop_assets, MockRuntime, INVOKE_KEY,
};
use tauri::webview::InvokeRequest;
use tauri::{App, WebviewWindow, WebviewWindowBuilder};
use typst::layout::{Frame, FrameItem};
use typst_layout::PagedDocument;

/// Folga para uma compilação em build de depuração (a primeira carrega as fontes).
const WAIT: Duration = Duration::from_secs(60);
/// Quanto esperar para concluir que algo não aconteceu.
const QUIET: Duration = Duration::from_secs(2);

const BIB: &str = "@article{silva2020,\n  author = {Silva, Ana},\n  title = {Edição colaborativa},\n  journal = {Revista de Testes},\n  year = {2020},\n}\n";

fn svg(height: u32) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10\" height=\"{height}\"><rect width=\"10\" height=\"{height}\" fill=\"red\"/></svg>"
    )
}

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

fn document_text(doc: &PagedDocument) -> String {
    let mut out = String::new();
    for page in doc.pages() {
        collect(&page.frame, &mut out);
    }
    out
}

struct Session {
    state: AppState,
    updates: Receiver<PreviewUpdate>,
    project: TempProject,
}

impl Session {
    fn open(project: TempProject) -> Self {
        let state = AppState::default();
        Self::open_with(state, project)
    }

    fn open_with(state: AppState, project: TempProject) -> Self {
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

    /// Próxima prévia que satisfaz `pred`.
    fn wait(&self, what: &str, pred: impl Fn(&PreviewUpdate) -> bool) -> PreviewUpdate {
        let deadline = Instant::now() + WAIT;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let update = self
                .updates
                .recv_timeout(left)
                .unwrap_or_else(|_| panic!("nenhuma prévia {what} no prazo"));
            if pred(&update) {
                return update;
            }
        }
    }

    /// Prévia da compilação `id` (ou de uma mais nova).
    fn wait_compile(&self, id: u64) -> PreviewUpdate {
        self.wait(&format!("da compilação {id}"), |u| u.compile_id >= id)
    }

    /// Texto da última prévia `Ok`.
    fn text(&self) -> String {
        let (source, _, _) = self.state.export_source().expect("prévia ok");
        document_text(&source.document)
    }

    /// Nenhuma prévia chega por `QUIET`.
    fn assert_quiet(&self) {
        if let Ok(u) = self.updates.recv_timeout(QUIET) {
            panic!("prévia inesperada: {u:?}");
        }
    }
}

#[test]
fn compilar_rele_do_disco_figura_bib_e_incluido_nao_abertos_sem_aviso_do_observador() {
    let project = TempProject::empty();
    project.write("fig.svg", svg(10));
    project.write("refs.bib", BIB);
    project.write("cap.typ", "Capítulo antigo.\n");
    project.write(
        "main.typ",
        "#set page(width: 8cm, height: auto, margin: 5pt)\n#image(\"fig.svg\", width: 2cm)\n\n#include \"cap.typ\"\n\nSegundo @silva2020.\n\n#bibliography(\"refs.bib\")\n",
    );
    let s = Session::open(project);
    let before = s.wait("inicial", |u| u.ok);
    let text = s.text();
    assert!(text.contains("colaborativa"), "{text}");
    assert!(text.contains("antigo"), "{text}");

    // Outro programa troca os três arquivos; o observador não avisa (evento perdido, sistema
    // de arquivos de rede). Só o "Compilar" relê.
    std::fs::write(s.project.path("fig.svg"), svg(80)).unwrap();
    std::fs::write(
        s.project.path("refs.bib"),
        BIB.replace("Edição colaborativa", "Escrita conjunta"),
    )
    .unwrap();
    std::fs::write(s.project.path("cap.typ"), "Capítulo novo.\n").unwrap();

    let id = s.state.compile_now().expect("compile_now");
    let after = s.wait_compile(id);
    assert!(after.ok, "{:?}", after.diagnostics);
    assert_eq!(after.compile_id, id);
    assert!(
        after.pages[0].height_pt > before.pages[0].height_pt,
        "a página acompanha a figura nova: {} → {}",
        before.pages[0].height_pt,
        after.pages[0].height_pt
    );
    let text = s.text();
    assert!(
        text.contains("conjunta") && !text.contains("colaborativa"),
        "{text}"
    );
    assert!(text.contains("novo") && !text.contains("antigo"), "{text}");
}

#[test]
fn compilar_usa_o_texto_nao_salvo_do_editor() {
    let project = TempProject::empty();
    project.write("main.typ", "Texto do disco.\n");
    let s = Session::open(project);
    s.wait("inicial", |u| u.ok);
    let opened = s.state.open_file("main.typ").expect("open_file");
    let units = opened.text.encode_utf16().count();
    s.state
        .edit_file(
            "main.typ",
            opened.version,
            &[Change {
                from: 0,
                to: units,
                insert: "Texto do editor.\n".into(),
            }],
        )
        .expect("edit_file");

    let id = s.state.compile_now().expect("compile_now");
    let u = s.wait_compile(id);
    assert!(u.ok, "{:?}", u.diagnostics);
    assert!(s.text().contains("editor"), "{}", s.text());
    let disk = std::fs::read_to_string(s.project.path("main.typ")).unwrap();
    assert_eq!(disk, "Texto do disco.\n", "compilar não grava o arquivo");
}

#[test]
fn compilar_sem_projeto_e_no_project() {
    assert_eq!(AppState::default().compile_now(), Err("no_project".into()));
}

#[test]
fn compilar_com_versao_diferente_e_recusado_sem_previa() {
    let project = TempProject::from_fixture("artigo-basico");
    project.write(
        ".prelo/project.json",
        json!({ "schema": 1, "compiler": { "typst": "0.12.0" }, "main": "main.typ" }).to_string(),
    );
    let s = Session::open(project);
    assert_eq!(
        s.state.compile_now(),
        Err("compiler_version_mismatch".into())
    );
    s.assert_quiet();
}

#[test]
fn compilar_mantem_o_plugin_recusado_sem_pedir_de_novo() {
    let data = tempfile::Builder::new()
        .prefix("prelo-dados-")
        .tempdir()
        .unwrap();
    let state = AppState::default();
    state.trust().set_data_dir(data.path().to_path_buf());
    let (req_tx, requests) = mpsc::channel::<PluginRequest>();
    state
        .trust()
        .set_listener(Arc::new(move |r: &PluginRequest| {
            let _ = req_tx.send(r.clone());
        }));
    let project = TempProject::empty();
    project.write(
        "main.typ",
        "#let p = plugin(\"teste.wasm\")\nPlugin diz: #str(p.hello())\n",
    );
    project.write(
        "teste.wasm",
        std::fs::read(fixtures_dir().join("plugin/teste.wasm")).unwrap(),
    );
    let s = Session::open_with(state, project);

    let request = requests.recv_timeout(WAIT).expect("pedido de confirmação");
    s.state
        .decide_plugin(&request.sha256, false)
        .expect("recusar");
    let refused = |u: &PreviewUpdate| {
        !u.ok
            && u.diagnostics
                .iter()
                .any(|d| d.message.contains("recusado neste projeto"))
    };
    s.wait("com o plugin recusado", refused);

    let id = s.state.compile_now().expect("compile_now");
    let u = s.wait_compile(id);
    assert!(refused(&u), "{:?}", u.diagnostics);
    assert!(
        requests.recv_timeout(QUIET).is_err(),
        "compilar não pode pedir confirmação de novo"
    );
}

// ------------------------------------------------------------------------------- IPC

fn app() -> (App<MockRuntime>, WebviewWindow<MockRuntime>) {
    let app = prelo_lib::configure(mock_builder())
        .build(mock_context(noop_assets()))
        .expect("app de teste não montou");
    let window = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("janela de teste não abriu");
    (app, window)
}

fn invoke(window: &WebviewWindow<MockRuntime>, body: InvokeBody) -> Result<Value, Value> {
    let url = if cfg!(windows) {
        "http://tauri.localhost"
    } else {
        "tauri://localhost"
    };
    get_ipc_response(
        window,
        InvokeRequest {
            cmd: "compile_now".into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: url.parse().unwrap(),
            body,
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .map(|b| b.deserialize::<Value>().expect("resposta não é JSON"))
}

#[test]
fn pelo_ipc_corpo_vazio_sem_projeto_e_no_project_e_argumento_e_recusado() {
    let (_app, window) = app();
    assert_eq!(
        invoke(&window, InvokeBody::Json(json!({}))),
        Err(json!("no_project"))
    );
    assert_eq!(
        invoke(&window, InvokeBody::Json(Value::Null)),
        Err(json!("no_project"))
    );
    for body in [
        InvokeBody::Json(json!({ "path": "main.typ" })),
        InvokeBody::Json(json!({ "force": true })),
        InvokeBody::Json(json!("main.typ")),
        InvokeBody::Json(json!([])),
        InvokeBody::Raw(vec![1, 2, 3]),
    ] {
        let shown = format!("{body:?}");
        assert_eq!(
            invoke(&window, body),
            Err(json!("invalid_arguments")),
            "aceitou {shown}"
        );
    }
}
