//! Criação de arquivo e pasta pedida pela interface (FR-034, FR-035; contracts/file-access.md,
//! "Criação de arquivo e pasta"; contracts/ipc.md, `create_file` e `create_dir`).
//!
//! Criar nunca sobrescreve nem trunca: o item nasce em criação exclusiva (`O_EXCL`), relativo
//! ao descritor da pasta-mãe, e o último componente nunca é seguido se for link. A pasta-mãe
//! passa pela regra de escopo ([`Scope::resolve`]) e é conferida de novo no descritor aberto,
//! de modo que trocar um link entre a conferência e a criação não cria nada fora da raiz.

use std::fmt;
use std::path::{Component, Path};

use super::scope::{descriptor_path, Refusal, Scope};

/// Tamanho máximo de um nome, em bytes (limite comum de APFS, ext4 e NTFS).
pub const MAX_NAME_BYTES: usize = 255;

/// Caracteres proibidos em nome em algum dos três sistemas (o Windows é o mais restrito).
const FORBIDDEN: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

/// Nomes de dispositivo do Windows, recusados com ou sem extensão e em qualquer caixa.
const RESERVED: &[&str] = &["CON", "PRN", "AUX", "NUL"];

/// Recusa de criação, com o código estável do contrato IPC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateError {
    /// `already_exists`: o nome já existe (arquivo, pasta ou link, mesmo quebrado).
    AlreadyExists,
    /// `invalid_name`: nome vazio, oculto, reservado ou com caractere proibido.
    InvalidName,
    /// `path_outside_project`: absoluto, `..` além da raiz ou mãe que resolve fora.
    OutsideProject,
    /// `not_found`: a pasta-mãe não existe ou não é pasta.
    NotFound,
    /// `write_failed`: o sistema recusou a criação (permissão, disco cheio).
    WriteFailed,
}

impl CreateError {
    /// Código do contrato IPC.
    pub fn code(self) -> &'static str {
        match self {
            CreateError::AlreadyExists => "already_exists",
            CreateError::InvalidName => "invalid_name",
            CreateError::OutsideProject => "path_outside_project",
            CreateError::NotFound => "not_found",
            CreateError::WriteFailed => "write_failed",
        }
    }
}

impl fmt::Display for CreateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for CreateError {}

/// O que criar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewItem {
    /// Arquivo vazio (0 bytes).
    File,
    /// Pasta vazia.
    Dir,
}

/// Cria `requested` (relativo à raiz) e devolve o caminho normalizado, relativo, com `/`.
pub fn create(scope: &Scope, requested: &str, item: NewItem) -> Result<String, CreateError> {
    // Passo 1: absoluto (`/`, `\`, `C:` ou o que o sistema considera absoluto).
    if is_absolute(requested) {
        return Err(CreateError::OutsideProject);
    }
    // Passo 2: separa pasta-mãe e nome à mão: `Path::components` apagaria o `.` interno e a
    // barra final (`capitulos/.` viraria `capitulos`).
    if requested.is_empty() || requested.ends_with(is_separator) {
        return Err(CreateError::InvalidName);
    }
    let mut parts: Vec<&str> = requested.split(is_separator).collect();
    let name = parts.pop().unwrap_or_default();
    let mut parent: Vec<&str> = Vec::new();
    for part in parts {
        match part {
            "" | "." => {}
            ".." => {
                if parent.pop().is_none() {
                    return Err(CreateError::OutsideProject);
                }
            }
            hidden if hidden.starts_with('.') => return Err(CreateError::InvalidName),
            normal => parent.push(normal),
        }
    }
    // Passo 3: regras do nome (inclui `.`, `..` e oculto).
    if !is_valid_name(name) {
        return Err(CreateError::InvalidName);
    }
    // Passo 4: pasta-mãe pela regra de escopo.
    let parent_dir = if parent.is_empty() {
        scope.root().to_owned()
    } else {
        scope
            .resolve(&parent.join("/"))
            .map_err(|refusal| match refusal {
                Refusal::OutsideProject { .. } => CreateError::OutsideProject,
                _ => CreateError::NotFound,
            })?
    };
    if !parent_dir.is_dir() {
        return Err(CreateError::NotFound);
    }
    // Passos 5–7.
    create_in(scope, &parent_dir, name, item)
}

