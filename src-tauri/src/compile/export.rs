//! Exportação do PDF (T067; US5). Esboço para os testes de T065: só os tipos.

use crate::diagnostic::Diagnostic;

/// Recusa de `export_pdf_dialog` (contracts/ipc.md "Exportação").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    NoProject,
    VersionMismatch,
    CompileErrors(Vec<Diagnostic>),
    WriteFailed,
}

impl serde::Serialize for ExportError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str("not_implemented")
    }
}

/// PDF do documento (esboço).
pub fn pdf(_document: &typst_layout::PagedDocument) -> Result<Vec<u8>, Vec<Diagnostic>> {
    Err(Vec::new())
}

/// PDF gravado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exported {
    pub compile_id: u64,
}
