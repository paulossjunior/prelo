//! Comandos IPC da feature 003 (specs/003-editor-local/contracts/ipc.md; T022).
//!
//! Toda entrada da interface é recusada por padrão: cada comando lê seus argumentos de um
//! objeto JSON com campos exatos (`deny_unknown_fields`) e todo `path` passa por
//! [`validate_path`] antes de qualquer outra coisa. O caminho validado é relativo à raiz,
//! canônico (links resolvidos) e nunca absoluto; caminho absoluto, `..` além da raiz ou link
//! para fora vira `path_outside_project` sem tocar o disco fora do projeto nem chegar ao motor.
//!
//! Abrir o projeto liga a agenda de compilação ao World do projeto; `edit_file` e
//! `choose_main_file` recompilam, e cada resultado chega à interface pelo canal de
//! `subscribe_preview`; `page_svg` serve o SVG de cada página (T035). O serviço de linguagem
//! (T036) passa pelo trait [`Engine`]: o padrão é o [`LangService`], que recebe o mesmo estado
//! da compilação ([`EngineContext`]). Saltos entre prévia e código e a estrutura de seções
//! (US4: `jump_from_click`, `jump_from_cursor`, `outline`) rodam sobre o documento da prévia e
//! o World que o compilou ([`crate::compile::sync`]).
//! Mudança externa (T034): o observador de [`crate::fs::watch`] repassa caminhos para
//! [`AppState::handle_disk_change`], que recarrega ou marca conflito; `resolve_external_change`
//! aplica a escolha da pessoa. Arquivo aberto e gravação (`open_file`, `edit_file`, `save_file`,
//! salvamento automático e último Salvar ao fechar; T033) moram em [`crate::fs::save`].
//! Arquivo não aberto que muda no disco (um `.bib` exportado por outro programa) é relido na
//! compilação seguinte ([`DiskChanges`]). Cópia para o projeto (D-13, T059): a compilação que
//! recusa um caminho de fora gera a oferta ([`crate::fs::copy_into`]); `copy_into_project`
//! recebe só o caminho pedido e o arquivo de origem vem do diálogo nativo do núcleo.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock};
use std::time::{Instant, SystemTime};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::ipc::{Channel, InvokeBody, Request};
use tauri::{Runtime, State, Window};
use tauri_plugin_dialog::DialogExt;

use typst_layout::PagedDocument;

use crate::compile::export::{ExportError, ExportSource, Exported};
use crate::compile::preview::{asset_response, PreviewHub, PreviewUpdate};
use crate::compile::project::{CompileInput, Compiled, DiskChanges, ProjectCompiler};
use crate::compile::scheduler::Scheduler;
use crate::compile::sync::{OutlineItem, SourcePosition, Synced};
use crate::fs::copy_into::{self, CopyOffers, UpdatedFile, NOT_OFFERED};
pub use crate::fs::copy_into::{CopiedIntoProject, CopyIntoProjectOffer};
use crate::fs::create::{self, NewItem};
pub use crate::fs::save::{apply_changes, Change};
use crate::fs::save::{format_utc, OpenFile};
use crate::fs::scope::{Refusal, Scope};
use crate::fs::watch::{watch, ProjectWatcher};
pub use crate::fs::watch::{DiskChange, Keep};
use crate::lang_service::{Completion, LangService, Token};
use crate::project::record::{self, LoadedRecord, RecordState};
use crate::project::{CompilerInfo, MAIN_FILE_MISSING};
use crate::trust::Trust;

pub use crate::project::EMBEDDED_TYPST;

const OUTSIDE: &str = "path_outside_project";
const NOT_FOUND: &str = "not_found";
const INVALID_ARGUMENTS: &str = "invalid_arguments";
const NO_PROJECT: &str = "no_project";
const NOT_IMPLEMENTED: &str = "not_implemented";
const VERSION_MISMATCH: &str = "compiler_version_mismatch";
const COMPILE_SUSPENDED: &str = "compile_suspended";

/// Profundidade e quantidade máximas de nós de `list_tree` (pasta enorme não trava o núcleo).
const TREE_MAX_DEPTH: usize = 32;
const TREE_MAX_NODES: usize = 10_000;

/// Caminho pedido pela interface, já validado: dentro da raiz, canônico.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectPath {
    relative: String,
    absolute: PathBuf,
}

impl ProjectPath {
    /// Relativo à raiz, com `/` como separador e sem `.`/`..` (ex.: `capitulos/intro.typ`).
    pub fn relative(&self) -> &str {
        &self.relative
    }

    /// Caminho canônico no disco (dentro da raiz).
    pub fn absolute(&self) -> &Path {
        &self.absolute
    }
}

/// Caminho absoluto em qualquer sistema: `/`, `\`, `C:` ou o que o sistema considera absoluto.
fn is_absolute(requested: &str) -> bool {
    let b = requested.as_bytes();
    requested.starts_with('/')
        || requested.starts_with('\\')
        || (b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':')
        || Path::new(requested).has_root()
        || Path::new(requested).is_absolute()
}

fn refusal_code(refusal: &Refusal) -> &'static str {
    match refusal {
        Refusal::OutsideProject { .. } => OUTSIDE,
        Refusal::NotFound { .. }
        | Refusal::SpecialFile { .. }
        | Refusal::PluginPending { .. }
        | Refusal::PluginRejected { .. } => NOT_FOUND,
        Refusal::TooLarge { .. } => "too_large",
    }
}

/// Validação comum a todo `path` vindo da interface (contracts/ipc.md): absoluto ou fora da
/// raiz (por `..` ou link) → `path_outside_project`; inexistente → `not_found`.
pub fn validate_path(scope: &Scope, requested: &str) -> Result<ProjectPath, &'static str> {
    if is_absolute(requested) {
        return Err(OUTSIDE);
    }
    let canonical = scope.resolve(requested).map_err(|r| refusal_code(&r))?;
    let inside = canonical.strip_prefix(scope.root()).map_err(|_| OUTSIDE)?;
    let mut parts = Vec::new();
    for component in inside.components() {
        match component {
            Component::Normal(name) => parts.push(name.to_str().ok_or(NOT_FOUND)?),
            _ => return Err(OUTSIDE),
        }
    }
    Ok(ProjectPath {
        relative: parts.join("/"),
        absolute: canonical,
    })
}

/// `{ page, xPt, yPt }` de `jump_from_cursor`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JumpTarget {
    pub page: usize,
    pub x_pt: f64,
    pub y_pt: f64,
}

/// Estado do projeto que o motor precisa para responder como a prévia veria o texto: a raiz,
/// o mesmo [`CompileInput`] da compilação (principal + textos abertos, ainda não salvos;
/// FR-011) e o último documento compilado com sucesso (rótulos), também com o World que o
/// compilou (`synced`, para os saltos da prévia).
#[derive(Clone)]
pub struct EngineContext {
    pub root: PathBuf,
    pub input: CompileInput,
    pub document: Option<Arc<PagedDocument>>,
    pub synced: Option<Arc<Synced>>,
}

impl std::fmt::Debug for EngineContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineContext")
            .field("root", &self.root)
            .field("input", &self.input)
            .field("document", &self.document.is_some())
            .field("synced", &self.synced.is_some())
            .finish()
    }
}

/// Consultas que dependem do World (T020) e dos serviços de prévia e linguagem (T035, T036).
/// Recebem só caminhos já validados; `offset` em unidades UTF-16 do texto atual.
pub trait Engine: Send + Sync + 'static {
    fn complete(
        &self,
        ctx: &EngineContext,
        path: &ProjectPath,
        offset: usize,
    ) -> Result<Vec<Completion>, String>;
    fn semantic_tokens(
        &self,
        ctx: &EngineContext,
        path: &ProjectPath,
    ) -> Result<Vec<Token>, String>;
    fn jump_from_cursor(
        &self,
        ctx: &EngineContext,
        path: &ProjectPath,
        offset: usize,
    ) -> Result<Vec<JumpTarget>, String>;
}

/// Motor que responde `not_implemented` a tudo (recurso ainda não entregue).
pub struct Unavailable;

impl Engine for Unavailable {
    fn complete(
        &self,
        _ctx: &EngineContext,
        _path: &ProjectPath,
        _offset: usize,
    ) -> Result<Vec<Completion>, String> {
        Err(NOT_IMPLEMENTED.into())
    }

