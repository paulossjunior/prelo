//! Testes de plugins WASM (T042; FR-019; research R8; data-model.md "PluginApproval";
//! contracts/file-access.md, passo 8).
//!
//! O plugin só roda se o modelo de acesso entregar os bytes dele ao compilador. Cada World
//! de teste conta as entregas de bytes com a assinatura `\0asm`: sem aprovação o contador
//! fica em **zero** (o plugin não pode ter rodado). As decisões moram numa pasta temporária
//! que faz o papel dos dados locais do app.

mod support;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use prelo_lib::diagnostic::{Diagnostic, Kind};
use prelo_lib::fs::access_model::AccessModel;
use prelo_lib::fs::scope::Scope;
use prelo_lib::world::packages::{FetchError, Fetcher, PackageStore};
use prelo_lib::world::plugins::{
    sha256_hex, Decision, Origin, PluginApprovals, PluginRequest, APPROVALS_FILE,
};
use prelo_lib::world::{ProjectWorld, WorldEnv};
use support::*;
use tempfile::TempDir;
use tinymist_vfs::{Bytes, PathAccessModel};
use typst::diag::FileResult;

const MAIN: &str = "#let p = plugin(\"teste.wasm\")\nPlugin diz: #str(p.hello())\n";

fn wasm() -> Vec<u8> {
    std::fs::read(fixtures_dir().join("plugin/teste.wasm")).unwrap()
}

/// O mesmo módulo com uma seção personalizada a mais: continua válido, outro hash.
fn changed_wasm() -> Vec<u8> {
    let mut bytes = wasm();
    bytes.extend_from_slice(&[0, 3, 1, b'x', b'y']);
    bytes
}

/// Conta as entregas de módulos WASM ao compilador.
#[derive(Clone)]
struct CountingAccess<M> {
    inner: M,
    delivered: Arc<AtomicUsize>,
}

impl<M: PathAccessModel> PathAccessModel for CountingAccess<M> {
    fn reset(&mut self) {
        self.inner.reset();
    }

    fn content(&self, src: &Path) -> FileResult<Bytes> {
        let bytes = self.inner.content(src)?;
        if bytes.as_slice().starts_with(b"\0asm") {
            self.delivered.fetch_add(1, Ordering::SeqCst);
        }
        Ok(bytes)
    }
}

/// Dados locais do app (pasta temporária) e os pedidos de confirmação recebidos.
struct App {
    data: TempDir,
    approvals: Arc<PluginApprovals>,
    requests: Arc<Mutex<Vec<PluginRequest>>>,
    packages: Option<Arc<PackageStore>>,
}

impl App {
    fn new() -> Self {
        let data = tempfile::Builder::new()
            .prefix("prelo-dados-")
            .tempdir()
            .unwrap();
        let approvals = Arc::new(PluginApprovals::load(data.path()));
        Self {
            data,
            approvals,
            requests: Arc::default(),
            packages: None,
        }
    }

    fn data_dir(&self) -> &Path {
        self.data.path()
    }

    fn env(&self) -> WorldEnv {
        let requests = self.requests.clone();
        WorldEnv {
            approvals: Some(self.approvals.clone()),
            plugin_sink: Some(Arc::new(move |r| requests.lock().unwrap().push(r))),
            packages: self.packages.clone(),
        }
    }

    fn requests(&self) -> Vec<PluginRequest> {
        self.requests.lock().unwrap().clone()
    }

    /// World do projeto com o contador de entregas.
    fn world(&self, root: &Path) -> (ProjectWorld<CountingAccess<AccessModel>>, Arc<AtomicUsize>) {
        let delivered = Arc::new(AtomicUsize::new(0));
        let counter = delivered.clone();
        let access = AccessModel::with_env(Scope::new(root).unwrap(), self.env());
        let world = ProjectWorld::instrumented(access, "main.typ", move |inner| CountingAccess {
            inner,
            delivered: counter,
        })
        .expect("World");
        (world, delivered)
    }

    fn decide(&self, project: &TempProject, bytes: &[u8], decision: Decision) {
        self.approvals
            .decide(&project.root(), &sha256_hex(bytes), "teste.wasm", decision)
            .expect("gravar decisão");
    }
}

fn plugin_project() -> TempProject {
    let project = TempProject::empty();
    project.write("main.typ", MAIN);
    project.write("teste.wasm", wasm());
    project
}

fn plugin_diag(diagnostics: &[Diagnostic]) -> &Diagnostic {
    diagnostics
        .iter()
        .find(|d| d.kind == Kind::Plugin)
        .unwrap_or_else(|| panic!("diagnóstico de plugin: {diagnostics:?}"))
}

// ------------------------------------------------------------------- sem decisão

