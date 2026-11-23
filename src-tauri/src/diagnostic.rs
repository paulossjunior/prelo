//! Diagnóstico entregue à interface (specs/003-editor-local/data-model.md, "Diagnostic").
//!
//! Quem produz cada `kind` (World, escopo de arquivo, plugins, pacotes, fontes, registro)
//! entra com a tarefa correspondente. A forma no IPC está no `Serialize` abaixo.

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
    /// Em unidades UTF-16 do texto do arquivo: as mesmas do editor (CodeMirror) e de
    /// `edit_file`.
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

/// Forma do diagnóstico no IPC (`src/ipc.ts`, `Diagnostic`): campos ausentes não aparecem.
#[derive(serde::Serialize)]
struct Wire<'a> {
    severity: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    range: Option<WireRange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<u32>,
    message: &'a str,
    hints: &'a [String],
    kind: &'static str,
}

#[derive(serde::Serialize)]
struct WireRange {
    from: usize,
    to: usize,
}

impl serde::Serialize for Diagnostic {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Wire {
            severity: match self.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            },
            path: self.path.as_deref(),
            range: self.range.as_ref().map(|r| WireRange {
                from: r.start,
                to: r.end,
            }),
            line: self.line,
            message: &self.message,
            hints: &self.hints,
            kind: match self.kind {
                Kind::Typst => "typst",
                Kind::Access => "access",
                Kind::Plugin => "plugin",
                Kind::Package => "package",
                Kind::Font => "font",
                Kind::Project => "project",
            },
        }
        .serialize(serializer)
    }
}
