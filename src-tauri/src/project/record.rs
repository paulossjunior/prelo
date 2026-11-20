//! Registro do projeto, `.prelo/project.json` (specs/003-editor-local/data-model.md,
//! "ProjectRecord").
//!
//! O arquivo é entrada não confiável (FR-024): JSON inválido, maior que 1 MiB, `schema`
//! desconhecido, tipo errado ou `main` fora da raiz não impedem a abertura do projeto. O
//! projeto abre com o aviso `project_json_invalid`, usa os padrões e o arquivo não é regravado
//! até a pessoa escolher "Recriar registro". Campos desconhecidos são preservados ao regravar e
//! nunca interpretados.

use std::fmt;
use std::path::Path;

use serde_json::{Map, Value};

/// Caminho do registro relativo à raiz do projeto.
pub const RECORD_PATH: &str = ".prelo/project.json";
/// Tamanho máximo aceito do registro (1 MiB).
pub const MAX_RECORD_BYTES: u64 = 1024 * 1024;
/// Única versão de esquema conhecida nesta fase.
pub const SCHEMA: u64 = 1;
/// Arquivo principal usado quando o registro não diz outro (ou é inválido).
pub const DEFAULT_MAIN: &str = "main.typ";
/// Código do aviso entregue à interface (contracts/ipc.md, `open_project_dialog`).
pub const WARNING_CODE: &str = "project_json_invalid";

/// Conteúdo de `.prelo/project.json`.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectRecord {
    compiler_typst: String,
    main: Option<String>,
    /// Campos desconhecidos do objeto raiz, preservados ao regravar.
    extra: Map<String, Value>,
    /// Campos desconhecidos de `compiler`, preservados ao regravar.
    compiler_extra: Map<String, Value>,
}

/// Por que o registro foi considerado inválido. Toda variante vira o aviso `project_json_invalid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidReason {
    /// Maior que [`MAX_RECORD_BYTES`].
    TooLarge,
    /// Não é um arquivo comum (link simbólico, pasta, FIFO, dispositivo).
    NotAFile,
    /// JSON malformado ou não UTF-8.
    Json(String),
    /// `schema` com valor desconhecido ou ausente.
    UnknownSchema(String),
    /// Campo com tipo errado ou obrigatório ausente; guarda o nome do campo.
    WrongType(&'static str),
    /// `main` vazio, absoluto, com `..` ou com byte nulo.
    BadMain(String),
    /// Erro de leitura.
    Io(String),
}

/// Estado do registro ao abrir o projeto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordState {
    /// Não existe; pode ser criado (FR-003).
    Missing,
    /// Lido e válido.
    Valid,
    /// Inválido: padrões em uso, aviso exibido, sem regravação.
    Invalid(InvalidReason),
}

/// Registro em uso pelo projeto aberto e o estado em que foi lido.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedRecord {
    pub record: ProjectRecord,
    pub state: RecordState,
}

/// Falha ao gravar o registro.
#[derive(Debug)]
pub enum SaveError {
    /// O registro lido era inválido; só "Recriar registro" pode sobrescrevê-lo.
    Blocked,
    Io(std::io::Error),
}

impl ProjectRecord {
    /// Registro novo com a versão do compilador embutido e sem `main`.
    pub fn new(embedded_typst: &str) -> Self {
        let _ = embedded_typst;
        todo!("T019")
    }

    /// Interpreta os bytes do arquivo.
    pub fn parse(bytes: &[u8]) -> Result<Self, InvalidReason> {
        let _ = bytes;
        todo!("T019")
    }

    /// Serializa o registro, com os campos desconhecidos preservados.
    pub fn to_json(&self) -> Vec<u8> {
        todo!("T019")
    }

    pub fn schema(&self) -> u64 {
        SCHEMA
    }

    pub fn compiler_typst(&self) -> &str {
        &self.compiler_typst
    }

