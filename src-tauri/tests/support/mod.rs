//! Apoio dos testes de integração da feature 003 (T009).
//!
//! - projeto temporário a partir de fixture (`tests/fixtures/<nome>`) ou vazio;
//! - criação de link simbólico (Unix e Windows) e de FIFO (Unix);
//! - gerador de `.tar.gz` com entradas arbitrárias, inclusive maliciosas (`..`, caminho
//!   absoluto, link simbólico, link físico, dispositivo), para os testes de pacotes;
//! - `RecordingAccess`: `PathAccessModel` instrumentado que registra toda abertura de
//!   arquivo, para os testes de escape conferirem zero aberturas fora da raiz.
//!
//! Links e FIFO são gerados em teste, não versionados (git e Windows; tasks.md, Notes).
#![allow(dead_code)]

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use flate2::write::GzEncoder;
use flate2::Compression;
use tempfile::TempDir;
use tinymist_vfs::{Bytes, PathAccessModel};
use typst::diag::FileResult;

/// Pasta das fixtures versionadas.
pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// Projeto vazio numa pasta temporária. A raiz real (canônica) fica em `root()`.
pub struct TempProject {
    dir: TempDir,
}

impl TempProject {
    pub fn empty() -> Self {
        Self {
            dir: tempfile::Builder::new()
                .prefix("prelo-teste-")
                .tempdir()
                .expect("pasta temporária"),
        }
    }

    /// Copia `tests/fixtures/<nome>` para uma pasta temporária.
    pub fn from_fixture(name: &str) -> Self {
        let project = Self::empty();
        copy_dir(&fixtures_dir().join(name), project.dir.path()).expect("copiar fixture");
        project
    }

    /// Raiz canônica (no macOS, `/var` → `/private/var`).
    pub fn root(&self) -> PathBuf {
        self.dir.path().canonicalize().expect("raiz canônica")
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.root().join(rel)
    }

    pub fn write(&self, rel: &str, content: impl AsRef<[u8]>) -> PathBuf {
        let path = self.path(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("criar pasta");
        }
        std::fs::write(&path, content).expect("gravar arquivo");
        path
    }
}

fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Pasta temporária fora do projeto com um arquivo "secreto" que nenhum teste pode ler.
pub struct Outside {
    dir: TempDir,
}

pub const SECRET: &str = "conteúdo secreto fora do projeto";

impl Outside {
    pub fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("prelo-fora-")
            .tempdir()
            .expect("pasta temporária");
        std::fs::write(dir.path().join("segredo.txt"), SECRET).expect("gravar segredo");
        Self { dir }
    }

    pub fn root(&self) -> PathBuf {
        self.dir.path().canonicalize().expect("raiz canônica")
    }

    pub fn secret(&self) -> PathBuf {
        self.root().join("segredo.txt")
    }
}

/// Cria link simbólico `link` → `target`. No Windows, criar link pode exigir modo de
/// desenvolvedor; o erro volta para o teste decidir.
pub fn symlink(target: &Path, link: &Path) -> io::Result<()> {
    if let Some(parent) = link.parent() {
        std::fs::create_dir_all(parent)?;
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(windows)]
    {
        if target.is_dir() {
            std::os::windows::fs::symlink_dir(target, link)
        } else {
            std::os::windows::fs::symlink_file(target, link)
        }
    }
}

/// Cria um FIFO (pipe nomeado). Abrir para leitura sem `O_NONBLOCK` bloquearia o teste.
#[cfg(unix)]
pub fn mkfifo(path: &Path) -> io::Result<()> {
    let status = std::process::Command::new("mkfifo").arg(path).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("mkfifo falhou: {status}")))
    }
}

/// Entrada de um `.tar.gz` de teste. Os nomes são gravados crus no cabeçalho, sem a
/// validação do `tar::Header::set_path`, para gerar entradas maliciosas.
#[derive(Debug, Clone)]
pub enum TarEntry {
    File { path: String, data: Vec<u8> },
    Dir { path: String },
    Symlink { path: String, target: String },
    HardLink { path: String, target: String },
    CharDevice { path: String },
    Fifo { path: String },
}

impl TarEntry {
    pub fn file(path: &str, data: impl AsRef<[u8]>) -> Self {
        Self::File {
            path: path.into(),
            data: data.as_ref().to_vec(),
        }
    }
}