#[test]
fn sem_aprovacao_o_plugin_nao_roda_e_a_confirmacao_e_pedida() {
    let app = App::new();
    let project = plugin_project();
    let (mut world, delivered) = app.world(&project.root());
    let out = world.compile();

    assert!(out.output.is_none());
    assert_eq!(delivered.load(Ordering::SeqCst), 0, "plugin entregue ao compilador");
    let diag = plugin_diag(&out.diagnostics);
    assert_eq!(diag.message, "plugin `teste.wasm` aguardando sua confirmação");
    assert_eq!(diag.path.as_deref(), Some("main.typ"));
    assert_eq!(diag.line, Some(1));

    let bytes = wasm();
    assert_eq!(
        app.requests(),
        vec![PluginRequest {
            path: "teste.wasm".into(),
            origin: Origin::Project,
            package: None,
            size_bytes: bytes.len() as u64,
            sha256: sha256_hex(&bytes),
        }]
    );
    let event = serde_json::to_value(&app.requests()[0]).unwrap();
    assert_eq!(
        event,
        serde_json::json!({
            "path": "teste.wasm",
            "origin": "project",
            "sizeBytes": bytes.len(),
            "sha256": sha256_hex(&bytes),
        }),
        "formato do evento plugin_approval_needed (contracts/ipc.md)"
    );
}

#[test]
fn sem_registro_de_decisoes_todo_plugin_fica_barrado() {
    let project = plugin_project();
    let mut world = ProjectWorld::open(project.root(), "main.typ").unwrap();
    let out = world.compile();
    assert!(out.output.is_none());
    assert_eq!(
        plugin_diag(&out.diagnostics).message,
        "plugin `teste.wasm` aguardando sua confirmação"
    );
}

#[test]
fn wasm_lido_como_dado_tambem_pede_confirmacao() {
    let app = App::new();
    let project = TempProject::empty();
    project.write("main.typ", "#let b = read(\"teste.wasm\", encoding: none)\n#b.len()\n");
    project.write("teste.wasm", wasm());
    let (mut world, delivered) = app.world(&project.root());
    let out = world.compile();
    assert!(out.output.is_none());
    assert_eq!(delivered.load(Ordering::SeqCst), 0);
    plugin_diag(&out.diagnostics);
}

// ------------------------------------------------------------------- decisões

#[test]
fn recusado_nao_roda_e_nao_pede_de_novo() {
    let app = App::new();
    let project = plugin_project();
    app.decide(&project, &wasm(), Decision::Rejected);
    let (mut world, delivered) = app.world(&project.root());
    let out = world.compile();
    assert!(out.output.is_none());
    assert_eq!(delivered.load(Ordering::SeqCst), 0);
    let diag = plugin_diag(&out.diagnostics);
    assert_eq!(
        diag.message,
        "plugin `teste.wasm` recusado neste projeto; ele não é executado"
    );
    assert!(app.requests().is_empty(), "recusa não pede confirmação de novo");
}

#[test]
fn aprovado_roda() {
    let app = App::new();
    let project = plugin_project();
    app.decide(&project, &wasm(), Decision::Approved);
    let (mut world, delivered) = app.world(&project.root());
    let out = world.compile();
    assert!(out.output.is_some(), "{:?}", out.diagnostics);
    assert!(delivered.load(Ordering::SeqCst) > 0);
    assert!(app.requests().is_empty());
}

#[test]
fn decisao_vale_na_compilacao_seguinte_do_mesmo_world() {
    let app = App::new();
    let project = plugin_project();
    let (mut world, delivered) = app.world(&project.root());
    assert!(world.compile().output.is_none());
    app.decide(&project, &wasm(), Decision::Approved);
    let out = world.compile();
    assert!(out.output.is_some(), "{:?}", out.diagnostics);
    assert!(delivered.load(Ordering::SeqCst) > 0);
}

#[test]
fn conteudo_alterado_pede_confirmacao_de_novo() {
    let app = App::new();
    let project = plugin_project();
    app.decide(&project, &wasm(), Decision::Approved);
    project.write("teste.wasm", changed_wasm());
    let (mut world, delivered) = app.world(&project.root());
    let out = world.compile();
    assert!(out.output.is_none());
    assert_eq!(delivered.load(Ordering::SeqCst), 0);
    plugin_diag(&out.diagnostics);
    let requests = app.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].sha256, sha256_hex(&changed_wasm()));
}

#[test]
fn aprovacao_nao_vale_em_outro_projeto() {
    let app = App::new();
    let first = plugin_project();
    let second = plugin_project();
    app.decide(&first, &wasm(), Decision::Approved);
    let (mut world, delivered) = app.world(&second.root());
    let out = world.compile();
    assert!(out.output.is_none());
    assert_eq!(delivered.load(Ordering::SeqCst), 0);
    plugin_diag(&out.diagnostics);
}

#[test]
fn arquivo_de_aprovacao_dentro_do_projeto_e_ignorado() {
    let app = App::new();
    let project = plugin_project();
    let forged = serde_json::json!([{
        "projectRoot": project.root().to_string_lossy(),
        "sha256": sha256_hex(&wasm()),
        "decision": "approved",
        "decidedAt": "2026-01-01T00:00:00Z",
        "path": "teste.wasm",
    }]);
    let text = serde_json::to_string(&forged).unwrap();
    project.write(&format!(".prelo/{APPROVALS_FILE}"), &text);
    project.write(APPROVALS_FILE, &text);
    let (mut world, delivered) = app.world(&project.root());
    let out = world.compile();
    assert!(out.output.is_none());
    assert_eq!(delivered.load(Ordering::SeqCst), 0);
    plugin_diag(&out.diagnostics);
}