/// Caminho absoluto em qualquer sistema: `/`, `\`, `C:` ou o que o sistema considera absoluto
/// (o mesmo critério de `commands::validate_path`).
fn is_absolute(requested: &str) -> bool {
    let b = requested.as_bytes();
    requested.starts_with('/')
        || requested.starts_with('\\')
        || (b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':')
        || Path::new(requested).has_root()
        || Path::new(requested).is_absolute()
}

/// Separador de pastas aceito no caminho pedido: `/` sempre; `\` também no Windows.
fn is_separator(c: char) -> bool {
    c == '/' || (cfg!(windows) && c == '\\')
}

/// Caminho relativo à raiz, com `/`, de `name` dentro de `inside` (relativo à raiz).
fn relative_path(inside: &Path, name: &str) -> Result<String, CreateError> {
    let mut parts = Vec::new();
    for component in inside.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_str().ok_or(CreateError::NotFound)?),
            _ => return Err(CreateError::OutsideProject),
        }
    }
    parts.push(name);
    Ok(parts.join("/"))
}

/// Passo 6: o nome já existe (sem seguir link)? Link que resolve fora da raiz é recusado como
/// fora do projeto; qualquer outra coisa (inclusive link quebrado) é `already_exists`.
fn refuse_existing(
    scope: &Scope,
    real_parent: &Path,
    name: &str,
    relative: &str,
) -> Result<(), CreateError> {
    let Ok(meta) = std::fs::symlink_metadata(real_parent.join(name)) else {
        return Ok(());
    };
    if meta.file_type().is_symlink()
        && matches!(scope.resolve(relative), Err(Refusal::OutsideProject { .. }))
    {
        return Err(CreateError::OutsideProject);
    }
    Err(CreateError::AlreadyExists)
}

fn creation_error(error: &std::io::Error) -> CreateError {
    if error.kind() == std::io::ErrorKind::AlreadyExists {
        CreateError::AlreadyExists
    } else {
        CreateError::WriteFailed
    }
}

/// Passos 5–7 em Unix: abre a mãe sem seguir link no último componente, confere o caminho
/// real do descritor e cria relativo a ele (`openat`/`mkdirat`), em criação exclusiva.
#[cfg(unix)]
fn create_in(
    scope: &Scope,
    parent: &Path,
    name: &str,
    item: NewItem,
) -> Result<String, CreateError> {
    use std::ffi::CString;
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::fs::OpenOptionsExt;

    let dir = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(parent)
        .map_err(|_| match std::fs::symlink_metadata(parent) {
            Ok(meta) if meta.file_type().is_symlink() => CreateError::OutsideProject,
            _ => CreateError::NotFound,
        })?;
    // Sem como confirmar o caminho real do descritor, recusa como fora do projeto.
    let real = descriptor_path(&dir).map_err(|_| CreateError::OutsideProject)?;
    let inside = real
        .strip_prefix(scope.root())
        .map_err(|_| CreateError::OutsideProject)?;
    let relative = relative_path(inside, name)?;
    refuse_existing(scope, &real, name, &relative)?;

    let c_name = CString::new(name).map_err(|_| CreateError::InvalidName)?;
    match item {
        NewItem::File => {
            // SAFETY: `dir` é um descritor de pasta aberto e `c_name` termina em NUL.
            let fd = unsafe {
                libc::openat(
                    dir.as_raw_fd(),
                    c_name.as_ptr(),
                    libc::O_WRONLY
                        | libc::O_CREAT
                        | libc::O_EXCL
                        | libc::O_NOFOLLOW
                        | libc::O_CLOEXEC,
                    0o666 as libc::c_uint,
                )
            };
            if fd == -1 {
                return Err(creation_error(&std::io::Error::last_os_error()));
            }
            // SAFETY: `fd` acabou de ser aberto e não tem outro dono; fecha ao sair.
            drop(unsafe { std::fs::File::from_raw_fd(fd) });
        }
        NewItem::Dir => {
            // SAFETY: `dir` é um descritor de pasta aberto e `c_name` termina em NUL.
            let rc =
                unsafe { libc::mkdirat(dir.as_raw_fd(), c_name.as_ptr(), 0o777 as libc::mode_t) };
            if rc == -1 {
                return Err(creation_error(&std::io::Error::last_os_error()));
            }
        }
    }
    Ok(relative)
}