    fn semantic_tokens(
        &self,
        _ctx: &EngineContext,
        _path: &ProjectPath,
    ) -> Result<Vec<Token>, String> {
        Err(NOT_IMPLEMENTED.into())
    }

    fn jump_from_cursor(
        &self,
        _ctx: &EngineContext,
        _path: &ProjectPath,
        _offset: usize,
    ) -> Result<Vec<JumpTarget>, String> {
        Err(NOT_IMPLEMENTED.into())
    }
}

/// `ProjectInfo` (data-model.md).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    pub name: String,
    pub main_file: String,
    pub compiler: CompilerVersions,
    pub warnings: Vec<String>,
}

/// `ProjectInfo.compiler`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CompilerVersions {
    pub embedded: String,
    pub registered: Option<String>,
}

/// Resposta de `open_file`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpenedFile {
    pub text: String,
    pub version: u64,
}

/// Nó de `list_tree`; `path` relativo à raiz, com `/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TreeNode {
    pub name: String,
    pub path: String,
    pub kind: TreeKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<TreeNode>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TreeKind {
    File,
    Dir,
}

struct OpenProject {
    scope: Scope,
    record: LoadedRecord,
    files: HashMap<String, OpenFile>,
    /// Agenda de compilação; `None` no estado `VersaoDiferente` (versão registrada ≠
    /// embutida, FR-012): sem compilação nem prévia até `update_project_compiler_version`.
    compiler: Option<Scheduler<ProjectCompiler>>,
    /// Observador de mudanças externas da raiz; `None` se o sistema recusou (o editor segue,
    /// só sem detectar mudanças externas).
    _watcher: Option<ProjectWatcher>,
    /// O arquivo principal (registrado ou `main.typ`) não existe: aviso `main_file_missing`
    /// até `choose_main_file` (a compilação responde com o diagnóstico de arquivo ausente).
    main_missing: bool,
    /// Arquivos não abertos que mudaram no disco, para a próxima compilação reler.
    disk: DiskChanges,
}

impl OpenProject {
    /// Estado `VersaoDiferente` (data-model.md): registro válido com `compiler.typst` ≠
    /// embutida. Registro ausente ou inválido usa a versão embutida.
    fn version_differs(&self) -> bool {
        self.record.state == RecordState::Valid
            && self.record.record.compiler_typst() != EMBEDDED_TYPST
    }

    /// O principal em uso existe como arquivo comum dentro da raiz.
    fn main_exists(&self) -> bool {
        validate_path(&self.scope, self.record.record.main_or_default())
            .is_ok_and(|path| path.absolute().is_file())
    }

    /// Registro ausente (FR-003): criado com a versão embutida e, se houver `main.typ`, com
    /// ele como principal. Só `.prelo/project.json` é gravado; falha de gravação (pasta só de
    /// leitura) não impede abrir o projeto, e o registro segue ausente.
    fn create_missing_record(&mut self) {
        if self.record.state != RecordState::Missing {
            return;
        }
        if self.main_exists() {
            let _ = self.record.record.set_main(record::DEFAULT_MAIN);
        }
        let root = self.scope.root().to_owned();
        if let Err(e) = record::save(&root, &mut self.record) {
            eprintln!("prelo: {e}");
        }
    }

    /// Estado atual da edição (principal + arquivos abertos), sem copiar texto.
    fn input(&self) -> CompileInput {
        CompileInput {
            main: self.record.record.main_or_default().to_owned(),
            files: self
                .files
                .iter()
                .map(|(path, file)| (path.clone(), file.shared_text()))
                .collect(),
        }
    }

    /// Põe o estado atual na fila de compilação.
    fn recompile(&self) {
        if let Some(compiler) = &self.compiler {
            compiler.submit(self.input());
        }
    }

    /// Troca, no texto de `relative`, cada literal `requested` por `literal`, como uma edição
    /// do arquivo aberto (abre se preciso; o salvamento automático grava). Devolve a versão
    /// nova, ou `None` se o literal não está mais lá ou o arquivo não pôde ser lido.
    fn replace_literal(&mut self, relative: &str, requested: &str, literal: &str) -> Option<u64> {
        let path = validate_path(&self.scope, relative).ok()?;
        if !self.files.contains_key(path.relative()) {
            let bytes = self.scope.read(path.relative()).ok()?;
            let file = OpenFile::from_bytes(bytes).ok()?;
            self.files.insert(path.relative().to_owned(), file);
        }
        let file = self.files.get_mut(path.relative())?;
        let changes = copy_into::literal_changes(file.text(), requested, literal);
        if changes.is_empty() {
            return None;
        }
        file.edit(file.version(), &changes).ok()
    }

    fn info(&self) -> ProjectInfo {
        let registered = match self.record.state {
            RecordState::Valid => Some(self.record.record.compiler_typst().to_owned()),
            RecordState::Missing | RecordState::Invalid(_) => None,
        };
        let mut warnings = Vec::new();
        if let Some(reason) = self.record.warning() {
            warnings.push(reason.code().to_owned());
        }
        if self.version_differs() {
            warnings.push(VERSION_MISMATCH.to_owned());
        }
        if self.main_missing {
            warnings.push(MAIN_FILE_MISSING.to_owned());
        }
        ProjectInfo {
            name: self
                .scope
                .root()
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            main_file: self.record.record.main_or_default().to_owned(),
            compiler: CompilerVersions {
                embedded: EMBEDDED_TYPST.to_owned(),
                registered,
            },
            warnings,
        }
    }

    fn tree(&self) -> Vec<TreeNode> {
        let mut budget = TREE_MAX_NODES;
        self.tree_dir(self.scope.root(), "", 0, &mut budget)
    }

    /// Lista `dir` sem seguir link para pasta; link só aparece se aponta para arquivo comum
    /// dentro da raiz. Ocultos (`.prelo`, `.git`) e especiais (FIFO, socket) ficam de fora.
    fn tree_dir(
        &self,
        dir: &Path,
        prefix: &str,
        depth: usize,
        budget: &mut usize,
    ) -> Vec<TreeNode> {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut nodes = Vec::new();
        for entry in entries.flatten() {
            if *budget == 0 {
                break;
            }
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            if name.starts_with('.') {
                continue;
            }
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            let path = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{prefix}/{name}")
            };
            let kind = if file_type.is_symlink() {
                match self.scope.resolve(&path) {
                    Ok(target) if target.is_file() => TreeKind::File,
                    _ => continue,
                }
            } else if file_type.is_dir() {
                TreeKind::Dir
            } else if file_type.is_file() {
                TreeKind::File
            } else {
                continue;
            };
            *budget -= 1;
            let children = (kind == TreeKind::Dir).then(|| {
                if depth + 1 >= TREE_MAX_DEPTH {
                    Vec::new()
                } else {
                    self.tree_dir(&entry.path(), &path, depth + 1, budget)
                }
            });
            nodes.push(TreeNode {
                name,
                path,
                kind,
                children,
            });
        }
        nodes.sort_by(|a, b| {
            (a.kind != TreeKind::Dir, &a.name).cmp(&(b.kind != TreeKind::Dir, &b.name))
        });
        nodes
    }
}

/// Estado do núcleo gerenciado pelo Tauri: projeto aberto, arquivos abertos, prévia e o motor.
pub struct AppState {
    project: Mutex<Option<OpenProject>>,
    preview: Arc<PreviewHub>,
    engine: Arc<dyn Engine>,
    /// Um pedido de fechar já foi recusado por falha de gravação (o próximo é aceito).
    close_refused: AtomicBool,
    /// Caminhos relativos avisados pelo observador; a ponta de leitura vai para a thread que
    /// chama [`AppState::handle_disk_change`] ([`AppState::take_disk_changes`]).
    disk_tx: Mutex<Sender<String>>,
    disk_rx: Mutex<Option<Receiver<String>>>,
    /// Decisões de plugin e cache de pacotes (dados locais do app).
    trust: Trust,
    /// Ofertas de cópia para o projeto (D-13) e quem as recebe (em produção, o `emit`).
    offers: Arc<Mutex<CopyOffers>>,
    offer_listener: Arc<RwLock<Option<CopyOfferListener>>>,
}

/// Quem recebe cada oferta nova de cópia para o projeto.
pub type CopyOfferListener = Arc<dyn Fn(&CopyIntoProjectOffer) + Send + Sync>;

impl Default for AppState {
    fn default() -> Self {
        Self::new(Arc::new(LangService::default()))
    }
}

