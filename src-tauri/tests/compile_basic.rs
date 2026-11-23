//! Testes de compilação do projeto aberto (T024; FR-013, FR-014; contracts/ipc.md
//! "Compilação e prévia"; data-model.md "Compilation", "Diagnostic", "ProjectRecord").
//!
//! Pelo estado do núcleo (`AppState`), como os comandos IPC o usam: abrir o projeto compila o
//! arquivo principal com o compilador embutido e entrega um `PreviewUpdate` ao assinante;
//! `edit_file` (texto ainda não salvo) recompila e muda a prévia; erro vira diagnóstico com
//! arquivo e intervalo, e a prévia mantém a última compilação `Ok`.

mod support;

use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use prelo_lib::commands::{AppState, Change};
use prelo_lib::compile::preview::PreviewUpdate;
use serde_json::{json, Value};
use support::TempProject;
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{
    get_ipc_response, mock_builder, mock_context, noop_assets, MockRuntime, INVOKE_KEY,
};
use tauri::webview::InvokeRequest;
use tauri::{Manager, WebviewWindow, WebviewWindowBuilder};

/// Folga para uma compilação em build de depuração (a primeira carrega as fontes).
const WAIT: Duration = Duration::from_secs(60);

struct Session {
    state: AppState,
    updates: Receiver<PreviewUpdate>,
    project: TempProject,
}

impl Session {
    /// Assina a prévia e abre o projeto.
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

    fn fixture(name: &str) -> Self {
        Self::open(TempProject::from_fixture(name))
    }

    /// Próximo `PreviewUpdate` que satisfaz `pred` (os anteriores são descartados).
    fn wait_for(&self, what: &str, pred: impl Fn(&PreviewUpdate) -> bool) -> PreviewUpdate {
        let deadline = Instant::now() + WAIT;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let update = self
                .updates
                .recv_timeout(left)
                .unwrap_or_else(|_| panic!("nenhum PreviewUpdate {what} no prazo"));
            if pred(&update) {
                return update;
            }
        }
    }

    fn next_ok(&self) -> PreviewUpdate {
        self.wait_for("ok", |u| u.ok)
    }

    fn next_error(&self) -> PreviewUpdate {
        self.wait_for("com erro", |u| !u.ok)
    }

    /// Abre `path` e acrescenta `insert` em `at` (unidades UTF-16); devolve o texto novo.
    fn insert(&self, path: &str, at: Option<usize>, insert: &str) -> String {
        let opened = self.state.open_file(path).expect("open_file");
        let at = at.unwrap_or_else(|| utf16_len(&opened.text));
        self.state
            .edit_file(
                path,
                opened.version,
                &[Change {
                    from: at,
                    to: at,
                    insert: insert.into(),
                }],
            )
            .expect("edit_file");
        self.state.open_file(path).expect("open_file").text
    }

    /// Troca o texto inteiro de `path`.
    fn replace_all(&self, path: &str, text: &str) {
        let opened = self.state.open_file(path).expect("open_file");
        self.state
            .edit_file(
                path,
                opened.version,
                &[Change {
                    from: 0,
                    to: utf16_len(&opened.text),
                    insert: text.into(),
                }],
            )
            .expect("edit_file");
    }
}

fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// Trecho de `text` no intervalo `from..to` em unidades UTF-16.
fn utf16_slice(text: &str, from: usize, to: usize) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    String::from_utf16(&units[from..to]).expect("intervalo em limite de caractere")
}

fn all_pages(update: &PreviewUpdate) -> Vec<usize> {
    (0..update.pages.len()).collect()
}

#[test]
fn abrir_projeto_compila_o_arquivo_principal() {
    let s = Session::fixture("artigo-basico");
    let update = s.next_ok();
    assert!(update.compile_id > 0);
    assert!(update.pages.len() >= 2, "{:?}", update.pages);
    assert_eq!(update.changed, all_pages(&update), "primeira prévia: todas");
    assert!(update.diagnostics.is_empty(), "{:?}", update.diagnostics);
    for (i, page) in update.pages.iter().enumerate() {
        assert_eq!(page.index, i);
        // A4 em pontos.
        assert!((page.width_pt - 595.28).abs() < 0.1, "{page:?}");
        assert!((page.height_pt - 841.89).abs() < 0.1, "{page:?}");
    }
    let svg = s
        .state
        .page_svg(0, &update.pages[0].hash)
        .expect("SVG da página 1");
    assert!(svg.contains("<svg"), "{svg:.80}");
}

