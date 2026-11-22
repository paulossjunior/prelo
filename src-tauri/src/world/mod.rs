//! World do Prelo (research R2, R6; D-10, D-15, D-17; contracts/file-access.md).
//!
//! `CompilerUniverse` do `tinymist-world` montado com:
//! - o [`AccessModel`] do Prelo como único `PathAccessModel` do VFS (RF-08);
//! - só as fontes embutidas ([`fonts`]), com os padrões de matemática e `raw` do Prelo;
//! - registro de pacotes vazio nesta etapa (o `@preview` entra com T048).
//!
//! Prévia, diagnósticos e, depois, autocompletar (`typst-ide`) saem do mesmo World.

pub mod fonts;

use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;

use tinymist_world::package::registry::DummyRegistry;
use tinymist_world::package::RegistryPathMapper;
use tinymist_world::vfs::{
    FileChangeSet, ImmutPath, PathAccessModel, Vfs, WorkspaceResolution, WorkspaceResolver,
};
use tinymist_world::{CompilerFeat, CompilerUniverse, CompilerWorld, EntryState};
use typst::diag::{Severity as TypstSeverity, SourceDiagnostic, Warned};
use typst::foundations::Dict;
use typst::syntax::ast::Str;
use typst::syntax::{LinkedNode, Side, SyntaxKind, VirtualPath, VirtualRoot};
use typst::utils::LazyHash;
use typst::{Features, Library, LibraryExt, World, WorldExt};
use typst_layout::PagedDocument;

use crate::compile::scheduler::Outcome;
use crate::diagnostic::{Diagnostic, Kind, Severity};
use crate::fs::access_model::{access_diagnostic, AccessModel};
use crate::fs::scope::{Refusal, Scope};

/// Peças do World do Prelo para o `tinymist-world`. `M` é o modelo de acesso do VFS: em
/// produção, o [`AccessModel`]; nos testes de segurança, ele embrulhado num instrumentado.
pub struct PreloFeat<M = AccessModel>(PhantomData<fn() -> M>);

impl<M> CompilerFeat for PreloFeat<M>
where
    M: PathAccessModel + Clone + Send + Sync + 'static,
{
    type FontResolver = tinymist_world::font::FontResolverImpl;
    type AccessModel = M;
    type Registry = DummyRegistry;
}

/// World de um projeto aberto.
pub struct ProjectWorld<M = AccessModel>
where
    M: PathAccessModel + Clone + Send + Sync + 'static,
{
    universe: CompilerUniverse<PreloFeat<M>>,
    access: AccessModel,
    library: Arc<LazyHash<Library>>,
    /// Arquivo principal como foi pedido, e como o escopo o vê (`/` + relativo à raiz).
    main: (String, String),
}

impl<M> std::fmt::Debug for ProjectWorld<M>
where
    M: PathAccessModel + Clone + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectWorld")
            .field("access", &self.access)
            .field("main", &self.main.0)
            .finish_non_exhaustive()
    }
}

impl ProjectWorld<AccessModel> {
    /// Abre o projeto em `root` (canonicalizada uma vez) com o arquivo principal `main`,
    /// relativo à raiz.
    pub fn open(root: impl AsRef<Path>, main: &str) -> Result<Self, Diagnostic> {
        let scope = Scope::new(root)
            .map_err(|_| Diagnostic::error(Kind::Project, "pasta do projeto não encontrada"))?;
        Self::new(AccessModel::new(scope), main)
    }

    /// World sobre um modelo de acesso já montado.
    pub fn new(access: AccessModel, main: &str) -> Result<Self, Diagnostic> {
        Self::instrumented(access, main, |model| model)
    }
}

