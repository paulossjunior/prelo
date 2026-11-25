//! Pacotes `@preview` (FR-017, FR-018; research R7; data-model.md "PackageCacheEntry";
//! contracts/file-access.md "Pacotes (download)").
//!
//! O [`PackageStore`] guarda os pacotes em `<dados locais do app>/packages/preview/<nome>/
//! <versão>/`, fora do projeto e fora do cache do `typst` de linha de comando. Na falta do
//! pacote, baixa `https://packages.typst.org/preview/<nome>-<versão>.tar.gz` pelo [`Fetcher`]
//! (em produção, [`HttpFetcher`]: `ureq` com TLS `rustls` e os certificados do sistema). Só
//! nome e versão saem na URL (Princípio II).
//!
//! O arquivo baixado é extraído numa pasta temporária dentro de `<cache>/packages/`, entrada
//! por entrada: só arquivo regular ou pasta, caminho relativo sem `..`, dentro dos limites
//! ([`Limits`]). Qualquer outra coisa recusa o pacote inteiro e apaga a pasta temporária.
//! Aceito, o pacote ganha o hash SHA-256 da árvore extraída, vai para o destino por `rename`
//! e entra no índice `index.json`. Na primeira resolução de cada pacote por sessão o hash é
//! recalculado: se não bate, o pacote está adulterado e é recusado até "Baixar de novo".
//!
//! Os arquivos do pacote são lidos pelo mesmo modelo de acesso do projeto, com a pasta do
//! pacote como raiz (RF-08; [`crate::fs::access_model`]).

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tinymist_world::package::registry::PackageRegistry;
use typst::diag::PackageError;
use typst::syntax::package::PackageSpec;

use crate::fs::save::format_utc;

/// Endereço do registro de pacotes do Typst.
pub const REGISTRY_URL: &str = "https://packages.typst.org";
/// Único namespace aceito (D-13): `@local` e outros pedem cópia para o projeto.
pub const PREVIEW: &str = "preview";
/// Pasta do cache de pacotes dentro dos dados locais do app.
pub const PACKAGES_DIR: &str = "packages";
/// Índice de hashes dentro de [`PACKAGES_DIR`].
pub const INDEX_FILE: &str = "index.json";

const MIB: u64 = 1024 * 1024;

/// Limites do download (data-model.md): 50 MB compactado, 200 MB e 5 000 arquivos extraídos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Bytes do `.tar.gz` baixado.
    pub compressed: u64,
    /// Soma dos bytes extraídos.
    pub extracted: u64,
    /// Entradas extraídas (arquivos e pastas).
    pub files: usize,
}

impl Limits {
    pub const DEFAULT: Limits = Limits {
        compressed: 50 * MIB,
        extracted: 200 * MIB,
        files: 5_000,
    };
}

impl Default for Limits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Falha ao buscar o arquivo do pacote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchError {
    /// Sem conexão, servidor fora do ar ou resposta de erro (404 inclusive).
    Unavailable(String),
    /// A resposta passou do limite de bytes pedido.
    TooLarge,
}

/// Quem busca os bytes de uma URL. Os testes injetam um falso, sem rede real.
pub trait Fetcher: Send + Sync {
    /// Bytes de `url`, até `limit` bytes; acima disso, [`FetchError::TooLarge`].
    fn fetch(&self, url: &str, limit: u64) -> Result<Vec<u8>, FetchError>;
}

/// Busca por HTTPS com `ureq` + `rustls` (`ring`) e as raízes de certificado do sistema
/// (`rustls-native-certs`), sem `webpki-roots` (research R7, R12). Só HTTPS, limite de tempo.
pub struct HttpFetcher {
    agent: ureq::Agent,
}

/// Prazo total de um download.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60);

impl HttpFetcher {
    pub fn new() -> Self {
        let native = rustls_native_certs::load_native_certs();
        for error in &native.errors {
            eprintln!("prelo: certificado do sistema ignorado: {error}");
        }
        let roots: Vec<ureq::tls::Certificate<'static>> = native
            .certs
            .iter()
            .map(|der| ureq::tls::Certificate::from_der(der.as_ref()).to_owned())
            .collect();
        let tls = ureq::tls::TlsConfig::builder()
            .provider(ureq::tls::TlsProvider::Rustls)
            .root_certs(ureq::tls::RootCerts::new_with_certs(&roots))
            .unversioned_rustls_crypto_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .build();
        let config = ureq::Agent::config_builder()
            .tls_config(tls)
            .https_only(true)
            .timeout_global(Some(DOWNLOAD_TIMEOUT))
            .user_agent("prelo")
            .build();
        Self {
            agent: config.into(),
        }
    }
}

