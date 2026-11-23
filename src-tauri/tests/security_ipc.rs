//! Testes de segurança do IPC (T015; FR-021, FR-022, SC-003; contracts/ipc.md).
//!
//! Pelo mock runtime do Tauri, como a webview chamaria:
//! - todo comando do contrato que recebe `path` recusa caminho absoluto, `..` além da raiz e
//!   link simbólico para fora com `path_outside_project`, sem nunca entregar o caminho ao
//!   motor (World, serviço de linguagem) nem devolver byte de fora do projeto;
//! - caminhos válidos (inclusive `..` dentro da raiz e link interno) chegam ao motor já
//!   normalizados e relativos à raiz;
//! - argumentos malformados são recusados sem derrubar o núcleo;
//! - comando desconhecido é recusado.

mod support;

use std::sync::{Arc, Mutex};

use prelo_lib::commands::{AppState, Engine, EngineContext, JumpTarget, ProjectPath};
use prelo_lib::lang_service::{Completion, Token};
use serde_json::{json, Value};
use support::{Outside, TempProject, SECRET};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{
    get_ipc_response, mock_builder, mock_context, noop_assets, MockRuntime, INVOKE_KEY,
};
use tauri::webview::InvokeRequest;
use tauri::{App, Manager, WebviewWindow, WebviewWindowBuilder};

/// Motor falso: registra cada chamada (comando, caminho relativo recebido).
#[derive(Default, Clone)]
struct FakeEngine {
    calls: Arc<Mutex<Vec<(String, String)>>>,
}

impl FakeEngine {
    fn calls(&self) -> Vec<(String, String)> {
        self.calls.lock().unwrap().clone()
    }

    fn record(&self, cmd: &str, path: &ProjectPath) {
        self.calls
            .lock()
            .unwrap()
            .push((cmd.into(), path.relative().into()));
    }
}

impl Engine for FakeEngine {
    fn complete(
        &self,
        _ctx: &EngineContext,
        path: &ProjectPath,
        _offset: usize,
    ) -> Result<Vec<Completion>, String> {
        self.record("complete", path);
        Ok(vec![Completion {
            label: "falso".into(),
            kind: None,
            detail: None,
            info: None,
            apply: None,
        }])
    }

    fn semantic_tokens(
        &self,
        _ctx: &EngineContext,
        path: &ProjectPath,
    ) -> Result<Vec<Token>, String> {
        self.record("semantic_tokens", path);
        Ok(Vec::new())
    }

    fn jump_from_cursor(
        &self,
        _ctx: &EngineContext,
        path: &ProjectPath,
        _offset: usize,
    ) -> Result<Vec<JumpTarget>, String> {
        self.record("jump_from_cursor", path);
        Ok(Vec::new())
    }
}

struct Harness {
    _app: App<MockRuntime>,
    window: WebviewWindow<MockRuntime>,
    engine: FakeEngine,
    project: TempProject,
    outside: Outside,
}

impl Harness {
    /// App com o motor falso e `artigo-basico` aberto como projeto, mais links de teste.
    fn new() -> Self {
        let engine = FakeEngine::default();
        let app = prelo_lib::configure_with_state(
            mock_builder(),
            AppState::new(Arc::new(engine.clone())),
        )
        .build(mock_context(noop_assets()))
        .expect("app de teste não montou");
        let window = WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .expect("janela de teste não abriu");
        let project = TempProject::from_fixture("artigo-basico");
        let outside = Outside::new();
        app.state::<AppState>()
            .open_project(&project.root())
            .expect("abrir projeto de teste");
        Self {
            _app: app,
            window,
            engine,
            project,
            outside,
        }
    }

    fn invoke(&self, cmd: &str, body: InvokeBody) -> Result<Value, Value> {
        invoke(&self.window, cmd, body)
    }

    fn call(&self, cmd: &str, args: Value) -> Result<Value, Value> {
        self.invoke(cmd, InvokeBody::Json(args))
    }

