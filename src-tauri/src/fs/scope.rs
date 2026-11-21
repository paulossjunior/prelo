//! Regra de escopo de arquivo (RF-08, FR-021, FR-022; specs/003-editor-local/contracts/
//! file-access.md, passos 2–7).
//!
//! Todo byte do projeto entregue ao compilador passa por [`Scope::read`]. A ordem é:
//! juntar raiz e caminho pedido, canonicalizar, conferir o prefixo da raiz, abrir sem bloquear
//! e sem seguir link no último componente, conferir **no descritor aberto** que é arquivo
//! regular e que o caminho real continua sob a raiz, conferir o tamanho e só então ler.

use std::fmt;
use std::path::{Path, PathBuf};
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
}

impl Refusal {
    /// Caminho como foi pedido no `.typ`.
    pub fn requested(&self) -> &str {
        todo!()
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

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
    pub fn new(_root: impl AsRef<Path>) -> std::io::Result<Self> {
        todo!()
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
    pub fn resolve_from(&self, _current: &str, _requested: &str) -> Result<PathBuf, Refusal> {
        todo!()
    }

    /// Passos 2–7 e leitura, para um caminho relativo à raiz.
    pub fn read(&self, requested: &str) -> Result<Vec<u8>, Refusal> {
        self.read_from("", requested)
    }

    /// Passos 2–7 e leitura, com o caminho pedido relativo à pasta de `current`.
    pub fn read_from(&self, _current: &str, _requested: &str) -> Result<Vec<u8>, Refusal> {
        todo!()
    }
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