impl Default for HttpFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Fetcher for HttpFetcher {
    fn fetch(&self, url: &str, limit: u64) -> Result<Vec<u8>, FetchError> {
        let mut response = self
            .agent
            .get(url)
            .call()
            .map_err(|e| FetchError::Unavailable(e.to_string()))?;
        match response
            .body_mut()
            .with_config()
            .limit(limit.saturating_add(1))
            .read_to_vec()
        {
            Ok(bytes) if bytes.len() as u64 > limit => Err(FetchError::TooLarge),
            Ok(bytes) => Ok(bytes),
            Err(ureq::Error::BodyExceedsLimit(_)) => Err(FetchError::TooLarge),
            Err(e) => Err(FetchError::Unavailable(e.to_string())),
        }
    }
}

/// Estado de um pacote (data-model.md): `Ausente → Baixando → Disponível`; validação falha →
/// `Recusado`; hash diferente → `Adulterado` → (baixar de novo) → `Baixando`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Absent,
    Downloading,
    Available,
    Rejected,
    Tampered,
}

/// Entrada do cache (`PackageCacheEntry`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageCacheEntry {
    /// `preview/<nome>/<versão>`.
    pub spec: String,
    /// Hash da árvore extraída, se o pacote está no índice.
    pub sha256: Option<String>,
    pub downloaded_at: Option<String>,
    pub state: State,
}

/// Recusa de pacote, com a mensagem do contrato (contracts/file-access.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageRefusal {
    /// `PacoteAdulterado`.
    Tampered { spec: String },
    /// `PacoteIndisponivel`.
    Unavailable { spec: String },
    /// `NamespaceNaoSuportado`.
    UnsupportedNamespace { namespace: String },
    /// O arquivo baixado não passou na validação (FR-018).
    Rejected { spec: String, reason: String },
}

impl fmt::Display for PackageRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tampered { spec } => {
                write!(f, "o pacote `{spec}` foi alterado depois do download")
            }
            Self::Unavailable { spec } => {
                write!(f, "pacote `{spec}` não está no computador e não há conexão")
            }
            Self::UnsupportedNamespace { namespace } => write!(
                f,
                "pacotes `@{namespace}` não são suportados; copie o pacote para o projeto"
            ),
            Self::Rejected { spec, reason } => {
                write!(f, "o pacote `{spec}` foi recusado: {reason}")
            }
        }
    }
}

impl std::error::Error for PackageRefusal {}

/// Entrada gravada no `index.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexEntry {
    sha256: String,
    downloaded_at: String,
}

type Index = BTreeMap<String, IndexEntry>;

/// Cache de pacotes `@preview` com download e verificação.
pub struct PackageStore {
    dir: PathBuf,
    fetcher: Arc<dyn Fetcher>,
    limits: Limits,
    /// Estado de cada pacote nesta sessão (`preview/<nome>/<versão>` → estado).
    states: Mutex<HashMap<String, State>>,
    /// Um download (ou verificação) de cada vez.
    work: Mutex<()>,
    /// Muda a cada "Baixar de novo": o World relê pacotes e arquivos.
    generation: AtomicU64,
}

impl fmt::Debug for PackageStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PackageStore")
            .field("dir", &self.dir)
            .finish_non_exhaustive()
    }
}

impl PackageStore {
    /// Cache em `<data_dir>/packages/` (criada se faltar).
    pub fn new(data_dir: &Path, fetcher: Arc<dyn Fetcher>) -> io::Result<Self> {
        let dir = data_dir.join(PACKAGES_DIR);
        fs::create_dir_all(&dir)?;
        Ok(Self {
            dir: fs::canonicalize(dir)?,
            fetcher,
            limits: Limits::DEFAULT,
            states: Mutex::default(),
            work: Mutex::default(),
            generation: AtomicU64::new(0),
        })
    }