    /// Links para fora (arquivo e pasta) e um link interno. No Windows, criar link pode exigir
    /// modo de desenvolvedor: sem permissão, os casos de link ficam de fora só lá.
    fn links(&self) -> Option<(Vec<String>, String)> {
        let made = support::symlink(&self.outside.secret(), &self.project.path("link-fora.typ"))
            .and_then(|_| support::symlink(&self.outside.root(), &self.project.path("pasta-fora")))
            .and_then(|_| {
                support::symlink(
                    &self.project.path("main.typ"),
                    &self.project.path("atalho.typ"),
                )
            });
        match made {
            Ok(()) => Some((
                vec!["link-fora.typ".into(), "pasta-fora/segredo.txt".into()],
                "atalho.typ".into(),
            )),
            Err(e) if cfg!(windows) => {
                eprintln!("links simbólicos indisponíveis neste Windows: {e}");
                None
            }
            Err(e) => panic!("criar link simbólico: {e}"),
        }
    }

    /// O núcleo segue respondendo depois de tudo.
    fn still_alive(&self) {
        let info = self
            .invoke("get_app_info", InvokeBody::default())
            .expect("get_app_info falhou: o núcleo caiu?");
        assert!(info["version"].is_string());
        let opened = self
            .call("open_file", json!({ "path": "main.typ" }))
            .expect("open_file de caminho válido falhou depois de entrada ruim");
        assert!(opened["text"].is_string());
    }
}

fn invoke(
    window: &WebviewWindow<MockRuntime>,
    cmd: &str,
    body: InvokeBody,
) -> Result<Value, Value> {
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
            body,
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .map(|b| b.deserialize::<Value>().expect("resposta não é JSON"))
}

/// Comandos do contrato que recebem `path`, com o restante dos argumentos válidos.
fn path_commands(path: &str) -> Vec<(&'static str, Value)> {
    vec![
        ("choose_main_file", json!({ "path": path })),
        ("open_file", json!({ "path": path })),
        (
            "edit_file",
            json!({ "path": path, "version": 0, "changes": [{ "from": 0, "to": 0, "insert": "x" }] }),
        ),
        ("save_file", json!({ "path": path })),
        (
            "resolve_external_change",
            json!({ "path": path, "keep": "mine" }),
        ),
        ("complete", json!({ "path": path, "offset": 0 })),
        ("semantic_tokens", json!({ "path": path })),
        ("jump_from_cursor", json!({ "path": path, "offset": 0 })),
        ("create_file", json!({ "path": path })),
        ("create_dir", json!({ "path": path })),
    ]
}

fn absolute_paths(h: &Harness) -> Vec<String> {
    let mut paths = vec![
        // Absoluto do sistema, inclusive o caminho absoluto de um arquivo *dentro* do projeto:
        // a interface só fala em caminho relativo à raiz.
        h.project.path("main.typ").to_string_lossy().into_owned(),
        h.outside.secret().to_string_lossy().into_owned(),
        "/etc/passwd".into(),
        "/main.typ".into(),
        "\\main.typ".into(),
        "//servidor/compartilhado/x.typ".into(),
        "\\\\servidor\\compartilhado\\x.typ".into(),
        "C:\\Windows\\win.ini".into(),
        "C:main.typ".into(),
        "c:/main.typ".into(),
    ];
    paths.sort();
    paths.dedup();
    paths
}

fn dotdot_paths(h: &Harness) -> Vec<String> {
    let outside_name = h
        .outside
        .root()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let mut paths = vec![
        "../fora.typ".to_string(),
        "capitulos/../../fora.typ".into(),
        "../../../../../../../../etc/passwd".into(),
        "./../fora.typ".into(),
        // Alvo que existe de fato: a recusa não pode depender de o alvo faltar.
        format!("../{outside_name}/segredo.txt"),
    ];
    if cfg!(windows) {
        paths.push("..\\fora.typ".into());
        paths.push("capitulos\\..\\..\\fora.typ".into());
    }
    paths
}

fn assert_refused_outside(h: &Harness, path: &str) {
    for (cmd, args) in path_commands(path) {
        let res = h.call(cmd, args);
        assert_eq!(
            res,
            Err(json!("path_outside_project")),
            "{cmd} com {path:?} deveria ser recusado"
        );
    }
}

