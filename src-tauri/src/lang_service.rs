//! Serviço de linguagem do editor (T036; research R2, R3; D-17; contracts/ipc.md).
//!
//! Camada fina sobre o `typst-ide` oficial (`autocomplete`, `tooltip`) e o `typst-syntax`
//! (`LinkedNode` + `highlight`), sem `tinymist-query`. Tudo parte do mesmo World da prévia
//! ([`ProjectCompiler::sync`] com o mesmo [`CompileInput`]), então o texto ainda não salvo
//! entra nas sugestões (FR-011).
//!
//! Deslocamentos de entrada e de saída são **unidades UTF-16** do texto atual (as do CodeMirror
//! e de `edit_file`); o Typst trabalha em bytes, e a conversão acontece só aqui.

use std::sync::{Mutex, PoisonError};

use serde::Serialize;
use typst::syntax::Source;
use typst::World;
use typst_layout::PagedDocument;

use crate::commands::{Engine, EngineContext, JumpTarget, ProjectPath};
use crate::compile::project::{CompileInput, ProjectCompiler};

/// Sugestão de autocompletar: `{ label, kind?, detail?, info?, apply? }` (src/editor/completion.ts).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Completion {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply: Option<String>,
}

/// *Token* de realce: `{ from, to, kind }` em UTF-16 (src/editor/highlight.ts).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Token {
    pub from: usize,
    pub to: usize,
    pub kind: String,
}

/// Texto de *hover*; `code` diz se é código Typst (assinatura, valor) ou texto comum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hover {
    pub text: String,
    pub code: bool,
}

/// Sugestões em `offset` (UTF-16) de `source`, no World `world`. `document` (última compilação
/// boa) habilita os rótulos.
pub fn complete<W: World>(
    _world: &W,
    _document: Option<&PagedDocument>,
    _source: &Source,
    _offset: usize,
) -> Vec<Completion> {
    Vec::new()
}

/// Descrição do item em `offset` (UTF-16).
pub fn hover<W: World>(
    _world: &W,
    _document: Option<&PagedDocument>,
    _source: &Source,
    _offset: usize,
) -> Option<Hover> {
    None
}

/// *Tokens* de realce de `source`.
pub fn semantic_tokens(_source: &Source) -> Vec<Token> {
    Vec::new()
}

/// Motor real do [`Engine`]: mantém um World do projeto em sincronia com o estado da edição.
#[derive(Default)]
pub struct LangService {
    world: Mutex<Option<ProjectCompiler>>,
}

impl LangService {
    #[allow(dead_code)]
    fn source(&self, _ctx: &EngineContext, _path: &ProjectPath) -> Option<Source> {
        let _guard = self.world.lock().unwrap_or_else(PoisonError::into_inner);
        let _: Option<CompileInput> = None;
        None
    }
}

impl Engine for LangService {
    fn complete(
        &self,
        _ctx: &EngineContext,
        _path: &ProjectPath,
        _offset: usize,
    ) -> Result<Vec<Completion>, String> {
        Ok(Vec::new())
    }

    fn semantic_tokens(
        &self,
        _ctx: &EngineContext,
        _path: &ProjectPath,
    ) -> Result<Vec<Token>, String> {
        Ok(Vec::new())
    }

    fn jump_from_cursor(
        &self,
        _ctx: &EngineContext,
        _path: &ProjectPath,
        _offset: usize,
    ) -> Result<Vec<JumpTarget>, String> {
        Err("not_implemented".into())
    }
}
