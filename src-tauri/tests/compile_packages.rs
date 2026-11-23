//! Pacote `@preview` pela prévia do app (T053; US3 cenário 9; FR-017; research R7).
//!
//! Com um *fetcher* local (sem rede real) ligado ao núcleo: a primeira compilação baixa o
//! pacote para o cache dos dados locais do app; depois, sem rede, o mesmo projeto compila
//! pelo cache e a prévia sai igual (mesmas páginas, mesmos hashes), sem pedir nada à rede.

mod support;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use prelo_lib::commands::AppState;
use prelo_lib::compile::preview::PreviewUpdate;
use prelo_lib::world::packages::{FetchError, Fetcher};
use support::{make_targz, TarEntry, TempProject};
use tempfile::TempDir;

const WAIT: Duration = Duration::from_secs(60);
const DEMO_URL: &str = "https://packages.typst.org/preview/demo-0.1.0.tar.gz";

/// *Fetcher* local: URL → bytes; desligado, responde "sem conexão". Anota cada pedido.
#[derive(Clone, Default)]
struct LocalFetcher {
    served: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    asked: Arc<Mutex<Vec<String>>>,
}

impl LocalFetcher {
    fn serving_demo() -> Self {
        let fetcher = Self::default();
        let package = make_targz(&[
            TarEntry::file(
                "typst.toml",
                "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nentrypoint = \"lib.typ\"\n",
            ),
            TarEntry::file("lib.typ", "#let saudacao = [Olá do pacote]\n"),
        ]);
        fetcher
            .served
            .lock()
            .unwrap()
            .insert(DEMO_URL.into(), package);
        fetcher
    }

    fn offline(&self) {
        self.served.lock().unwrap().clear();
    }

    fn asked(&self) -> Vec<String> {
        self.asked.lock().unwrap().clone()
    }
}

impl Fetcher for LocalFetcher {
    fn fetch(&self, url: &str, limit: u64) -> Result<Vec<u8>, FetchError> {
        self.asked.lock().unwrap().push(url.into());
        match self.served.lock().unwrap().get(url) {
            Some(bytes) if bytes.len() as u64 > limit => Err(FetchError::TooLarge),
            Some(bytes) => Ok(bytes.clone()),
            None => Err(FetchError::Unavailable("sem conexão".into())),
        }
    }
}

/// Núcleo com os dados locais em `data` e o *fetcher* dado; abre `project` e devolve a
/// primeira prévia que terminou (ok ou com erro).
fn open(data: &PathBuf, fetcher: &LocalFetcher, project: &TempProject) -> (AppState, PreviewUpdate) {
    let state = AppState::default();
    state.trust().set_data_dir(data.clone());
    state.trust().set_fetcher(Arc::new(fetcher.clone()));
    let (tx, updates): (_, Receiver<PreviewUpdate>) = mpsc::channel();
    state.subscribe_preview(move |u: &PreviewUpdate| {
        let _ = tx.send(u.clone());
    });
    state.open_project(&project.root()).expect("abrir projeto");
    let deadline = Instant::now() + WAIT;
    let update = updates
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .expect("prévia no prazo");
    (state, update)
}

fn hashes(update: &PreviewUpdate) -> Vec<String> {
    update.pages.iter().map(|p| p.hash.clone()).collect()
}

fn importing() -> TempProject {
    let project = TempProject::empty();
    project.write(
        "main.typ",
        "#import \"@preview/demo:0.1.0\": saudacao\n= Artigo\n#saudacao\n",
    );
    project
}

#[test]
fn primeira_compilacao_baixa_e_a_segunda_sem_rede_usa_o_cache_com_a_mesma_previa() {
    let data_dir = TempDir::new().unwrap();
    let data = data_dir.path().canonicalize().unwrap();
    let fetcher = LocalFetcher::serving_demo();
    let project = importing();

    // Primeira sessão, com rede: baixa uma vez e compila.
    let (first_state, first) = open(&data, &fetcher, &project);
    assert!(first.ok, "{:?}", first.diagnostics);
    assert_eq!(fetcher.asked(), vec![DEMO_URL.to_owned()], "baixou o pacote");
    assert!(
        data.join("packages/preview/demo/0.1.0/lib.typ").is_file(),
        "pacote no cache dos dados locais"
    );
    drop(first_state);

    // Segunda sessão, sem rede: o cache resolve, nada é pedido, a prévia é a mesma.
    fetcher.offline();
    let (_state, second) = open(&data, &fetcher, &project);
    assert!(second.ok, "{:?}", second.diagnostics);
    assert_eq!(fetcher.asked().len(), 1, "nada foi baixado de novo");
    assert_eq!(hashes(&second), hashes(&first), "mesma prévia");
}

#[test]
fn sem_rede_e_sem_cache_e_diagnostico_de_pacote() {
    let data_dir = TempDir::new().unwrap();
    let data = data_dir.path().canonicalize().unwrap();
    let fetcher = LocalFetcher::default();
    let (_state, update) = open(&data, &fetcher, &importing());
    assert!(!update.ok);
    let messages: Vec<_> = update.diagnostics.iter().map(|d| d.message.clone()).collect();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("@preview/demo:0.1.0") && m.contains("não há conexão")),
        "{messages:?}"
    );
}
