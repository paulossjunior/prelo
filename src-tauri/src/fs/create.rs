//! Criação de arquivo e pasta pedida pela interface (FR-034, FR-035; contracts/file-access.md,
//! "Criação de arquivo e pasta"; contracts/ipc.md, `create_file` e `create_dir`).
//!
//! Criar nunca sobrescreve nem trunca: o item nasce em criação exclusiva (`O_EXCL`), relativo
//! ao descritor da pasta-mãe, e o último componente nunca é seguido se for link. A pasta-mãe
//! passa pela regra de escopo ([`Scope::resolve`]) e é conferida de novo no descritor aberto,
//! de modo que trocar um link entre a conferência e a criação não cria nada fora da raiz.

use std::fmt;

use super::scope::Scope;

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
    let _ = (scope, requested, item);
    Err(CreateError::WriteFailed)
}

/// Cria `name` em `parent` (caminho já conferido pela regra de escopo), conferindo de novo a
/// mãe no descritor aberto.
fn create_in(
    scope: &Scope,
    parent: &std::path::Path,
    name: &str,
    item: NewItem,
) -> Result<String, CreateError> {
    let _ = (scope, parent, name, item);
    Err(CreateError::WriteFailed)
}

/// O nome (último componente) segue as regras de FR-035?
pub fn is_valid_name(name: &str) -> bool {
    let _ = name;
    false
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
            "a:b.typ",
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