impl AppState {
    pub fn new(engine: Arc<dyn Engine>) -> Self {
        let (disk_tx, disk_rx) = mpsc::channel();
        Self {
            project: Mutex::new(None),
            preview: Arc::new(PreviewHub::default()),
            engine,
            close_refused: AtomicBool::new(false),
            disk_tx: Mutex::new(disk_tx),
            disk_rx: Mutex::new(Some(disk_rx)),
            trust: Trust::default(),
            offers: Arc::default(),
            offer_listener: Arc::default(),
        }
    }

    /// Quem recebe as ofertas novas de `copy_into_project_offer`.
    pub fn set_copy_offer_listener(&self, listener: CopyOfferListener) {
        *self
            .offer_listener
            .write()
            .unwrap_or_else(PoisonError::into_inner) = Some(listener);
    }

    fn offers(&self) -> MutexGuard<'_, CopyOffers> {
        self.offers.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Caminhos pedidos fora do projeto com oferta de cópia em aberto.
    pub fn copy_offers(&self) -> Vec<String> {
        self.offers().list()
    }

    /// Recusa (`not_offered`) o pedido de cópia que o núcleo não ofereceu no projeto aberto.
    /// O comando confere antes de mostrar o diálogo.
    pub fn check_copy_offer(&self, requested: &str) -> Result<(), String> {
        if self.lock().is_none() {
            return Err(NO_PROJECT.into());
        }
        match self.offers().referrers(requested) {
            Some(_) => Ok(()),
            None => Err(NOT_OFFERED.into()),
        }
    }

    /// Copia `source` (escolhido pela pessoa no diálogo nativo de `copy_into_project`; nunca
    /// vem da interface) para dentro do projeto como a cópia de `requested`, troca o caminho
    /// nos `.typ` que o pediam e recompila (D-13, FR-023). Só para oferta em aberto; nunca
    /// sobrescreve.
    pub fn copy_into_project(
        &self,
        requested: &str,
        source: &Path,
    ) -> Result<CopiedIntoProject, String> {
        let mut guard = self.lock();
        let project = guard.as_mut().ok_or(NO_PROJECT)?;
        let referrers = self
            .offers()
            .referrers(requested)
            .cloned()
            .ok_or(NOT_OFFERED)?;
        let destination = copy_into::destination(&referrers, requested)?;
        let bytes = copy_into::read_source(source)?;
        let created = copy_into::write_new(&project.scope, &destination, &bytes)?;
        let mut updated = Vec::new();
        for referrer in &referrers {
            let literal = copy_into::literal_for(referrer, &created);
            if let Some(version) = project.replace_literal(referrer, requested, &literal) {
                updated.push(UpdatedFile {
                    path: referrer.clone(),
                    version,
                });
            }
        }
        self.offers().remove(requested);
        project.disk.push(created.clone());
        project.recompile();
        Ok(CopiedIntoProject {
            path: created,
            updated,
        })
    }

    /// Decisões de plugin e cache de pacotes.
    pub fn trust(&self) -> &Trust {
        &self.trust
    }

    /// `decide_plugin`: grava a decisão sobre o pedido pendente `sha256` no projeto aberto e
    /// recompila.
    pub fn decide_plugin(&self, sha256: &str, approve: bool) -> Result<(), String> {
        let guard = self.lock();
        let project = guard.as_ref().ok_or(NO_PROJECT)?;
        self.trust.decide(project.scope.root(), sha256, approve)?;
        project.recompile();
        Ok(())
    }

    /// `redownload_package`: baixa `spec` de novo (sem segurar a trava do projeto durante o
    /// download) e recompila.
    pub fn redownload_package(&self, spec: &str) -> Result<(), String> {
        if self.lock().is_none() {
            return Err(NO_PROJECT.into());
        }
        self.trust.redownload(spec)?;
        if let Some(project) = self.lock().as_ref() {
            project.recompile();
        }
        Ok(())
    }

    fn lock(&self) -> MutexGuard<'_, Option<OpenProject>> {
        self.project.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// "Novo projeto" (US6; FR-002): cria o projeto mínimo em `root` e o abre. Chamado pelo
    /// diálogo nativo do núcleo (`create_project_dialog`) e pelos testes.
    ///
    /// Pasta não vazia → `folder_not_empty` sem escrever nada; o projeto aberto, se houver,
    /// continua aberto em qualquer recusa.
    pub fn create_project(&self, folder: &Path) -> Result<ProjectInfo, String> {
        let root = crate::project::create::create_project(folder, EMBEDDED_TYPST)
            .map_err(|e| e.code().to_owned())?;
        self.open_project(&root).map_err(|_| "not_found".to_owned())
    }

    /// Abre o projeto em `root` (canonicalizada uma vez). Chamado pelo diálogo nativo do
    /// núcleo (`open_project_dialog`) e pelos testes; nunca exposto à interface com caminho.
    ///
    /// Registro ausente é criado (FR-003); registro existente não é regravado. Começa a
    /// compilar o arquivo principal com o compilador embutido; o resultado chega ao assinante
    /// da prévia. Não compila no estado `VersaoDiferente` (FR-012).
    pub fn open_project(&self, root: &Path) -> std::io::Result<ProjectInfo> {
        let scope = Scope::new(root)?;
        let record = record::load(scope.root(), EMBEDDED_TYPST);
        let watcher = self.watch_root(scope.root());
        let mut project = OpenProject {
            scope,
            record,
            files: HashMap::new(),
            compiler: None,
            _watcher: watcher,
            main_missing: false,
            disk: DiskChanges::default(),
        };
        project.create_missing_record();
        project.main_missing = !project.main_exists();
        let mut guard = self.lock();
        self.trust.reset_pending();
        // A prévia do projeto anterior deixa de valer antes de qualquer compilação nova.
        let generation = self.preview.reset();
        if !project.version_differs() {
            self.start_compiler(&mut project, generation);
        }
        project.recompile();
        let info = project.info();
        *guard = Some(project);
        Ok(info)
    }

    /// Liga o observador de mudanças externas a `root`; os avisos vão para a fila de
    /// [`take_disk_changes`](Self::take_disk_changes).
    fn watch_root(&self, root: &Path) -> Option<ProjectWatcher> {
        let tx = self
            .disk_tx
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        watch(root, move |relative| {
            let _ = tx.send(relative);
        })
        .map_err(|e| eprintln!("prelo: observador de arquivos indisponível: {e}"))
        .ok()
    }

    /// Fila de caminhos avisados pelo observador (só a primeira chamada recebe). Quem a lê
    /// chama [`handle_disk_change`](Self::handle_disk_change) para cada caminho.
    pub fn take_disk_changes(&self) -> Option<Receiver<String>> {
        self.disk_rx
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }

    /// O observador avisou que `relative` pode ter mudado no disco: relê pela regra de escopo
    /// e decide pelo conteúdo (FR-008). `Reloaded` já recompilou; `Conflict` → a interface
    /// recebe `external_change_conflict`. Arquivo não aberto, fora da raiz (inclusive por link)
    /// ou removido → `Ignored`.
    pub fn handle_disk_change(&self, relative: &str) -> DiskChange {
        let mut guard = self.lock();
        let Some(project) = guard.as_mut() else {
            return DiskChange::Ignored;
        };
        let path = validate_path(&project.scope, relative);
        let is_open = path
            .as_ref()
            .is_ok_and(|p| project.files.contains_key(p.relative()));
        let (Ok(path), true) = (&path, is_open) else {
            // Arquivo não aberto (ex.: `.bib` exportado pelo gerenciador de referências),
            // removido ou trocado por link: a compilação seguinte relê o disco pela regra de
            // escopo (FR-025). Nada a avisar à interface. Itens ocultos (`.prelo/`, `.git/`)
            // não entram na compilação.
            if !is_absolute(relative)
                && !relative.split(['/', '\\']).any(|c| c.starts_with('.'))
                && !project.files.contains_key(relative)
            {
                project.disk.push(relative);
                if let Ok(path) = &path {
                    if path.relative() != relative {
                        project.disk.push(path.relative());
                    }
                }
                project.recompile();
            }
            return DiskChange::Ignored;
        };
        let Some(file) = project.files.get_mut(path.relative()) else {
            return DiskChange::Ignored;
        };
        let outcome = match project.scope.read(path.relative()) {
            Ok(bytes) => file.disk_changed(bytes),
            Err(Refusal::TooLarge { .. }) => file.mark_conflict(),
            Err(_) => DiskChange::Ignored,
        };
        if outcome == DiskChange::Reloaded {
            project.recompile();
        }
        outcome
    }

