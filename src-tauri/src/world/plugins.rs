//! Aprovação de plugins WASM (FR-019; research R8; data-model.md "PluginApproval";
//! contracts/file-access.md, passo 8).
//!
//! O World não sabe se um arquivo foi pedido por `plugin()` ou por `read()`, então o modelo
//! de acesso barra pela assinatura: bytes que começam com `\0asm` só são entregues ao
//! compilador se houver aprovação para (raiz canônica do projeto, SHA-256 do arquivo). Sem
//! aprovação, a leitura vira o diagnóstico "plugin aguardando sua confirmação" e o núcleo
//! avisa a interface (`plugin_approval_needed`); recusado, vira diagnóstico sem novo aviso.
//!
//! As decisões ficam em `<dados locais do app>/plugin-approvals.json`, nunca no projeto (um
//! coautor não pode pré-aprovar para os outros). Conteúdo alterado tem outro hash e pergunta
//! de novo. Arquivo corrompido vale como vazio e é reescrito, com aviso.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::packages::{hex, PackageStore};
use crate::fs::save::format_utc;

/// Nome do arquivo de decisões nos dados locais do app.
pub const APPROVALS_FILE: &str = "plugin-approvals.json";
/// Assinatura de módulo WASM.
pub const WASM_MAGIC: &[u8; 4] = b"\0asm";

/// Se `bytes` é um módulo WASM (pela assinatura).
pub fn is_wasm(bytes: &[u8]) -> bool {
    bytes.starts_with(WASM_MAGIC)
}

/// SHA-256 em hexadecimal minúsculo.
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// `PluginApproval.decision`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    Approved,
    Rejected,
}

/// Uma decisão gravada (`PluginApproval`). Chave: (`project_root`, `sha256`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginApproval {
    /// Raiz canônica do projeto.
    pub project_root: String,
    pub sha256: String,
    pub decision: Decision,
    pub decided_at: String,
    /// Caminho do plugin como foi pedido (só para a pessoa reconhecer a decisão).
    pub path: String,
}

/// Decisões de plugin deste computador.
pub struct PluginApprovals {
    file: PathBuf,
    entries: Mutex<Vec<PluginApproval>>,
    /// Muda a cada decisão: o World relê os arquivos barrados.
    generation: AtomicU64,
}

impl fmt::Debug for PluginApprovals {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PluginApprovals")
            .field("file", &self.file)
            .finish_non_exhaustive()
    }
}

impl PluginApprovals {
    /// Lê `<data_dir>/plugin-approvals.json`. Ausente → vazio; corrompido → vazio, com aviso, e
    /// reescrito vazio.
    pub fn load(data_dir: &Path) -> Self {
        let file = data_dir.join(APPROVALS_FILE);
        let entries = match fs::read(&file) {
            Err(_) => Vec::new(),
            Ok(bytes) => match serde_json::from_slice::<Vec<PluginApproval>>(&bytes) {
                Ok(entries) => entries,
                Err(e) => {
                    eprintln!(
                        "prelo: {APPROVALS_FILE} corrompido ({e}); as aprovações de plugin serão pedidas de novo"
                    );
                    if let Err(e) = write_atomic(&file, &[]) {
                        eprintln!("prelo: não foi possível reescrever {APPROVALS_FILE}: {e}");
                    }
                    Vec::new()
                }
            },
        };
        Self {
            file,
            entries: Mutex::new(entries),
            generation: AtomicU64::new(0),
        }
    }

    /// Decisão para o plugin de hash `sha256` no projeto de raiz canônica `project_root`.
    pub fn decision(&self, project_root: &Path, sha256: &str) -> Option<Decision> {
        let root = project_root.to_string_lossy();
        lock(&self.entries)
            .iter()
            .find(|e| e.project_root == root && e.sha256 == sha256)
            .map(|e| e.decision)
    }

    /// Grava a decisão (substitui a anterior da mesma chave), de forma atômica.
    pub fn decide(
        &self,
        project_root: &Path,
        sha256: &str,
        path: &str,
        decision: Decision,
    ) -> io::Result<()> {
        let root = project_root.to_string_lossy().into_owned();
        let mut entries = lock(&self.entries);
        let mut next = entries.clone();
        next.retain(|e| !(e.project_root == root && e.sha256 == sha256));
        next.push(PluginApproval {
            project_root: root,
            sha256: sha256.to_owned(),
            decision,
            decided_at: format_utc(SystemTime::now()),
            path: path.to_owned(),
        });
        write_atomic(&self.file, &next)?;
        *entries = next;
        self.generation.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Contador de decisões.
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }
}

fn write_atomic(file: &Path, entries: &[PluginApproval]) -> io::Result<()> {
    let dir = file.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(dir)?;
    let mut tmp = tempfile::Builder::new()
        .prefix(".plugin-approvals-")
        .tempfile_in(dir)?;
    serde_json::to_writer_pretty(&mut tmp, entries)?;
    tmp.as_file().sync_all()?;
    tmp.persist(file).map(|_| ()).map_err(|e| e.error)
}

/// Origem do plugin no evento `plugin_approval_needed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Origin {
    Project,
    Package,
}

/// Conteúdo do evento `plugin_approval_needed` (contracts/ipc.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRequest {
    /// Caminho relativo à raiz do projeto, ou à pasta do pacote.
    pub path: String,
    pub origin: Origin,
    /// `@preview/<nome>:<versão>` quando a origem é um pacote.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    pub size_bytes: u64,
    pub sha256: String,
}

/// Quem recebe os pedidos de confirmação (em produção, o evento para a interface).
pub type PluginSink = Arc<dyn Fn(PluginRequest) + Send + Sync>;

/// Recursos do World fora do projeto: decisões de plugin, aviso à interface e cache de
/// pacotes. Sem decisões, todo plugin fica barrado; sem cache, todo `@preview` é indisponível.
#[derive(Clone, Default)]
pub struct WorldEnv {
    pub approvals: Option<Arc<PluginApprovals>>,
    pub plugin_sink: Option<PluginSink>,
    pub packages: Option<Arc<PackageStore>>,
}

impl fmt::Debug for WorldEnv {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorldEnv")
            .field("approvals", &self.approvals)
            .field("packages", &self.packages)
            .finish_non_exhaustive()
    }
}

/// Resultado da conferência de um plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    Approved,
    Pending,
    Rejected,
}

impl WorldEnv {
    /// Confere o plugin `bytes` pedido como `path` (relativo à raiz do projeto ou do pacote)
    /// no projeto `project_root`. Sem decisão, avisa quem escuta.
    pub fn gate_plugin(
        &self,
        project_root: &Path,
        path: &str,
        package: Option<String>,
        bytes: &[u8],
    ) -> Gate {
        let sha256 = sha256_hex(bytes);
        match self
            .approvals
            .as_ref()
            .and_then(|a| a.decision(project_root, &sha256))
        {
            Some(Decision::Approved) => Gate::Approved,
            Some(Decision::Rejected) => Gate::Rejected,
            None => {
                if let Some(sink) = &self.plugin_sink {
                    sink(PluginRequest {
                        path: path.to_owned(),
                        origin: if package.is_some() {
                            Origin::Package
                        } else {
                            Origin::Project
                        },
                        package,
                        size_bytes: bytes.len() as u64,
                        sha256,
                    });
                }
                Gate::Pending
            }
        }
    }

    /// Soma dos contadores de decisões e de "Baixar de novo": mudou → o World relê.
    pub fn generation(&self) -> u64 {
        self.approvals.as_ref().map_or(0, |a| a.generation())
            + self.packages.as_ref().map_or(0, |p| p.generation())
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
