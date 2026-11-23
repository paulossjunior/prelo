//! World do Prelo (research R2, R6; D-10, D-15, D-17; contracts/file-access.md).
//!
//! `CompilerUniverse` do `tinymist-world` montado com:
//! - o [`AccessModel`] do Prelo como único `PathAccessModel` do VFS (RF-08);
//! - só as fontes embutidas ([`fonts`]), com os padrões de matemática e `raw` do Prelo;
//! - o registro de pacotes `@preview` do Prelo ([`packages`]), com cache e verificação;
//! - plugins WASM barrados até a pessoa aprovar ([`plugins`], no modelo de acesso).
//!
//! Prévia, diagnósticos e, depois, autocompletar (`typst-ide`) saem do mesmo World.

pub mod fonts;
pub mod packages;
pub mod plugins;

use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;

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
use packages::PreloRegistry;
pub use plugins::WorldEnv;

/// Peças do World do Prelo para o `tinymist-world`. `M` é o modelo de acesso do VFS: em
/// produção, o [`AccessModel`]; nos testes de segurança, ele embrulhado num instrumentado.
pub struct PreloFeat<M = AccessModel>(PhantomData<fn() -> M>);

impl<M> CompilerFeat for PreloFeat<M>
where
    M: PathAccessModel + Clone + Send + Sync + 'static,
{
    type FontResolver = tinymist_world::font::FontResolverImpl;
    type AccessModel = M;
    type Registry = PreloRegistry;
}

/// World de um projeto aberto.
pub struct ProjectWorld<M = AccessModel>
where
    M: PathAccessModel + Clone + Send + Sync + 'static,
{
    universe: CompilerUniverse<PreloFeat<M>>,
    access: AccessModel,
    library: Arc<LazyHash<Library>>,
    registry: Arc<PreloRegistry>,
    /// Contador de decisões de plugin e de "Baixar de novo" visto na última compilação.
    seen_generation: u64,
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
        Self::open_with(root, main, WorldEnv::default())
    }

    /// Como [`ProjectWorld::open`], com decisões de plugin e cache de pacotes.
    pub fn open_with(
        root: impl AsRef<Path>,
        main: &str,
        env: WorldEnv,
    ) -> Result<Self, Diagnostic> {
        let scope = Scope::new(root)
            .map_err(|_| Diagnostic::error(Kind::Project, "pasta do projeto não encontrada"))?;
        Self::new(AccessModel::with_env(scope, env), main)
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
        let registry = Arc::new(PreloRegistry::new(access.env().packages.clone()));
        let seen_generation = access.env().generation();
        let resolver = Arc::new(RegistryPathMapper::new(registry.clone()));
        let vfs = Vfs::new(resolver, wrap(access.clone()));
        let universe = CompilerUniverse::new_raw(
            entry,
            Features::default(),
            None,
            vfs,
            registry.clone(),
            fonts::resolver(),
            None,
        );
        Ok(Self {
            universe,
            access,
            library: library(),
            registry,
            seen_generation,
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

    /// O arquivo `requested` (relativo à raiz, não aberto no editor) mudou no disco: a próxima
    /// compilação o relê pela regra de escopo. Caminho recusado é ignorado.
    pub fn reload_from_disk(&mut self, requested: &str) {
        if self.access.is_open(requested) {
            return;
        }
        if let Ok(path) = self.access.lexical_path(requested) {
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

    /// Depois de uma decisão de plugin ou de "Baixar de novo", relê arquivos e pacotes: o VFS
    /// guardaria a recusa anterior.
    fn refresh_trust(&mut self) {
        let generation = self.access.env().generation();
        if generation != self.seen_generation {
            self.seen_generation = generation;
            let registry = self.registry.clone();
            self.universe
                .increment_revision(|universe| universe.set_package(registry));
        }
    }

    /// Compila o estado atual (disco + overlay). Erros e avisos viram [`Diagnostic`].
    pub fn compile(&mut self) -> Outcome<PagedDocument> {
        self.compile_with_snapshot().0
    }

    /// Como [`ProjectWorld::compile`], e devolve também o instantâneo que compilou: os
    /// *spans* do documento só valem nas fontes dele (sincronização com a prévia, US4).
    pub fn compile_with_snapshot(
        &mut self,
    ) -> (Outcome<PagedDocument>, CompilerWorld<PreloFeat<M>>) {
        self.refresh_trust();
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
        (
            Outcome {
                output,
                diagnostics,
            },
            world,
        )
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

        let utf8_literal = (diag.message == INVALID_UTF8).then(|| literal.clone());
        let refusal = self
            .access
            .refusal_for_message(&diag.message)
            .or_else(|| escape_refusal(&diag.message));
        let package = self.registry.refusal_for_message(&diag.message);
        let (kind, message, hints) = match (refusal, package) {
            (None, Some(package)) => (Kind::Package, package.to_string(), Vec::new()),
            (Some(refusal), _) => {
                let refusal = match literal {
                    Some(requested) => with_requested(refusal, requested),
                    // Sem literal (ex.: o próprio arquivo principal): cita o principal como
                    // foi pedido ao abrir.
                    None if refusal.requested() == self.main.1 => {
                        with_requested(refusal, self.main.0.clone())
                    }
                    None => refusal,
                };
                let kind = if refusal.is_plugin() {
                    Kind::Plugin
                } else {
                    Kind::Access
                };
                // As dicas do Typst para esse caso falam do `--root` do CLI.
                (kind, refusal.to_string(), Vec::new())
            }
            (None, None) => (
                Kind::Typst,
                diag.message.to_string(),
                diag.hints.iter().map(|h| h.v.to_string()).collect(),
            ),
        };
        let mut converted = Diagnostic {
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
        };
        if let Some(literal) = utf8_literal {
            self.locate_invalid_utf8(&mut converted, literal.as_deref());
        }
        converted
    }

    /// Arquivo que não é UTF-8 (FR-024): o Typst não diz qual nem onde. Quem falhou é o
    /// arquivo do literal no ponto do erro (`#include "cap.typ"`) ou, sem posição, o
    /// principal. Relê pela regra de escopo, acha a linha do primeiro byte inválido e a cita
    /// numa dica; sem posição, o diagnóstico passa a apontar o principal nessa linha.
    fn locate_invalid_utf8(&self, diag: &mut Diagnostic, literal: Option<&str>) {
        let (current, requested) = match (literal, diag.path.as_deref()) {
            (Some(literal), Some(path)) => (path.to_owned(), literal.to_owned()),
            (_, None) => (String::new(), self.main.0.clone()),
            (None, Some(_)) => return,
        };
        let Ok(bytes) = self.access.scope().read_from(&current, &requested) else {
            return;
        };
        let Err(error) = std::str::from_utf8(&bytes) else {
            return;
        };
        let newlines = bytes[..error.valid_up_to()]
            .iter()
            .filter(|&&b| b == b'\n')
            .count();
        let line = u32::try_from(newlines).unwrap_or(u32::MAX - 1) + 1;
        if diag.path.is_none() {
            diag.path = Some(requested.clone());
            diag.line = Some(line);
        }
        diag.hints.push(format!(
            "o arquivo `{requested}` não está em UTF-8: primeiro byte inválido na linha {line}"
        ));
    }
}

/// Mensagem do Typst para arquivo que não é UTF-8 (`FileError::InvalidUtf8`).
const INVALID_UTF8: &str = "file is not valid UTF-8";

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
    refusal.with_requested(requested)
}