    /// Versão atual de `relative` se estiver aberto (o evento `external_change_reloaded` leva
    /// essa versão; `None` sem projeto, fora da raiz ou não aberto).
    pub fn file_version(&self, relative: &str) -> Option<u64> {
        self.with_path(relative, |project, path| {
            project
                .files
                .get(path.relative())
                .map(OpenFile::version)
                .ok_or_else(|| NOT_FOUND.to_owned())
        })
        .ok()
    }

    /// Escolha da pessoa diante de uma mudança externa: `Mine` mantém o texto em memória (e
    /// volta a gravá-lo); `Disk` relê o disco e troca o texto, numa versão nova.
    pub fn resolve_external_change(
        &self,
        requested: &str,
        keep: Keep,
    ) -> Result<OpenedFile, String> {
        self.with_path(requested, |project, path| {
            let file = project.files.get_mut(path.relative()).ok_or(NOT_FOUND)?;
            match keep {
                Keep::Mine => file.keep_mine(),
                Keep::Disk => {
                    let bytes = project
                        .scope
                        .read(path.relative())
                        .map_err(|r| refusal_code(&r))?;
                    file.keep_disk(bytes)?;
                }
            }
            let opened = OpenedFile {
                text: file.text().to_owned(),
                version: file.version(),
            };
            project.recompile();
            Ok(opened)
        })
    }

    /// Liga a agenda de compilação do projeto à prévia da geração `generation`.
    fn start_compiler(&self, project: &mut OpenProject, generation: u64) {
        let root = project.scope.root().to_owned();
        let main = project.record.record.main_or_default().to_owned();
        let env = self.trust.env();
        let disk = project.disk.clone();
        let (scheduler, compilations) = Scheduler::new(move || {
            ProjectCompiler::with_env(root.clone(), &main, env.clone())
                .with_disk_changes(disk.clone())
        });
        let compilations = self.watch_offers(generation, compilations);
        self.preview.spawn_pump(generation, compilations);
        project.compiler = Some(scheduler);
    }

    /// Repassa as compilações à prévia e, no caminho, atualiza as ofertas de cópia para o
    /// projeto (D-13) e avisa as novas. Termina quando a agenda é largada.
    fn watch_offers(
        &self,
        generation: u64,
        compilations: Receiver<crate::compile::scheduler::Compilation<Compiled>>,
    ) -> Receiver<crate::compile::scheduler::Compilation<Compiled>> {
        self.offers().reset(generation);
        let offers = self.offers.clone();
        let listener = self.offer_listener.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name(format!("prelo-offers-{generation}"))
            .spawn(move || {
                for compilation in compilations {
                    let new = offers
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .update(generation, &compilation);
                    if tx.send(compilation).is_err() {
                        break;
                    }
                    let listener = listener
                        .read()
                        .unwrap_or_else(PoisonError::into_inner)
                        .clone();
                    if let Some(listener) = listener {
                        for offer in &new {
                            listener(offer);
                        }
                    }
                }
            })
            .expect("falha ao criar a thread das ofertas de cópia");
        rx
    }

    /// Assina a prévia: `sink` recebe cada `PreviewUpdate`, e logo a prévia atual, se houver.
    pub fn subscribe_preview(&self, sink: impl Fn(&PreviewUpdate) + Send + Sync + 'static) {
        self.preview.subscribe(Arc::new(sink));
    }

    /// SVG da página `index` da prévia atual, se `hash` for o atual; senão `stale_page`. O SVG
    /// é gerado fora das travas, com as imagens citadas por `prelo-asset://<sha256>` (T058).
    pub fn page_svg(&self, index: usize, hash: &str) -> Result<String, String> {
        Ok(self.preview.page_svg(index, hash)?)
    }

    /// `preview_asset`: imagem `sha256` citada num SVG da prévia atual, como `data:` URL.
    /// Hash malformado → `invalid_arguments`; hash que não é de imagem da prévia atual →
    /// `not_found`.
    pub fn preview_asset(&self, sha256: &str) -> Result<String, String> {
        if sha256.len() != 64
            || !sha256
                .bytes()
                .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        {
            return Err(INVALID_ARGUMENTS.into());
        }
        self.preview
            .asset_data_url(sha256)
            .ok_or_else(|| NOT_FOUND.into())
    }

    /// Resposta do protocolo `prelo-asset` (ver [`asset_response`]).
    pub fn preview_asset_response(
        &self,
        request: &tauri::http::Request<Vec<u8>>,
    ) -> tauri::http::Response<Vec<u8>> {
        asset_response(&self.preview, request)
    }

    /// Projeto aberto e o `path` validado.
    fn with_path<T>(
        &self,
        requested: &str,
        f: impl FnOnce(&mut OpenProject, ProjectPath) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut guard = self.lock();
        let project = guard.as_mut().ok_or(NO_PROJECT)?;
        let path = validate_path(&project.scope, requested)?;
        f(project, path)
    }

    /// Valida e copia o estado para o motor (só `Arc`s), sem segurar a trava durante a
    /// consulta.
    fn engine_query(&self, requested: &str) -> Result<(EngineContext, ProjectPath), String> {
        let document = self.preview.document();
        let synced = self.preview.synced();
        self.with_path(requested, |project, path| {
            let ctx = EngineContext {
                root: project.scope.root().to_owned(),
                input: project.input(),
                document,
                synced,
            };
            Ok((ctx, path))
        })
    }

    /// Sugestões de autocompletar em `offset` (UTF-16) do texto atual de `requested`.
    pub fn complete(&self, requested: &str, offset: usize) -> Result<Vec<Completion>, String> {
        let (ctx, path) = self.engine_query(requested)?;
        self.engine.complete(&ctx, &path, offset)
    }

    /// *Tokens* de realce do texto atual de `requested`.
    pub fn semantic_tokens(&self, requested: &str) -> Result<Vec<Token>, String> {
        let (ctx, path) = self.engine_query(requested)?;
        self.engine.semantic_tokens(&ctx, &path)
    }

    /// Origem no projeto do ponto `(x_pt, y_pt)` da página `page` da prévia (a partir de 0);
    /// `None` quando o ponto não tem origem no projeto ou ainda não há prévia (US4, FR-020).
    pub fn jump_from_click(
        &self,
        page: usize,
        x_pt: f64,
        y_pt: f64,
    ) -> Result<Option<SourcePosition>, String> {
        if self.lock().is_none() {
            return Err(NO_PROJECT.into());
        }
        Ok(self
            .preview
            .synced()
            .and_then(|synced| synced.jump_from_click(page, x_pt, y_pt)))
    }

    /// Títulos de seção do último documento bom, em ordem (US4, FR-005); vazio sem prévia.
    pub fn outline(&self) -> Result<Vec<OutlineItem>, String> {
        if self.lock().is_none() {
            return Err(NO_PROJECT.into());
        }
        Ok(self
            .preview
            .synced()
            .map(|synced| synced.outline())
            .unwrap_or_default())
    }

    /// Posições na prévia do trecho em `offset` (US4).
    pub fn jump_from_cursor(
        &self,
        requested: &str,
        offset: usize,
    ) -> Result<Vec<JumpTarget>, String> {
        let (ctx, path) = self.engine_query(requested)?;
        self.engine.jump_from_cursor(&ctx, &path, offset)
    }

    pub fn list_tree(&self) -> Result<Vec<TreeNode>, String> {
        let guard = self.lock();
        let project = guard.as_ref().ok_or(NO_PROJECT)?;
        Ok(project.tree())
    }

    pub fn open_file(&self, requested: &str) -> Result<OpenedFile, String> {
        self.with_path(requested, |project, path| {
            if let Some(file) = project.files.get(path.relative()) {
                return Ok(OpenedFile {
                    text: file.text().to_owned(),
                    version: file.version(),
                });
            }
            let bytes = project
                .scope
                .read(path.relative())
                .map_err(|r| refusal_code(&r))?;
            let file = OpenFile::from_bytes(bytes)?;
            let opened = OpenedFile {
                text: file.text().to_owned(),
                version: file.version(),
            };
            project.files.insert(path.relative().to_owned(), file);
            Ok(opened)
        })
    }

