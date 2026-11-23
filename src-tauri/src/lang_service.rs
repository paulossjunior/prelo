//! Serviço de linguagem do editor (T036; research R2, R3; D-17; contracts/ipc.md).
//!
//! Camada fina sobre o `typst-ide` oficial (`autocomplete`, `tooltip`) e o `typst-syntax`
//! (`LinkedNode` + `highlight`), sem `tinymist-query`. Tudo parte do mesmo World da prévia
//! ([`ProjectCompiler::sync`] com o mesmo [`CompileInput`](crate::compile::project::CompileInput)),
//! então o texto ainda não salvo entra nas sugestões (FR-011). Rótulos vêm do último documento
//! compilado com sucesso pela prévia.
//!
//! Deslocamentos de entrada e de saída são **unidades UTF-16** do texto atual (as do CodeMirror
//! e de `edit_file`); o Typst trabalha em bytes, e a conversão acontece só aqui.

use std::path::Path;
use std::sync::{Mutex, PoisonError};

use serde::Serialize;
use tinymist_world::CompilerWorld;
use typst::diag::FileResult;
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{highlight, FileId, LinkedNode, Side, Source, Tag, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};
use typst_ide::{CompletionKind, IdeWorld, Tooltip};
use typst_layout::PagedDocument;

use crate::commands::{Engine, EngineContext, JumpTarget, ProjectPath};
use crate::compile::project::ProjectCompiler;
use crate::fs::scope::Scope;
use crate::world::PreloFeat;

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

/// Visão de qualquer [`World`] como [`IdeWorld`], com a lista de arquivos do projeto já
/// filtrada pela regra de escopo ([`path_candidates`]). Sem lista de pacotes.
pub(crate) struct Ide<'a, W> {
    world: &'a W,
    files: Vec<FileId>,
}

impl<'a, W> Ide<'a, W> {
    /// Sem lista de arquivos (saltos da prévia, US4: nada a sugerir).
    pub(crate) fn new(world: &'a W) -> Self {
        Self {
            world,
            files: Vec::new(),
        }
    }
}

/// Profundidade máxima de pastas varridas para sugerir caminhos.
const MAX_SCAN_DEPTH: usize = 16;
/// Quantidade máxima de caminhos sugeridos (e de entradas de pasta examinadas, ×10): projeto
/// enorme não trava o autocompletar.
const MAX_PATH_CANDIDATES: usize = 2_000;

/// Arquivos do projeto que podem ser sugeridos no autocompletar de caminhos (RF-08, FR-021;
/// contracts/file-access.md, "Autocompletar de caminhos"), relativos à raiz com `/`.
///
/// - Cada entrada passa por [`Scope::resolve`] (passos 2–4): link simbólico que leva para
///   fora some; link interno para arquivo fica.
/// - Só arquivo comum entra (FIFO, dispositivo e socket não).
/// - Pasta que é link simbólico não é varrida, mesmo interna: evita laço e não lista nada de
///   fora.
/// - Nomes iniciados por `.` (`.prelo/`, `.git/`) e não UTF-8 ficam de fora.
///
/// Só lista pastas e consulta metadados; nenhum arquivo é aberto.
pub fn path_candidates(scope: &Scope) -> Vec<String> {
    let mut out = Vec::new();
    let mut examined = 0usize;
    let mut stack: Vec<(std::path::PathBuf, String, usize)> =
        vec![(scope.root().to_path_buf(), String::new(), 0)];
    while let Some((dir, prefix, depth)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            examined += 1;
            if out.len() >= MAX_PATH_CANDIDATES || examined > MAX_PATH_CANDIDATES * 10 {
                out.sort();
                return out;
            }
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            if name.starts_with('.') {
                continue;
            }
            // `file_type` da entrada não segue link simbólico.
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let relative = format!("{prefix}{name}");
            if kind.is_dir() {
                if depth < MAX_SCAN_DEPTH {
                    stack.push((entry.path(), format!("{relative}/"), depth + 1));
                }
                continue;
            }
            if !(kind.is_file() || kind.is_symlink()) {
                continue;
            }
            let Ok(canonical) = scope.resolve(&relative) else {
                continue;
            };
            if std::fs::metadata(&canonical).is_ok_and(|m| m.is_file()) {
                out.push(relative);
            }
        }
    }
    out.sort();
    out
}

