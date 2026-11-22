//! Projeto aberto (specs/003-editor-local). Registro `.prelo/project.json` em [`record`];
//! diálogo de abertura, versão do compilador e arquivo principal entram com T032.

pub mod record;

/// Versão do compilador embutido (T026 confere contra o Cargo.lock).
pub const EMBEDDED_TYPST: &str = "0.15.1";