impl<M> ProjectWorld<M>
where
    M: PathAccessModel + Clone + Send + Sync + 'static,
{
    /// Como [`ProjectWorld::new`], com o modelo do VFS embrulhado por `wrap` (testes de
    /// segurança, FR-030). `wrap` recebe o [`AccessModel`] e deve delegar a ele.
    #[doc(hidden)]
    pub fn instrumented(
        access: AccessModel,
        main: &str,
        wrap: impl FnOnce(AccessModel) -> M,
    ) -> Result<Self, Diagnostic> {
        let root: ImmutPath = access.scope().root().into();
        let main_path = access
            .lexical_path(main)
            .map_err(|r| access_diagnostic(&r))?;
        let vpath = VirtualPath::virtualize(&root, &main_path).map_err(|_| {
            access_diagnostic(&Refusal::NotFound {
                requested: main.into(),
            })
        })?;
        let vpath_str = vpath.get_without_slash().to_owned();
        let entry = EntryState::new_rooted(root, Some(vpath));
        let registry = Arc::new(DummyRegistry);
        let resolver = Arc::new(RegistryPathMapper::new(registry.clone()));
        let vfs = Vfs::new(resolver, wrap(access.clone()));
        let universe = CompilerUniverse::new_raw(
            entry,
            Features::default(),
            None,
            vfs,
            registry,
            fonts::resolver(),
            None,
        );
        Ok(Self {
            universe,
            access,
            library: library(),
            main: (main.to_owned(), format!("/{}", vpath_str)),
        })
    }

    /// Modelo de acesso do projeto.
    pub fn access(&self) -> &AccessModel {
        &self.access
    }

    /// O arquivo `requested` (relativo à raiz) está aberto no editor com `text`, ainda não
    /// salvo: a próxima compilação usa esse texto (passo 1 do contrato).
    pub fn set_open_file(&mut self, requested: &str, text: &str) -> Result<(), Refusal> {
        let path = self.access.set_overlay(requested, text)?;
        self.invalidate(path.as_path());
        Ok(())
    }

    /// O arquivo foi fechado no editor: volta a ser lido do disco.
    pub fn close_file(&mut self, requested: &str) {
        if let Some(path) = self.access.remove_overlay(requested) {
            self.invalidate(path.as_path());
        }
    }

    fn invalidate(&mut self, path: &Path) {
        let path: ImmutPath = path.into();
        self.universe.increment_revision(|universe| {
            universe
                .vfs()
                .notify_fs_changes(FileChangeSet::new_removes(vec![path]));
        });
    }

    /// Instantâneo do World para compilar ou analisar (`typst-ide`), com a biblioteca e os
    /// padrões de fonte do Prelo.
    pub fn snapshot(&self) -> CompilerWorld<PreloFeat<M>> {
        let mut world = self.universe.snapshot();
        world.library = self.library.clone();
        world
    }

    /// Compila o estado atual (disco + overlay). Erros e avisos viram [`Diagnostic`].
    pub fn compile(&mut self) -> Outcome<PagedDocument> {
        let world = self.snapshot();
        let Warned { output, warnings } = typst::compile::<PagedDocument>(&world);
        let (output, errors) = match output {
            Ok(doc) => (Some(doc), Default::default()),
            Err(errors) => (None, errors),
        };
        let diagnostics = errors
            .iter()
            .chain(warnings.iter())
            .map(|diag| self.convert(&world, diag))
            .collect();
        Outcome {
            output,
            diagnostics,
        }
    }

    /// Diagnóstico do Typst → [`Diagnostic`]. Recusas do modelo de acesso e caminhos que
    /// escapam da raiz viram `kind = access`, com a mensagem do contrato citando o caminho
    /// como foi escrito no `.typ`.
    fn convert(&self, world: &CompilerWorld<PreloFeat<M>>, diag: &SourceDiagnostic) -> Diagnostic {
        let range = world.range(diag.span);
        let source = diag.span.id().and_then(|id| world.source(id).ok());
        let line = source
            .as_ref()
            .zip(range.as_ref())
            .and_then(|(s, r)| s.lines().byte_to_line(r.start))
            .map(|l| l as u32 + 1);
        // O Tinymist representa a raiz do projeto como um pacote interno (`@ws/…`).
        let path = diag
            .span
            .id()
            .map(|id| match (WorkspaceResolver::resolve(id), id.root()) {
                (Ok(WorkspaceResolution::Package), VirtualRoot::Package(spec)) => {
                    format!("{spec}{}", id.vpath().get_with_slash())
                }
                _ => id.vpath().get_without_slash().to_owned(),
            });
        let literal = source
            .as_ref()
            .zip(range.as_ref())
            .and_then(|(s, r)| string_literal(s.root(), r));
        // Bytes → unidades UTF-16, as do editor e de `edit_file`.
        let range = source.as_ref().zip(range).and_then(|(s, r)| {
            let lines = s.lines();
            Some(lines.byte_to_utf16(r.start)?..lines.byte_to_utf16(r.end)?)
        });

        let refusal = self
            .access
            .refusal_for_message(&diag.message)
            .or_else(|| escape_refusal(&diag.message));
        let (kind, message, hints) = match refusal {
            Some(refusal) => {
                let refusal = match literal {
                    Some(requested) => with_requested(refusal, requested),
                    // Sem literal (ex.: o próprio arquivo principal): cita o principal como
                    // foi pedido ao abrir.
                    None if refusal.requested() == self.main.1 => {
                        with_requested(refusal, self.main.0.clone())
                    }
                    None => refusal,
                };
                // As dicas do Typst para esse caso falam do `--root` do CLI.
                (Kind::Access, refusal.to_string(), Vec::new())
            }
            None => (
                Kind::Typst,
                diag.message.to_string(),
                diag.hints.iter().map(|h| h.v.to_string()).collect(),
            ),
        };
        Diagnostic {
            severity: match diag.severity {
                TypstSeverity::Error => Severity::Error,
                TypstSeverity::Warning => Severity::Warning,
            },
            path,
            range,
            line,
            message,
            hints,
            kind,
        }
    }
}