    /// Troca os limites (os testes usam limites pequenos; os de produção são [`Limits::DEFAULT`]).
    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    /// `<dados locais>/packages`, canônica.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Contador de "Baixar de novo".
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    /// Pasta do pacote no cache, baixando e validando se preciso.
    pub fn resolve(&self, spec: &PackageSpec) -> Result<PathBuf, PackageRefusal> {
        let (key, dest) = self.locate(spec)?;
        let _work = lock(&self.work);
        match self.state(&key) {
            State::Available if dest.is_dir() => return Ok(dest),
            State::Tampered => return Err(tampered(spec)),
            State::Rejected => return Err(self.rejected(spec, "o download anterior foi recusado")),
            _ => {}
        }
        match read_index(&self.dir).get(&key) {
            Some(entry) if dest.is_dir() => {
                // Primeira resolução nesta sessão: confere o hash gravado no download.
                if tree_hash(&dest).ok().as_deref() == Some(entry.sha256.as_str()) {
                    self.set_state(&key, State::Available);
                    Ok(dest)
                } else {
                    self.set_state(&key, State::Tampered);
                    Err(tampered(spec))
                }
            }
            _ => self.download(spec, &key, &dest),
        }
    }

    /// "Baixar de novo": apaga o pacote do cache e baixa outra vez.
    pub fn redownload(&self, spec: &PackageSpec) -> Result<PathBuf, PackageRefusal> {
        let (key, dest) = self.locate(spec)?;
        let result = {
            let _work = lock(&self.work);
            self.download(spec, &key, &dest)
        };
        self.generation.fetch_add(1, Ordering::SeqCst);
        result
    }

    /// Estado e índice de `spec`.
    pub fn entry(&self, spec: &PackageSpec) -> PackageCacheEntry {
        let key = index_key(spec);
        let indexed = read_index(&self.dir).remove(&key);
        PackageCacheEntry {
            state: self.state(&key),
            sha256: indexed.as_ref().map(|e| e.sha256.clone()),
            downloaded_at: indexed.map(|e| e.downloaded_at),
            spec: key,
        }
    }

    /// Chave do índice e pasta de destino; recusa namespace e nomes fora do formato.
    fn locate(&self, spec: &PackageSpec) -> Result<(String, PathBuf), PackageRefusal> {
        if spec.namespace != PREVIEW {
            return Err(PackageRefusal::UnsupportedNamespace {
                namespace: spec.namespace.to_string(),
            });
        }
        let valid = !spec.name.is_empty()
            && spec
                .name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if !valid {
            return Err(self.rejected(spec, "nome de pacote inválido"));
        }
        let dest = self
            .dir
            .join(PREVIEW)
            .join(spec.name.as_str())
            .join(spec.version.to_string());
        Ok((index_key(spec), dest))
    }

    fn state(&self, key: &str) -> State {
        lock(&self.states)
            .get(key)
            .copied()
            .unwrap_or(State::Absent)
    }

    fn set_state(&self, key: &str, state: State) {
        lock(&self.states).insert(key.to_owned(), state);
    }

    fn rejected(&self, spec: &PackageSpec, reason: &str) -> PackageRefusal {
        PackageRefusal::Rejected {
            spec: display_spec(spec),
            reason: reason.to_owned(),
        }
    }

    fn download(
        &self,
        spec: &PackageSpec,
        key: &str,
        dest: &Path,
    ) -> Result<PathBuf, PackageRefusal> {
        self.set_state(key, State::Downloading);
        let result = self.download_inner(spec, key, dest);
        self.set_state(
            key,
            match &result {
                Ok(_) => State::Available,
                Err(PackageRefusal::Unavailable { .. }) => State::Absent,
                Err(_) => State::Rejected,
            },
        );
        result
    }