fn assert_no_secret(value: &Result<Value, Value>) {
    let text = match value {
        Ok(v) | Err(v) => v.to_string(),
    };
    assert!(!text.contains(SECRET), "conteúdo de fora vazou: {text}");
}

#[test]
fn caminho_absoluto_e_recusado_em_todo_comando_com_path() {
    let h = Harness::new();
    for path in absolute_paths(&h) {
        assert_refused_outside(&h, &path);
    }
    assert!(
        h.engine.calls().is_empty(),
        "motor recebeu caminho recusado"
    );
    h.still_alive();
}

#[test]
fn ponto_ponto_alem_da_raiz_e_recusado_em_todo_comando_com_path() {
    let h = Harness::new();
    for path in dotdot_paths(&h) {
        assert_refused_outside(&h, &path);
    }
    assert!(
        h.engine.calls().is_empty(),
        "motor recebeu caminho recusado"
    );
    h.still_alive();
}

#[test]
fn link_para_fora_e_recusado_em_todo_comando_com_path() {
    let h = Harness::new();
    let Some((outward, _inward)) = h.links() else {
        return;
    };
    for path in &outward {
        assert_refused_outside(&h, path);
        for (cmd, args) in path_commands(path) {
            assert_no_secret(&h.call(cmd, args));
        }
    }
    assert!(
        h.engine.calls().is_empty(),
        "motor recebeu caminho recusado"
    );
    h.still_alive();
}

#[test]
fn caminhos_validos_chegam_ao_motor_normalizados() {
    let h = Harness::new();
    let mut valid = vec![
        ("main.typ", "main.typ"),
        ("capitulos/../main.typ", "main.typ"),
        ("./capitulos/metodo.typ", "capitulos/metodo.typ"),
    ];
    let links = h.links();
    if let Some((_, inward)) = &links {
        valid.push((inward.as_str(), "main.typ"));
    }
    for (requested, normalized) in &valid {
        let opened = h
            .call("open_file", json!({ "path": requested }))
            .unwrap_or_else(|e| panic!("open_file {requested:?}: {e}"));
        assert!(opened["text"].is_string());
        assert!(opened["version"].is_u64());
        for cmd in ["complete", "jump_from_cursor"] {
            h.call(cmd, json!({ "path": requested, "offset": 0 }))
                .unwrap_or_else(|e| panic!("{cmd} {requested:?}: {e}"));
        }
        h.call("semantic_tokens", json!({ "path": requested }))
            .unwrap_or_else(|e| panic!("semantic_tokens {requested:?}: {e}"));
        let calls = h.engine.calls();
        let last: Vec<_> = calls[calls.len() - 3..].to_vec();
        assert_eq!(
            last,
            vec![
                ("complete".to_string(), normalized.to_string()),
                ("jump_from_cursor".to_string(), normalized.to_string()),
                ("semantic_tokens".to_string(), normalized.to_string()),
            ],
            "caminho entregue ao motor para {requested:?}"
        );
    }
}

#[test]
fn caminho_inexistente_dentro_da_raiz_nao_e_fora_do_projeto() {
    let h = Harness::new();
    assert_eq!(
        h.call("open_file", json!({ "path": "nada.typ" })),
        Err(json!("not_found"))
    );
    assert_eq!(
        h.call("complete", json!({ "path": "nada.typ", "offset": 0 })),
        Err(json!("not_found"))
    );
    assert!(h.engine.calls().is_empty());
}