/// Biblioteca padrão do Typst com os padrões de fonte do Prelo.
fn library() -> Arc<LazyHash<Library>> {
    let mut library = Library::builder().with_inputs(Dict::new()).build();
    library.styles.apply(fonts::default_styles());
    Arc::new(LazyHash::new(library))
}

/// Valor do literal de string que ocupa exatamente `range`, se houver.
fn string_literal(
    root: &typst::syntax::SyntaxNode,
    range: &std::ops::Range<usize>,
) -> Option<String> {
    let leaf = LinkedNode::new(root).leaf_at(range.start + 1, Side::Before)?;
    (leaf.kind() == SyntaxKind::Str && leaf.range() == *range)
        .then(|| leaf.cast::<Str>())
        .flatten()
        .map(|s| s.get().to_string())
}

/// Erro do Typst para caminho que escapa da raiz léxica (`..` além dela): o Typst recusa
/// antes de pedir ao VFS. Vira `ForaDoProjeto` com o caminho como escrito.
fn escape_refusal(message: &str) -> Option<Refusal> {
    let rest = message.strip_prefix("path `")?;
    let quoted = rest
        .strip_suffix("` would escape the project root")
        .or_else(|| rest.strip_suffix("` would escape the package root"))?;
    let requested = quoted
        .strip_prefix('"')
        .and_then(|q| q.strip_suffix('"'))
        .unwrap_or(quoted);
    Some(Refusal::OutsideProject {
        requested: requested.replace("\\\"", "\"").replace("\\\\", "\\"),
    })
}

fn with_requested(refusal: Refusal, requested: String) -> Refusal {
    match refusal {
        Refusal::NotFound { .. } => Refusal::NotFound { requested },
        Refusal::OutsideProject { .. } => Refusal::OutsideProject { requested },
        Refusal::SpecialFile { .. } => Refusal::SpecialFile { requested },
        Refusal::TooLarge { limit, .. } => Refusal::TooLarge { requested, limit },
    }
}
