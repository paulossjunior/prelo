//! Regra de escopo de arquivo (RF-08, FR-021, FR-022; specs/003-editor-local/contracts/
//! file-access.md, passos 2–7).
//!
//! Todo byte do projeto entregue ao compilador passa por [`Scope::read`]. A ordem é:
//! juntar raiz e caminho pedido, canonicalizar, conferir o prefixo da raiz, abrir sem bloquear
//! e sem seguir link no último componente, conferir **no descritor aberto** que é arquivo
//! regular e que o caminho real continua sob a raiz, conferir o tamanho e só então ler.

use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

/// Limite de tamanho por arquivo lido: 256 MB (MiB), research R11.
pub const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;
/// Limite para estilos `.csl`: 1 MiB (RUSTSEC-2026-0194/0195, research R14).
pub const MAX_CSL_BYTES: u64 = 1024 * 1024;

/// Recusa de acesso (contracts/file-access.md, "Recusas e diagnóstico"). Guarda sempre o
/// caminho **pedido**, nunca o resolvido, para o diagnóstico não revelar pastas de fora.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// `NaoEncontrado`: caminho vazio, inválido ou que não existe.
    NotFound { requested: String },
    /// `ForaDoProjeto`: caminho real fora da raiz (inclusive por troca de link).
    OutsideProject { requested: String },
    /// `ArquivoEspecial`: pasta, FIFO, dispositivo, socket ou outro não regular.
    SpecialFile { requested: String },
    /// `GrandeDemais`: acima de `limit` bytes.
    TooLarge { requested: String, limit: u64 },
    /// `PluginNaoAprovado`: módulo WASM sem decisão para (projeto, sha256) (R8). Lido só para
    /// calcular o hash; não é entregue.
    PluginPending { requested: String },
    /// Módulo WASM que a pessoa recusou neste projeto.
    PluginRejected { requested: String },
}

impl Refusal {
    /// Caminho como foi pedido no `.typ`.
    pub fn requested(&self) -> &str {
        match self {
            Refusal::NotFound { requested }
            | Refusal::OutsideProject { requested }
            | Refusal::SpecialFile { requested }
            | Refusal::TooLarge { requested, .. }
            | Refusal::PluginPending { requested }
            | Refusal::PluginRejected { requested } => requested,
        }
    }

    /// Recusa de plugin (`Diagnostic.kind = plugin`), não de acesso.
    pub fn is_plugin(&self) -> bool {
        matches!(
            self,
            Refusal::PluginPending { .. } | Refusal::PluginRejected { .. }
        )
    }

    /// A mesma recusa citando `requested` (o caminho como foi escrito no `.typ`).
    pub fn with_requested(self, requested: String) -> Self {
        match self {
            Refusal::NotFound { .. } => Refusal::NotFound { requested },
            Refusal::OutsideProject { .. } => Refusal::OutsideProject { requested },
            Refusal::SpecialFile { .. } => Refusal::SpecialFile { requested },
            Refusal::TooLarge { limit, .. } => Refusal::TooLarge { requested, limit },
            Refusal::PluginPending { .. } => Refusal::PluginPending { requested },
            Refusal::PluginRejected { .. } => Refusal::PluginRejected { requested },
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::NotFound { requested } => {
                write!(f, "arquivo não encontrado: `{requested}`")
            }
            Refusal::OutsideProject { requested } => write!(
                f,
                "acesso recusado: `{requested}` fica fora da pasta do projeto"
            ),
            Refusal::SpecialFile { requested } => {
                write!(f, "acesso recusado: `{requested}` não é um arquivo comum")
            }
            Refusal::TooLarge { requested, limit } if *limit == MAX_CSL_BYTES => {
                write!(f, "estilo grande demais: `{requested}` (limite 1 MiB)")
            }
            Refusal::TooLarge { requested, limit } => write!(
                f,
                "arquivo grande demais: `{requested}` (limite {} MB)",
                limit / (1024 * 1024)
            ),
            Refusal::PluginPending { requested } => {
                write!(f, "plugin `{requested}` aguardando sua confirmação")
            }
            Refusal::PluginRejected { requested } => write!(
                f,
                "plugin `{requested}` recusado neste projeto; ele não é executado"
            ),
        }
    }
}

