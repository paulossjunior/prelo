//! Comandos IPC da feature 003 (specs/003-editor-local/contracts/ipc.md). Esqueleto: só a
//! superfície usada pelos testes (T015); a implementação entra com T022.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;

/// Versão do compilador embutido (`typst` no Cargo.toml; T026 confere contra o Cargo.lock).
pub const EMBEDDED_TYPST: &str = "0.15.1";

/// Caminho pedido pela interface, já validado: dentro da raiz, canônico.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectPath {
    relative: String,
    absolute: PathBuf,
}

impl ProjectPath {
    pub fn relative(&self) -> &str {
        &self.relative
    }

    pub fn absolute(&self) -> &Path {
        &self.absolute
    }
}

/// `{ page, xPt, yPt }` de `jump_from_cursor`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JumpTarget {
    pub page: usize,
    pub x_pt: f64,
    pub y_pt: f64,
}

/// Consultas que dependem do World (T020) e dos serviços de prévia e linguagem.
pub trait Engine: Send + Sync + 'static {
    fn complete(&self, path: &ProjectPath, offset: usize) -> Result<Vec<Value>, String>;
    fn semantic_tokens(&self, path: &ProjectPath) -> Result<Vec<Value>, String>;
    fn jump_from_cursor(
        &self,
        path: &ProjectPath,
        offset: usize,
    ) -> Result<Vec<JumpTarget>, String>;
}

/// Estado do núcleo gerenciado pelo Tauri.
pub struct AppState {
    #[allow(dead_code)]
    engine: Arc<dyn Engine>,
}

impl AppState {
    pub fn new(engine: Arc<dyn Engine>) -> Self {
        Self { engine }
    }

    pub fn open_project(&self, _root: &Path) -> std::io::Result<ProjectInfo> {
        Ok(ProjectInfo::default())
    }

    pub fn open_file(&self, _path: &str) -> Result<OpenedFile, String> {
        Err("not_implemented".into())
    }

    pub fn edit_file(
        &self,
        _path: &str,
        _version: u64,
        _changes: &[Change],
    ) -> Result<u64, String> {
        Err("not_implemented".into())
    }

    pub fn choose_main_file(&self, _path: &str) -> Result<ProjectInfo, String> {
        Err("not_implemented".into())
    }
}

/// `ProjectInfo` (data-model.md).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    pub name: String,
    pub main_file: String,
    pub compiler: CompilerVersions,
    pub warnings: Vec<String>,
}

/// `ProjectInfo.compiler`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CompilerVersions {
    pub embedded: String,
    pub registered: Option<String>,
}

/// Resposta de `open_file`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpenedFile {
    pub text: String,
    pub version: u64,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(Arc::new(Unavailable))
    }
}

/// `{ from, to, insert }` de `edit_file`, em unidades UTF-16 do texto anterior à edição.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub from: usize,
    pub to: usize,
    pub insert: String,
}

/// Aplica as mudanças (esqueleto).
pub fn apply_changes(_text: &str, _changes: &[Change]) -> Result<String, &'static str> {
    Err("not_implemented")
}

/// Motor enquanto o World (T020) não existe.
pub struct Unavailable;

impl Engine for Unavailable {
    fn complete(&self, _path: &ProjectPath, _offset: usize) -> Result<Vec<Value>, String> {
        Err("not_implemented".into())
    }

    fn semantic_tokens(&self, _path: &ProjectPath) -> Result<Vec<Value>, String> {
        Err("not_implemented".into())
    }

