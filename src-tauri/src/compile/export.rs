//! Exportação do PDF (T067; US5; FR-011, FR-028, FR-029; contracts/ipc.md "Exportação").
//!
//! O PDF sai do documento da última prévia `Ok` ([`PreviewHub::export_source`]): mesma
//! compilação, mesmo World e mesma versão do compilador da prévia e dos diagnósticos (FR-011).
//! Nada é recompilado para exportar. Se a última compilação teve erro, a exportação é recusada
//! com `compile_errors` e os diagnósticos dela (FR-029); com versão registrada ≠ embutida,
//! `compiler_version_mismatch` (FR-012), porque nada pode sair de versão diferente da
//! registrada.
//!
//! O PDF é gerado com `typst-pdf` e as opções padrão, as mesmas do `typst compile` (PDF com
//! *tags*, sem padrão PDF/A), menos a data de criação: o CLI grava a hora atual, o Prelo não
//! grava data (PDF reprodutível; `set document(date: ..)` no `.typ` continua valendo). A
//! gravação é atômica ([`write_atomic`]): falha não deixa arquivo parcial nem temporário e
//! mantém o PDF anterior no mesmo caminho.
//!
//! `export_pdf_dialog` confere tudo isso antes de mostrar o diálogo nativo de salvar, que é do
//! núcleo: a webview não tem permissão de diálogo nem de arquivo, e o caminho escolhido nunca
//! passa pela interface.
//!
//! [`PreviewHub::export_source`]: super::preview::PreviewHub::export_source

use std::path::{Path, PathBuf};

use serde::ser::SerializeMap;
use serde::Serialize;
use tauri::ipc::Request;
use tauri::{Runtime, State, Window};
use tauri_plugin_dialog::DialogExt;
use typst_layout::PagedDocument;
use typst_pdf::PdfOptions;

use crate::commands::{no_args, AppState};
use crate::diagnostic::{Diagnostic, Kind};
use crate::fs::save::write_atomic;

pub const COMPILE_ERRORS: &str = "compile_errors";
pub const VERSION_MISMATCH: &str = "compiler_version_mismatch";
pub const WRITE_FAILED: &str = "write_failed";
pub const NO_PROJECT: &str = "no_project";
pub const INVALID_ARGUMENTS: &str = "invalid_arguments";

/// Recusa da exportação. No IPC, um código em texto ou, para `compile_errors`,
/// `{ code, diagnostics }` (o formato que `src/ipc.ts` lê em `toIpcError`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    NoProject,
    VersionMismatch,
    CompileErrors(Vec<Diagnostic>),
    WriteFailed,
    InvalidArguments,
}

impl ExportError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NoProject => NO_PROJECT,
            Self::VersionMismatch => VERSION_MISMATCH,
            Self::CompileErrors(_) => COMPILE_ERRORS,
            Self::WriteFailed => WRITE_FAILED,
            Self::InvalidArguments => INVALID_ARGUMENTS,
        }
    }
}

impl Serialize for ExportError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::CompileErrors(diagnostics) => {
                let mut map = s.serialize_map(Some(2))?;
                map.serialize_entry("code", COMPILE_ERRORS)?;
                map.serialize_entry("diagnostics", diagnostics)?;
                map.end()
            }
            other => s.serialize_str(other.code()),
        }
    }
}

/// PDF gravado: de qual compilação veio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exported {
    pub compile_id: u64,
}

/// Resposta de `export_pdf_dialog`: o caminho gravado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportResult {
    pub path: String,
}

/// Documento da última prévia `Ok`, pronto para exportar.
pub struct ExportSource {
    pub compile_id: u64,
    pub document: std::sync::Arc<PagedDocument>,
}

/// Diagnóstico de quando nenhuma compilação terminou ainda (projeto recém-aberto).
pub fn not_compiled_yet() -> Diagnostic {
    Diagnostic::error(
        Kind::Project,
        "O documento ainda está compilando; tente exportar de novo em instantes.",
    )
}

/// PDF do documento com as opções do `typst compile` (ver o topo do módulo).
pub fn pdf(document: &PagedDocument) -> Result<Vec<u8>, Vec<Diagnostic>> {
    typst_pdf::pdf(document, &PdfOptions::default()).map_err(|errors| {
        errors
            .iter()
            .map(|e| {
                let mut diag = Diagnostic::error(Kind::Typst, e.message.as_str());
                diag.hints = e.hints.iter().map(|h| h.v.to_string()).collect();
                diag
            })
            .collect()
    })
}

/// Gera o PDF de `source` e grava em `path` de forma atômica.
pub fn export(source: &ExportSource, path: &Path) -> Result<Exported, ExportError> {
    let bytes = pdf(&source.document).map_err(ExportError::CompileErrors)?;
    write_atomic(path, &bytes).map_err(|e| {
        eprintln!("prelo: falha ao gravar o PDF: {e}");
        ExportError::WriteFailed
    })?;
    Ok(Exported {
        compile_id: source.compile_id,
    })
}

/// Caminho escolhido sem a extensão `.pdf` ganha `.pdf` no fim.
fn with_pdf_extension(path: PathBuf) -> PathBuf {
    if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
    {
        return path;
    }
    let mut name = path.into_os_string();
    name.push(".pdf");
    name.into()
}

/// "Exportar PDF": confere o projeto, mostra o diálogo nativo de salvar e grava o PDF da
/// última prévia `Ok`. Cancelado → `null`.
#[tauri::command(async)]
pub fn export_pdf_dialog<R: Runtime>(
    window: Window<R>,
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<Option<ExportResult>, ExportError> {
    no_args(&request).map_err(|_| ExportError::InvalidArguments)?;
    let (source, name, root) = state.export_source()?;
    let Some(chosen) = window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title("Exportar PDF")
        .set_directory(&root)
        .set_file_name(format!("{name}.pdf"))
        .add_filter("PDF", &["pdf"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path = with_pdf_extension(chosen.into_path().map_err(|_| ExportError::WriteFailed)?);
    export(&source, &path)?;
    Ok(Some(ExportResult {
        path: path.to_string_lossy().into_owned(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensao_pdf_so_quando_falta() {
        assert_eq!(
            with_pdf_extension(PathBuf::from("/x/artigo")),
            PathBuf::from("/x/artigo.pdf")
        );
        assert_eq!(
            with_pdf_extension(PathBuf::from("/x/artigo.pdf")),
            PathBuf::from("/x/artigo.pdf")
        );
        assert_eq!(
            with_pdf_extension(PathBuf::from("/x/artigo.v2.PDF")),
            PathBuf::from("/x/artigo.v2.PDF")
        );
        assert_eq!(
            with_pdf_extension(PathBuf::from("/x/artigo.v2")),
            PathBuf::from("/x/artigo.v2.pdf")
        );
    }
}