impl std::error::Error for Refusal {}

/// Observador chamado com o caminho canônico logo antes de cada abertura.
pub type OpenObserver = Arc<dyn Fn(&Path) + Send + Sync>;

/// Raiz canônica de um projeto (ou de um pacote no cache) e as leituras dentro dela.
#[derive(Clone)]
pub struct Scope {
    root: PathBuf,
    observer: Option<OpenObserver>,
}

impl fmt::Debug for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Scope").field("root", &self.root).finish()
    }
}

impl Scope {
    /// Canonicaliza a raiz uma vez, ao abrir.
    pub fn new(root: impl AsRef<Path>) -> std::io::Result<Self> {
        Ok(Self {
            root: std::fs::canonicalize(root)?,
            observer: None,
        })
    }

    /// Raiz canônica.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Ponto de observação para os testes de segurança (FR-030): `observer` recebe o caminho
    /// canônico imediatamente antes de cada abertura. Permite contar aberturas e simular a
    /// troca de link entre a conferência e a leitura (FR-022).
    #[doc(hidden)]
    pub fn with_open_observer(mut self, observer: OpenObserver) -> Self {
        self.observer = Some(observer);
        self
    }

    /// Passos 2–4 para um caminho relativo à raiz (`/` inicial também é a raiz). Devolve o
    /// caminho canônico, sem abrir o arquivo. Serve ao autocompletar de caminhos.
    pub fn resolve(&self, requested: &str) -> Result<PathBuf, Refusal> {
        self.resolve_from("", requested)
    }

    /// Como [`Scope::resolve`], com o caminho pedido relativo à pasta de `current`, o arquivo
    /// (relativo à raiz) que faz o pedido. `/` inicial em `requested` é a raiz.
    pub fn resolve_from(&self, current: &str, requested: &str) -> Result<PathBuf, Refusal> {
        let not_found = || Refusal::NotFound {
            requested: requested.into(),
        };
        let outside = || Refusal::OutsideProject {
            requested: requested.into(),
        };
        if requested.is_empty() || requested.contains('\0') || current.contains('\0') {
            return Err(not_found());
        }

        // Passo 2: junta, com normalização léxica. `..` além da raiz é recusado aqui, sem
        // tocar o disco fora do projeto (nem para saber se o alvo existe).
        let relative = match requested.strip_prefix('/') {
            Some(from_root) => PathBuf::from(from_root),
            None => {
                let current = current.strip_prefix('/').unwrap_or(current);
                Path::new(current)
                    .parent()
                    .unwrap_or(Path::new(""))
                    .join(requested)
            }
        };
        let mut joined = self.root.clone();
        let mut depth = 0usize;
        for component in relative.components() {
            match component {
                Component::Normal(name) => {
                    joined.push(name);
                    depth += 1;
                }
                Component::CurDir => {}
                Component::ParentDir => {
                    if depth == 0 {
                        return Err(outside());
                    }
                    joined.pop();
                    depth -= 1;
                }
                // Caminho absoluto do sistema ou prefixo de unidade (Windows).
                Component::RootDir | Component::Prefix(_) => return Err(outside()),
            }
        }

        // Passo 3: resolve links simbólicos (e junções no Windows).
        let canonical = std::fs::canonicalize(&joined).map_err(|_| not_found())?;

        // Passo 4: prefixo da raiz canônica, por componentes.
        if !canonical.starts_with(&self.root) {
            return Err(outside());
        }
        Ok(canonical)
    }

    /// Passos 2–7 e leitura, para um caminho relativo à raiz.
    pub fn read(&self, requested: &str) -> Result<Vec<u8>, Refusal> {
        self.read_from("", requested)
    }