#[test]
fn argumentos_malformados_sao_recusados_sem_derrubar_o_nucleo() {
    let h = Harness::new();
    let commands = path_commands("main.typ");
    for (cmd, valid) in &commands {
        let mut extra = valid.clone();
        extra["extra"] = json!("/etc/passwd");
        let mut nul = valid.clone();
        nul["path"] = json!("main.typ\0");
        let bad_bodies = [
            InvokeBody::Json(json!({})),
            InvokeBody::Json(Value::Null),
            InvokeBody::Json(json!({ "path": 42 })),
            InvokeBody::Json(json!({ "path": null })),
            InvokeBody::Json(json!({ "path": ["main.typ"] })),
            InvokeBody::Json(json!({ "path": { "path": "main.typ" } })),
            InvokeBody::Json(json!("main.typ")),
            InvokeBody::Json(json!([valid.clone()])),
            InvokeBody::Json(extra),
            InvokeBody::Json(nul),
            InvokeBody::Raw(vec![0xff, 0x00, 0x13]),
            InvokeBody::Raw(b"{\"path\":\"main.typ\"}".to_vec()),
        ];
        for body in bad_bodies {
            let shown = format!("{body:?}");
            let res = h.invoke(cmd, body);
            assert!(res.is_err(), "{cmd} aceitou {shown}: {res:?}");
        }
    }

    // Argumentos além de `path` com tipo ou valor errado.
    let specific = [
        (
            "edit_file",
            json!({ "path": "main.typ", "version": -1, "changes": [] }),
        ),
        (
            "edit_file",
            json!({ "path": "main.typ", "version": "0", "changes": [] }),
        ),
        (
            "edit_file",
            json!({ "path": "main.typ", "version": 0, "changes": "x" }),
        ),
        (
            "edit_file",
            json!({ "path": "main.typ", "version": 0, "changes": [{ "from": -1, "to": 0, "insert": "" }] }),
        ),
        (
            "edit_file",
            json!({ "path": "main.typ", "version": 0, "changes": [{ "from": 0, "to": 0 }] }),
        ),
        (
            "edit_file",
            json!({ "path": "main.typ", "version": 0, "changes": [{ "from": 0, "to": 0, "insert": "", "x": 1 }] }),
        ),
        (
            "resolve_external_change",
            json!({ "path": "main.typ", "keep": "theirs" }),
        ),
        ("resolve_external_change", json!({ "path": "main.typ" })),
        ("complete", json!({ "path": "main.typ", "offset": -1 })),
        ("complete", json!({ "path": "main.typ", "offset": 1.5 })),
        ("complete", json!({ "path": "main.typ" })),
        (
            "jump_from_cursor",
            json!({ "path": "main.typ", "offset": "0" }),
        ),
        ("jump_from_click", json!({ "page": -1, "xPt": 1, "yPt": 1 })),
        (
            "jump_from_click",
            json!({ "page": 0, "xPt": "1", "yPt": 1 }),
        ),
        ("jump_from_click", json!({ "page": 0, "xPt": 1 })),
        (
            "jump_from_click",
            json!({ "page": 0, "xPt": 1, "yPt": 1, "path": "main.typ" }),
        ),
    ];
    for (cmd, args) in specific {
        let shown = args.to_string();
        assert!(h.call(cmd, args).is_err(), "{cmd} aceitou {shown}");
    }

    // Sem argumentos: qualquer corpo além de objeto vazio é recusado.
    for body in [
        InvokeBody::Json(json!({ "path": "/etc" })),
        InvokeBody::Json(json!("x")),
        InvokeBody::Raw(vec![1, 2, 3]),
    ] {
        assert!(h.invoke("list_tree", body.clone()).is_err());
        assert!(h.invoke("outline", body).is_err());
    }
    assert!(
        h.engine.calls().is_empty(),
        "motor recebeu argumento malformado"
    );
    h.still_alive();
}

#[test]
fn comando_desconhecido_e_recusado() {
    let h = Harness::new();
    for cmd in [
        "read_file",
        "write_file",
        "plugin:fs|read_file",
        "plugin:dialog|open",
        "plugin:shell|execute",
        "open_project",
        "OPEN_FILE",
        "open_file ",
        "",
    ] {
        let res = h.call(cmd, json!({ "path": "main.typ" }));
        assert!(
            res.is_err(),
            "comando {cmd:?} deveria ser recusado: {res:?}"
        );
    }
    h.still_alive();
}