fn raw_header(path: &str, kind: tar::EntryType, size: u64) -> tar::Header {
    let mut header = tar::Header::new_old();
    let name = path.as_bytes();
    assert!(
        name.len() < 100,
        "nome de entrada longo demais para o teste"
    );
    header.as_old_mut().name[..name.len()].copy_from_slice(name);
    header.set_entry_type(kind);
    header.set_size(size);
    header.set_mode(0o644);
    header.set_mtime(0);
    header
}

fn set_raw_link(header: &mut tar::Header, target: &str) {
    let bytes = target.as_bytes();
    assert!(
        bytes.len() < 100,
        "destino de link longo demais para o teste"
    );
    header.as_old_mut().linkname[..bytes.len()].copy_from_slice(bytes);
}

/// Gera os bytes de um `.tar.gz` com as entradas dadas, na ordem.
pub fn make_targz(entries: &[TarEntry]) -> Vec<u8> {
    let mut builder = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::fast()));
    for entry in entries {
        let (mut header, data): (tar::Header, &[u8]) = match entry {
            TarEntry::File { path, data } => (
                raw_header(path, tar::EntryType::Regular, data.len() as u64),
                data,
            ),
            TarEntry::Dir { path } => (raw_header(path, tar::EntryType::Directory, 0), &[]),
            TarEntry::Symlink { path, target } => {
                let mut h = raw_header(path, tar::EntryType::Symlink, 0);
                set_raw_link(&mut h, target);
                (h, &[])
            }
            TarEntry::HardLink { path, target } => {
                let mut h = raw_header(path, tar::EntryType::Link, 0);
                set_raw_link(&mut h, target);
                (h, &[])
            }
            TarEntry::CharDevice { path } => (raw_header(path, tar::EntryType::Char, 0), &[]),
            TarEntry::Fifo { path } => (raw_header(path, tar::EntryType::Fifo, 0), &[]),
        };
        header.set_cksum();
        builder.append(&header, data).expect("anexar entrada");
    }
    let encoder = builder.into_inner().expect("finalizar tar");
    encoder.finish().expect("finalizar gzip")
}

/// Bytes que não são gzip válido.
pub fn malformed_targz() -> Vec<u8> {
    let mut bytes = vec![0x1f, 0x8b, 0x08, 0x00];
    bytes.extend_from_slice(b"isto nao e um tar.gz");
    bytes
}

/// Gzip válido de bytes arbitrários (para `.tar` truncado/corrompido).
pub fn gzip(data: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(data).expect("gzip");
    encoder.finish().expect("gzip")
}

/// Registro compartilhado das aberturas de arquivo feitas pelo modelo instrumentado.
#[derive(Clone, Default)]
pub struct AccessLog(Arc<Mutex<Vec<PathBuf>>>);

impl AccessLog {
    pub fn record(&self, path: &Path) {
        self.0.lock().expect("log").push(path.to_path_buf());
    }

    pub fn all(&self) -> Vec<PathBuf> {
        self.0.lock().expect("log").clone()
    }

    /// Caminhos registrados cujo caminho real fica fora de `root`. Caminho que não existe
    /// é comparado como foi pedido (normalizado léxico do que der).
    pub fn outside(&self, root: &Path) -> Vec<PathBuf> {
        self.all()
            .into_iter()
            .filter(|p| {
                let real = p.canonicalize().unwrap_or_else(|_| p.clone());
                !real.starts_with(root)
            })
            .collect()
    }
}

/// `PathAccessModel` instrumentado: registra cada caminho pedido ao modelo de baixo, antes
/// de delegar. Os testes de escape exigem zero registros fora da raiz. Clonar compartilha o
/// registro (o VFS do Tinymist clona o modelo a cada instantâneo do World).
#[derive(Clone)]
pub struct RecordingAccess<M> {
    pub inner: M,
    pub log: AccessLog,
}

impl<M: PathAccessModel> RecordingAccess<M> {
    pub fn new(inner: M) -> Self {
        Self {
            inner,
            log: AccessLog::default(),
        }
    }
}

impl<M: PathAccessModel> PathAccessModel for RecordingAccess<M> {
    fn reset(&mut self) {
        self.inner.reset();
    }

    fn content(&self, src: &Path) -> FileResult<Bytes> {
        self.log.record(src);
        self.inner.content(src)
    }
}

/// Modelo ingênuo (lê qualquer caminho), só para testar a instrumentação.
pub struct NaiveAccess;

impl PathAccessModel for NaiveAccess {
    fn content(&self, src: &Path) -> FileResult<Bytes> {
        std::fs::read(src)
            .map(Bytes::new)
            .map_err(|e| typst::diag::FileError::from_io(e, src))
    }
}