    /// Só `update_project_compiler_version` chama isto (FR-012).
    pub fn set_compiler_typst(&mut self, version: &str) {
        let _ = version;
        todo!("T019")
    }

    pub fn main(&self) -> Option<&str> {
        self.main.as_deref()
    }

    /// `main` registrado ou [`DEFAULT_MAIN`].
    pub fn main_or_default(&self) -> &str {
        todo!("T019")
    }

    /// Só `choose_main_file` chama isto. Recusa caminho absoluto, com `..`, vazio ou com `\0`.
    pub fn set_main(&mut self, main: &str) -> Result<(), InvalidReason> {
        let _ = main;
        todo!("T019")
    }
}

impl InvalidReason {
    /// Código do aviso para a interface.
    pub fn code(&self) -> &'static str {
        WARNING_CODE
    }
}

impl fmt::Display for InvalidReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = f;
        todo!("T019")
    }
}

impl LoadedRecord {
    /// O arquivo pode ser regravado (criado ou atualizado) sem a pessoa pedir.
    pub fn may_rewrite(&self) -> bool {
        todo!("T019")
    }

    /// Aviso a mostrar ao abrir o projeto, se houver.
    pub fn warning(&self) -> Option<&InvalidReason> {
        todo!("T019")
    }
}

/// Lê o registro de `root`. Nunca falha: problemas viram [`RecordState::Invalid`] com padrões.
pub fn load(root: &Path, embedded_typst: &str) -> LoadedRecord {
    let _ = (root, embedded_typst);
    todo!("T019")
}

/// Grava o registro de forma atômica. Recusa com [`SaveError::Blocked`] se ele foi lido
/// inválido. Depois de gravar, o estado passa a [`RecordState::Valid`].
pub fn save(root: &Path, loaded: &mut LoadedRecord) -> Result<(), SaveError> {
    let _ = (root, loaded);
    todo!("T019")
}

