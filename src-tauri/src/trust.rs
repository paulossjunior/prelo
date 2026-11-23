//! Recursos do núcleo fora do projeto: decisões de plugin e cache de pacotes (FR-017–FR-019;
//! contracts/ipc.md, "Arquivo externo, plugins e pacotes").
//!
//! Moram nos dados locais do app, informados pelo `setup` do app real
//! ([`Trust::set_data_dir`]). Sem essa pasta (testes de IPC, motor de linguagem) todo plugin
//! fica barrado e `@preview` não resolve, e nada é gravado no computador.
//!
//! Pedidos de confirmação: cada plugin sem decisão vira um pedido pendente, avisado uma vez à
//! interface (`plugin_approval_needed`). `decide_plugin` só aceita o hash de um pedido
//! pendente do projeto aberto: a interface não aprova conteúdo que o núcleo não pediu.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError, RwLock};

use typst::syntax::package::PackageSpec;

use crate::world::packages::{Fetcher, HttpFetcher, PackageRefusal, PackageStore};
use crate::world::plugins::{Decision, PluginApprovals, PluginRequest, PluginSink, WorldEnv};

/// Nome do evento de confirmação de plugin.
pub const PLUGIN_APPROVAL_NEEDED: &str = "plugin_approval_needed";

/// Quem avisa a interface (em produção, o `emit` do Tauri).
pub type PluginListener = Arc<dyn Fn(&PluginRequest) + Send + Sync>;

#[derive(Default)]
pub struct Trust {
    data_dir: RwLock<Option<PathBuf>>,
    approvals: OnceLock<Arc<PluginApprovals>>,
    packages: OnceLock<Option<Arc<PackageStore>>>,
    /// Quem baixa os pacotes; sem valor, [`HttpFetcher`] (os testes ligam um local).
    fetcher: RwLock<Option<Arc<dyn Fetcher>>>,
    /// sha256 → pedido ainda sem decisão (do projeto aberto).
    pending: Arc<Mutex<HashMap<String, PluginRequest>>>,
    listener: Arc<RwLock<Option<PluginListener>>>,
}

impl Trust {
    /// Pasta dos dados locais do app. Só vale antes do primeiro uso.
    pub fn set_data_dir(&self, dir: PathBuf) {
        *self
            .data_dir
            .write()
            .unwrap_or_else(PoisonError::into_inner) = Some(dir);
    }

    /// Quem recebe os pedidos de confirmação novos.
    pub fn set_listener(&self, listener: PluginListener) {
        *self
            .listener
            .write()
            .unwrap_or_else(PoisonError::into_inner) = Some(listener);
    }

    /// Troca quem baixa os pacotes (testes com *fetcher* local, sem rede). Só vale antes do
    /// primeiro uso do cache.
    pub fn set_fetcher(&self, fetcher: Arc<dyn Fetcher>) {
        *self.fetcher.write().unwrap_or_else(PoisonError::into_inner) = Some(fetcher);
    }

    fn data_dir(&self) -> Option<PathBuf> {
        self.data_dir
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn approvals(&self) -> Option<Arc<PluginApprovals>> {
        let dir = self.data_dir()?;
        Some(
            self.approvals
                .get_or_init(|| Arc::new(PluginApprovals::load(&dir)))
                .clone(),
        )
    }

    fn packages(&self) -> Option<Arc<PackageStore>> {
        let dir = self.data_dir()?;
        self.packages
            .get_or_init(|| {
                let fetcher = self
                    .fetcher
                    .read()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone()
                    .unwrap_or_else(|| Arc::new(HttpFetcher::new()));
                PackageStore::new(&dir, fetcher)
                    .map(Arc::new)
                    .map_err(|e| eprintln!("prelo: cache de pacotes indisponível: {e}"))
                    .ok()
            })
            .clone()
    }

    /// Projeto novo aberto: pedidos do anterior não valem mais.
    pub fn reset_pending(&self) {
        lock(&self.pending).clear();
    }

    /// Recursos para o World do projeto aberto.
    pub fn env(&self) -> WorldEnv {
        let pending = self.pending.clone();
        let listener = self.listener.clone();
        let sink: PluginSink = Arc::new(move |request: PluginRequest| {
            let new = lock(&pending)
                .insert(request.sha256.clone(), request.clone())
                .is_none();
            if new {
                if let Some(listener) = listener
                    .read()
                    .unwrap_or_else(PoisonError::into_inner)
                    .as_ref()
                {
                    listener(&request);
                }
            }
        });
        WorldEnv {
            approvals: self.approvals(),
            plugin_sink: Some(sink),
            packages: self.packages(),
        }
    }

    /// Decisão da pessoa sobre o pedido pendente `sha256` no projeto `root`.
    pub fn decide(&self, root: &Path, sha256: &str, approve: bool) -> Result<(), String> {
        let valid = sha256.len() == 64
            && sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if !valid {
            return Err("invalid_arguments".into());
        }
        let request = lock(&self.pending)
            .get(sha256)
            .cloned()
            .ok_or("not_found")?;
        let approvals = self.approvals().ok_or("write_failed")?;
        let decision = if approve {
            Decision::Approved
        } else {
            Decision::Rejected
        };
        approvals
            .decide(root, sha256, &request.path, decision)
            .map_err(|e| {
                eprintln!("prelo: decisão de plugin não gravada: {e}");
                "write_failed".to_owned()
            })?;
        lock(&self.pending).remove(sha256);
        Ok(())
    }

    /// "Baixar de novo" `spec` (`@preview/<nome>:<versão>`).
    pub fn redownload(&self, spec: &str) -> Result<(), String> {
        let spec: PackageSpec = spec.parse().map_err(|_| "invalid_arguments")?;
        if spec.namespace != crate::world::packages::PREVIEW {
            return Err("invalid_arguments".into());
        }
        let store = self.packages().ok_or("network_unavailable")?;
        match store.redownload(&spec) {
            Ok(_) => Ok(()),
            Err(PackageRefusal::Unavailable { .. }) => Err("network_unavailable".into()),
            Err(PackageRefusal::UnsupportedNamespace { .. }) => Err("invalid_arguments".into()),
            Err(_) => Err("package_rejected".into()),
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
