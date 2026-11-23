//! "Criar um projeto novo" (T073; US6; FR-002, FR-003; contracts/ipc.md `create_project_dialog`).
//!
//! A pasta escolhida no diálogo do núcleo MUST estar vazia (só o `.DS_Store` do Finder é
//! tolerado, e fica intacto). Nela nascem o [`MAIN_TEMPLATE`] em `main.typ` e o registro
//! `.prelo/project.json` com a versão do compilador embutido e `main`.
//!
//! Nunca sobrescreve nem escreve fora da pasta: cada item nasce em criação exclusiva
//! (`O_EXCL`; pasta com `mkdir`, que falha se o nome já existe, inclusive como link). No Unix,
//! tudo é criado relativo ao descritor da pasta escolhida e de `.prelo/`, abertos sem seguir
//! link, de modo que trocar um nome por um link no meio do caminho não leva a escrita para fora.
//! Fora do Unix, cria pelo caminho e confere o caminho real do que foi criado. Se algo falha
//! no meio, desfaz só o que esta chamada criou.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::record::{ProjectRecord, DEFAULT_MAIN};

/// Arquivo principal mínimo: compila no Prelo e no `typst` de linha de comando sem pacotes,
/// fontes extras nem arquivos auxiliares (Princípio I).
pub const MAIN_TEMPLATE: &str = "#set document(title: [Novo artigo])\n\
#set text(lang: \"pt\")\n\
\n\
= Novo artigo\n\
\n\
Escreva aqui o seu texto.\n";

/// Nomes que o sistema põe sozinho numa pasta recém-criada e que não contam como conteúdo.
const IGNORED: &[&str] = &[".DS_Store"];

/// Recusa de criação, com o código estável do contrato IPC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateProjectError {
    /// `folder_not_empty`: a pasta tem algum item (ou ganhou um durante a criação).
    NotEmpty,
    /// `not_found`: a pasta não existe ou não é pasta.
    NotFound,
    /// `path_outside_project`: o caminho real da pasta mudou entre a escolha e a criação.
    Outside,
    /// `write_failed`: erro de disco; nada criado fica para trás.
    WriteFailed,
}

impl CreateProjectError {
    pub fn code(self) -> &'static str {
        match self {
            Self::NotEmpty => "folder_not_empty",
            Self::NotFound => "not_found",
            Self::Outside => "path_outside_project",
            Self::WriteFailed => "write_failed",
        }
    }

    fn from_io(e: &io::Error) -> Self {
        if e.kind() == io::ErrorKind::AlreadyExists {
            Self::NotEmpty
        } else {
            Self::WriteFailed
        }
    }
}

/// Cria o projeto mínimo em `folder` e devolve a raiz canônica, pronta para abrir.
pub fn create_project(folder: &Path, embedded_typst: &str) -> Result<PathBuf, CreateProjectError> {
    let root = std::fs::canonicalize(folder).map_err(|_| CreateProjectError::NotFound)?;
    if !root.is_dir() {
        return Err(CreateProjectError::NotFound);
    }
    let dir = Dir::open(&root)?;
    if !is_empty(&root)? {
        return Err(CreateProjectError::NotEmpty);
    }

    let mut record = ProjectRecord::new(embedded_typst);
    record
        .set_main(DEFAULT_MAIN)
        .expect("DEFAULT_MAIN é um principal válido");

    dir.create_file(DEFAULT_MAIN, MAIN_TEMPLATE.as_bytes())?;
    let rest = dir.create_dir(".prelo").and_then(|()| {
        let written = dir
            .subdir(".prelo")
            .and_then(|prelo| prelo.create_file("project.json", &record.to_json()));
        if written.is_err() {
            dir.remove_dir(".prelo");
        }
        written
    });
    if let Err(e) = rest {
        dir.remove_file(DEFAULT_MAIN);
        return Err(e);
    }
    Ok(root)
}

