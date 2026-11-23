//! Testes de pacotes `@preview` (T043; FR-017, FR-018; research R7; data-model.md
//! "PackageCacheEntry"; contracts/file-access.md "Pacotes (download)").
//!
//! Nenhum teste usa a rede: o [`Fetcher`] é um falso que serve bytes gerados no teste e anota
//! cada URL pedida. Os dados locais do app ficam numa pasta temporária com uma "vizinha" para
//! conferir que nada é gravado fora de `<cache>/packages/`.

mod support;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use prelo_lib::diagnostic::Kind;
use prelo_lib::world::packages::{
    FetchError, Fetcher, Limits, PackageRefusal, PackageStore, State, INDEX_FILE, REGISTRY_URL,
};
use prelo_lib::world::{ProjectWorld, WorldEnv};
use support::*;
use tempfile::TempDir;
use typst::syntax::package::PackageSpec;

const DEMO: &str = "@preview/demo:0.1.0";
const DEMO_URL: &str = "https://packages.typst.org/preview/demo-0.1.0.tar.gz";
const TOML: &str = "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nentrypoint = \"lib.typ\"\n";
const LIB: &str = "#let saudacao = [Olá do pacote]\n";

fn spec(text: &str) -> PackageSpec {
    text.parse().expect("spec válida")
}

/// Pacote válido: `typst.toml`, `lib.typ` e `dados/nota.txt`.
fn demo_entries() -> Vec<TarEntry> {
    vec![
        TarEntry::file("typst.toml", TOML),
        TarEntry::file("lib.typ", LIB),
        TarEntry::Dir {
            path: "dados/".into(),
        },
        TarEntry::file("dados/nota.txt", "nota do pacote"),
    ]
}

/// *Fetcher* falso: URL → bytes. Sem a URL, ou desligado, responde "sem conexão".
#[derive(Clone, Default)]
struct FakeFetcher {
    served: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    asked: Arc<Mutex<Vec<String>>>,
}

impl FakeFetcher {
    fn serving(url: &str, bytes: Vec<u8>) -> Self {
        let fake = Self::default();
        fake.serve(url, bytes);
        fake
    }

    fn serve(&self, url: &str, bytes: Vec<u8>) {
        self.served.lock().unwrap().insert(url.into(), bytes);
    }

    fn offline(&self) {
        self.served.lock().unwrap().clear();
    }

    fn asked(&self) -> Vec<String> {
        self.asked.lock().unwrap().clone()
    }
}

impl Fetcher for FakeFetcher {
    fn fetch(&self, url: &str, limit: u64) -> Result<Vec<u8>, FetchError> {
        self.asked.lock().unwrap().push(url.into());
        match self.served.lock().unwrap().get(url) {
            Some(bytes) if bytes.len() as u64 > limit => Err(FetchError::TooLarge),
            Some(bytes) => Ok(bytes.clone()),
            None => Err(FetchError::Unavailable("sem conexão".into())),
        }
    }
}

/// Dados locais do app numa pasta temporária: `<tmp>/dados` (o app) e `<tmp>/vizinha`.
struct AppData {
    tmp: TempDir,
}

impl AppData {
    fn new() -> Self {
        let tmp = tempfile::Builder::new()
            .prefix("prelo-dados-")
            .tempdir()
            .unwrap();
        std::fs::create_dir(tmp.path().join("dados")).unwrap();
        std::fs::create_dir(tmp.path().join("vizinha")).unwrap();
        Self { tmp }
    }

    fn dir(&self) -> PathBuf {
        self.tmp.path().canonicalize().unwrap().join("dados")
    }

    fn store(&self, fetcher: &FakeFetcher) -> PackageStore {
        PackageStore::new(&self.dir(), Arc::new(fetcher.clone())).expect("cache de pacotes")
    }

    fn package_dir(&self) -> PathBuf {
        self.dir().join("packages/preview/demo/0.1.0")
    }

    /// Todo arquivo gravado fora de `<dados>/packages/`.
    fn written_outside_cache(&self) -> Vec<PathBuf> {
        let root = self.tmp.path().canonicalize().unwrap();
        let cache = self.dir().join("packages");
        let mut out = Vec::new();
        walk(&root, &mut |p| {
            if !p.starts_with(&cache) {
                out.push(p.to_path_buf());
            }
        });
        out
    }

