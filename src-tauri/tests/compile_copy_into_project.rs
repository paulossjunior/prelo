//! Oferta de cópia para dentro do projeto (T054; D-13, FR-023; US3 cenário 6;
//! contracts/ipc.md "Arquivo externo, plugins e pacotes").
//!
//! - `.typ` que aponta para fora do projeto (`../…`) é recusado e o núcleo emite
//!   `copy_into_project_offer` com **só** o caminho pedido no `.typ`;
//! - `copy_into_project` só copia depois da escolha da pessoa (o arquivo de origem vem do
//!   diálogo nativo do núcleo; aqui, do teste, como faria o diálogo), grava a cópia ao lado do
//!   `.typ` que a pediu, troca o caminho no `.typ` e a compilação passa;
//! - sem confirmação nada é copiado; pedido que o núcleo não ofereceu é recusado; a cópia
//!   nunca sobrescreve arquivo do projeto.

mod support;

use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use prelo_lib::commands::{AppState, CopyIntoProjectOffer};
use prelo_lib::compile::preview::PreviewUpdate;
use serde_json::json;
use support::TempProject;
use tempfile::TempDir;

const WAIT: Duration = Duration::from_secs(60);

const BIB: &str = "@book{souza2021,\n  author = {Souza, Carla},\n  title = {Tipografia digital},\n  publisher = {Editora Exemplo},\n  year = {2021},\n}\n";

/// `.bib` compartilhado numa pasta irmã do projeto (mesma pasta temporária do sistema).
struct Shared {
    dir: TempDir,
}

impl Shared {
    fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("prelo-compartilhado-")
            .tempdir()
            .unwrap();
        std::fs::write(dir.path().join("refs.bib"), BIB).unwrap();
        Self { dir }
    }

    fn file(&self) -> std::path::PathBuf {
        self.dir.path().canonicalize().unwrap().join("refs.bib")
    }

    /// Caminho como o `.typ` escreve, relativo à raiz do projeto.
    fn requested(&self, project: &TempProject) -> String {
        let shared = self.dir.path().canonicalize().unwrap();
        assert_eq!(
            shared.parent(),
            project.root().parent(),
            "projeto e pasta compartilhada são irmãos"
        );
        format!(
            "../{}/refs.bib",
            shared.file_name().unwrap().to_string_lossy()
        )
    }
}

struct Session {
    state: AppState,
    updates: Receiver<PreviewUpdate>,
    offers: Arc<Mutex<Vec<serde_json::Value>>>,
    project: TempProject,
}

impl Session {
    fn open(project: TempProject) -> Self {
        let state = AppState::default();
        let offers = Arc::new(Mutex::new(Vec::new()));
        let sink = offers.clone();
        state.set_copy_offer_listener(Arc::new(move |offer: &CopyIntoProjectOffer| {
            sink.lock()
                .unwrap()
                .push(serde_json::to_value(offer).unwrap());
        }));
        let (tx, updates) = mpsc::channel();
        state.subscribe_preview(move |u: &PreviewUpdate| {
            let _ = tx.send(u.clone());
        });
        state.open_project(&project.root()).expect("abrir projeto");
        Self {
            state,
            updates,
            offers,
            project,
        }
    }

    fn wait(&self, ok: bool) -> PreviewUpdate {
        let deadline = Instant::now() + WAIT;
        loop {
            let update = self
                .updates
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .unwrap_or_else(|_| panic!("nenhuma prévia ok={ok} no prazo"));
            if update.ok == ok {
                return update;
            }
        }
    }

    fn offers(&self) -> Vec<serde_json::Value> {
        self.offers.lock().unwrap().clone()
    }

    /// Nomes na raiz do projeto, fora `.prelo`.
    fn root_names(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(self.project.root())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n != ".prelo")
            .collect();
        names.sort();
        names
    }
}

fn cite(project: &TempProject, requested: &str) {
    project.write(
        "main.typ",
        format!("= Artigo\n\nSegundo @souza2021.\n\n#bibliography(\"{requested}\")\n"),
    );
}

#[test]
fn typ_apontando_para_fora_emite_oferta_so_com_o_caminho_pedido() {
    let shared = Shared::new();
    let project = TempProject::empty();
    let requested = shared.requested(&project);
    project.write(
        "main.typ",
        format!("Segundo @souza2021.\n#bibliography(\"{requested}\")\n"),
    );
    let s = Session::open(project);
    let failed = s.wait(false);
    assert!(
        failed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("fica fora da pasta do projeto")),
        "{:?}",
        failed.diagnostics
    );
    assert_eq!(s.offers(), vec![json!({ "requested": requested })]);
    assert_eq!(s.state.copy_offers(), vec![requested.clone()]);
    let shown = serde_json::to_string(&s.offers()).unwrap();
    let real = shared.file().to_string_lossy().into_owned();
    assert!(!shown.contains(&real), "a oferta não revela o caminho real");

    // Recompilar com o mesmo pedido não repete a oferta.
    s.state.open_file("main.typ").unwrap();
    let edit = prelo_lib::commands::Change {
        from: 0,
        to: 0,
        insert: "= Título\n".into(),
    };
    s.state.edit_file("main.typ", 0, &[edit]).unwrap();
    s.wait(false);
    assert_eq!(s.offers().len(), 1, "uma oferta por caminho");
}