    /// Passos 2–7 e leitura, com o caminho pedido relativo à pasta de `current`.
    pub fn read_from(&self, current: &str, requested: &str) -> Result<Vec<u8>, Refusal> {
        let canonical = self.resolve_from(current, requested)?;
        let not_found = || Refusal::NotFound {
            requested: requested.into(),
        };
        let outside = || Refusal::OutsideProject {
            requested: requested.into(),
        };
        let special = || Refusal::SpecialFile {
            requested: requested.into(),
        };

        // Antes de abrir: pasta, FIFO e dispositivo nem chegam a ser abertos.
        let meta = std::fs::symlink_metadata(&canonical).map_err(|_| not_found())?;
        if !meta.file_type().is_file() {
            return Err(special());
        }

        // Passo 5: abre sem bloquear e sem seguir link no último componente.
        if let Some(observer) = &self.observer {
            observer(&canonical);
        }
        let file = open_no_follow(&canonical).map_err(|e| {
            if is_link_error(&e) {
                outside()
            } else {
                not_found()
            }
        })?;

        // Passo 6: no descritor aberto, arquivo regular e caminho real sob a raiz.
        let meta = file.metadata().map_err(|_| not_found())?;
        if !meta.file_type().is_file() {
            return Err(special());
        }
        // Sem como confirmar o caminho real do descritor, recusa como fora do projeto.
        let real = descriptor_path(&file).map_err(|_| outside())?;
        if !real.starts_with(&self.root) {
            return Err(outside());
        }

        // Passo 7: limite de tamanho (1 MiB para `.csl`).
        let is_csl = |p: &Path| {
            p.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("csl"))
        };
        let limit = if is_csl(Path::new(requested)) || is_csl(&canonical) || is_csl(&real) {
            MAX_CSL_BYTES
        } else {
            MAX_FILE_BYTES
        };
        let too_large = || Refusal::TooLarge {
            requested: requested.into(),
            limit,
        };
        if meta.len() > limit {
            return Err(too_large());
        }

        // Lê no máximo `limit` + 1 bytes: arquivo que cresceu depois da conferência também
        // é recusado.
        let mut bytes = Vec::with_capacity(meta.len() as usize);
        file.take(limit + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| not_found())?;
        if bytes.len() as u64 > limit {
            return Err(too_large());
        }
        Ok(bytes)
    }
}

/// Abre para leitura com `O_NONBLOCK` (FIFO não bloqueia) e `O_NOFOLLOW` (último componente
/// não pode ser link) em Unix; no Windows, abre o ponto de reanálise em si, sem segui-lo.
fn open_no_follow(path: &Path) -> std::io::Result<File> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT);
    }
    options.open(path)
}

/// `O_NOFOLLOW` sobre link simbólico falha com `ELOOP` (Linux, macOS) ou `EMLINK` (BSD).
fn is_link_error(error: &std::io::Error) -> bool {
    #[cfg(unix)]
    {
        matches!(error.raw_os_error(), Some(libc::ELOOP) | Some(libc::EMLINK))
    }
    #[cfg(not(unix))]
    {
        let _ = error;
        false
    }
}

/// Caminho real do arquivo aberto, lido do descritor (research R11).
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) fn descriptor_path(file: &File) -> std::io::Result<PathBuf> {
    use std::os::fd::AsRawFd;
    std::fs::read_link(format!("/proc/self/fd/{}", file.as_raw_fd()))
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) fn descriptor_path(file: &File) -> std::io::Result<PathBuf> {
    use std::ffi::OsStr;
    use std::os::fd::AsRawFd;
    use std::os::unix::ffi::OsStrExt;

    let mut buf = vec![0u8; libc::PATH_MAX as usize];
    // SAFETY: F_GETPATH grava no máximo PATH_MAX bytes, terminados em NUL, em `buf`.
    let rc = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETPATH, buf.as_mut_ptr()) };
    if rc == -1 {
        return Err(std::io::Error::last_os_error());
    }
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    Ok(PathBuf::from(OsStr::from_bytes(&buf[..len])))
}