/// Passos 5–7 fora de Unix: cria pelo caminho em criação exclusiva e confere o caminho real
/// do criado; se ficou fora da raiz (troca de link no meio), apaga e recusa.
#[cfg(not(unix))]
fn create_in(
    scope: &Scope,
    parent: &Path,
    name: &str,
    item: NewItem,
) -> Result<String, CreateError> {
    let real = std::fs::canonicalize(parent).map_err(|_| CreateError::NotFound)?;
    let inside = real
        .strip_prefix(scope.root())
        .map_err(|_| CreateError::OutsideProject)?;
    let relative = relative_path(inside, name)?;
    refuse_existing(scope, &real, name, &relative)?;
    let target = real.join(name);
    match item {
        NewItem::File => {
            let file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)
                .map_err(|e| creation_error(&e))?;
            let inside = descriptor_path(&file).is_ok_and(|p| p.starts_with(scope.root()));
            drop(file);
            if !inside {
                let _ = std::fs::remove_file(&target);
                return Err(CreateError::OutsideProject);
            }
        }
        NewItem::Dir => {
            std::fs::create_dir(&target).map_err(|e| creation_error(&e))?;
            let inside = std::fs::canonicalize(&target).is_ok_and(|p| p.starts_with(scope.root()));
            if !inside {
                let _ = std::fs::remove_dir(&target);
                return Err(CreateError::OutsideProject);
            }
        }
    }
    Ok(relative)
}

