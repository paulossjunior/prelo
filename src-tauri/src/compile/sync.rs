//! Sincronização código ↔ prévia e estrutura de seções (US4; FR-005, FR-020; research R5;
//! contracts/ipc.md `jump_from_click`, `jump_from_cursor`, `outline`; data-model.md
//! `OutlineItem`).
//!
//! Tudo roda sobre o [`Synced`] da prévia: o último documento compilado com sucesso **e o
//! instantâneo do World que o compilou**. Os *spans* do documento apontam para nós das fontes
//! daquela compilação; resolvê-los no World do serviço de linguagem (que já pode ter visto
//! edições, ou numerar os nós de outro jeito) daria a linha errada.
//!
//! Convenções do contrato: `page` é o índice da página a partir de 0 (o de
//! `PreviewUpdate.pages[].index` e de `page_svg`); `xPt`/`yPt` em pontos tipográficos a partir
//! do canto superior esquerdo da página; `line` e `column` começam em 1, `column` em unidades
//! UTF-16 (as do editor). Só arquivos do projeto são destino: conteúdo de pacote, link externo
//! ou ponto sem *span* (numeração de página, margem) vira "sem origem" (`None`).

use std::num::NonZeroUsize;
use std::sync::Arc;

use serde::Serialize;
use tinymist_world::vfs::{WorkspaceResolution, WorkspaceResolver};
use tinymist_world::CompilerWorld;
use typst::foundations::{NativeElement, StyleChain};
use typst::introspection::{Introspector, PagedPosition};
use typst::layout::{Abs, Point};
use typst::model::HeadingElem;
use typst::syntax::{FileId, Source, Span, VirtualPath};
use typst::{World, WorldExt};
use typst_ide::Jump;
use typst_layout::PagedDocument;

use crate::commands::JumpTarget;
use crate::lang_service::Ide;
use crate::world::PreloFeat;

/// Último documento bom da prévia e o instantâneo do World que o compilou.
pub struct Synced {
    pub document: Arc<PagedDocument>,
    world: CompilerWorld<PreloFeat>,
}

impl Synced {
    pub fn new(document: Arc<PagedDocument>, world: CompilerWorld<PreloFeat>) -> Self {
        Self { document, world }
    }
}

/// `{ path, line, column }` de `jump_from_click`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourcePosition {
    /// Relativo à raiz, com `/`.
    pub path: String,
    /// A partir de 1.
    pub line: u32,
    /// A partir de 1, em unidades UTF-16.
    pub column: u32,
}

/// `OutlineItem` (data-model.md): `{ level, title, path, line, page, yPt }`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutlineItem {
    pub level: u32,
    /// Título como aparece no documento, com a numeração quando houver ("2.1 Coleta").
    pub title: String,
    pub path: String,
    pub line: u32,
    pub page: usize,
    pub y_pt: f64,
}

/// Caminho relativo à raiz de `id`, se for arquivo do projeto (não de pacote).
fn project_path(id: FileId) -> Option<String> {
    match WorkspaceResolver::resolve(id) {
        Ok(WorkspaceResolution::Workspace(_)) => Some(id.vpath().get_without_slash().to_owned()),
        _ => None,
    }
}

/// `(caminho, linha, coluna)` do byte `offset` de `id` no instantâneo.
fn position(world: &CompilerWorld<PreloFeat>, id: FileId, offset: usize) -> Option<SourcePosition> {
    let path = project_path(id)?;
    let source = world.source(id).ok()?;
    let lines = source.lines();
    let offset = offset.min(source.text().len());
    let line = lines.byte_to_line(offset)?;
    let start = lines.line_to_byte(line)?;
    let column = lines.byte_to_utf16(offset)? - lines.byte_to_utf16(start)?;
    Some(SourcePosition {
        path,
        line: u32::try_from(line + 1).ok()?,
        column: u32::try_from(column + 1).ok()?,
    })
}

/// Ponto da prévia no formato do contrato.
fn target(pos: &PagedPosition) -> JumpTarget {
    JumpTarget {
        page: pos.page.get() - 1,
        x_pt: pos.point.x.to_pt(),
        y_pt: pos.point.y.to_pt(),
    }
}

impl Synced {
    /// Origem do ponto `(x_pt, y_pt)` da página `page` (a partir de 0). Clique num link
    /// interno (referência, sumário) leva à origem do destino; link externo, pacote ou ponto
    /// sem origem → `None`.
    pub fn jump_from_click(&self, page: usize, x_pt: f64, y_pt: f64) -> Option<SourcePosition> {
        if !(x_pt.is_finite() && y_pt.is_finite()) {
            return None;
        }
        let ide = Ide::new(&self.world);
        let mut at = PagedPosition {
            page: NonZeroUsize::new(page.checked_add(1)?)?,
            point: Point::new(Abs::pt(x_pt), Abs::pt(y_pt)),
        };
        if at.page.get() > self.document.pages().len() {
            return None;
        }
        // Um salto por link interno, no máximo: o destino de um link não é outro link.
        for _ in 0..2 {
            match typst_ide::jump_from_click(&ide, self.document.as_ref(), &at)? {
                Jump::File(id, offset) => return position(&self.world, id, offset),
                Jump::Position(dest) => at = dest,
                Jump::Url(_) => return None,
            }
        }
        None
    }

    /// Pontos da prévia do texto em `offset` (UTF-16) de `path`, relativo à raiz. O texto é o
    /// da compilação da prévia; deslocamento além dele fica no fim.
    pub fn jump_from_cursor(&self, path: &str, offset: usize) -> Vec<JumpTarget> {
        let Some(source) = self.source(path) else {
            return Vec::new();
        };
        let lines = source.lines();
        let offset = offset.min(lines.len_utf16());
        let Some(cursor) = lines.utf16_to_byte(offset) else {
            return Vec::new();
        };
        typst_ide::jump_from_cursor(self.document.as_ref(), &source, cursor)
            .iter()
            .map(target)
            .collect()
    }

    /// Fonte de `path` no instantâneo.
    fn source(&self, path: &str) -> Option<Source> {
        let vpath = VirtualPath::new(path).ok()?;
        let id = FileId::new(self.world.main().get().map(|_| vpath));
        self.world.source(id).ok()
    }

    /// Títulos de seção em ordem de documento, com nível, origem e posição na prévia. Ficam
    /// de fora os marcados `outlined: false` e os que não vêm de arquivo do projeto.
    pub fn outline(&self) -> Vec<OutlineItem> {
        let introspector = self.document.introspector();
        introspector
            .query(&HeadingElem::ELEM.select())
            .iter()
            .filter_map(|elem| {
                let heading = elem.to_packed::<HeadingElem>()?;
                if !heading.outlined.get(StyleChain::default()) {
                    return None;
                }
                let origin = self.span_position(heading.span())?;
                let at = introspector.position(elem.location()?)?;
                let text = heading.body.plain_text();
                let title = match &heading.numbers {
                    Some(numbers) if !numbers.is_empty() => format!("{numbers} {text}"),
                    _ => text.to_string(),
                };
                let page = target(&at);
                Some(OutlineItem {
                    level: u32::try_from(heading.resolve_level(StyleChain::default()).get())
                        .ok()?,
                    title: title.trim().to_owned(),
                    path: origin.path,
                    line: origin.line,
                    page: page.page,
                    y_pt: page.y_pt,
                })
            })
            .collect()
    }

    fn span_position(&self, span: Span) -> Option<SourcePosition> {
        let id = span.id()?;
        let range = self.world.range(span)?;
        position(&self.world, id, range.start)
    }
}