#[test]
fn sem_projeto_aberto_comandos_com_path_sao_recusados() {
    let engine = FakeEngine::default();
    let app =
        prelo_lib::configure_with_state(mock_builder(), AppState::new(Arc::new(engine.clone())))
            .build(mock_context(noop_assets()))
            .expect("app de teste não montou");
    let window = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("janela de teste não abriu");
    for (cmd, args) in path_commands("main.typ") {
        assert_eq!(
            invoke(&window, cmd, InvokeBody::Json(args)),
            Err(json!("no_project")),
            "{cmd} sem projeto"
        );
    }
    assert_eq!(
        invoke(&window, "list_tree", InvokeBody::Json(json!({}))),
        Err(json!("no_project"))
    );
    assert!(engine.calls().is_empty());
}

#[test]
fn arvore_nao_lista_links_para_fora_nem_pastas_ocultas() {
    let h = Harness::new();
    let links = h.links();
    h.project.write(".prelo/project.json", "{}");
    let tree = h.call("list_tree", json!({})).expect("list_tree");
    let text = tree.to_string();
    assert!(text.contains("\"main.typ\""), "{text}");
    assert!(text.contains("capitulos/metodo.typ"), "{text}");
    assert!(!text.contains(".prelo"), "{text}");
    if links.is_some() {
        assert!(!text.contains("link-fora"), "{text}");
        assert!(!text.contains("pasta-fora"), "{text}");
        assert!(!text.contains("segredo"), "{text}");
    }
    let root = h.project.root();
    assert!(
        !text.contains(root.to_string_lossy().as_ref()),
        "árvore revela caminho absoluto: {text}"
    );
}