#[cfg(windows)]
pub(crate) fn descriptor_path(file: &File) -> std::io::Result<PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFinalPathNameByHandleW, FILE_NAME_NORMALIZED, VOLUME_NAME_DOS,
    };

    let handle = file.as_raw_handle();
    let mut buf = vec![0u16; 1024];
    loop {
        // SAFETY: `handle` é válido enquanto `file` vive; `buf` tem `buf.len()` u16.
        let len = unsafe {
            GetFinalPathNameByHandleW(
                handle,
                buf.as_mut_ptr(),
                buf.len() as u32,
                FILE_NAME_NORMALIZED | VOLUME_NAME_DOS,
            )
        } as usize;
        if len == 0 {
            return Err(std::io::Error::last_os_error());
        }
        if len < buf.len() {
            // Mesmo formato `\\?\C:\...` de `std::fs::canonicalize`.
            return Ok(PathBuf::from(OsString::from_wide(&buf[..len])));
        }
        buf.resize(len + 1, 0);
    }
}

/// Sistema sem forma conhecida de ler o caminho do descritor: recusa toda leitura.
#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios",
    windows
)))]
pub(crate) fn descriptor_path(_file: &File) -> std::io::Result<PathBuf> {
    Err(std::io::Error::other(
        "caminho do descritor indisponível neste sistema",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn project() -> (TempDir, Scope) {
        let dir = tempfile::tempdir().expect("pasta temporária");
        std::fs::write(dir.path().join("main.typ"), "principal").unwrap();
        std::fs::write(dir.path().join("utils.typ"), "utilidades").unwrap();
        std::fs::create_dir(dir.path().join("capitulos")).unwrap();
        std::fs::write(dir.path().join("capitulos/intro.typ"), "introdução").unwrap();
        let scope = Scope::new(dir.path()).expect("raiz");
        (dir, scope)
    }

    #[test]
    fn raiz_e_canonica() {
        let (dir, scope) = project();
        assert_eq!(scope.root(), dir.path().canonicalize().unwrap());
    }

    #[test]
    fn barra_inicial_e_relativa_a_raiz() {
        let (_dir, scope) = project();
        assert_eq!(scope.read("/main.typ").unwrap(), b"principal");
        assert_eq!(scope.read("main.typ").unwrap(), b"principal");
        assert_eq!(
            scope
                .read_from("capitulos/intro.typ", "/utils.typ")
                .unwrap(),
            b"utilidades"
        );
        assert_eq!(
            scope.resolve("/capitulos/intro.typ").unwrap(),
            scope.root().join("capitulos").join("intro.typ")
        );
    }

    #[test]
    fn relativo_ao_arquivo_que_pede() {
        let (_dir, scope) = project();
        assert_eq!(
            scope.read_from("capitulos/intro.typ", "intro.typ").unwrap(),
            "introdução".as_bytes()
        );
        assert_eq!(
            scope
                .read_from("/capitulos/intro.typ", "./intro.typ")
                .unwrap(),
            "introdução".as_bytes()
        );
    }

    #[test]
    fn ponto_ponto_dentro_da_raiz_e_aceito() {
        let (_dir, scope) = project();
        assert_eq!(
            scope
                .read_from("capitulos/intro.typ", "../utils.typ")
                .unwrap(),
            b"utilidades"
        );
        assert_eq!(scope.read("capitulos/../main.typ").unwrap(), b"principal");
    }

    #[test]
    fn ponto_ponto_alem_da_raiz_e_recusado() {
        let (_dir, scope) = project();
        let pedidos = [
            ("capitulos/intro.typ", "../../fora.typ"),
            ("main.typ", "../fora.typ"),
            ("", "/../fora.typ"),
            ("", "capitulos/../../fora.typ"),
            ("", "../../../../../../../../etc/passwd"),
        ];
        for (current, requested) in pedidos {
            assert_eq!(
                scope.read_from(current, requested),
                Err(Refusal::OutsideProject {
                    requested: requested.into()
                }),
                "{requested} a partir de {current:?}"
            );
            assert!(scope.resolve_from(current, requested).is_err());
        }
    }

    #[test]
    fn ponto_ponto_alem_da_raiz_para_arquivo_existente_e_recusado() {
        let (dir, scope) = project();
        // Irmão da raiz que existe de fato: a recusa não pode depender de o alvo faltar.
        let fora = tempfile::tempdir_in(dir.path().parent().unwrap()).unwrap();
        std::fs::write(fora.path().join("segredo.txt"), "segredo").unwrap();
        let nome = fora.path().file_name().unwrap().to_str().unwrap();
        let requested = format!("../{nome}/segredo.txt");
        assert_eq!(
            scope.read(&requested),
            Err(Refusal::OutsideProject {
                requested: requested.clone()
            })
        );
    }

    #[test]
    fn caminho_absoluto_do_sistema_nao_escapa() {
        let (_dir, scope) = project();
        // `/` inicial é a raiz; `/etc/passwd` vira `<raiz>/etc/passwd`, que não existe.
        assert_eq!(
            scope.read("/etc/passwd"),
            Err(Refusal::NotFound {
                requested: "/etc/passwd".into()
            })
        );
        // Duas barras não viram caminho absoluto do sistema.
        assert!(matches!(
            scope.read("//etc/passwd"),
            Err(Refusal::OutsideProject { .. })
        ));
    }

    #[test]
    fn caminho_vazio_e_recusado() {
        let (_dir, scope) = project();
        assert_eq!(
            scope.read(""),
            Err(Refusal::NotFound {
                requested: String::new()
            })
        );
        assert!(scope.resolve("").is_err());
    }

    #[test]
    fn caminho_com_nul_e_recusado() {
        let (_dir, scope) = project();
        for requested in ["main.typ\0", "\0", "capitulos/\0intro.typ"] {
            assert_eq!(
                scope.read(requested),
                Err(Refusal::NotFound {
                    requested: requested.into()
                })
            );
            assert!(scope.resolve(requested).is_err());
        }
    }

    #[test]
    fn pasta_nao_e_arquivo_comum() {
        let (_dir, scope) = project();
        assert_eq!(
            scope.read("capitulos"),
            Err(Refusal::SpecialFile {
                requested: "capitulos".into()
            })
        );
        assert!(matches!(scope.read("/"), Err(Refusal::SpecialFile { .. })));
    }

    #[test]
    fn inexistente_e_nao_encontrado() {
        let (_dir, scope) = project();
        assert_eq!(
            scope.read("nada.typ"),
            Err(Refusal::NotFound {
                requested: "nada.typ".into()
            })
        );
    }

    #[test]
    fn diagnosticos_em_portugues_citam_o_caminho_pedido() {
        let r = |requested: &str| requested.to_string();
        assert_eq!(
            Refusal::NotFound {
                requested: r("a.typ")
            }
            .to_string(),
            "arquivo não encontrado: `a.typ`"
        );
        assert_eq!(
            Refusal::OutsideProject {
                requested: r("../a.png")
            }
            .to_string(),
            "acesso recusado: `../a.png` fica fora da pasta do projeto"
        );
        assert_eq!(
            Refusal::SpecialFile {
                requested: r("fifo")
            }
            .to_string(),
            "acesso recusado: `fifo` não é um arquivo comum"
        );
        assert_eq!(
            Refusal::TooLarge {
                requested: r("grande.bin"),
                limit: MAX_FILE_BYTES
            }
            .to_string(),
            "arquivo grande demais: `grande.bin` (limite 256 MB)"
        );
        assert_eq!(
            Refusal::TooLarge {
                requested: r("abnt.csl"),
                limit: MAX_CSL_BYTES
            }
            .to_string(),
            "estilo grande demais: `abnt.csl` (limite 1 MiB)"
        );
        assert_eq!(Refusal::SpecialFile { requested: r("x") }.requested(), "x");
    }
}