impl<W: World> World for Ide<'_, W> {
    fn library(&self) -> &LazyHash<Library> {
        self.world.library()
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.world.book()
    }

    fn main(&self) -> FileId {
        self.world.main()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.world.source(id)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.world.file(id)
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.world.font(index)
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        self.world.today(offset)
    }
}

impl<W: World> IdeWorld for Ide<'_, W> {
    fn upcast(&self) -> &dyn World {
        self
    }

    fn files(&self) -> Vec<FileId> {
        self.files.clone()
    }
}

/// `FileId`s, na raiz do arquivo principal de `world`, dos caminhos de [`path_candidates`].
fn project_files<W: World>(world: &W, scope: &Scope) -> Vec<FileId> {
    let main = world.main();
    path_candidates(scope)
        .into_iter()
        .filter_map(|relative| {
            let vpath = VirtualPath::new(&relative).ok()?;
            Some(FileId::new(main.get().map(|_| vpath)))
        })
        .collect()
}

/// UTF-16 → byte em `source`; `None` fora do texto ou no meio de um caractere.
fn to_byte(source: &Source, offset: usize) -> Option<usize> {
    let byte = source.lines().utf16_to_byte(offset)?;
    source.text().is_char_boundary(byte).then_some(byte)
}

/// Tipo do item no vocabulário do CodeMirror (`type` de `Completion`).
fn kind_name(kind: &CompletionKind) -> &'static str {
    match kind {
        CompletionKind::Syntax => "keyword",
        CompletionKind::Func => "function",
        CompletionKind::Type => "type",
        CompletionKind::Param => "property",
        CompletionKind::Constant => "variable",
        CompletionKind::Path => "text",
        CompletionKind::Package => "namespace",
        CompletionKind::Label => "label",
        CompletionKind::Font => "text",
        CompletionKind::Symbol(_) => "constant",
    }
}