/// Pasta sem itens além de [`IGNORED`].
fn is_empty(root: &Path) -> Result<bool, CreateProjectError> {
    let entries = std::fs::read_dir(root).map_err(|_| CreateProjectError::NotFound)?;
    for entry in entries {
        let entry = entry.map_err(|_| CreateProjectError::WriteFailed)?;
        let name = entry.file_name();
        if !IGNORED.iter().any(|ignored| name == *ignored) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn write_all(mut file: std::fs::File, bytes: &[u8]) -> io::Result<()> {
    file.write_all(bytes)?;
    file.sync_all()
}

// ---------------------------------------------------------------- Unix: relativo ao descritor

#[cfg(unix)]
struct Dir {
    file: std::fs::File,
}

#[cfg(unix)]
impl Dir {
    /// Abre a raiz sem seguir link e confere que o descritor é mesmo a pasta canônica.
    fn open(root: &Path) -> Result<Self, CreateProjectError> {
        use std::os::unix::fs::OpenOptionsExt;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(root)
            .map_err(|_| CreateProjectError::NotFound)?;
        let real =
            crate::fs::scope::descriptor_path(&file).map_err(|_| CreateProjectError::Outside)?;
        if real != root {
            return Err(CreateProjectError::Outside);
        }
        Ok(Self { file })
    }

    fn c_name(name: &str) -> std::ffi::CString {
        std::ffi::CString::new(name).expect("nomes fixos, sem NUL")
    }

    fn create_file(&self, name: &str, bytes: &[u8]) -> Result<(), CreateProjectError> {
        use std::os::fd::{AsRawFd, FromRawFd};
        let c_name = Self::c_name(name);
        // SAFETY: `self.file` é um descritor de pasta aberto e `c_name` termina em NUL.
        let fd = unsafe {
            libc::openat(
                self.file.as_raw_fd(),
                c_name.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o666 as libc::c_uint,
            )
        };
        if fd == -1 {
            return Err(CreateProjectError::from_io(&io::Error::last_os_error()));
        }
        // SAFETY: `fd` acabou de ser aberto e não tem outro dono.
        let file = unsafe { std::fs::File::from_raw_fd(fd) };
        write_all(file, bytes).map_err(|_| {
            self.remove_file(name);
            CreateProjectError::WriteFailed
        })
    }

    fn create_dir(&self, name: &str) -> Result<(), CreateProjectError> {
        use std::os::fd::AsRawFd;
        let c_name = Self::c_name(name);
        // SAFETY: `self.file` é um descritor de pasta aberto e `c_name` termina em NUL.
        let rc = unsafe {
            libc::mkdirat(
                self.file.as_raw_fd(),
                c_name.as_ptr(),
                0o777 as libc::mode_t,
            )
        };
        if rc == -1 {
            return Err(CreateProjectError::from_io(&io::Error::last_os_error()));
        }
        Ok(())
    }

    /// Abre a subpasta `name` sem seguir link.
    fn subdir(&self, name: &str) -> Result<Self, CreateProjectError> {
        use std::os::fd::{AsRawFd, FromRawFd};
        let c_name = Self::c_name(name);
        // SAFETY: `self.file` é um descritor de pasta aberto e `c_name` termina em NUL.
        let fd = unsafe {
            libc::openat(
                self.file.as_raw_fd(),
                c_name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd == -1 {
            return Err(CreateProjectError::Outside);
        }
        // SAFETY: `fd` acabou de ser aberto e não tem outro dono.
        Ok(Self {
            file: unsafe { std::fs::File::from_raw_fd(fd) },
        })
    }

    /// Desfaz um arquivo criado por esta chamada (melhor esforço).
    fn remove_file(&self, name: &str) {
        use std::os::fd::AsRawFd;
        let c_name = Self::c_name(name);
        // SAFETY: descritor de pasta aberto e nome terminado em NUL.
        unsafe { libc::unlinkat(self.file.as_raw_fd(), c_name.as_ptr(), 0) };
    }

    /// Desfaz uma pasta criada por esta chamada (só se vazia; melhor esforço).
    fn remove_dir(&self, name: &str) {
        use std::os::fd::AsRawFd;
        let c_name = Self::c_name(name);
        // SAFETY: descritor de pasta aberto e nome terminado em NUL.
        unsafe { libc::unlinkat(self.file.as_raw_fd(), c_name.as_ptr(), libc::AT_REMOVEDIR) };
    }
}

// ---------------------------------------------------------------- fora do Unix: pelo caminho

#[cfg(not(unix))]
struct Dir {
    path: PathBuf,
    root: PathBuf,
}

#[cfg(not(unix))]
impl Dir {
    fn open(root: &Path) -> Result<Self, CreateProjectError> {
        Ok(Self {
            path: root.to_owned(),
            root: root.to_owned(),
        })
    }

    fn create_file(&self, name: &str, bytes: &[u8]) -> Result<(), CreateProjectError> {
        let target = self.path.join(name);
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|e| CreateProjectError::from_io(&e))?;
        let inside =
            crate::fs::scope::descriptor_path(&file).is_ok_and(|p| p.starts_with(&self.root));
        if !inside {
            drop(file);
            let _ = std::fs::remove_file(&target);
            return Err(CreateProjectError::Outside);
        }
        write_all(file, bytes).map_err(|_| {
            self.remove_file(name);
            CreateProjectError::WriteFailed
        })
    }

    fn create_dir(&self, name: &str) -> Result<(), CreateProjectError> {
        let target = self.path.join(name);
        std::fs::create_dir(&target).map_err(|e| CreateProjectError::from_io(&e))?;
        if !std::fs::canonicalize(&target).is_ok_and(|p| p.starts_with(&self.root)) {
            let _ = std::fs::remove_dir(&target);
            return Err(CreateProjectError::Outside);
        }
        Ok(())
    }

    fn subdir(&self, name: &str) -> Result<Self, CreateProjectError> {
        Ok(Self {
            path: self.path.join(name),
            root: self.root.clone(),
        })
    }

    fn remove_file(&self, name: &str) {
        let _ = std::fs::remove_file(self.path.join(name));
    }

    fn remove_dir(&self, name: &str) {
        let _ = std::fs::remove_dir(self.path.join(name));
    }
}
