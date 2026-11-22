//! `PathAccessModel` do Prelo (RF-08, RF-51; specs/003-editor-local/contracts/file-access.md,
//! passos 1 e 9; data-model.md "Diagnostic" e "Cache de conteúdo").
//!
//! É o único modelo de acesso que o VFS do Tinymist usa. Recebe o caminho absoluto que o VFS
//! monta (raiz do projeto + caminho virtual do Typst) e:
//!
//! 1. devolve o texto do overlay se o arquivo está aberto no editor, sem tocar o disco;
//! 2. caso contrário, lê pela regra de escopo ([`Scope::read`], passos 2–7);
//! 3. guarda os bytes num cache por sha256 e devolve a cópia compartilhada, para que o Typst
//!    reaproveite a imagem decodificada enquanto o conteúdo não muda (RF-51).
//!
//! Toda recusa vira `FileError::Other` com a mensagem do contrato, citando o caminho relativo
//! à raiz (nunca o real), e é registrada para que o World a reconheça na mensagem do Typst e a
//! entregue como `Diagnostic.kind = access`.

use std::collections::HashMap;
use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError, RwLock};

use sha2::{Digest, Sha256};
use tinymist_vfs::{Bytes, PathAccessModel};
use typst::diag::{FileError, FileResult};

use super::scope::{Refusal, Scope};
use crate::diagnostic::{Diagnostic, Kind};

/// Overlay de arquivos abertos: caminho absoluto léxico sob a raiz → texto em memória.
type Overlay = HashMap<PathBuf, Bytes>;

/// Modelo de acesso de um projeto. Clonar compartilha overlay, cache e registro de recusas:
/// o World guarda um clone e o núcleo atualiza o overlay pelo outro.
#[derive(Clone)]
pub struct AccessModel {
    scope: Scope,
    overlay: Arc<RwLock<Overlay>>,
    cache: Arc<Mutex<HashMap<[u8; 32], Bytes>>>,
    refusals: Arc<Mutex<HashMap<String, Refusal>>>,
}

impl fmt::Debug for AccessModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccessModel")
            .field("scope", &self.scope)
            .finish_non_exhaustive()
    }
}

impl AccessModel {
    pub fn new(scope: Scope) -> Self {
        Self {
            scope,
            overlay: Arc::default(),
            cache: Arc::default(),
            refusals: Arc::default(),
        }
    }

    /// Regra de escopo do projeto.
    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    /// Caminho absoluto léxico de `requested` (relativo à raiz; `/` inicial também é a raiz),
    /// sem tocar o disco. `..` além da raiz, caminho vazio ou com NUL são recusados.
    pub fn lexical_path(&self, requested: &str) -> Result<PathBuf, Refusal> {
        let outside = || Refusal::OutsideProject {
            requested: requested.into(),
        };
        if requested.is_empty() || requested.contains('\0') {
            return Err(Refusal::NotFound {
                requested: requested.into(),
            });
        }
        let mut parts: Vec<&std::ffi::OsStr> = Vec::new();
        for component in Path::new(requested.trim_start_matches('/')).components() {
            match component {
                Component::Normal(name) => parts.push(name),
                Component::CurDir => {}
                Component::ParentDir => {
                    parts.pop().ok_or_else(outside)?;
                }
                Component::RootDir | Component::Prefix(_) => return Err(outside()),
            }
        }
        if parts.is_empty() {
            return Err(Refusal::SpecialFile {
                requested: requested.into(),
            });
        }
        let mut path = self.scope.root().to_path_buf();
        path.extend(parts);
        Ok(path)
    }

    /// Passo 1: o arquivo `requested` (relativo à raiz) está aberto no editor com `text`.
    pub fn set_overlay(&self, requested: &str, text: impl Into<String>) -> Result<PathBuf, Refusal> {
        let path = self.lexical_path(requested)?;
        self.overlay_write()
            .insert(path.clone(), Bytes::from_string(text.into()));
        Ok(path)
    }

    /// O arquivo foi fechado no editor: volta a ser lido do disco.
    pub fn remove_overlay(&self, requested: &str) -> Option<PathBuf> {
        let path = self.lexical_path(requested).ok()?;
        self.overlay_write().remove(&path).map(|_| path)
    }

    /// Se `requested` está no overlay.
    pub fn is_open(&self, requested: &str) -> bool {
        self.lexical_path(requested)
            .is_ok_and(|path| self.overlay_read().contains_key(&path))
    }

    /// Quantidade de conteúdos distintos no cache por sha256.
    pub fn cached_entries(&self) -> usize {
        lock(&self.cache).len()
    }

    /// Recusa feita por este modelo cuja mensagem aparece em `message` (texto de um
    /// `FileError` ou de um diagnóstico do Typst que o embrulha).
    pub fn refusal_for_message(&self, message: &str) -> Option<Refusal> {
        let inner = message
            .strip_prefix("failed to load file (")
            .and_then(|rest| rest.strip_suffix(')'))
            .unwrap_or(message);
        lock(&self.refusals).get(inner).cloned()
    }

    /// Caminho pedido ao escopo para o caminho absoluto `src` vindo do VFS: relativo à raiz,
    /// com `/` inicial. Fora da raiz, só o nome do arquivo (não revela pastas de fora).
    fn requested_for(&self, src: &Path) -> Result<String, Refusal> {
        match src.strip_prefix(self.scope.root()) {
            Ok(rel) => {
                let parts: Vec<_> = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect();
                Ok(format!("/{}", parts.join("/")))
            }
            Err(_) => Err(Refusal::OutsideProject {
                requested: src
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            }),
        }
    }

    fn refuse(&self, refusal: Refusal) -> FileError {
        let message = refusal.to_string();
        lock(&self.refusals).insert(message.clone(), refusal);
        FileError::Other(Some(message.into()))
    }

    fn overlay_read(&self) -> std::sync::RwLockReadGuard<'_, Overlay> {
        self.overlay.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn overlay_write(&self) -> std::sync::RwLockWriteGuard<'_, Overlay> {
        self.overlay.write().unwrap_or_else(PoisonError::into_inner)
    }
}

impl PathAccessModel for AccessModel {
    fn reset(&mut self) {
        lock(&self.cache).clear();
    }

    fn content(&self, src: &Path) -> FileResult<Bytes> {
        // Passo 1: overlay de arquivos abertos.
        if let Some(text) = self.overlay_read().get(src) {
            return Ok(text.clone());
        }

        // Passos 2–8: regra de escopo; toda leitura confere de novo, mesmo com cache.
        let requested = self.requested_for(src).map_err(|r| self.refuse(r))?;
        let bytes = self.scope.read(&requested).map_err(|r| self.refuse(r))?;

        // Passo 9: cache por conteúdo.
        let hash: [u8; 32] = Sha256::digest(&bytes).into();
        let mut cache = lock(&self.cache);
        Ok(cache.entry(hash).or_insert_with(|| Bytes::new(bytes)).clone())
    }
}

/// Recusa de acesso como diagnóstico (`kind = access`), sem posição: `path` e a mensagem
/// citam só o caminho pedido.
pub fn access_diagnostic(refusal: &Refusal) -> Diagnostic {
    let mut diag = Diagnostic::error(Kind::Access, refusal.to_string());
    diag.path = Some(refusal.requested().to_owned());
    diag
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