/// Nomes da pasta: o que existe nela agora.
fn entries(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|it| {
            it.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// `create_file` e `create_dir` (FR-034, FR-035): nenhum pedido recusado cria algo ao lado da
/// raiz ou na pasta de fora, nem altera o arquivo de fora.
#[test]
fn criar_nunca_cria_nada_fora_do_projeto() {
    let h = Harness::new();
    let links = h.links();
    let parent = h.project.root().parent().unwrap().to_owned();
    let before_outside = entries(&h.outside.root());
    let mut paths = absolute_paths(&h);
    paths.extend(dotdot_paths(&h));
    paths.push("../novo-fora.typ".into());
    paths.push(
        h.outside
            .root()
            .join("novo.typ")
            .to_string_lossy()
            .into_owned(),
    );
    if links.is_some() {
        paths.push("pasta-fora/novo.typ".into());
        paths.push("link-fora.typ".into());
    }
    for path in &paths {
        for cmd in ["create_file", "create_dir"] {
            assert_eq!(
                h.call(cmd, json!({ "path": path })),
                Err(json!("path_outside_project")),
                "{cmd} com {path:?}"
            );
        }
    }
    // A pasta temporária do sistema muda com os outros testes: confere os nomes pedidos.
    for name in ["fora.typ", "novo-fora.typ", "novo.typ"] {
        assert!(!parent.join(name).exists(), "criou {name} ao lado da raiz");
    }
    assert_eq!(
        entries(&h.outside.root()),
        before_outside,
        "criou algo fora"
    );
    assert_eq!(
        std::fs::read_to_string(h.outside.secret()).unwrap(),
        SECRET,
        "arquivo de fora foi alterado"
    );
    assert!(h.engine.calls().is_empty());
    h.still_alive();
}

#[test]
fn criar_dentro_do_projeto_devolve_caminho_normalizado_sem_sobrescrever() {
    let h = Harness::new();
    let main = std::fs::read(h.project.path("main.typ")).unwrap();
    assert_eq!(
        h.call("create_file", json!({ "path": "capitulos/../novo.typ" })),
        Ok(json!({ "path": "novo.typ" }))
    );
    assert_eq!(
        h.call("create_dir", json!({ "path": "./capitulos/anexos" })),
        Ok(json!({ "path": "capitulos/anexos" }))
    );
    for cmd in ["create_file", "create_dir"] {
        assert_eq!(
            h.call(cmd, json!({ "path": "main.typ" })),
            Err(json!("already_exists"))
        );
        assert_eq!(
            h.call(cmd, json!({ "path": ".prelo/project.json" })),
            Err(json!("invalid_name"))
        );
    }
    assert_eq!(std::fs::read(h.project.path("main.typ")).unwrap(), main);
    let tree = h.call("list_tree", json!({})).unwrap().to_string();
    assert!(tree.contains("\"novo.typ\""), "{tree}");
    assert!(tree.contains("capitulos/anexos"), "{tree}");
    let opened = h.call("open_file", json!({ "path": "novo.typ" })).unwrap();
    assert_eq!(opened["text"], json!(""));
}

/// `decide_plugin` e `redownload_package` não recebem caminho: só um hash pedido pelo núcleo
/// e um `@preview/<nome>:<versão>`. Caminho, hash inventado ou spec fora do formato são
/// recusados sem tocar disco nem rede.
#[test]
fn plugins_e_pacotes_nao_aceitam_caminho_nem_hash_inventado() {
    let h = Harness::new();
    let secret = h.outside.secret().to_string_lossy().into_owned();
    let upper = "A".repeat(64);
    for sha in [
        "../../etc/passwd",
        "/etc/passwd",
        secret.as_str(),
        "",
        upper.as_str(),
    ] {
        assert_eq!(
            h.call("decide_plugin", json!({ "sha256": sha, "approve": true })),
            Err(json!("invalid_arguments")),
            "{sha:?}"
        );
    }
    assert_eq!(
        h.call(
            "decide_plugin",
            json!({ "sha256": "a".repeat(64), "approve": true })
        ),
        Err(json!("not_found")),
        "hash que o núcleo não pediu"
    );
    assert_eq!(
        h.call(
            "decide_plugin",
            json!({ "sha256": "a".repeat(64), "approve": true, "path": "main.typ" })
        ),
        Err(json!("invalid_arguments"))
    );
    for spec in [
        "/etc/passwd",
        "../x",
        secret.as_str(),
        "@preview/../x:1.0.0",
        "@preview/x:1.0.0/../../y",
        "@local/x:1.0.0",
        "https://exemplo.com/x.tar.gz",
    ] {
        let res = h.call("redownload_package", json!({ "spec": spec }));
        assert_eq!(res, Err(json!("invalid_arguments")), "{spec:?}");
    }
    assert_eq!(
        h.call(
            "redownload_package",
            json!({ "spec": "@preview/x:1.0.0", "path": "a" })
        ),
        Err(json!("invalid_arguments"))
    );
    // Sem dados locais (app de teste), nada é baixado.
    assert_eq!(
        h.call("redownload_package", json!({ "spec": "@preview/x:1.0.0" })),
        Err(json!("network_unavailable"))
    );
    h.still_alive();
}

/// `copy_into_project` (D-13, FR-023) recebe só `{ requested }`: o arquivo de origem é escolhido
/// no diálogo nativo do núcleo, nunca enviado pela webview. `source` (ou qualquer campo a
/// mais) é recusado, e pedido que o núcleo não ofereceu é recusado antes de qualquer diálogo,
/// sem gravar nada no projeto.
#[test]
fn copy_into_project_nao_aceita_origem_da_webview_nem_pedido_nao_oferecido() {
    let h = Harness::new();
    let secret = h.outside.secret().to_string_lossy().into_owned();
    let before = std::fs::read_dir(h.project.root()).unwrap().count();
    for args in [
        json!({ "requested": "../x/refs.bib", "source": secret }),
        json!({ "source": secret }),
        json!({ "requested": 1 }),
        json!({}),
    ] {
        assert_eq!(
            h.call("copy_into_project", args.clone()),
            Err(json!("invalid_arguments")),
            "{args}"
        );
    }
    for requested in ["../x/refs.bib", secret.as_str(), "/etc/passwd", "main.typ"] {
        assert_eq!(
            h.call("copy_into_project", json!({ "requested": requested })),
            Err(json!("not_offered")),
            "{requested:?}"
        );
    }
    assert_eq!(
        std::fs::read_dir(h.project.root()).unwrap().count(),
        before,
        "nada gravado no projeto"
    );
    h.still_alive();
}