    /// Arquivos dentro do cache (fora o índice).
    fn cached_files(&self) -> Vec<PathBuf> {
        let cache = self.dir().join("packages");
        let mut out = Vec::new();
        walk(&cache, &mut |p| {
            if p.file_name().and_then(|n| n.to_str()) != Some(INDEX_FILE) {
                out.push(p.to_path_buf());
            }
        });
        out
    }
}

/// Visita arquivos e links (não pastas), sem seguir links.
fn walk(dir: &Path, f: &mut dyn FnMut(&Path)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            walk(&entry.path(), f);
        } else {
            f(&entry.path());
        }
    }
}

/// Projeto que importa o pacote e o World com o cache dado.
fn world_with(project: &TempProject, store: Arc<PackageStore>) -> ProjectWorld {
    let env = WorldEnv {
        packages: Some(store),
        ..WorldEnv::default()
    };
    ProjectWorld::open_with(project.root(), "main.typ", env).expect("World")
}

fn importing(import: &str) -> TempProject {
    let project = TempProject::empty();
    project.write(
        "main.typ",
        format!("#import \"{import}\": saudacao\n= Artigo\n#saudacao\n"),
    );
    project
}

// ------------------------------------------------------------------- download e cache

#[test]
fn pacote_valido_e_baixado_verificado_e_guardado() {
    let data = AppData::new();
    let fetcher = FakeFetcher::serving(DEMO_URL, make_targz(&demo_entries()));
    let store = data.store(&fetcher);

    let dir = store.resolve(&spec(DEMO)).expect("pacote resolvido");
    assert_eq!(dir, data.package_dir());
    assert_eq!(
        std::fs::read_to_string(dir.join("lib.typ")).unwrap(),
        LIB,
        "conteúdo extraído"
    );

    let entry = store.entry(&spec(DEMO));
    assert_eq!(entry.spec, "preview/demo/0.1.0");
    assert_eq!(entry.state, State::Available);
    let sha = entry.sha256.expect("hash gravado no índice");
    assert_eq!(sha.len(), 64);
    assert!(sha.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    assert!(entry.downloaded_at.is_some());

    let index: serde_json::Value = serde_json::from_slice(
        &std::fs::read(data.dir().join("packages").join(INDEX_FILE)).unwrap(),
    )
    .unwrap();
    assert_eq!(index["preview/demo/0.1.0"]["sha256"], sha.as_str());
    assert!(data.written_outside_cache().is_empty());
}

#[test]
fn url_pedida_contem_so_nome_e_versao() {
    let data = AppData::new();
    let fetcher = FakeFetcher::serving(DEMO_URL, make_targz(&demo_entries()));
    let project = importing(DEMO);
    let mut world = world_with(&project, Arc::new(data.store(&fetcher)));
    let out = world.compile();
    assert!(out.output.is_some(), "{:?}", out.diagnostics);
    assert_eq!(REGISTRY_URL, "https://packages.typst.org");
    assert_eq!(fetcher.asked(), vec![DEMO_URL.to_owned()]);
    let root = project.root();
    let name = root.file_name().unwrap().to_string_lossy().into_owned();
    for url in fetcher.asked() {
        assert!(!url.contains(&name), "a URL não leva nada do projeto: {url}");
        assert!(!url.contains("main"), "a URL não leva nada do projeto: {url}");
    }
}

#[test]
fn pacote_no_cache_compila_depois_sem_rede() {
    let data = AppData::new();
    let fetcher = FakeFetcher::serving(DEMO_URL, make_targz(&demo_entries()));
    data.store(&fetcher).resolve(&spec(DEMO)).unwrap();

    fetcher.offline();
    let asked = fetcher.asked().len();
    // Sessão nova: confere o hash e usa o cache, sem rede.
    let project = importing(DEMO);
    let mut world = world_with(&project, Arc::new(data.store(&fetcher)));
    let out = world.compile();
    assert!(out.output.is_some(), "{:?}", out.diagnostics);
    assert_eq!(fetcher.asked().len(), asked, "nada foi baixado de novo");
}

#[test]
fn ausente_sem_rede_e_pacote_indisponivel() {
    let data = AppData::new();
    let fetcher = FakeFetcher::default();
    let store = data.store(&fetcher);
    assert_eq!(
        store.resolve(&spec(DEMO)),
        Err(PackageRefusal::Unavailable { spec: DEMO.into() })
    );
    assert_eq!(store.entry(&spec(DEMO)).state, State::Absent);

    let project = importing(DEMO);
    let mut world = world_with(&project, Arc::new(data.store(&fetcher)));
    let out = world.compile();
    assert!(out.output.is_none());
    let diag = out
        .diagnostics
        .iter()
        .find(|d| d.kind == Kind::Package)
        .unwrap_or_else(|| panic!("diagnóstico de pacote: {:?}", out.diagnostics));
    assert_eq!(
        diag.message,
        "pacote `@preview/demo:0.1.0` não está no computador e não há conexão"
    );
    assert_eq!(diag.path.as_deref(), Some("main.typ"));
    assert_eq!(diag.line, Some(1));
}

#[test]
fn namespace_local_nao_e_suportado_e_nada_e_pedido() {
    let data = AppData::new();
    let fetcher = FakeFetcher::default();
    let store = data.store(&fetcher);
    assert_eq!(
        store.resolve(&spec("@local/x:1.0.0")),
        Err(PackageRefusal::UnsupportedNamespace {
            namespace: "local".into()
        })
    );

    let project = importing("@local/x:1.0.0");
    let mut world = world_with(&project, Arc::new(data.store(&fetcher)));
    let out = world.compile();
    let diag = out
        .diagnostics
        .iter()
        .find(|d| d.kind == Kind::Package)
        .unwrap_or_else(|| panic!("diagnóstico de pacote: {:?}", out.diagnostics));
    assert_eq!(
        diag.message,
        "pacotes `@local` não são suportados; copie o pacote para o projeto"
    );
    assert!(fetcher.asked().is_empty(), "nenhum pedido à rede");
}

// ------------------------------------------------------------------- validação (FR-018)

/// Confere que o pacote foi recusado por inteiro e que nada ficou gravado.
fn assert_rejected(case: &str, bytes: Vec<u8>, limits: Limits) {
    let data = AppData::new();
    let fetcher = FakeFetcher::serving(DEMO_URL, bytes);
    let store = data.store(&fetcher).with_limits(limits);
    match store.resolve(&spec(DEMO)) {
        Err(PackageRefusal::Rejected { spec, .. }) => assert_eq!(spec, DEMO, "{case}"),
        other => panic!("{case}: pacote deveria ser recusado, veio {other:?}"),
    }
    assert_eq!(store.entry(&spec(DEMO)).state, State::Rejected, "{case}");
    assert_eq!(store.entry(&spec(DEMO)).sha256, None, "{case}: fora do índice");
    assert!(!data.package_dir().exists(), "{case}: pasta do pacote criada");
    assert_eq!(
        data.cached_files(),
        Vec::<PathBuf>::new(),
        "{case}: sobrou arquivo no cache"
    );
    assert_eq!(
        data.written_outside_cache(),
        Vec::<PathBuf>::new(),
        "{case}: gravou fora de <cache>/packages/"
    );
}

fn with_extra(entry: TarEntry) -> Vec<u8> {
    let mut entries = demo_entries();
    entries.push(entry);
    make_targz(&entries)
}

#[test]
fn arquivo_compactado_malformado_e_recusado() {
    assert_rejected("gzip inválido", malformed_targz(), Limits::DEFAULT);
    assert_rejected(
        "tar corrompido",
        gzip(b"isto nao e um tar, mas tem bytes suficientes para um cabecalho? nao"),
        Limits::DEFAULT,
    );
    let mut truncated = make_targz(&demo_entries());
    truncated.truncate(truncated.len() / 2);
    assert_rejected("gzip truncado", truncated, Limits::DEFAULT);
}

#[test]
fn entrada_com_ponto_ponto_e_recusada() {
    assert_rejected(
        "..",
        with_extra(TarEntry::file("../../vizinha/fora.typ", "fora")),
        Limits::DEFAULT,
    );
    assert_rejected(
        ".. no meio",
        with_extra(TarEntry::file("dados/../../fora.typ", "fora")),
        Limits::DEFAULT,
    );
}

#[test]
fn caminho_absoluto_e_recusado() {
    let target = AppData::new().tmp.path().join("vizinha/absoluto.typ");
    assert_rejected(
        "absoluto",
        with_extra(TarEntry::file(target.to_str().unwrap(), "fora")),
        Limits::DEFAULT,
    );
    assert_rejected(
        "unidade do Windows",
        with_extra(TarEntry::file("C:\\fora.typ", "fora")),
        Limits::DEFAULT,
    );
}

#[test]
fn links_e_arquivos_especiais_sao_recusados() {
    assert_rejected(
        "link simbólico",
        with_extra(TarEntry::Symlink {
            path: "atalho".into(),
            target: "/etc/passwd".into(),
        }),
        Limits::DEFAULT,
    );
    assert_rejected(
        "link simbólico relativo",
        with_extra(TarEntry::Symlink {
            path: "atalho".into(),
            target: "lib.typ".into(),
        }),
        Limits::DEFAULT,
    );
    assert_rejected(
        "link físico",
        with_extra(TarEntry::HardLink {
            path: "duro".into(),
            target: "lib.typ".into(),
        }),
        Limits::DEFAULT,
    );
    assert_rejected(
        "dispositivo",
        with_extra(TarEntry::CharDevice {
            path: "dispositivo".into(),
        }),
        Limits::DEFAULT,
    );
    assert_rejected(
        "FIFO",
        with_extra(TarEntry::Fifo {
            path: "fila".into(),
        }),
        Limits::DEFAULT,
    );
}

#[test]
fn limites_de_tamanho_e_de_arquivos_recusam_o_pacote() {
    let bytes = make_targz(&demo_entries());
    assert_rejected(
        "compactado acima do limite",
        bytes.clone(),
        Limits {
            compressed: bytes.len() as u64 - 1,
            ..Limits::DEFAULT
        },
    );
    let extracted: u64 = (TOML.len() + LIB.len() + "nota do pacote".len()) as u64;
    assert_rejected(
        "extraído acima do limite",
        bytes.clone(),
        Limits {
            extracted: extracted - 1,
            ..Limits::DEFAULT
        },
    );
    assert_rejected(
        "arquivos demais",
        bytes,
        Limits {
            files: 3,
            ..Limits::DEFAULT
        },
    );
}

#[test]
fn limites_de_producao() {
    assert_eq!(
        Limits::DEFAULT,
        Limits {
            compressed: 50 * 1024 * 1024,
            extracted: 200 * 1024 * 1024,
            files: 5_000,
        }
    );
}

#[test]
fn pacote_recusado_vira_diagnostico_de_pacote() {
    let data = AppData::new();
    let fetcher = FakeFetcher::serving(
        DEMO_URL,
        with_extra(TarEntry::file("../fora.typ", "fora")),
    );
    let project = importing(DEMO);
    let mut world = world_with(&project, Arc::new(data.store(&fetcher)));
    let out = world.compile();
    assert!(out.output.is_none());
    let diag = out
        .diagnostics
        .iter()
        .find(|d| d.kind == Kind::Package)
        .unwrap_or_else(|| panic!("diagnóstico de pacote: {:?}", out.diagnostics));
    assert!(
        diag.message.starts_with("o pacote `@preview/demo:0.1.0` foi recusado"),
        "{diag:?}"
    );
}

// ------------------------------------------------------------------- adulteração

#[test]
fn pacote_alterado_no_cache_e_adulterado_e_baixar_de_novo_resolve() {
    let data = AppData::new();
    let fetcher = FakeFetcher::serving(DEMO_URL, make_targz(&demo_entries()));
    data.store(&fetcher).resolve(&spec(DEMO)).unwrap();
    std::fs::write(data.package_dir().join("lib.typ"), "#let saudacao = [mal]\n").unwrap();

    // Sessão nova: o hash não bate.
    let store = Arc::new(data.store(&fetcher));
    assert_eq!(
        store.resolve(&spec(DEMO)),
        Err(PackageRefusal::Tampered { spec: DEMO.into() })
    );
    assert_eq!(store.entry(&spec(DEMO)).state, State::Tampered);

    let project = importing(DEMO);
    let mut world = world_with(&project, store.clone());
    let out = world.compile();
    assert!(out.output.is_none());
    let diag = out
        .diagnostics
        .iter()
        .find(|d| d.kind == Kind::Package)
        .unwrap_or_else(|| panic!("diagnóstico de pacote: {:?}", out.diagnostics));
    assert_eq!(
        diag.message,
        "o pacote `@preview/demo:0.1.0` foi alterado depois do download"
    );

    // "Baixar de novo" volta ao conteúdo do registro, e a mesma sessão compila.
    store.redownload(&spec(DEMO)).expect("baixado de novo");
    assert_eq!(store.entry(&spec(DEMO)).state, State::Available);
    assert_eq!(
        std::fs::read_to_string(data.package_dir().join("lib.typ")).unwrap(),
        LIB
    );
    let out = world.compile();
    assert!(out.output.is_some(), "{:?}", out.diagnostics);
}

#[test]
fn arquivo_novo_ou_link_no_cache_tambem_e_adulteracao() {
    let data = AppData::new();
    let fetcher = FakeFetcher::serving(DEMO_URL, make_targz(&demo_entries()));
    data.store(&fetcher).resolve(&spec(DEMO)).unwrap();
    std::fs::write(data.package_dir().join("extra.typ"), "novo").unwrap();
    assert_eq!(
        data.store(&fetcher).resolve(&spec(DEMO)),
        Err(PackageRefusal::Tampered { spec: DEMO.into() })
    );

    let data = AppData::new();
    data.store(&fetcher).resolve(&spec(DEMO)).unwrap();
    let outside = Outside::new();
    std::fs::remove_file(data.package_dir().join("dados/nota.txt")).unwrap();
    if symlink(&outside.secret(), &data.package_dir().join("dados/nota.txt")).is_err() {
        eprintln!("sem permissão para criar link; parte do teste pulada");
        return;
    }
    assert_eq!(
        data.store(&fetcher).resolve(&spec(DEMO)),
        Err(PackageRefusal::Tampered { spec: DEMO.into() })
    );
}

// ------------------------------------------------------------------- escopo do pacote

#[test]
fn arquivo_de_pacote_fora_da_pasta_do_pacote_e_fora_do_projeto() {
    let data = AppData::new();
    let outside = Outside::new();
    let mut entries = demo_entries();
    entries[1] = TarEntry::file(
        "lib.typ",
        "#let saudacao = [#read(\"dados/nota.txt\")]\n",
    );
    let fetcher = FakeFetcher::serving(DEMO_URL, make_targz(&entries));
    let store = Arc::new(data.store(&fetcher));
    let project = importing(DEMO);
    let mut world = world_with(&project, store.clone());
    let out = world.compile();
    assert!(out.output.is_some(), "{:?}", out.diagnostics);

    // Já verificado nesta sessão: um link trocado depois aponta para fora da pasta do pacote.
    let note = data.package_dir().join("dados/nota.txt");
    std::fs::remove_file(&note).unwrap();
    if symlink(&outside.secret(), &note).is_err() {
        eprintln!("sem permissão para criar link; teste pulado");
        return;
    }
    let log = AccessLog::default();
    let recorder = log.clone();
    let env = WorldEnv {
        packages: Some(store),
        ..WorldEnv::default()
    };
    let scope = prelo_lib::fs::scope::Scope::new(project.root()).unwrap();
    let access = prelo_lib::fs::access_model::AccessModel::with_env(scope, env);
    let mut world = ProjectWorld::instrumented(access, "main.typ", move |m| {
        let mut r = RecordingAccess::new(m);
        r.log = recorder;
        r
    })
    .unwrap();
    let out = world.compile();
    assert!(out.output.is_none());
    let diag = out
        .diagnostics
        .iter()
        .find(|d| d.kind == Kind::Access)
        .unwrap_or_else(|| panic!("diagnóstico de acesso: {:?}", out.diagnostics));
    assert_eq!(
        diag.message,
        "acesso recusado: `dados/nota.txt` fica fora da pasta do projeto"
    );
    for d in &out.diagnostics {
        assert!(!d.message.contains(SECRET), "conteúdo de fora vazou: {d:?}");
    }
    assert!(
        log.all().iter().all(|p| !p.starts_with(outside.root())),
        "pedido de leitura fora: {:?}",
        log.all()
    );
}