    fn jump_from_cursor(
        &self,
        _path: &ProjectPath,
        _offset: usize,
    ) -> Result<Vec<JumpTarget>, String> {
        Err("not_implemented".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn change(from: usize, to: usize, insert: &str) -> Change {
        Change {
            from,
            to,
            insert: insert.into(),
        }
    }

    fn project() -> (TempDir, AppState) {
        let dir = tempfile::tempdir().expect("pasta temporária");
        std::fs::write(dir.path().join("main.typ"), "= Título\n").unwrap();
        std::fs::create_dir(dir.path().join("capitulos")).unwrap();
        std::fs::write(dir.path().join("capitulos/intro.typ"), "intro").unwrap();
        std::fs::write(dir.path().join("dados.bib"), "@x{}").unwrap();
        std::fs::write(dir.path().join("latin1.typ"), [0x63, 0xe7, 0x0a]).unwrap();
        let state = AppState::default();
        state.open_project(dir.path()).expect("abrir projeto");
        (dir, state)
    }

    #[test]
    fn mudancas_sao_aplicadas_nas_coordenadas_do_texto_anterior() {
        assert_eq!(apply_changes("abc", &[]).unwrap(), "abc");
        assert_eq!(apply_changes("abc", &[change(1, 2, "X")]).unwrap(), "aXc");
        assert_eq!(
            apply_changes("abcdef", &[change(0, 1, "1"), change(3, 5, "")]).unwrap(),
            "1bcf"
        );
        assert_eq!(apply_changes("abc", &[change(3, 3, "!")]).unwrap(), "abc!");
        // Inserção e troca no mesmo ponto, em ordem.
        assert_eq!(
            apply_changes("ab", &[change(1, 1, "X"), change(1, 2, "Y")]).unwrap(),
            "aXY"
        );
    }

    #[test]
    fn deslocamentos_sao_em_unidades_utf16() {
        // "ç" ocupa 1 unidade UTF-16 (2 bytes UTF-8); "𝑥" ocupa 2 (4 bytes).
        assert_eq!(apply_changes("çã", &[change(1, 2, "a")]).unwrap(), "ça");
        assert_eq!(apply_changes("𝑥y", &[change(2, 3, "z")]).unwrap(), "𝑥z");
        assert_eq!(apply_changes("𝑥", &[change(2, 2, "!")]).unwrap(), "𝑥!");
    }

    #[test]
    fn intervalo_invalido_e_bad_range_sem_aplicar_nada() {
        for changes in [
            vec![change(0, 4, "")],
            vec![change(2, 1, "")],
            vec![change(4, 4, "x")],
            vec![change(2, 3, ""), change(0, 1, "")],
            vec![change(0, 2, ""), change(1, 3, "")],
        ] {
            assert_eq!(
                apply_changes("abc", &changes),
                Err("bad_range"),
                "{changes:?}"
            );
        }
        // No meio de um par substituto.
        assert_eq!(apply_changes("𝑥", &[change(1, 1, "a")]), Err("bad_range"));
        assert_eq!(apply_changes("𝑥", &[change(0, 1, "")]), Err("bad_range"));
    }

    #[test]
    fn open_file_devolve_texto_e_versao_e_mantem_em_memoria() {
        let (dir, state) = project();
        let opened = state.open_file("main.typ").unwrap();
        assert_eq!(opened.text, "= Título\n");
        assert_eq!(opened.version, 0);
        let v = state
            .edit_file("main.typ", 0, &[change(0, 0, "x")])
            .unwrap();
        assert_eq!(v, 1);
        // Reabrir devolve o texto em memória (overlay), não o do disco.
        let again = state.open_file("main.typ").unwrap();
        assert_eq!(again.text, "x= Título\n");
        assert_eq!(again.version, 1);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("main.typ")).unwrap(),
            "= Título\n",
            "edit_file não grava no disco (save_file grava)"
        );
    }

    #[test]
    fn open_file_recusa_nao_utf8_pasta_e_inexistente() {
        let (_dir, state) = project();
        assert_eq!(state.open_file("latin1.typ"), Err("not_utf8".into()));
        assert_eq!(state.open_file("capitulos"), Err("not_found".into()));
        assert_eq!(state.open_file("nada.typ"), Err("not_found".into()));
        assert_eq!(state.open_file(""), Err("not_found".into()));
    }