#[test]
fn assinar_depois_de_compilar_recebe_a_previa_atual() {
    let project = TempProject::from_fixture("artigo-basico");
    let state = AppState::default();
    state.open_project(&project.root()).expect("abrir projeto");
    // Espera a compilação terminar sem assinante: o SVG passa a existir.
    let deadline = Instant::now() + WAIT;
    let (tx, rx) = mpsc::channel();
    state.subscribe_preview(move |u: &PreviewUpdate| {
        let _ = tx.send(u.clone());
    });
    let update = loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let u: PreviewUpdate = rx.recv_timeout(left).expect("PreviewUpdate no prazo");
        if u.ok {
            break u;
        }
    };
    assert_eq!(
        update.changed,
        all_pages(&update),
        "quem assina recebe todas"
    );
}

#[test]
fn edicao_nao_salva_muda_a_previa() {
    let s = Session::fixture("artigo-basico");
    let first = s.next_ok();
    let last = first.pages.len() - 1;

    // Acrescenta texto no fim do documento: só a(s) última(s) página(s) muda(m).
    s.insert("main.typ", None, "\nTexto acrescentado pela edição.\n");
    let second = s.next_ok();
    assert!(second.compile_id > first.compile_id);
    assert!(!second.changed.is_empty(), "a edição muda a prévia");
    assert!(
        second.changed.iter().all(|&i| i >= last),
        "só a última página muda: {:?}",
        second.changed
    );
    assert_eq!(second.pages[0].hash, first.pages[0].hash);
    assert_ne!(second.pages[last].hash, first.pages[last].hash);

    // Hash velho da página alterada: `stale_page`; o novo devolve o SVG.
    assert_eq!(
        s.state.page_svg(last, &first.pages[last].hash),
        Err("stale_page".to_string())
    );
    assert!(s.state.page_svg(last, &second.pages[last].hash).is_ok());

    // Nada foi gravado no disco.
    let disk = std::fs::read_to_string(s.project.path("main.typ")).unwrap();
    assert!(!disk.contains("acrescentado"));
}

#[test]
fn edicao_em_arquivo_incluido_recompila() {
    let s = Session::fixture("artigo-basico");
    let first = s.next_ok();
    s.insert(
        "capitulos/metodo.typ",
        Some(0),
        "Frase nova no começo do capítulo incluído.\n\n",
    );
    let second = s.next_ok();
    assert!(!second.changed.is_empty());
    assert_ne!(
        first.pages.iter().map(|p| &p.hash).collect::<Vec<_>>(),
        second.pages.iter().map(|p| &p.hash).collect::<Vec<_>>()
    );
}

#[test]
fn erro_gera_diagnostico_com_arquivo_e_intervalo_e_mantem_a_ultima_ok() {
    let s = Session::fixture("artigo-basico");
    let good = s.next_ok();

    // Linha 5 de metodo.typ tem acentos antes do erro: o intervalo é em unidades UTF-16, as
    // mesmas de `edit_file` e do editor.
    let original = s.state.open_file("capitulos/metodo.typ").unwrap().text;
    let anchor = "vindo de capitulos/metodo.typ.";
    let at = utf16_len(&original[..original.find(anchor).unwrap() + anchor.len()]);
    let text = s.insert("capitulos/metodo.typ", Some(at), " #nao_existe");

    let bad = s.next_error();
    assert_eq!(bad.pages, good.pages, "prévia mantém a última Ok (FR-014)");
    assert!(bad.changed.is_empty());
    let diag = bad
        .diagnostics
        .iter()
        .find(|d| d.message.contains("nao_existe"))
        .unwrap_or_else(|| panic!("sem diagnóstico do erro: {:?}", bad.diagnostics));
    assert_eq!(diag.path.as_deref(), Some("capitulos/metodo.typ"));
    assert_eq!(diag.line, Some(5));
    let range = diag.range.clone().expect("intervalo");
    assert_eq!(utf16_slice(&text, range.start, range.end), "nao_existe");

    // As páginas da última Ok continuam servidas.
    assert!(s.state.page_svg(0, &good.pages[0].hash).is_ok());

    // Corrigido, volta a compilar.
    s.replace_all("capitulos/metodo.typ", &original);
    let fixed = s.next_ok();
    assert!(fixed.diagnostics.is_empty(), "{:?}", fixed.diagnostics);
    assert_eq!(
        fixed.pages.iter().map(|p| &p.hash).collect::<Vec<_>>(),
        good.pages.iter().map(|p| &p.hash).collect::<Vec<_>>()
    );
}