    fn download_inner(
        &self,
        spec: &PackageSpec,
        key: &str,
        dest: &Path,
    ) -> Result<PathBuf, PackageRefusal> {
        // Só nome e versão saem na URL (FR-017, Princípio II).
        let url = format!(
            "{REGISTRY_URL}/{PREVIEW}/{}-{}.tar.gz",
            spec.name, spec.version
        );
        let bytes = match self.fetcher.fetch(&url, self.limits.compressed) {
            Ok(bytes) if bytes.len() as u64 <= self.limits.compressed => bytes,
            Ok(_) | Err(FetchError::TooLarge) => {
                return Err(self.rejected(spec, "arquivo compactado acima de 50 MB"))
            }
            Err(FetchError::Unavailable(_)) => {
                return Err(PackageRefusal::Unavailable {
                    spec: display_spec(spec),
                })
            }
        };
        let io_rejected =
            |e: io::Error| self.rejected(spec, &format!("falha ao gravar no cache ({e})"));

        // Pasta temporária dentro do cache: apagada ao sair, se não for aceita.
        let staging = tempfile::Builder::new()
            .prefix(".baixando-")
            .tempdir_in(&self.dir)
            .map_err(io_rejected)?;
        extract(&bytes, staging.path(), self.limits)
            .map_err(|reason| self.rejected(spec, &reason))?;
        let sha256 = tree_hash(staging.path())
            .map_err(|_| self.rejected(spec, "conteúdo extraído inválido"))?;

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(io_rejected)?;
        }
        if fs::symlink_metadata(dest).is_ok() {
            remove_any(dest).map_err(io_rejected)?;
        }
        let mut index = read_index(&self.dir);
        index.remove(key);
        write_index(&self.dir, &index).map_err(io_rejected)?;
        fs::rename(staging.path(), dest).map_err(io_rejected)?;
        // Já movida: a pasta temporária não existe mais para ser apagada.
        let _ = staging.keep();
        index.insert(
            key.to_owned(),
            IndexEntry {
                sha256,
                downloaded_at: format_utc(SystemTime::now()),
            },
        );
        write_index(&self.dir, &index).map_err(io_rejected)?;
        Ok(dest.to_path_buf())
    }
}

/// `preview/<nome>/<versão>`.
fn index_key(spec: &PackageSpec) -> String {
    format!("{}/{}/{}", spec.namespace, spec.name, spec.version)
}

/// `@preview/<nome>:<versão>`, como no `.typ`.
fn display_spec(spec: &PackageSpec) -> String {
    format!("@{}/{}:{}", spec.namespace, spec.name, spec.version)
}

fn tampered(spec: &PackageSpec) -> PackageRefusal {
    PackageRefusal::Tampered {
        spec: display_spec(spec),
    }
}

fn read_index(dir: &Path) -> Index {
    fs::read(dir.join(INDEX_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Grava o índice de forma atômica (temporário na mesma pasta + `rename`).
fn write_index(dir: &Path, index: &Index) -> io::Result<()> {
    let mut file = tempfile::Builder::new()
        .prefix(".index-")
        .tempfile_in(dir)?;
    serde_json::to_writer_pretty(&mut file, index)?;
    file.as_file().sync_all()?;
    file.persist(dir.join(INDEX_FILE))
        .map(|_| ())
        .map_err(|e| e.error)
}

/// Remove arquivo, link ou pasta sem seguir links.
fn remove_any(path: &Path) -> io::Result<()> {
    if fs::symlink_metadata(path)?.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// Extrai `bytes` (`.tar.gz`) em `into`, validando cada entrada (FR-018). Devolve o motivo
/// da recusa; quem chama apaga a pasta.
fn extract(bytes: &[u8], into: &Path, limits: Limits) -> Result<(), String> {
    let malformed = |e: io::Error| format!("arquivo compactado malformado ({e})");
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(bytes));
    let mut count = 0usize;
    let mut total = 0u64;
    for entry in archive.entries().map_err(malformed)? {
        let mut entry = entry.map_err(malformed)?;
        let kind = entry.header().entry_type();
        // Cabeçalho global pax (gerado por `git archive`): só metadados, nada a gravar.
        if kind == tar::EntryType::XGlobalHeader {
            continue;
        }
        let raw = entry.path_bytes().into_owned();
        let name = String::from_utf8(raw).map_err(|_| "nome de entrada não é UTF-8".to_owned())?;
        let relative = safe_relative(&name)?;
        count += 1;
        if count > limits.files {
            return Err(format!("mais de {} arquivos", limits.files));
        }
        match kind {
            tar::EntryType::Directory => {
                if let Some(relative) = relative {
                    fs::create_dir_all(into.join(relative)).map_err(malformed)?;
                }
            }
            tar::EntryType::Regular | tar::EntryType::Continuous => {
                let relative = relative.ok_or_else(|| format!("entrada sem nome: `{name}`"))?;
                let size = entry.header().size().map_err(malformed)?;
                total = total.saturating_add(size);
                if total > limits.extracted {
                    return Err(format!(
                        "conteúdo extraído acima de {} MB",
                        limits.extracted / MIB
                    ));
                }
                let target = into.join(&relative);
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(malformed)?;
                }
                let mut file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&target)
                    .map_err(|_| format!("entrada repetida ou inválida: `{name}`"))?;
                let copied =
                    io::copy(&mut (&mut entry).take(size), &mut file).map_err(malformed)?;
                if copied != size {
                    return Err("arquivo compactado truncado".to_owned());
                }
                file.flush().map_err(malformed)?;
            }
            tar::EntryType::Symlink => return Err(format!("link simbólico: `{name}`")),
            tar::EntryType::Link => return Err(format!("link físico: `{name}`")),
            _ => return Err(format!("arquivo especial: `{name}`")),
        }
    }
    if total == 0 && fs::read_dir(into).map_or(true, |mut d| d.next().is_none()) {
        return Err("arquivo compactado vazio".to_owned());
    }
    Ok(())
}

/// Caminho relativo seguro de uma entrada: sem `..`, sem raiz, sem prefixo de unidade, sem
/// `\` nem `:` (mesma regra em todo sistema). `None` para `.` e `./`.
fn safe_relative(name: &str) -> Result<Option<PathBuf>, String> {
    let refuse = || Err(format!("caminho não permitido: `{name}`"));
    if name.starts_with('/') || name.contains('\\') || name.contains(':') || name.contains('\0') {
        return refuse();
    }
    let mut out = PathBuf::new();
    for component in Path::new(name).components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return refuse(),
        }
    }
    Ok((!out.as_os_str().is_empty()).then_some(out))
}