/// Tira a sintaxe de *snippet* do `typst-ide` (`${nome}` → `nome`, `${}` → nada): a interface
/// insere `apply` como texto puro.
fn plain_snippet(snippet: &str) -> String {
    let mut out = String::with_capacity(snippet.len());
    let mut rest = snippet;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find('}') {
            Some(end) => {
                out.push_str(&after[..end]);
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

fn convert(c: typst_ide::Completion) -> Completion {
    let label = c.label.to_string();
    let apply = c
        .apply
        .map(|a| plain_snippet(&a))
        .filter(|a| *a != label && !a.is_empty());
    let detail = match (&c.kind, c.detail) {
        (_, Some(d)) => Some(d.to_string()),
        (CompletionKind::Symbol(s), None) => Some(s.to_string()),
        _ => None,
    };
    Completion {
        kind: Some(kind_name(&c.kind).to_owned()),
        label: plain_snippet(&label),
        detail,
        info: None,
        apply,
    }
}

/// Sugestões em `offset` (UTF-16) de `source`, no World `world`. `document` (última compilação
/// boa) habilita os rótulos. O `typst-ide` decide quando há o que sugerir (`explicit = false`):
/// a interface pergunta a cada palavra digitada, e texto comum não deve abrir lista. Com
/// `scope`, sugere caminhos de arquivo do projeto, só os que a regra de escopo aceita.
pub fn complete<W: World>(
    world: &W,
    scope: Option<&Scope>,
    document: Option<&PagedDocument>,
    source: &Source,
    offset: usize,
) -> Vec<Completion> {
    let Some(cursor) = to_byte(source, offset) else {
        return Vec::new();
    };
    let ide = Ide {
        world,
        files: scope.map(|s| project_files(world, s)).unwrap_or_default(),
    };
    typst_ide::autocomplete(&ide, document, source, cursor, false)
        .map(|(_, items)| items.into_iter().map(convert).collect())
        .unwrap_or_default()
}

/// Descrição do item em `offset` (UTF-16).
pub fn hover<W: World>(
    world: &W,
    document: Option<&PagedDocument>,
    source: &Source,
    offset: usize,
) -> Option<Hover> {
    let cursor = to_byte(source, offset)?;
    let ide = Ide {
        world,
        files: Vec::new(),
    };
    match typst_ide::tooltip(&ide, document, source, cursor, Side::After)? {
        Tooltip::Text(text) => Some(Hover {
            text: text.to_string(),
            code: false,
        }),
        Tooltip::Code(text) => Some(Hover {
            text: text.to_string(),
            code: true,
        }),
    }
}

/// Nome do *token*: a classe CSS do Typst sem o prefixo (`typ-key` → `key`).
fn tag_kind(tag: Tag) -> &'static str {
    let class = tag.css_class();
    class.strip_prefix("typ-").unwrap_or(class)
}

/// *Tokens* de realce de `source`, um por nó com marca (aninhados quando o Typst aninha, como
/// em `highlight_html`), na ordem do texto. Nós de erro ficam de fora: os diagnósticos já os
/// mostram.
pub fn semantic_tokens(source: &Source) -> Vec<Token> {
    fn walk(node: &LinkedNode, source: &Source, out: &mut Vec<Token>) {
        if let Some(tag) = highlight(node).filter(|t| *t != Tag::Error) {
            let range = node.range();
            let lines = source.lines();
            if let (Some(from), Some(to)) = (
                lines.byte_to_utf16(range.start),
                lines.byte_to_utf16(range.end),
            ) {
                if from < to {
                    out.push(Token {
                        from,
                        to,
                        kind: tag_kind(tag).to_owned(),
                    });
                }
            }
        }
        for child in node.children() {
            walk(&child, source, out);
        }
    }
    let mut out = Vec::new();
    walk(&LinkedNode::new(source.root()), source, &mut out);
    out
}

fn is_typ(path: &ProjectPath) -> bool {
    Path::new(path.relative())
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("typ"))
}

/// Motor real do [`Engine`]: mantém um World do projeto em sincronia com o estado da edição,
/// pelo mesmo caminho da compilação ([`ProjectCompiler::sync`]).
#[derive(Default)]
pub struct LangService {
    world: Mutex<Option<ProjectCompiler>>,
}

impl LangService {
    /// Leva o World ao estado de `ctx` e chama `f` com um instantâneo dele e a fonte atual de
    /// `path`. A trava só cobre a sincronia; a análise roda no instantâneo. `None`: arquivo que
    /// não é `.typ` ou que o World não lê.
    fn with_source<T>(
        &self,
        ctx: &EngineContext,
        path: &ProjectPath,
        f: impl FnOnce(&CompilerWorld<PreloFeat>, &Scope, &Source) -> T,
    ) -> Option<T> {
        if !is_typ(path) {
            return None;
        }
        let (snapshot, scope) = {
            let mut guard = self.world.lock().unwrap_or_else(PoisonError::into_inner);
            let stale = guard.as_ref().is_none_or(|c| c.root() != ctx.root);
            if stale {
                *guard = Some(ProjectCompiler::new(ctx.root.clone(), &ctx.input.main));
            }
            let compiler = guard.as_mut()?;
            let world = compiler.sync(ctx.input.clone()).ok()?;
            (world.snapshot(), world.access().scope().clone())
        };
        let vpath = VirtualPath::new(path.relative()).ok()?;
        let id = FileId::new(snapshot.main().get().map(|_| vpath));
        let source = snapshot.source(id).ok()?;
        Some(f(&snapshot, &scope, &source))
    }
}

impl Engine for LangService {
    fn complete(
        &self,
        ctx: &EngineContext,
        path: &ProjectPath,
        offset: usize,
    ) -> Result<Vec<Completion>, String> {
        let doc = ctx.document.as_deref();
        Ok(self
            .with_source(ctx, path, |world, scope, source| {
                complete(world, Some(scope), doc, source, offset)
            })
            .unwrap_or_default())
    }

    fn semantic_tokens(
        &self,
        ctx: &EngineContext,
        path: &ProjectPath,
    ) -> Result<Vec<Token>, String> {
        Ok(self
            .with_source(ctx, path, |_, _, source| semantic_tokens(source))
            .unwrap_or_default())
    }

    /// Pontos da prévia do texto em `offset` (US4). Roda no documento da prévia com o World
    /// que o compilou (`ctx.synced`), não no World deste serviço: os *spans* do documento só
    /// valem nas fontes da compilação que o gerou. Sem prévia ou sem `.typ`: nenhum ponto.
    fn jump_from_cursor(
        &self,
        ctx: &EngineContext,
        path: &ProjectPath,
        offset: usize,
    ) -> Result<Vec<JumpTarget>, String> {
        if !is_typ(path) {
            return Ok(Vec::new());
        }
        Ok(ctx
            .synced
            .as_ref()
            .map(|synced| synced.jump_from_cursor(path.relative(), offset))
            .unwrap_or_default())
    }
}