#[test]
fn escolher_principal_grava_o_registro_e_compila_o_escolhido() {
    let project = TempProject::empty();
    project.write("a.typ", "Arquivo A\n");
    project.write(
        "b.typ",
        "#set page(width: 120pt, height: 60pt)\nArquivo B\n",
    );
    let s = Session::open(project);

    // Sem `main.typ`, nada compila.
    let missing = s.next_error();
    assert!(missing.pages.is_empty());
    assert!(!missing.diagnostics.is_empty());

    let info = s.state.choose_main_file("b.typ").expect("choose_main_file");
    assert_eq!(info.main_file, "b.typ");
    let record = std::fs::read_to_string(s.project.path(".prelo/project.json")).unwrap();
    let record: Value = serde_json::from_str(&record).unwrap();
    assert_eq!(record["main"], "b.typ");

    let update = s.next_ok();
    assert_eq!(update.pages.len(), 1);
    assert!((update.pages[0].width_pt - 120.0).abs() < 0.01);
}

// Parte de projeto de T024 (T032): vários `.typ` e nenhum `main.typ` → aviso
// `main_file_missing` em `ProjectInfo` (a interface pede a escolha) e a prévia só traz o
// diagnóstico de arquivo ausente; a escolha
// grava `ProjectRecord.main` e compila o principal escolhido.
#[test]
fn varios_typ_sem_main_pedem_escolha() {
    let project = TempProject::empty();
    project.write("a.typ", "A\n");
    project.write("b.typ", "B\n");
    let state = AppState::default();
    let (tx, updates) = mpsc::channel();
    state.subscribe_preview(move |update: &PreviewUpdate| {
        let _ = tx.send(update.clone());
    });
    let info = state.open_project(&project.root()).expect("abrir projeto");
    let s = Session {
        state,
        updates,
        project,
    };
    assert_eq!(info.warnings, ["main_file_missing"], "{:?}", info.warnings);
    let missing = s.next_error();
    assert!(missing.pages.is_empty());
    let record: Value =
        serde_json::from_slice(&std::fs::read(s.project.path(".prelo/project.json")).unwrap())
            .unwrap();
    assert!(record.get("main").is_none(), "{record}");

    let info = s.state.choose_main_file("b.typ").expect("choose_main_file");
    assert!(info.warnings.is_empty(), "{:?}", info.warnings);
    assert_eq!(info.main_file, "b.typ");
    let record: Value =
        serde_json::from_slice(&std::fs::read(s.project.path(".prelo/project.json")).unwrap())
            .unwrap();
    assert_eq!(record["main"], "b.typ");
    s.next_ok();

    // Reaberto, o principal registrado vale e não há aviso.
    let info = AppState::default()
        .open_project(&s.project.root())
        .expect("reabrir");
    assert!(info.warnings.is_empty(), "{:?}", info.warnings);
    assert_eq!(info.main_file, "b.typ");
}

// ---------------------------------------------------------------- IPC

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
fn comandos_de_previa_pelo_ipc() {
    let app = prelo_lib::configure_with_state(mock_builder(), AppState::default())
        .build(mock_context(noop_assets()))
        .expect("app de teste não montou");
    let window = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("janela de teste não abriu");
    let project = TempProject::from_fixture("artigo-basico");
    let state = app.state::<AppState>();
    let (tx, rx) = mpsc::channel();
    state.subscribe_preview(move |u: &PreviewUpdate| {
        let _ = tx.send(u.clone());
    });
    state.open_project(&project.root()).expect("abrir projeto");
    let update = loop {
        let u: PreviewUpdate = rx.recv_timeout(WAIT).expect("PreviewUpdate no prazo");
        if u.ok {
            break u;
        }
    };

    let hash = update.pages[0].hash.clone();
    let svg = invoke(&window, "page_svg", json!({ "index": 0, "hash": hash })).expect("page_svg");
    assert!(svg.as_str().expect("SVG em texto").contains("<svg"));
    assert_eq!(
        invoke(&window, "page_svg", json!({ "index": 0, "hash": "0" })),
        Err(json!("stale_page"))
    );
    for bad in [
        json!({ "index": 0 }),
        json!({ "index": -1, "hash": hash }),
        json!({ "index": 0, "hash": hash, "path": "/etc/passwd" }),
        json!("x"),
    ] {
        assert_eq!(
            invoke(&window, "page_svg", bad.clone()),
            Err(json!("invalid_arguments")),
            "{bad}"
        );
    }
    // `subscribe_preview` só aceita o canal.
    for bad in [
        json!({}),
        json!({ "channel": "__CHANNEL__:7", "path": "main.typ" }),
    ] {
        assert!(
            invoke(&window, "subscribe_preview", bad.clone()).is_err(),
            "{bad}"
        );
    }
    assert_eq!(
        invoke(
            &window,
            "subscribe_preview",
            json!({ "channel": "__CHANNEL__:7" })
        ),
        Ok(Value::Null)
    );
}
