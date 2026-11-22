//! Diagnóstico entregue à interface (specs/003-editor-local/data-model.md, "Diagnostic").
//!
//! Nesta etapa só o tipo; quem produz cada `kind` (World, escopo de arquivo, plugins,
//! pacotes, fontes, registro) entra com a tarefa correspondente.

use std::ops::Range;

/// Gravidade do diagnóstico.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// Origem do diagnóstico (`Diagnostic.kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Typst,
    Access,
    Plugin,
    Package,
    Font,
    Project,
}

/// `{ severity, path?, range?, line?, message, hints[], kind }`. Diagnóstico de acesso usa o
/// caminho pedido, nunca o real.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub path: Option<String>,
    pub range: Option<Range<usize>>,
    pub line: Option<u32>,
    pub message: String,
    pub hints: Vec<String>,
    pub kind: Kind,
}

impl Diagnostic {
    /// Erro sem posição.
    pub fn error(kind: Kind, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            path: None,
            range: None,
            line: None,
            message: message.into(),
            hints: Vec::new(),
            kind,
        }
    }
}

impl serde::Serialize for Diagnostic {
    fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
        todo!("T035")
    }
}
