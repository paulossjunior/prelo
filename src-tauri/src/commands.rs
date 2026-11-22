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
//! (T036) passa pelo trait [`Engine`]; até lá, [`Unavailable`] responde `not_implemented`.
//! Mudança externa (`resolve_external_change`, T034) já valida o caminho e responde
//! `not_implemented`. Arquivo aberto e gravação (`open_file`, `edit_file`, `save_file`,
//! salvamento automático e último Salvar ao fechar; T033) moram em [`crate::fs::save`].

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Instant, SystemTime};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::ipc::{Channel, InvokeBody, Request};
use tauri::State;

use crate::compile::preview::{render_svg, PreviewHub, PreviewUpdate};
use crate::compile::project::{CompileInput, ProjectCompiler};
use crate::compile::scheduler::Scheduler;
pub use crate::fs::save::{apply_changes, Change};
use crate::fs::save::{format_utc, OpenFile};
use crate::fs::scope::{Refusal, Scope};
use crate::project::record::{self, LoadedRecord, RecordState};
use crate::project::{CompilerInfo, MAIN_FILE_MISSING};

pub use crate::project::EMBEDDED_TYPST;

const OUTSIDE: &str = "path_outside_project";
const NOT_FOUND: &str = "not_found";
const INVALID_ARGUMENTS: &str = "invalid_arguments";
const NO_PROJECT: &str = "no_project";
const NOT_IMPLEMENTED: &str = "not_implemented";
const VERSION_MISMATCH: &str = "compiler_version_mismatch";

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
        Refusal::NotFound { .. } | Refusal::SpecialFile { .. } => NOT_FOUND,
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

/// Consultas que dependem do World (T020) e dos serviços de prévia e linguagem (T035, T036).
/// Recebem só caminhos já validados. Formatos de `Completion` e `Token` saem com T036.
pub trait Engine: Send + Sync + 'static {
    fn complete(&self, path: &ProjectPath, offset: usize) -> Result<Vec<Value>, String>;
    fn semantic_tokens(&self, path: &ProjectPath) -> Result<Vec<Value>, String>;
    fn jump_from_cursor(
        &self,
        path: &ProjectPath,
        offset: usize,
    ) -> Result<Vec<JumpTarget>, String>;
}

/// Motor enquanto o World (T020) não existe.
pub struct Unavailable;

impl Engine for Unavailable {
    fn complete(&self, _path: &ProjectPath, _offset: usize) -> Result<Vec<Value>, String> {
        Err(NOT_IMPLEMENTED.into())
    }

    fn semantic_tokens(&self, _path: &ProjectPath) -> Result<Vec<Value>, String> {
        Err(NOT_IMPLEMENTED.into())
    }

    fn jump_from_cursor(
        &self,
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
    /// O arquivo principal (registrado ou `main.typ`) não existe: aviso `main_file_missing`
    /// até `choose_main_file` (a compilação responde com o diagnóstico de arquivo ausente).
    main_missing: bool,
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

    /// Põe o estado atual (principal + arquivos abertos) na fila de compilação.
    fn recompile(&self) {
        if let Some(compiler) = &self.compiler {
            compiler.submit(CompileInput {
                main: self.record.record.main_or_default().to_owned(),
                files: self
                    .files
                    .iter()
                    .map(|(path, file)| (path.clone(), file.shared_text()))
                    .collect(),
            });
        }
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
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(Arc::new(Unavailable))
    }
}

impl AppState {
    pub fn new(engine: Arc<dyn Engine>) -> Self {
        Self {
            project: Mutex::new(None),
            preview: Arc::new(PreviewHub::default()),
            engine,
            close_refused: AtomicBool::new(false),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Option<OpenProject>> {
        self.project.lock().unwrap_or_else(PoisonError::into_inner)
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
        let mut project = OpenProject {
            scope,
            record,
            files: HashMap::new(),
            compiler: None,
            main_missing: false,
        };
        project.create_missing_record();
        project.main_missing = !project.main_exists();
        let mut guard = self.lock();
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

    /// Liga a agenda de compilação do projeto à prévia da geração `generation`.
    fn start_compiler(&self, project: &mut OpenProject, generation: u64) {
        let root = project.scope.root().to_owned();
        let main = project.record.record.main_or_default().to_owned();
        let (scheduler, compilations) =
            Scheduler::new(move || ProjectCompiler::new(root.clone(), &main));
        self.preview.spawn_pump(generation, compilations);
        project.compiler = Some(scheduler);
    }

    /// Assina a prévia: `sink` recebe cada `PreviewUpdate`, e logo a prévia atual, se houver.
    pub fn subscribe_preview(&self, sink: impl Fn(&PreviewUpdate) + Send + Sync + 'static) {
        self.preview.subscribe(Arc::new(sink));
    }

    /// SVG da página `index` da prévia atual, se `hash` for o atual; senão `stale_page`. O SVG
    /// é gerado fora das travas.
    pub fn page_svg(&self, index: usize, hash: &str) -> Result<String, String> {
        let page = self.preview.page(index, hash)?;
        Ok(render_svg(&page))
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

    /// Só valida (sem segurar o estado durante a consulta ao motor).
    fn validated(&self, requested: &str) -> Result<ProjectPath, String> {
        self.with_path(requested, |_, path| Ok(path))
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

    /// Versões embutida e registrada (contracts/ipc.md `compiler_info`).
    pub fn compiler_info(&self) -> CompilerInfo {
        CompilerInfo {
            embedded: EMBEDDED_TYPST.to_owned(),
            registered: self
                .lock()
                .as_ref()
                .and_then(|project| project.info().compiler.registered),
            fonts: Vec::new(),
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
#[serde(rename_all = "lowercase")]
enum Keep {
    Mine,
    Disk,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KeepArgs {
    path: String,
    #[allow(dead_code)] // usado por T034
    keep: Keep,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PageArgs {
    index: usize,
    hash: String,
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

/// Conflito com mudança externa entra com T034; aqui só a validação.
#[tauri::command(async)]
pub fn resolve_external_change(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<Value, String> {
    let a: KeepArgs = args(&request)?;
    state.validated(&a.path)?;
    Err(NOT_IMPLEMENTED.into())
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

#[tauri::command(async)]
pub fn complete(state: State<'_, AppState>, request: Request<'_>) -> Result<Vec<Value>, String> {
    let a: OffsetArgs = args(&request)?;
    let path = state.validated(&a.path)?;
    state.engine.complete(&path, a.offset)
}

#[tauri::command(async)]
pub fn semantic_tokens(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<Vec<Value>, String> {
    let a: PathArgs = args(&request)?;
    let path = state.validated(&a.path)?;
    state.engine.semantic_tokens(&path)
}

#[tauri::command(async)]
pub fn jump_from_cursor(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<Vec<JumpTarget>, String> {
    let a: OffsetArgs = args(&request)?;
    let path = state.validated(&a.path)?;
    state.engine.jump_from_cursor(&path, a.offset)
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

    #[test]
    fn motor_indisponivel_responde_not_implemented() {
        let p = ProjectPath {
            relative: "main.typ".into(),
            absolute: PathBuf::from("main.typ"),
        };
        assert_eq!(Unavailable.complete(&p, 0), Err("not_implemented".into()));
        assert_eq!(
            Unavailable.semantic_tokens(&p),
            Err("not_implemented".into())
        );
        assert_eq!(
            Unavailable.jump_from_cursor(&p, 0),
            Err("not_implemented".into())
        );
    }
}