#[test]
fn copia_so_depois_da_escolha_troca_o_caminho_e_a_compilacao_passa() {
    let shared = Shared::new();
    let project = TempProject::empty();
    let requested = shared.requested(&project);
    cite(&project, &requested);
    let s = Session::open(project);
    s.wait(false);
    assert_eq!(s.root_names(), vec!["main.typ"], "nada copiado antes da escolha");

    let copied = s
        .state
        .copy_into_project(&requested, &shared.file())
        .expect("copy_into_project");
    assert_eq!(
        serde_json::to_value(&copied).unwrap(),
        json!({ "path": "refs.bib", "updated": [{ "path": "main.typ", "version": 1 }] })
    );
    assert_eq!(
        std::fs::read_to_string(s.project.path("refs.bib")).unwrap(),
        BIB,
        "cópia fiel"
    );
    assert_eq!(std::fs::read_to_string(shared.file()).unwrap(), BIB, "origem intacta");
    let main = s.state.open_file("main.typ").unwrap();
    assert_eq!(main.version, 1);
    assert!(
        main.text.contains("#bibliography(\"refs.bib\")"),
        "caminho trocado no .typ: {}",
        main.text
    );
    let ok = s.wait(true);
    assert!(ok.diagnostics.is_empty(), "{:?}", ok.diagnostics);
    assert!(s.state.copy_offers().is_empty(), "oferta atendida");
}

#[test]
fn copia_para_a_pasta_do_typ_que_pediu() {
    let shared = Shared::new();
    let project = TempProject::empty();
    let requested_from_root = shared.requested(&project);
    let requested = format!("../{requested_from_root}");
    project.write("main.typ", "= Artigo\n#include \"capitulos/intro.typ\"\n");
    project.write(
        "capitulos/intro.typ",
        format!("Segundo @souza2021.\n#bibliography(\"{requested}\")\n"),
    );
    let s = Session::open(project);
    s.wait(false);
    assert_eq!(s.offers(), vec![json!({ "requested": requested })]);

    let copied = s
        .state
        .copy_into_project(&requested, &shared.file())
        .expect("copy_into_project");
    assert_eq!(
        serde_json::to_value(&copied).unwrap(),
        json!({ "path": "capitulos/refs.bib", "updated": [{ "path": "capitulos/intro.typ", "version": 1 }] })
    );
    assert!(s.project.path("capitulos/refs.bib").is_file());
    s.wait(true);
}

#[test]
fn sem_confirmacao_nada_e_copiado_e_pedido_nao_oferecido_e_recusado() {
    let shared = Shared::new();
    let project = TempProject::empty();
    let requested = shared.requested(&project);
    project.write(
        "main.typ",
        format!("Segundo @souza2021.\n#bibliography(\"{requested}\")\n"),
    );
    let s = Session::open(project);
    s.wait(false);
    assert_eq!(s.offers().len(), 1);
    // A oferta sozinha não copia nada.
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(s.root_names(), vec!["main.typ"]);

    // Pedido que o núcleo não ofereceu: recusado sem copiar.
    for other in ["../outro/refs.bib", "refs.bib", "/etc/passwd", ""] {
        assert_eq!(
            s.state.copy_into_project(other, &shared.file()),
            Err("not_offered".to_owned()),
            "{other:?}"
        );
    }
    assert_eq!(s.root_names(), vec!["main.typ"]);
    let main = std::fs::read_to_string(s.project.path("main.typ")).unwrap();
    assert!(main.contains(&requested), ".typ intacto");
}

#[test]
fn nunca_sobrescreve_arquivo_do_projeto() {
    let shared = Shared::new();
    let project = TempProject::empty();
    let requested = shared.requested(&project);
    project.write("refs.bib", "% minha versão\n");
    project.write(
        "main.typ",
        format!("Segundo @souza2021.\n#bibliography(\"{requested}\")\n"),
    );
    let s = Session::open(project);
    s.wait(false);
    assert_eq!(
        s.state.copy_into_project(&requested, &shared.file()),
        Err("already_exists".to_owned())
    );
    assert_eq!(
        std::fs::read_to_string(s.project.path("refs.bib")).unwrap(),
        "% minha versão\n"
    );
    let main = s.state.open_file("main.typ").unwrap();
    assert_eq!(main.version, 0, ".typ intacto");
    assert_eq!(s.state.copy_offers(), vec![requested], "a oferta continua");
}

#[test]
fn origem_que_nao_e_arquivo_comum_e_recusada() {
    let shared = Shared::new();
    let project = TempProject::empty();
    let requested = shared.requested(&project);
    project.write(
        "main.typ",
        format!("Segundo @souza2021.\n#bibliography(\"{requested}\")\n"),
    );
    let s = Session::open(project);
    s.wait(false);
    let folder = shared.file().parent().unwrap().to_path_buf();
    assert_eq!(
        s.state.copy_into_project(&requested, &folder),
        Err("not_found".to_owned())
    );
    assert_eq!(s.root_names(), vec!["main.typ"]);
}

#[cfg(unix)]
#[test]
fn link_dentro_do_projeto_para_fora_nao_gera_oferta() {
    // O caminho pedido fica dentro do projeto; copiar para o mesmo lugar sobrescreveria o
    // link. Só a recusa de acesso aparece.
    let shared = Shared::new();
    let project = TempProject::empty();
    support::symlink(&shared.file(), &project.path("refs.bib")).unwrap();
    project.write("main.typ", "Segundo @souza2021.\n#bibliography(\"refs.bib\")\n");
    let s = Session::open(project);
    s.wait(false);
    assert!(s.offers().is_empty(), "{:?}", s.offers());
    assert!(s.state.copy_offers().is_empty());
}