    pub fn edit_file(
        &self,
        requested: &str,
        version: u64,
        changes: &[Change],
    ) -> Result<u64, String> {
        self.with_path(requested, |project, path| {
            let file = project.files.get_mut(path.relative()).ok_or(NOT_FOUND)?;
            let version = file.edit(version, changes)?;
            // A prévia acompanha o texto não salvo (FR-013).
            project.recompile();
            Ok(version)
        })
    }

    /// Grava o arquivo aberto (⌘S) de forma atômica e devolve `savedAt`. Falha →
    /// `write_failed`, com o texto ainda em memória e o arquivo sujo.
    pub fn save_file(&self, requested: &str) -> Result<String, String> {
        self.with_path(requested, |project, path| {
            let file = project.files.get_mut(path.relative()).ok_or(NOT_FOUND)?;
            file.save(path.absolute())?;
            Ok(format_utc(SystemTime::now()))
        })
    }

    /// Salvamento automático: grava os arquivos sujos e ociosos há
    /// [`AUTOSAVE_DELAY`](crate::fs::save::AUTOSAVE_DELAY) em `now`.
    /// Devolve os caminhos que falharam (continuam sujos e são tentados de novo mais tarde).
    pub fn autosave(&self, now: Instant) -> Vec<String> {
        self.save_where(|file| file.autosave_due(now))
    }

    /// Último Salvar antes de fechar (Princípio III): grava todo arquivo sujo. Devolve se a
    /// janela pode fechar: sim se tudo foi gravado; se algo falhou, recusa o primeiro pedido
    /// (o texto continua em memória) e aceita o seguinte, para a pessoa nunca ficar presa.
    pub fn flush_for_close(&self) -> bool {
        if self.save_where(OpenFile::is_dirty).is_empty() {
            self.close_refused.store(false, Ordering::SeqCst);
            return true;
        }
        self.close_refused.swap(true, Ordering::SeqCst)
    }

    /// Grava os arquivos abertos que satisfazem `which`; devolve os que falharam. Cada caminho é
    /// validado de novo antes de gravar (uma pasta trocada por link desde a abertura não leva a
    /// gravação para fora da raiz). O estado fica travado durante a gravação, de modo que
    /// gravações do mesmo arquivo nunca se cruzam.
    fn save_where(&self, which: impl Fn(&OpenFile) -> bool) -> Vec<String> {
        let mut guard = self.lock();
        let Some(project) = guard.as_mut() else {
            return Vec::new();
        };
        let mut failed = Vec::new();
        for (relative, file) in project.files.iter_mut() {
            if !which(file) {
                continue;
            }
            let saved = validate_path(&project.scope, relative)
                .map_err(|_| "write_failed")
                .and_then(|path| file.save(path.absolute()));
            if saved.is_err() {
                failed.push(relative.clone());
            }
        }
        failed.sort();
        failed
    }

    pub fn choose_main_file(&self, requested: &str) -> Result<ProjectInfo, String> {
        self.with_path(requested, |project, path| {
            let is_typ = Path::new(path.relative())
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("typ"));
            if !is_typ || !path.absolute().is_file() {
                return Err("not_a_typ_file".into());
            }
            project
                .record
                .record
                .set_main(path.relative())
                .map_err(|_| OUTSIDE)?;
            // Registro lido inválido não é regravado (data-model.md): a escolha vale em memória.
            if project.record.may_rewrite() {
                let root = project.scope.root().to_owned();
                record::save(&root, &mut project.record).map_err(|_| "write_failed")?;
            }
            project.main_missing = false;
            project.recompile();
            Ok(project.info())
        })
    }

    /// "Atualizar o projeto para a versão embutida" (FR-012): regrava `compiler.typst` com a
    /// versão embutida (campos desconhecidos preservados) e libera a compilação. Sem efeito com
    /// registro inválido, que só "Recriar registro" sobrescreve (e que já usa a embutida).
    pub fn update_project_compiler_version(&self) -> Result<ProjectInfo, String> {
        let mut guard = self.lock();
        let project = guard.as_mut().ok_or(NO_PROJECT)?;
        if !project.record.may_rewrite() {
            return Ok(project.info());
        }
        if project.record.record.compiler_typst() != EMBEDDED_TYPST
            || project.record.state == RecordState::Missing
        {
            let previous = project.record.record.compiler_typst().to_owned();
            project.record.record.set_compiler_typst(EMBEDDED_TYPST);
            let root = project.scope.root().to_owned();
            if record::save(&root, &mut project.record).is_err() {
                project.record.record.set_compiler_typst(&previous);
                return Err("write_failed".into());
            }
        }
        if project.compiler.is_none() {
            // Nenhuma compilação aconteceu desde a abertura: a prévia (vazia) recomeça.
            let generation = self.preview.reset();
            self.start_compiler(project, generation);
        }
        project.recompile();
        Ok(project.info())
    }

    /// "Compilar" (FR-014a; contracts/ipc.md `compile_now`): a próxima compilação relê do
    /// disco todos os arquivos não abertos, e o estado atual entra na fila na hora,
    /// abandonando a compilação em andamento dentro do limite de threads
    /// ([`Scheduler::compile_now`]). Devolve o `compileId` pedido; o resultado chega pela
    /// prévia. Decisões de plugin e pacote não mudam. Sem projeto: `no_project`; versão
    /// registrada diferente: `compiler_version_mismatch`; agenda suspensa:
    /// `compile_suspended`.
    pub fn compile_now(&self) -> Result<u64, String> {
        let guard = self.lock();
        let project = guard.as_ref().ok_or(NO_PROJECT)?;
        let compiler = match &project.compiler {
            Some(compiler) if !project.version_differs() => compiler,
            _ => return Err(VERSION_MISMATCH.into()),
        };
        project.disk.reload_all();
        compiler
            .compile_now(project.input())
            .map_err(|_| COMPILE_SUSPENDED.into())
    }

    /// Cria um arquivo vazio em `requested` (FR-034, FR-035). Devolve o caminho normalizado.
    pub fn create_file(&self, requested: &str) -> Result<String, String> {
        self.create(requested, NewItem::File)
    }

    /// Cria uma pasta vazia em `requested` (FR-034, FR-035). Devolve o caminho normalizado.
    pub fn create_dir(&self, requested: &str) -> Result<String, String> {
        self.create(requested, NewItem::Dir)
    }

    /// Regra de escopo e criação exclusiva em [`crate::fs::create`]. Recompila: um `#include`
    /// ou `#image` que pedia o arquivo novo deixa de acusar arquivo não encontrado.
    fn create(&self, requested: &str, item: NewItem) -> Result<String, String> {
        let guard = self.lock();
        let project = guard.as_ref().ok_or(NO_PROJECT)?;
        if is_absolute(requested) {
            return Err(OUTSIDE.into());
        }
        let path = create::create(&project.scope, requested, item).map_err(|e| e.code())?;
        project.recompile();
        Ok(path)
    }

    /// O que exportar (FR-011, FR-012, FR-029): o documento da última prévia `Ok`, o nome do
    /// projeto (nome sugerido do PDF) e a raiz (pasta inicial do diálogo). Recusa sem projeto,
    /// com versão registrada ≠ embutida e quando a última compilação teve erro.
    pub fn export_source(&self) -> Result<(ExportSource, String, PathBuf), ExportError> {
        let (name, root) = {
            let guard = self.lock();
            let project = guard.as_ref().ok_or(ExportError::NoProject)?;
            if project.version_differs() || project.compiler.is_none() {
                return Err(ExportError::VersionMismatch);
            }
            (project.info().name, project.scope.root().to_owned())
        };
        match self.preview.export_source() {
            Some(Ok((compile_id, document))) => Ok((
                ExportSource {
                    compile_id,
                    document,
                },
                name,
                root,
            )),
            Some(Err(diagnostics)) => Err(ExportError::CompileErrors(diagnostics)),
            None => Err(ExportError::CompileErrors(vec![
                crate::compile::export::not_compiled_yet(),
            ])),
        }
    }

    /// Exporta o PDF da última prévia `Ok` para `path` (o caminho que o diálogo nativo de
    /// `export_pdf_dialog` devolve; nunca vem da interface).
    pub fn export_pdf(&self, path: &Path) -> Result<Exported, ExportError> {
        let (source, _, _) = self.export_source()?;
        crate::compile::export::export(&source, path)
    }

    /// Versões embutida e registrada (contracts/ipc.md `compiler_info`).
    pub fn compiler_info(&self) -> CompilerInfo {
        CompilerInfo {
            embedded: EMBEDDED_TYPST.to_owned(),
            registered: self
                .lock()
                .as_ref()
                .and_then(|project| project.info().compiler.registered),
            // Licenças das fontes embutidas (FR-016, tela "Sobre").
            fonts: crate::world::fonts::licenses()
                .iter()
                .filter_map(|font| serde_json::to_value(font).ok())
                .collect(),
        }
    }
}