    #[test]
    fn open_file_recusa_arquivo_grande_demais() {
        let (dir, state) = project();
        let f = std::fs::File::create(dir.path().join("grande.typ")).unwrap();
        // Arquivo esparso: não ocupa disco.
        f.set_len(crate::fs::scope::MAX_FILE_BYTES + 1).unwrap();
        assert_eq!(state.open_file("grande.typ"), Err("too_large".into()));
    }

    #[test]
    fn edit_file_confere_versao_e_intervalo() {
        let (_dir, state) = project();
        assert_eq!(
            state.edit_file("main.typ", 0, &[change(0, 0, "x")]),
            Err("not_found".into()),
            "arquivo não aberto"
        );
        state.open_file("main.typ").unwrap();
        assert_eq!(
            state.edit_file("main.typ", 3, &[change(0, 0, "x")]),
            Err("version_mismatch".into())
        );
        assert_eq!(
            state.edit_file("main.typ", 0, &[change(0, 99, "")]),
            Err("bad_range".into())
        );
        // Recusa não muda a versão.
        assert_eq!(state.edit_file("main.typ", 0, &[change(0, 0, "a")]), Ok(1));
        assert_eq!(state.edit_file("main.typ", 1, &[]), Ok(2));
        assert_eq!(state.open_file("main.typ").unwrap().text, "a= Título\n");
    }

    #[test]
    fn edit_file_pelo_caminho_alternativo_altera_o_mesmo_arquivo() {
        let (_dir, state) = project();
        state.open_file("main.typ").unwrap();
        assert_eq!(
            state.edit_file("capitulos/../main.typ", 0, &[change(0, 0, "y")]),
            Ok(1)
        );
        assert_eq!(state.open_file("./main.typ").unwrap().text, "y= Título\n");
    }

    #[test]
    fn choose_main_file_aceita_so_typ_e_grava_o_registro() {
        let (dir, state) = project();
        assert_eq!(
            state.choose_main_file("dados.bib"),
            Err("not_a_typ_file".into())
        );
        assert_eq!(
            state.choose_main_file("capitulos"),
            Err("not_a_typ_file".into())
        );
        assert_eq!(
            state.choose_main_file("../main.typ"),
            Err("path_outside_project".into())
        );
        let info = state.choose_main_file("capitulos/intro.typ").unwrap();
        assert_eq!(info.main_file, "capitulos/intro.typ");
        assert_eq!(info.compiler.embedded, EMBEDDED_TYPST);
        let record = std::fs::read_to_string(dir.path().join(".prelo/project.json")).unwrap();
        assert!(record.contains("capitulos/intro.typ"), "{record}");
    }

    #[test]
    fn registro_invalido_nao_e_regravado_ao_escolher_principal() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.typ"), "a").unwrap();
        std::fs::create_dir(dir.path().join(".prelo")).unwrap();
        std::fs::write(dir.path().join(".prelo/project.json"), "{ quebrado").unwrap();
        let state = AppState::default();
        let info = state.open_project(dir.path()).unwrap();
        assert_eq!(info.warnings, vec!["project_json_invalid".to_string()]);
        assert_eq!(info.main_file, "main.typ");
        let info = state.choose_main_file("a.typ").unwrap();
        assert_eq!(info.main_file, "a.typ");
        assert_eq!(info.warnings, vec!["project_json_invalid".to_string()]);
        assert_eq!(
            std::fs::read_to_string(dir.path().join(".prelo/project.json")).unwrap(),
            "{ quebrado"
        );
    }

    #[test]
    fn motor_indisponivel_responde_not_implemented() {
        let p = ProjectPath {
            relative: "main.typ".into(),
            absolute: PathBuf::from("main.typ"),
        };
        assert_eq!(Unavailable.complete(&p, 0), Err("not_implemented".into()));
        assert_eq!(
            Unavailable.semantic_tokens(&p),
            Err("not_implemented".into())
        );
        assert_eq!(
            Unavailable.jump_from_cursor(&p, 0),
            Err("not_implemented".into())
        );
    }
}