/// "Recriar registro": sobrescreve o arquivo com um registro novo, descartando o anterior.
pub fn recreate(root: &Path, embedded_typst: &str) -> Result<LoadedRecord, SaveError> {
    let _ = (root, embedded_typst);
    todo!("T019")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    const EMBEDDED: &str = "0.15.1";

    fn project_with(contents: &[u8]) -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join(".prelo")).unwrap();
        fs::write(dir.path().join(RECORD_PATH), contents).unwrap();
        dir
    }

    fn json(bytes: &[u8]) -> Value {
        serde_json::from_slice(bytes).unwrap()
    }

    fn assert_invalid_with_defaults(contents: &[u8]) -> InvalidReason {
        let dir = project_with(contents);
        let loaded = load(dir.path(), EMBEDDED);
        let reason = match &loaded.state {
            RecordState::Invalid(r) => r.clone(),
            other => panic!("esperava Invalid, veio {other:?}"),
        };
        assert_eq!(loaded.warning().map(InvalidReason::code), Some(WARNING_CODE));
        assert_eq!(loaded.record.main_or_default(), DEFAULT_MAIN);
        assert_eq!(loaded.record.compiler_typst(), EMBEDDED);
        assert!(!loaded.may_rewrite());
        reason
    }

    // --- Leitura válida -------------------------------------------------------------------

    #[test]
    fn parses_valid_record() {
        let r = ProjectRecord::parse(
            br#"{ "schema": 1, "compiler": { "typst": "0.15.1" }, "main": "capitulos/artigo.typ" }"#,
        )
        .unwrap();
        assert_eq!(r.schema(), 1);
        assert_eq!(r.compiler_typst(), "0.15.1");
        assert_eq!(r.main(), Some("capitulos/artigo.typ"));
        assert_eq!(r.main_or_default(), "capitulos/artigo.typ");
    }

    #[test]
    fn main_absent_is_valid_and_defaults() {
        let r = ProjectRecord::parse(br#"{ "schema": 1, "compiler": { "typst": "0.14.0" } }"#)
            .unwrap();
        assert_eq!(r.main(), None);
        assert_eq!(r.main_or_default(), DEFAULT_MAIN);
        assert_eq!(r.compiler_typst(), "0.14.0");
    }

    #[test]
    fn load_valid_record_from_disk() {
        let dir = project_with(br#"{"schema":1,"compiler":{"typst":"0.14.0"},"main":"a.typ"}"#);
        let loaded = load(dir.path(), EMBEDDED);
        assert_eq!(loaded.state, RecordState::Valid);
        assert!(loaded.warning().is_none());
        assert!(loaded.may_rewrite());
        // A versão registrada é mantida, mesmo diferente da embutida (estado VersaoDiferente
        // é decidido fora do registro, T032).
        assert_eq!(loaded.record.compiler_typst(), "0.14.0");
        assert_eq!(loaded.record.main(), Some("a.typ"));
    }

    // --- Ausente: padrões e criação permitida (FR-003) -----------------------------------

    #[test]
    fn missing_record_uses_defaults_and_may_be_created() {
        let dir = TempDir::new().unwrap();
        let mut loaded = load(dir.path(), EMBEDDED);
        assert_eq!(loaded.state, RecordState::Missing);
        assert!(loaded.warning().is_none());
        assert!(loaded.may_rewrite());
        assert_eq!(loaded.record.compiler_typst(), EMBEDDED);
        assert_eq!(loaded.record.main_or_default(), DEFAULT_MAIN);

        save(dir.path(), &mut loaded).unwrap();
        assert_eq!(loaded.state, RecordState::Valid);
        let written = json(&fs::read(dir.path().join(RECORD_PATH)).unwrap());
        assert_eq!(written["schema"], 1);
        assert_eq!(written["compiler"]["typst"], EMBEDDED);
        // Nenhum outro arquivo criado na raiz além de .prelo/project.json.
        let entries: Vec<_> = fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(entries.len(), 1);
        let prelo: Vec<_> = fs::read_dir(dir.path().join(".prelo")).unwrap().collect();
        assert_eq!(prelo.len(), 1);
    }

    #[test]
    fn new_record_serializes_to_data_model_shape() {
        let mut r = ProjectRecord::new(EMBEDDED);
        r.set_main("main.typ").unwrap();
        assert_eq!(
            json(&r.to_json()),
            json(br#"{ "schema": 1, "compiler": { "typst": "0.15.1" }, "main": "main.typ" }"#)
        );
        let without_main = ProjectRecord::new(EMBEDDED);
        assert!(json(&without_main.to_json()).get("main").is_none());
    }

    // --- Inválido: aviso, padrões, sem regravação -----------------------------------------

    #[test]
    fn malformed_json_is_invalid() {
        let reason = assert_invalid_with_defaults(br#"{ "schema": 1, "compiler": "#);
        assert!(matches!(reason, InvalidReason::Json(_)), "{reason:?}");
    }

    #[test]
    fn non_utf8_is_invalid() {
        let reason = assert_invalid_with_defaults(b"{\"schema\":1,\"main\":\"\xff\xfe\"}");
        assert!(matches!(reason, InvalidReason::Json(_)), "{reason:?}");
    }

    #[test]
    fn empty_file_is_invalid() {
        assert_invalid_with_defaults(b"");
    }

    #[test]
    fn larger_than_limit_is_invalid() {
        let mut big = br#"{"schema":1,"compiler":{"typst":"0.15.1"}}"#.to_vec();
        big.resize(MAX_RECORD_BYTES as usize + 1, b' ');
        assert_eq!(ProjectRecord::parse(&big), Err(InvalidReason::TooLarge));
        assert_eq!(assert_invalid_with_defaults(&big), InvalidReason::TooLarge);
    }

    #[test]
    fn exactly_at_limit_is_accepted() {
        let mut at = br#"{"schema":1,"compiler":{"typst":"0.15.1"}}"#.to_vec();
        at.resize(MAX_RECORD_BYTES as usize, b' ');
        assert!(ProjectRecord::parse(&at).is_ok());
    }

    #[test]
    fn unknown_schema_is_invalid() {
        for bad in [
            &br#"{"schema":2,"compiler":{"typst":"0.15.1"}}"#[..],
            br#"{"schema":0,"compiler":{"typst":"0.15.1"}}"#,
            br#"{"compiler":{"typst":"0.15.1"}}"#,
        ] {
            let r = ProjectRecord::parse(bad);
            assert!(matches!(r, Err(InvalidReason::UnknownSchema(_))), "{r:?}");
        }
        assert!(matches!(
            assert_invalid_with_defaults(br#"{"schema":99,"compiler":{"typst":"0.15.1"}}"#),
            InvalidReason::UnknownSchema(_)
        ));
    }

    #[test]
    fn wrong_types_are_invalid() {
        for bad in [
            &br#"[1, 2, 3]"#[..],
            br#""texto""#,
            br#"{"schema":"1","compiler":{"typst":"0.15.1"}}"#,
            br#"{"schema":1.5,"compiler":{"typst":"0.15.1"}}"#,
            br#"{"schema":1,"compiler":"0.15.1"}"#,
            br#"{"schema":1,"compiler":{"typst":15}}"#,
            br#"{"schema":1,"compiler":{"typst":""}}"#,
            br#"{"schema":1,"compiler":{}}"#,
            br#"{"schema":1}"#,
            br#"{"schema":1,"compiler":{"typst":"0.15.1"},"main":7}"#,
            br#"{"schema":1,"compiler":{"typst":"0.15.1"},"main":null}"#,
        ] {
            assert!(
                ProjectRecord::parse(bad).is_err(),
                "aceitou {}",
                String::from_utf8_lossy(bad)
            );
        }
        assert_invalid_with_defaults(br#"{"schema":1,"compiler":{"typst":15}}"#);
    }

    #[test]
    fn main_outside_root_is_invalid() {
        for main in [
            "/etc/passwd",
            "/main.typ",
            "\\\\servidor\\c\\main.typ",
            "\\main.typ",
            "C:\\main.typ",
            "C:main.typ",
            "../fora.typ",
            "capitulos/../../fora.typ",
            "capitulos/../main.typ",
            "capitulos\\..\\..\\fora.typ",
            "..",
            "",
            "a\0b.typ",
        ] {
            let text = serde_json::json!({
                "schema": 1, "compiler": { "typst": "0.15.1" }, "main": main
            })
            .to_string();
            let r = ProjectRecord::parse(text.as_bytes());
            assert!(matches!(r, Err(InvalidReason::BadMain(_))), "{main:?}: {r:?}");
            assert!(matches!(
                assert_invalid_with_defaults(text.as_bytes()),
                InvalidReason::BadMain(_)
            ));
        }
    }

    #[test]
    fn main_inside_root_is_valid() {
        for main in ["main.typ", "capitulos/intro.typ", "./a.typ"] {
            let mut r = ProjectRecord::new(EMBEDDED);
            assert!(r.set_main(main).is_ok(), "{main}");
            assert_eq!(r.main(), Some(main));
        }
    }

    #[test]
    fn set_main_rejects_escape_and_keeps_previous() {
        let mut r = ProjectRecord::new(EMBEDDED);
        r.set_main("main.typ").unwrap();
        assert!(matches!(r.set_main("../x.typ"), Err(InvalidReason::BadMain(_))));
        assert!(matches!(r.set_main("/x.typ"), Err(InvalidReason::BadMain(_))));
        assert_eq!(r.main(), Some("main.typ"));
    }

    #[test]
    fn invalid_record_is_never_rewritten() {
        let original = br#"{ "schema": 7, "compiler": { "typst": "0.15.1" }, "x": 1 }"#;
        let dir = project_with(original);
        let mut loaded = load(dir.path(), EMBEDDED);
        assert!(!loaded.may_rewrite());
        loaded.record.set_main("outro.typ").unwrap();
        assert!(matches!(save(dir.path(), &mut loaded), Err(SaveError::Blocked)));
        assert_eq!(fs::read(dir.path().join(RECORD_PATH)).unwrap(), original);
        assert!(matches!(loaded.state, RecordState::Invalid(_)));
    }

    #[test]
    fn recreate_replaces_invalid_record() {
        let dir = project_with(b"isto nao e json");
        let loaded = recreate(dir.path(), EMBEDDED).unwrap();
        assert_eq!(loaded.state, RecordState::Valid);
        let again = load(dir.path(), EMBEDDED);
        assert_eq!(again.state, RecordState::Valid);
        assert_eq!(again.record.compiler_typst(), EMBEDDED);
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_record_is_not_followed() {
        let outside = TempDir::new().unwrap();
        let target = outside.path().join("fora.json");
        fs::write(&target, br#"{"schema":1,"compiler":{"typst":"0.1.0"},"main":"x.typ"}"#)
            .unwrap();
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join(".prelo")).unwrap();
        std::os::unix::fs::symlink(&target, dir.path().join(RECORD_PATH)).unwrap();
        let loaded = load(dir.path(), EMBEDDED);
        assert_eq!(loaded.state, RecordState::Invalid(InvalidReason::NotAFile));
        assert_eq!(loaded.record.compiler_typst(), EMBEDDED);
    }

    #[test]
    fn directory_in_place_of_record_is_invalid() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(RECORD_PATH)).unwrap();
        let loaded = load(dir.path(), EMBEDDED);
        assert_eq!(loaded.state, RecordState::Invalid(InvalidReason::NotAFile));
    }

    // --- Campos desconhecidos preservados ao regravar -------------------------------------

    #[test]
    fn unknown_fields_are_preserved_on_rewrite() {
        let dir = project_with(
            br#"{
                "schema": 1,
                "compiler": { "typst": "0.15.1", "flags": ["a"] },
                "main": "main.typ",
                "participants": [{ "id": "p1", "role": "autor" }],
                "futuro": { "aninhado": true }
            }"#,
        );
        let mut loaded = load(dir.path(), EMBEDDED);
        assert_eq!(loaded.state, RecordState::Valid);
        loaded.record.set_main("capitulos/artigo.typ").unwrap();
        save(dir.path(), &mut loaded).unwrap();

        let written = json(&fs::read(dir.path().join(RECORD_PATH)).unwrap());
        assert_eq!(written["main"], "capitulos/artigo.typ");
        assert_eq!(written["schema"], 1);
        assert_eq!(written["compiler"]["typst"], "0.15.1");
        assert_eq!(written["compiler"]["flags"], serde_json::json!(["a"]));
        assert_eq!(
            written["participants"],
            serde_json::json!([{ "id": "p1", "role": "autor" }])
        );
        assert_eq!(written["futuro"], serde_json::json!({ "aninhado": true }));
    }

    #[test]
    fn set_compiler_version_is_written() {
        let dir = project_with(br#"{"schema":1,"compiler":{"typst":"0.14.0"}}"#);
        let mut loaded = load(dir.path(), EMBEDDED);
        loaded.record.set_compiler_typst(EMBEDDED);
        save(dir.path(), &mut loaded).unwrap();
        assert_eq!(load(dir.path(), EMBEDDED).record.compiler_typst(), EMBEDDED);
    }

    #[test]
    fn warning_message_is_portuguese_and_names_the_file() {
        let msg = InvalidReason::TooLarge.to_string();
        assert!(msg.contains(RECORD_PATH), "{msg}");
        assert_eq!(InvalidReason::TooLarge.code(), "project_json_invalid");
    }
}