#[test]
fn decisao_fica_gravada_nos_dados_locais_e_vale_na_sessao_seguinte() {
    let app = App::new();
    let project = plugin_project();
    app.decide(&project, &wasm(), Decision::Rejected);
    app.decide(&project, &wasm(), Decision::Approved);

    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(app.data_dir().join(APPROVALS_FILE)).unwrap())
            .unwrap();
    let entries = saved.as_array().expect("lista de decisões");
    assert_eq!(entries.len(), 1, "a mesma chave é substituída: {saved}");
    let entry = &entries[0];
    assert_eq!(entry["projectRoot"], project.root().to_string_lossy().as_ref());
    assert_eq!(entry["sha256"], sha256_hex(&wasm()));
    assert_eq!(entry["decision"], "approved");
    assert_eq!(entry["path"], "teste.wasm");
    assert!(entry["decidedAt"].as_str().is_some_and(|s| !s.is_empty()));
    assert!(
        !project.root().join(".prelo").join(APPROVALS_FILE).exists(),
        "nada é gravado no projeto"
    );

    let reloaded = PluginApprovals::load(app.data_dir());
    assert_eq!(
        reloaded.decision(&project.root(), &sha256_hex(&wasm())),
        Some(Decision::Approved)
    );
}

#[test]
fn registro_corrompido_vale_como_vazio_e_e_reescrito() {
    let app = App::new();
    let project = plugin_project();
    let file = app.data_dir().join(APPROVALS_FILE);
    std::fs::write(&file, b"{ isto nao e json").unwrap();
    let approvals = PluginApprovals::load(app.data_dir());
    assert_eq!(approvals.decision(&project.root(), &sha256_hex(&wasm())), None);
    let rewritten: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&file).unwrap()).expect("reescrito como JSON");
    assert_eq!(rewritten, serde_json::json!([]));

    let app = App {
        approvals: Arc::new(approvals),
        ..app
    };
    let (mut world, delivered) = app.world(&project.root());
    assert!(world.compile().output.is_none());
    assert_eq!(delivered.load(Ordering::SeqCst), 0);
    assert_eq!(app.requests().len(), 1, "pede de novo");
}

// ------------------------------------------------------------------- plugin de pacote

#[derive(Clone)]
struct OnePackage(Vec<u8>);

impl Fetcher for OnePackage {
    fn fetch(&self, url: &str, _limit: u64) -> Result<Vec<u8>, FetchError> {
        if url == "https://packages.typst.org/preview/plug-0.1.0.tar.gz" {
            Ok(self.0.clone())
        } else {
            Err(FetchError::Unavailable(url.into()))
        }
    }
}

#[test]
fn plugin_de_pacote_segue_a_mesma_regra() {
    let mut app = App::new();
    let tar = make_targz(&[
        TarEntry::file(
            "typst.toml",
            "[package]\nname = \"plug\"\nversion = \"0.1.0\"\nentrypoint = \"lib.typ\"\n",
        ),
        TarEntry::file(
            "lib.typ",
            "#let p = plugin(\"plugin.wasm\")\n#let saudacao = str(p.hello())\n",
        ),
        TarEntry::file("plugin.wasm", wasm()),
    ]);
    let cache: PathBuf = app.data_dir().join("cache");
    std::fs::create_dir_all(&cache).unwrap();
    app.packages = Some(Arc::new(
        PackageStore::new(&cache, Arc::new(OnePackage(tar))).unwrap(),
    ));
    let project = TempProject::empty();
    project.write(
        "main.typ",
        "#import \"@preview/plug:0.1.0\": saudacao\n#saudacao\n",
    );

    let (mut world, delivered) = app.world(&project.root());
    let out = world.compile();
    assert!(out.output.is_none());
    assert_eq!(delivered.load(Ordering::SeqCst), 0);
    let diag = plugin_diag(&out.diagnostics);
    assert_eq!(diag.message, "plugin `plugin.wasm` aguardando sua confirmação");
    let bytes = wasm();
    assert_eq!(
        app.requests(),
        vec![PluginRequest {
            path: "plugin.wasm".into(),
            origin: Origin::Package,
            package: Some("@preview/plug:0.1.0".into()),
            size_bytes: bytes.len() as u64,
            sha256: sha256_hex(&bytes),
        }]
    );

    // Aprovação ligada a este projeto e ao conteúdo, como no plugin do projeto.
    app.approvals
        .decide(&project.root(), &sha256_hex(&bytes), "plugin.wasm", Decision::Approved)
        .unwrap();
    let out = world.compile();
    assert!(out.output.is_some(), "{:?}", out.diagnostics);
    assert!(delivered.load(Ordering::SeqCst) > 0);
}