/// O nome (último componente) segue as regras de FR-035? Vazio, `.`, `..`, oculto (começa com
/// `.`), com caractere de controle ou proibido, terminado em espaço ou ponto, nome reservado
/// do Windows ou acima de [`MAX_NAME_BYTES`] → não.
pub fn is_valid_name(name: &str) -> bool {
    if name.is_empty() || name.len() > MAX_NAME_BYTES || name.starts_with('.') {
        return false;
    }
    if name
        .chars()
        .any(|c| c.is_control() || FORBIDDEN.contains(&c))
    {
        return false;
    }
    if name.ends_with(' ') || name.ends_with('.') {
        return false;
    }
    let stem = name
        .split('.')
        .next()
        .unwrap_or(name)
        .trim_end()
        .to_ascii_uppercase();
    let device = RESERVED.contains(&stem.as_str())
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'));
    !device
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    fn project() -> (TempDir, Scope) {
        let dir = tempfile::tempdir().expect("pasta temporária");
        std::fs::write(dir.path().join("main.typ"), "principal").unwrap();
        std::fs::create_dir(dir.path().join("capitulos")).unwrap();
        std::fs::write(dir.path().join("capitulos/intro.typ"), "introdução").unwrap();
        let scope = Scope::new(dir.path()).expect("raiz");
        (dir, scope)
    }

    fn outside() -> TempDir {
        let dir = tempfile::tempdir().expect("pasta temporária de fora");
        std::fs::write(dir.path().join("segredo.txt"), "segredo").unwrap();
        dir
    }

    fn entries(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn cria_arquivo_vazio_na_raiz_e_em_subpasta() {
        let (_dir, scope) = project();
        assert_eq!(
            create(&scope, "novo.typ", NewItem::File),
            Ok("novo.typ".into())
        );
        assert_eq!(
            create(&scope, "capitulos/metodo.typ", NewItem::File),
            Ok("capitulos/metodo.typ".into())
        );
        for rel in ["novo.typ", "capitulos/metodo.typ"] {
            let meta = std::fs::symlink_metadata(scope.root().join(rel)).unwrap();
            assert!(meta.is_file(), "{rel}");
            assert_eq!(meta.len(), 0, "{rel} deveria nascer vazio");
        }
    }

    #[test]
    fn cria_pasta_vazia_na_raiz_e_em_subpasta() {
        let (_dir, scope) = project();
        assert_eq!(
            create(&scope, "figuras", NewItem::Dir),
            Ok("figuras".into())
        );
        assert_eq!(
            create(&scope, "capitulos/anexos", NewItem::Dir),
            Ok("capitulos/anexos".into())
        );
        assert!(scope.root().join("figuras").is_dir());
        assert!(entries(&scope.root().join("capitulos/anexos")).is_empty());
    }

    #[test]
    fn caminho_devolvido_e_normalizado_e_ponto_ponto_dentro_da_raiz_e_aceito() {
        let (_dir, scope) = project();
        assert_eq!(
            create(&scope, "capitulos/../novo.typ", NewItem::File),
            Ok("novo.typ".into())
        );
        assert_eq!(
            create(&scope, "./capitulos/./b.typ", NewItem::File),
            Ok("capitulos/b.typ".into())
        );
        assert!(scope.root().join("novo.typ").is_file());
    }

    #[test]
    fn nome_com_acentos_e_255_bytes_e_aceito() {
        let (_dir, scope) = project();
        assert_eq!(
            create(&scope, "introdução.typ", NewItem::File),
            Ok("introdução.typ".into())
        );
        let long = format!("{}.typ", "a".repeat(251));
        assert_eq!(long.len(), 255);
        assert_eq!(create(&scope, &long, NewItem::File), Ok(long.clone()));
    }

    #[test]
    fn nome_existente_e_recusado_sem_sobrescrever() {
        let (_dir, scope) = project();
        for item in [NewItem::File, NewItem::Dir] {
            assert_eq!(
                create(&scope, "main.typ", item),
                Err(CreateError::AlreadyExists)
            );
            assert_eq!(
                create(&scope, "capitulos", item),
                Err(CreateError::AlreadyExists)
            );
            assert_eq!(
                create(&scope, "capitulos/intro.typ", item),
                Err(CreateError::AlreadyExists)
            );
        }
        assert_eq!(
            std::fs::read_to_string(scope.root().join("main.typ")).unwrap(),
            "principal"
        );
        assert_eq!(
            std::fs::read_to_string(scope.root().join("capitulos/intro.typ")).unwrap(),
            "introdução"
        );
        assert_eq!(entries(&scope.root().join("capitulos")), ["intro.typ"]);
    }

    #[test]
    fn nomes_invalidos_sao_recusados_sem_criar_nada() {
        let (_dir, scope) = project();
        let before = entries(scope.root());
        let long = "a".repeat(256);
        let mut names = vec![
            "",
            ".",
            "..",
            "capitulos/..",
            "capitulos/.",
            "novo/",
            "capitulos/",
            "a\0b.typ",
            "a\u{1}b.typ",
            "a\nb.typ",
            "a\u{7f}b.typ",
            "ab:c.typ",
            "a*b.typ",
            "a?b.typ",
            "a\"b.typ",
            "a<b.typ",
            "a>b.typ",
            "a|b.typ",
            "fim ",
            "fim.",
            "CON",
            "con.typ",
            "nul.typ",
            "Aux",
            "prn.txt",
            "COM1",
            "lpt9.bib",
            long.as_str(),
            ".oculto",
            ".env",
            "capitulos/.oculto.typ",
            ".prelo/project.json",
            ".git/hooks/pre-commit",
            "capitulos/.x/novo.typ",
        ];
        if cfg!(unix) {
            // No Windows `\` separa pastas; nos outros sistemas é caractere proibido no nome.
            names.push("a\\b.typ");
        }
        for name in names {
            for item in [NewItem::File, NewItem::Dir] {
                assert_eq!(
                    create(&scope, name, item),
                    Err(CreateError::InvalidName),
                    "{name:?} ({item:?})"
                );
            }
        }
        assert_eq!(entries(scope.root()), before);
        assert_eq!(entries(&scope.root().join("capitulos")), ["intro.typ"]);
    }

    #[test]
    fn regras_de_nome() {
        for ok in [
            "main.typ",
            "a",
            "introdução.typ",
            "refs.bib",
            "con-texto.typ",
            "COM10",
            "a b.typ",
        ] {
            assert!(is_valid_name(ok), "{ok:?} deveria ser aceito");
        }
        for bad in [
            "", ".", "..", ".x", "a/b", "a\0", "nul", "NUL.txt", "x.", "x ",
        ] {
            assert!(!is_valid_name(bad), "{bad:?} deveria ser recusado");
        }
    }

    #[test]
    fn mae_inexistente_ou_que_nao_e_pasta_e_not_found() {
        let (_dir, scope) = project();
        for (requested, item) in [
            ("nada/novo.typ", NewItem::File),
            ("nada/sub", NewItem::Dir),
            ("main.typ/novo.typ", NewItem::File),
            ("capitulos/intro.typ/sub", NewItem::Dir),
        ] {
            assert_eq!(
                create(&scope, requested, item),
                Err(CreateError::NotFound),
                "{requested}"
            );
        }
        assert!(!scope.root().join("nada").exists());
    }

    #[test]
    fn absoluto_e_ponto_ponto_alem_da_raiz_sao_recusados_sem_criar_fora() {
        let (dir, scope) = project();
        let parent = dir.path().parent().unwrap().to_owned();
        let fora = outside();
        let fora_abs = fora.path().join("novo.typ").to_string_lossy().into_owned();
        let mut pedidos = vec![
            "/".to_string(),
            "/novo.typ".into(),
            "//servidor/x.typ".into(),
            "../novo-fora.typ".into(),
            "capitulos/../../novo-fora.typ".into(),
            "./../novo-fora.typ".into(),
            "../../../../../../../../tmp/novo-fora.typ".into(),
            fora_abs,
        ];
        if cfg!(windows) {
            pedidos.push("C:\\novo.typ".into());
            pedidos.push("..\\novo-fora.typ".into());
        }
        for requested in &pedidos {
            for item in [NewItem::File, NewItem::Dir] {
                assert_eq!(
                    create(&scope, requested, item),
                    Err(CreateError::OutsideProject),
                    "{requested:?} ({item:?})"
                );
            }
        }
        // A pasta temporária do sistema muda com os outros testes: confere o nome pedido.
        assert!(
            !parent.join("novo-fora.typ").exists(),
            "criou ao lado da raiz"
        );
        assert_eq!(entries(fora.path()), ["segredo.txt"]);
    }

    #[cfg(unix)]
    #[test]
    fn links_para_fora_sao_recusados_sem_criar_fora() {
        use std::os::unix::fs::symlink;
        let (_dir, scope) = project();
        let fora = outside();
        // Mãe que é link para pasta de fora.
        symlink(fora.path(), scope.root().join("pasta-fora")).unwrap();
        // Último componente que é link para arquivo de fora (existente e inexistente).
        symlink(
            fora.path().join("segredo.txt"),
            scope.root().join("link-fora.typ"),
        )
        .unwrap();
        symlink(
            fora.path().join("criado.typ"),
            scope.root().join("quebrado-fora.typ"),
        )
        .unwrap();
        // Link interno (existente) e link interno quebrado.
        symlink(
            scope.root().join("main.typ"),
            scope.root().join("atalho.typ"),
        )
        .unwrap();
        symlink(
            scope.root().join("nada.typ"),
            scope.root().join("quebrado.typ"),
        )
        .unwrap();

        for item in [NewItem::File, NewItem::Dir] {
            assert_eq!(
                create(&scope, "pasta-fora/novo.typ", item),
                Err(CreateError::OutsideProject)
            );
            assert_eq!(
                create(&scope, "link-fora.typ", item),
                Err(CreateError::OutsideProject)
            );
            assert!(create(&scope, "quebrado-fora.typ", item).is_err());
            assert_eq!(
                create(&scope, "atalho.typ", item),
                Err(CreateError::AlreadyExists)
            );
            assert_eq!(
                create(&scope, "quebrado.typ", item),
                Err(CreateError::AlreadyExists)
            );
        }
        assert_eq!(entries(fora.path()), ["segredo.txt"]);
        assert_eq!(
            std::fs::read_to_string(fora.path().join("segredo.txt")).unwrap(),
            "segredo"
        );
        assert!(!scope.root().join("nada.typ").exists());
        assert_eq!(
            std::fs::read_to_string(scope.root().join("main.typ")).unwrap(),
            "principal"
        );
    }

    /// Troca de link entre a conferência da mãe e a criação (FR-035): a criação relativa ao
    /// descritor confere a mãe de novo e não cria nada fora.
    #[cfg(unix)]
    #[test]
    fn mae_trocada_por_link_depois_da_conferencia_nao_cria_fora() {
        use std::os::unix::fs::symlink;
        let (_dir, scope) = project();
        let fora = outside();
        std::fs::create_dir(fora.path().join("sub")).unwrap();
        // Como se `capitulos` tivesse sido conferida e depois trocada por link para fora.
        let checked = scope.root().join("capitulos");
        std::fs::rename(&checked, scope.root().join("capitulos-antiga")).unwrap();
        symlink(fora.path(), &checked).unwrap();
        for item in [NewItem::File, NewItem::Dir] {
            assert_eq!(
                create_in(&scope, &checked, "novo.typ", item),
                Err(CreateError::OutsideProject)
            );
            assert_eq!(
                create_in(&scope, &checked.join("sub"), "novo.typ", item),
                Err(CreateError::OutsideProject)
            );
        }
        assert_eq!(entries(fora.path()), ["segredo.txt", "sub"]);
        assert!(entries(&fora.path().join("sub")).is_empty());
    }
}