/// SHA-256 da árvore: caminhos relativos ordenados (com `/`) e conteúdo de cada arquivo.
/// Link, arquivo especial ou nome não UTF-8 → erro (árvore não confiável).
pub fn tree_hash(root: &Path) -> io::Result<String> {
    let mut files = Vec::new();
    collect(root, "", &mut files)?;
    files.sort();
    let mut hasher = Sha256::new();
    for (relative, path) in files {
        let mut content = Vec::new();
        File::open(&path)?.read_to_end(&mut content)?;
        hasher.update(relative.as_bytes());
        hasher.update([0]);
        hasher.update((content.len() as u64).to_le_bytes());
        hasher.update(&content);
    }
    Ok(hex(&hasher.finalize()))
}

fn collect(dir: &Path, prefix: &str, out: &mut Vec<(String, PathBuf)>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| io::Error::other("nome não UTF-8"))?;
        let relative = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let kind = entry.file_type()?;
        if kind.is_dir() {
            collect(&entry.path(), &relative, out)?;
        } else if kind.is_file() {
            out.push((relative, entry.path()));
        } else {
            return Err(io::Error::other("link ou arquivo especial"));
        }
    }
    Ok(())
}

/// Hexadecimal minúsculo.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Registro de pacotes do World: resolve pelo [`PackageStore`] e guarda as recusas para o
/// World reconhecê-las na mensagem do Typst (`Diagnostic.kind = package`).
#[derive(Default)]
pub struct PreloRegistry {
    store: Option<Arc<PackageStore>>,
    refusals: Mutex<HashMap<String, PackageRefusal>>,
}

impl PreloRegistry {
    pub fn new(store: Option<Arc<PackageStore>>) -> Self {
        Self {
            store,
            refusals: Mutex::default(),
        }
    }

    /// Recusa de pacote cuja mensagem aparece em `message`.
    pub fn refusal_for_message(&self, message: &str) -> Option<PackageRefusal> {
        lock(&self.refusals)
            .iter()
            .find(|(text, _)| message.contains(text.as_str()))
            .map(|(_, refusal)| refusal.clone())
    }

    /// Contador de "Baixar de novo" do cache (0 sem cache).
    pub fn generation(&self) -> u64 {
        self.store.as_ref().map_or(0, |s| s.generation())
    }

    fn refuse(&self, refusal: PackageRefusal) -> PackageError {
        let message = refusal.to_string();
        lock(&self.refusals).insert(message.clone(), refusal);
        PackageError::Other(Some(message.into()))
    }
}

impl PackageRegistry for PreloRegistry {
    fn resolve(&self, spec: &PackageSpec) -> Result<Arc<Path>, PackageError> {
        if spec.namespace != PREVIEW {
            return Err(self.refuse(PackageRefusal::UnsupportedNamespace {
                namespace: spec.namespace.to_string(),
            }));
        }
        let Some(store) = &self.store else {
            return Err(self.refuse(PackageRefusal::Unavailable {
                spec: display_spec(spec),
            }));
        };
        store
            .resolve(spec)
            .map(Arc::from)
            .map_err(|r| self.refuse(r))
    }
}