// ---------------------------------------------------------------- comandos

fn invalid() -> String {
    INVALID_ARGUMENTS.into()
}

/// Argumentos como objeto JSON com campos exatos.
fn args<T: DeserializeOwned>(request: &Request<'_>) -> Result<T, String> {
    match request.body() {
        InvokeBody::Json(value @ Value::Object(_)) => {
            serde_json::from_value(value.clone()).map_err(|_| invalid())
        }
        _ => Err(invalid()),
    }
}

/// Sem argumentos: só objeto vazio ou nulo.
pub(crate) fn no_args(request: &Request<'_>) -> Result<(), String> {
    match request.body() {
        InvokeBody::Json(Value::Null) => Ok(()),
        InvokeBody::Json(Value::Object(map)) if map.is_empty() => Ok(()),
        _ => Err(invalid()),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DecidePluginArgs {
    sha256: String,
    approve: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpecArgs {
    spec: String,
}

/// Decisão sobre `plugin_approval_needed` (FR-019) → recompila.
#[tauri::command(async)]
pub fn decide_plugin(state: State<'_, AppState>, request: Request<'_>) -> Result<(), String> {
    let a: DecidePluginArgs = args(&request)?;
    state.decide_plugin(&a.sha256, a.approve)
}

/// "Baixar de novo" um pacote adulterado (FR-018) → recompila. Erros: `network_unavailable`,
/// `package_rejected`.
#[tauri::command(async)]
pub fn redownload_package(state: State<'_, AppState>, request: Request<'_>) -> Result<(), String> {
    let a: SpecArgs = args(&request)?;
    state.redownload_package(&a.spec)
}

/// Corpo de `copy_into_project`: só o caminho pedido no `.typ`. O arquivo de origem nunca vem
/// da interface (`source` ou outro campo → `invalid_arguments`).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestedArgs {
    requested: String,
}

/// "Escolher arquivo para copiar…" da oferta `copy_into_project_offer` (D-13, FR-023): confere
/// a oferta, mostra o diálogo nativo do núcleo para a pessoa escolher o arquivo e copia.
/// Cancelado → `null`.
#[tauri::command(async)]
pub fn copy_into_project<R: Runtime>(
    window: Window<R>,
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<Option<CopiedIntoProject>, String> {
    let a: RequestedArgs = args(&request)?;
    state.check_copy_offer(&a.requested)?;
    let name = a.requested.rsplit(['/', '\\']).next().unwrap_or_default();
    let Some(file) = window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title(format!("Escolher o arquivo para copiar como “{name}”"))
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let source = file.into_path().map_err(|_| NOT_FOUND.to_owned())?;
    state.copy_into_project(&a.requested, &source).map(Some)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PathArgs {
    path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OffsetArgs {
    path: String,
    offset: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EditArgs {
    path: String,
    version: u64,
    changes: Vec<Change>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KeepArgs {
    path: String,
    keep: Keep,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PageArgs {
    index: usize,
    hash: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AssetArgs {
    sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ClickArgs {
    page: usize,
    x_pt: f64,
    y_pt: f64,
}

/// Corpo de `subscribe_preview`: só o canal (o Tauri o entrega à parte, como `Channel`).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChannelArgs {
    #[allow(dead_code)] // lido pelo Tauri como `Channel`
    channel: String,
}

#[derive(Serialize)]
pub struct EditResult {
    version: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveResult {
    saved_at: String,
}

#[tauri::command(async)]
pub fn list_tree(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<Vec<TreeNode>, String> {
    no_args(&request)?;
    state.list_tree()
}

#[tauri::command(async)]
pub fn choose_main_file(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<ProjectInfo, String> {
    let a: PathArgs = args(&request)?;
    state.choose_main_file(&a.path)
}

/// Resposta de `create_file` e `create_dir`: o caminho normalizado do item criado.
#[derive(Debug, Serialize)]
pub struct CreatedPath {
    path: String,
}

/// Arquivo vazio novo, sem nunca sobrescrever (FR-034, FR-035).
#[tauri::command(async)]
pub fn create_file(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<CreatedPath, String> {
    let a: PathArgs = args(&request)?;
    let path = state.create_file(&a.path)?;
    Ok(CreatedPath { path })
}

/// Pasta vazia nova, sem nunca sobrescrever (FR-034, FR-035).
#[tauri::command(async)]
pub fn create_dir(state: State<'_, AppState>, request: Request<'_>) -> Result<CreatedPath, String> {
    let a: PathArgs = args(&request)?;
    let path = state.create_dir(&a.path)?;
    Ok(CreatedPath { path })
}

#[tauri::command(async)]
pub fn open_file(state: State<'_, AppState>, request: Request<'_>) -> Result<OpenedFile, String> {
    let a: PathArgs = args(&request)?;
    state.open_file(&a.path)
}

#[tauri::command(async)]
pub fn edit_file(state: State<'_, AppState>, request: Request<'_>) -> Result<EditResult, String> {
    let a: EditArgs = args(&request)?;
    let version = state.edit_file(&a.path, a.version, &a.changes)?;
    Ok(EditResult { version })
}

#[tauri::command(async)]
pub fn save_file(state: State<'_, AppState>, request: Request<'_>) -> Result<SaveResult, String> {
    let a: PathArgs = args(&request)?;
    let saved_at = state.save_file(&a.path)?;
    Ok(SaveResult { saved_at })
}

/// Escolha diante de `external_change_conflict` (FR-008): `{ text, version }` do arquivo.
#[tauri::command(async)]
pub fn resolve_external_change(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<OpenedFile, String> {
    let a: KeepArgs = args(&request)?;
    state.resolve_external_change(&a.path, a.keep)
}

/// Canal da prévia: recebe um `PreviewUpdate` a cada compilação (e logo a prévia atual).
/// Assinar de novo troca o canal.
#[tauri::command(async)]
pub fn subscribe_preview(
    state: State<'_, AppState>,
    request: Request<'_>,
    channel: Channel<PreviewUpdate>,
) -> Result<(), String> {
    let _: ChannelArgs = args(&request)?;
    state.subscribe_preview(move |update| {
        // Webview fechada: a próxima assinatura troca o canal.
        let _ = channel.send(update.clone());
    });
    Ok(())
}

/// SVG da página (texto); hash velho → `stale_page`.
#[tauri::command(async)]
pub fn page_svg(state: State<'_, AppState>, request: Request<'_>) -> Result<String, String> {
    let a: PageArgs = args(&request)?;
    state.page_svg(a.index, &a.hash)
}

/// Imagem citada no SVG da prévia (`prelo-asset://<sha256>`) como `data:` URL; a interface a
/// devolve ao SVG antes de mostrá-lo. Erros: `invalid_arguments`, `not_found`.
#[tauri::command(async)]
pub fn preview_asset(state: State<'_, AppState>, request: Request<'_>) -> Result<String, String> {
    let a: AssetArgs = args(&request)?;
    state.preview_asset(&a.sha256)
}

#[tauri::command(async)]
pub fn complete(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<Vec<Completion>, String> {
    let a: OffsetArgs = args(&request)?;
    state.complete(&a.path, a.offset)
}

#[tauri::command(async)]
pub fn semantic_tokens(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<Vec<Token>, String> {
    let a: PathArgs = args(&request)?;
    state.semantic_tokens(&a.path)
}

#[tauri::command(async)]
pub fn jump_from_cursor(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<Vec<JumpTarget>, String> {
    let a: OffsetArgs = args(&request)?;
    state.jump_from_cursor(&a.path, a.offset)
}

#[tauri::command(async)]
pub fn jump_from_click(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<Option<SourcePosition>, String> {
    let a: ClickArgs = args(&request)?;
    state.jump_from_click(a.page, a.x_pt, a.y_pt)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileNowResult {
    compile_id: u64,
}

/// "Compilar" (FR-014a): sem argumentos; o resultado chega pelo canal da prévia.
#[tauri::command(async)]
pub fn compile_now(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<CompileNowResult, String> {
    no_args(&request)?;
    state
        .compile_now()
        .map(|compile_id| CompileNowResult { compile_id })
}

#[tauri::command(async)]
pub fn outline(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<Vec<OutlineItem>, String> {
    no_args(&request)?;
    state.outline()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::save::AUTOSAVE_DELAY;
    use tempfile::TempDir;

    fn change(from: usize, to: usize, insert: &str) -> Change {
        Change {
            from,
            to,
            insert: insert.into(),
        }
    }

    fn project() -> (TempDir, AppState) {
        let dir = tempfile::tempdir().expect("pasta temporária");
        std::fs::write(dir.path().join("main.typ"), "= Título\n").unwrap();
        std::fs::create_dir(dir.path().join("capitulos")).unwrap();
        std::fs::write(dir.path().join("capitulos/intro.typ"), "intro").unwrap();
        std::fs::write(dir.path().join("dados.bib"), "@x{}").unwrap();
        std::fs::write(dir.path().join("latin1.typ"), [0x63, 0xe7, 0x0a]).unwrap();
        let state = AppState::default();
        state.open_project(dir.path()).expect("abrir projeto");
        (dir, state)
    }

    #[test]
    fn mudancas_sao_aplicadas_nas_coordenadas_do_texto_anterior() {
        assert_eq!(apply_changes("abc", &[]).unwrap(), "abc");
        assert_eq!(apply_changes("abc", &[change(1, 2, "X")]).unwrap(), "aXc");
        assert_eq!(
            apply_changes("abcdef", &[change(0, 1, "1"), change(3, 5, "")]).unwrap(),
            "1bcf"
        );
        assert_eq!(apply_changes("abc", &[change(3, 3, "!")]).unwrap(), "abc!");
        // Inserção e troca no mesmo ponto, em ordem.
        assert_eq!(
            apply_changes("ab", &[change(1, 1, "X"), change(1, 2, "Y")]).unwrap(),
            "aXY"
        );
    }

    #[test]
    fn deslocamentos_sao_em_unidades_utf16() {
        // "ç" ocupa 1 unidade UTF-16 (2 bytes UTF-8); "𝑥" ocupa 2 (4 bytes).
        assert_eq!(apply_changes("çã", &[change(1, 2, "a")]).unwrap(), "ça");
        assert_eq!(apply_changes("𝑥y", &[change(2, 3, "z")]).unwrap(), "𝑥z");
        assert_eq!(apply_changes("𝑥", &[change(2, 2, "!")]).unwrap(), "𝑥!");
    }

    #[test]
    fn intervalo_invalido_e_bad_range_sem_aplicar_nada() {
        for changes in [
            vec![change(0, 4, "")],
            vec![change(2, 1, "")],
            vec![change(4, 4, "x")],
            vec![change(2, 3, ""), change(0, 1, "")],
            vec![change(0, 2, ""), change(1, 3, "")],
        ] {
            assert_eq!(
                apply_changes("abc", &changes),
                Err("bad_range"),
                "{changes:?}"
            );
        }
        // No meio de um par substituto.
        assert_eq!(apply_changes("𝑥", &[change(1, 1, "a")]), Err("bad_range"));
        assert_eq!(apply_changes("𝑥", &[change(0, 1, "")]), Err("bad_range"));
    }

    #[test]
    fn open_file_devolve_texto_e_versao_e_mantem_em_memoria() {
        let (dir, state) = project();
        let opened = state.open_file("main.typ").unwrap();
        assert_eq!(opened.text, "= Título\n");
        assert_eq!(opened.version, 0);
        let v = state
            .edit_file("main.typ", 0, &[change(0, 0, "x")])
            .unwrap();
        assert_eq!(v, 1);
        // Reabrir devolve o texto em memória (overlay), não o do disco.
        let again = state.open_file("main.typ").unwrap();
        assert_eq!(again.text, "x= Título\n");
        assert_eq!(again.version, 1);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("main.typ")).unwrap(),
            "= Título\n",
            "edit_file não grava no disco (save_file grava)"
        );
    }

    #[test]
    fn open_file_recusa_nao_utf8_pasta_e_inexistente() {
        let (_dir, state) = project();
        assert_eq!(state.open_file("latin1.typ"), Err("not_utf8".into()));
        assert_eq!(state.open_file("capitulos"), Err("not_found".into()));
        assert_eq!(state.open_file("nada.typ"), Err("not_found".into()));
        assert_eq!(state.open_file(""), Err("not_found".into()));
    }

    #[test]
    fn open_file_recusa_arquivo_grande_demais() {
        let (dir, state) = project();
        let f = std::fs::File::create(dir.path().join("grande.typ")).unwrap();
        // Arquivo esparso: não ocupa disco.
        f.set_len(crate::fs::scope::MAX_FILE_BYTES + 1).unwrap();
        assert_eq!(state.open_file("grande.typ"), Err("too_large".into()));
    }

    #[test]
    fn edit_file_confere_versao_e_intervalo() {
        let (_dir, state) = project();
        assert_eq!(
            state.edit_file("main.typ", 0, &[change(0, 0, "x")]),
            Err("not_found".into()),
            "arquivo não aberto"
        );
        state.open_file("main.typ").unwrap();
        assert_eq!(
            state.edit_file("main.typ", 3, &[change(0, 0, "x")]),
            Err("version_mismatch".into())
        );
        assert_eq!(
            state.edit_file("main.typ", 0, &[change(0, 99, "")]),
            Err("bad_range".into())
        );
        // Recusa não muda a versão.
        assert_eq!(state.edit_file("main.typ", 0, &[change(0, 0, "a")]), Ok(1));
        assert_eq!(state.edit_file("main.typ", 1, &[]), Ok(2));
        assert_eq!(state.open_file("main.typ").unwrap().text, "a= Título\n");
    }

    #[test]
    fn edit_file_pelo_caminho_alternativo_altera_o_mesmo_arquivo() {
        let (_dir, state) = project();
        state.open_file("main.typ").unwrap();
        assert_eq!(
            state.edit_file("capitulos/../main.typ", 0, &[change(0, 0, "y")]),
            Ok(1)
        );
        assert_eq!(state.open_file("./main.typ").unwrap().text, "y= Título\n");
    }

    #[test]
    fn choose_main_file_aceita_so_typ_e_grava_o_registro() {
        let (dir, state) = project();
        assert_eq!(
            state.choose_main_file("dados.bib"),
            Err("not_a_typ_file".into())
        );
        assert_eq!(
            state.choose_main_file("capitulos"),
            Err("not_a_typ_file".into())
        );
        assert_eq!(
            state.choose_main_file("../main.typ"),
            Err("path_outside_project".into())
        );
        let info = state.choose_main_file("capitulos/intro.typ").unwrap();
        assert_eq!(info.main_file, "capitulos/intro.typ");
        assert_eq!(info.compiler.embedded, EMBEDDED_TYPST);
        let record = std::fs::read_to_string(dir.path().join(".prelo/project.json")).unwrap();
        assert!(record.contains("capitulos/intro.typ"), "{record}");
    }

    #[test]
    fn registro_invalido_nao_e_regravado_ao_escolher_principal() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.typ"), "a").unwrap();
        std::fs::create_dir(dir.path().join(".prelo")).unwrap();
        std::fs::write(dir.path().join(".prelo/project.json"), "{ quebrado").unwrap();
        let state = AppState::default();
        let info = state.open_project(dir.path()).unwrap();
        // Sem main.typ: o padrão não existe e a interface pede a escolha.
        assert_eq!(info.warnings, ["project_json_invalid", "main_file_missing"]);
        assert_eq!(info.main_file, "main.typ");
        let info = state.choose_main_file("a.typ").unwrap();
        assert_eq!(info.main_file, "a.typ");
        assert_eq!(info.warnings, vec!["project_json_invalid".to_string()]);
        assert_eq!(
            std::fs::read_to_string(dir.path().join(".prelo/project.json")).unwrap(),
            "{ quebrado"
        );
    }

    #[test]
    fn save_file_grava_o_texto_em_memoria_e_devolve_saved_at() {
        let (dir, state) = project();
        state.open_file("main.typ").unwrap();
        state
            .edit_file("main.typ", 0, &[change(0, 0, "x")])
            .unwrap();
        let saved_at = state.save_file("./main.typ").unwrap();
        assert!(
            saved_at.ends_with('Z') && saved_at.contains('T'),
            "{saved_at}"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("main.typ")).unwrap(),
            "x= Título\n"
        );
        // A versão continua; a edição seguinte parte dela.
        assert_eq!(state.edit_file("main.typ", 1, &[change(0, 1, "")]), Ok(2));
    }

    #[test]
    fn save_file_de_arquivo_nao_aberto_e_not_found() {
        let (_dir, state) = project();
        assert_eq!(state.save_file("main.typ"), Err("not_found".into()));
        assert_eq!(state.save_file("../fora.typ"), Err(OUTSIDE.into()));
    }

    #[test]
    fn salvamento_automatico_grava_so_o_que_esta_ocioso() {
        let (dir, state) = project();
        state.open_file("main.typ").unwrap();
        state.open_file("capitulos/intro.typ").unwrap();
        let before = Instant::now();
        state
            .edit_file("main.typ", 0, &[change(0, 0, "x")])
            .unwrap();
        assert!(state.autosave(before + AUTOSAVE_DELAY / 2).is_empty());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("main.typ")).unwrap(),
            "= Título\n",
            "antes de 2 s não grava"
        );
        assert!(state.autosave(Instant::now() + AUTOSAVE_DELAY).is_empty());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("main.typ")).unwrap(),
            "x= Título\n"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("capitulos/intro.typ")).unwrap(),
            "intro",
            "arquivo limpo não é regravado"
        );
    }

    #[test]
    fn fechar_grava_todos_os_arquivos_sujos() {
        let (dir, state) = project();
        state.open_file("main.typ").unwrap();
        state.open_file("capitulos/intro.typ").unwrap();
        state
            .edit_file("main.typ", 0, &[change(0, 0, "a")])
            .unwrap();
        state
            .edit_file("capitulos/intro.typ", 0, &[change(5, 5, "!")])
            .unwrap();
        assert!(state.flush_for_close());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("main.typ")).unwrap(),
            "a= Título\n"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("capitulos/intro.typ")).unwrap(),
            "intro!"
        );
    }

    #[test]
    fn fechar_sem_projeto_aberto_e_permitido() {
        assert!(AppState::default().flush_for_close());
    }

    #[cfg(unix)]
    #[test]
    fn falha_ao_gravar_recusa_o_primeiro_pedido_de_fechar_e_mantem_o_texto() {
        use std::os::unix::fs::PermissionsExt;
        let (dir, state) = project();
        state.open_file("capitulos/intro.typ").unwrap();
        state
            .edit_file("capitulos/intro.typ", 0, &[change(0, 0, "novo ")])
            .unwrap();
        let sub = dir.path().join("capitulos");
        std::fs::set_permissions(&sub, std::fs::Permissions::from_mode(0o555)).unwrap();
        assert_eq!(
            state.save_file("capitulos/intro.typ"),
            Err("write_failed".into())
        );
        assert_eq!(
            state.autosave(Instant::now() + AUTOSAVE_DELAY * 10),
            ["capitulos/intro.typ"]
        );
        let first = state.flush_for_close();
        let second = state.flush_for_close();
        std::fs::set_permissions(&sub, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(!first, "falha recusa o fechamento");
        assert!(second, "o segundo pedido fecha (a pessoa não fica presa)");
        assert_eq!(
            std::fs::read_to_string(sub.join("intro.typ")).unwrap(),
            "intro"
        );
        assert_eq!(
            state.open_file("capitulos/intro.typ").unwrap().text,
            "novo intro",
            "o texto continua em memória"
        );
        // Com o disco de volta, salvar funciona.
        state.save_file("capitulos/intro.typ").unwrap();
        assert_eq!(
            std::fs::read_to_string(sub.join("intro.typ")).unwrap(),
            "novo intro"
        );
    }

    fn tree_paths(nodes: &[TreeNode], out: &mut Vec<(String, TreeKind)>) {
        for node in nodes {
            out.push((node.path.clone(), node.kind));
            if let Some(children) = &node.children {
                tree_paths(children, out);
            }
        }
    }

    fn listed(state: &AppState) -> Vec<(String, TreeKind)> {
        let mut out = Vec::new();
        tree_paths(&state.list_tree().unwrap(), &mut out);
        out
    }

    #[test]
    fn arquivo_criado_aparece_na_arvore_e_abre_vazio() {
        let (dir, state) = project();
        assert_eq!(
            state.create_file("capitulos/metodo.typ"),
            Ok("capitulos/metodo.typ".into())
        );
        assert!(listed(&state).contains(&("capitulos/metodo.typ".into(), TreeKind::File)));
        let opened = state.open_file("capitulos/metodo.typ").unwrap();
        assert_eq!(opened.text, "");
        assert_eq!(opened.version, 0);
        assert_eq!(
            std::fs::metadata(dir.path().join("capitulos/metodo.typ"))
                .unwrap()
                .len(),
            0
        );
    }

    #[test]
    fn pasta_criada_aparece_na_arvore_vazia() {
        let (_dir, state) = project();
        assert_eq!(state.create_dir("figuras"), Ok("figuras".into()));
        let tree = state.list_tree().unwrap();
        let node = tree
            .iter()
            .find(|n| n.path == "figuras")
            .expect("pasta nova");
        assert_eq!(node.kind, TreeKind::Dir);
        assert_eq!(node.children.as_deref(), Some(&[][..]));
        assert_eq!(
            state.create_file("figuras/legenda.typ"),
            Ok("figuras/legenda.typ".into())
        );
    }

    #[test]
    fn observador_nao_acusa_conflito_no_arquivo_criado() {
        let (_dir, state) = project();
        state.create_file("novo.typ").unwrap();
        // Aviso do observador antes de abrir: arquivo não aberto é ignorado.
        assert_eq!(state.handle_disk_change("novo.typ"), DiskChange::Ignored);
        state.open_file("novo.typ").unwrap();
        // Aviso atrasado da criação, com o arquivo já aberto e editado (sujo).
        assert_eq!(state.handle_disk_change("novo.typ"), DiskChange::Ignored);
        state
            .edit_file("novo.typ", 0, &[change(0, 0, "= Novo\n")])
            .unwrap();
        assert_eq!(state.handle_disk_change("novo.typ"), DiskChange::Ignored);
        assert_eq!(state.open_file("novo.typ").unwrap().text, "= Novo\n");
        state.save_file("novo.typ").unwrap();
        assert_eq!(state.handle_disk_change("novo.typ"), DiskChange::Ignored);
    }

    #[test]
    fn criar_responde_os_codigos_do_contrato() {
        let (dir, state) = project();
        for create in [AppState::create_file, AppState::create_dir] {
            assert_eq!(create(&state, "main.typ"), Err("already_exists".into()));
            assert_eq!(create(&state, ".oculto"), Err("invalid_name".into()));
            assert_eq!(create(&state, "ab:c"), Err("invalid_name".into()));
            assert_eq!(create(&state, "../fora"), Err(OUTSIDE.into()));
            assert_eq!(create(&state, "/fora"), Err(OUTSIDE.into()));
            let absolute = dir.path().join("novo-abs").to_string_lossy().into_owned();
            assert_eq!(create(&state, &absolute), Err(OUTSIDE.into()));
            assert_eq!(create(&state, "nada/novo"), Err(NOT_FOUND.into()));
        }
        assert_eq!(
            std::fs::read_to_string(dir.path().join("main.typ")).unwrap(),
            "= Título\n"
        );
        assert!(!dir.path().join("novo-abs").exists());
    }

    #[test]
    fn criar_sem_projeto_aberto_e_no_project() {
        let state = AppState::default();
        assert_eq!(state.create_file("novo.typ"), Err(NO_PROJECT.into()));
        assert_eq!(state.create_dir("nova"), Err(NO_PROJECT.into()));
    }

    #[test]
    fn motor_indisponivel_responde_not_implemented() {
        let p = ProjectPath {
            relative: "main.typ".into(),
            absolute: PathBuf::from("main.typ"),
        };
        let ctx = EngineContext {
            root: PathBuf::from("."),
            input: CompileInput {
                main: "main.typ".into(),
                files: Vec::new(),
            },
            document: None,
            synced: None,
        };
        assert_eq!(
            Unavailable.complete(&ctx, &p, 0),
            Err("not_implemented".into())
        );
        assert_eq!(
            Unavailable.semantic_tokens(&ctx, &p),
            Err("not_implemented".into())
        );
        assert_eq!(
            Unavailable.jump_from_cursor(&ctx, &p, 0),
            Err("not_implemented".into())
        );
    }
}
