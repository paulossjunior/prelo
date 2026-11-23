//! Compilação do projeto aberto pela agenda (research R2, R9): o [`ProjectWorld`] atrás do
//! trait [`Compiler`].
//!
//! Cada edição leva o estado inteiro (arquivo principal e texto de todos os arquivos abertos),
//! não só a diferença: depois de uma compilação abandonada a agenda monta um World novo, e ele
//! precisa de tudo. Os textos são `Arc<str>`, então enviar o estado não copia texto; só o
//! arquivo que mudou é reaplicado ao World, o que mantém o cache incremental do Typst.
//!
//! Arquivo **não aberto** que mudou no disco (um `.bib` exportado pelo gerenciador de
//! referências, a cópia de `copy_into_project`) entra em [`DiskChanges`]: antes de compilar, o
//! World descarta o conteúdo que guardava dele e o relê pela regra de escopo (FR-025).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use tinymist_world::CompilerWorld;
use typst_layout::PagedDocument;

use super::scheduler::{Compiler, Outcome};
use crate::diagnostic::Diagnostic;
use crate::world::{PreloFeat, ProjectWorld, WorldEnv};

/// Documento compilado e o instantâneo do World que o compilou. Saltos entre prévia e código
/// (US4) resolvem os *spans* do documento nas fontes deste instantâneo, não nas de um World
/// que já viu edições mais novas.
pub struct Compiled {
    pub document: PagedDocument,
    pub world: CompilerWorld<PreloFeat>,
}

/// Estado da edição a compilar.
#[derive(Debug, Clone)]
pub struct CompileInput {
    /// Arquivo principal, relativo à raiz.
    pub main: String,
    /// Arquivos abertos no editor (caminho relativo à raiz já validado → texto não salvo).
    pub files: Vec<(String, Arc<str>)>,
}

/// Caminhos (relativos à raiz) de arquivos não abertos que mudaram no disco desde a última
/// compilação. O núcleo anota; a compilação seguinte consome. Fica fora de [`CompileInput`]
/// porque a agenda descarta entradas intermediárias, e um aviso perdido deixaria o World com o
/// conteúdo velho.
#[derive(Debug, Clone, Default)]
pub struct DiskChanges(Arc<Mutex<Vec<String>>>);

impl DiskChanges {
    pub fn push(&self, relative: impl Into<String>) {
        self.lock().push(relative.into());
    }

    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.lock())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<String>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// World de um projeto, recriado quando o arquivo principal muda.
pub struct ProjectCompiler {
    root: PathBuf,
    main: String,
    env: WorldEnv,
    world: Result<ProjectWorld, Diagnostic>,
    /// Textos já aplicados ao World como arquivos abertos.
    applied: HashMap<String, Arc<str>>,
    disk: DiskChanges,
}

impl ProjectCompiler {
    /// Sem decisões de plugin (todo plugin fica barrado) nem cache de pacotes.
    pub fn new(root: PathBuf, main: &str) -> Self {
        Self::with_env(root, main, WorldEnv::default())
    }

    /// Com decisões de plugin e cache de pacotes (o núcleo do app).
    pub fn with_env(root: PathBuf, main: &str, env: WorldEnv) -> Self {
        let world = ProjectWorld::open_with(&root, main, env.clone());
        Self {
            root,
            main: main.to_owned(),
            env,
            world,
            applied: HashMap::new(),
            disk: DiskChanges::default(),
        }
    }

    /// Liga a fila de arquivos não abertos alterados no disco ([`DiskChanges`]).
    pub fn with_disk_changes(mut self, disk: DiskChanges) -> Self {
        self.disk = disk;
        self
    }
}

impl ProjectCompiler {
    /// Raiz do projeto deste World.
    pub fn root(&self) -> &std::path::Path {
        &self.root
    }

    /// Leva o World ao estado de `input` (arquivo principal e textos abertos) sem compilar.
    /// A compilação e o serviço de linguagem (T036) partem daqui, então veem o mesmo texto.
    pub fn sync(&mut self, input: CompileInput) -> Result<&mut ProjectWorld, Diagnostic> {
        if input.main != self.main {
            let disk = self.disk.clone();
            *self = Self::with_env(self.root.clone(), &input.main, self.env.clone())
                .with_disk_changes(disk);
        }
        let changed = self.disk.take();
        let world = match &mut self.world {
            Ok(world) => world,
            Err(diag) => return Err(diag.clone()),
        };
        let wanted: HashMap<String, Arc<str>> = input.files.into_iter().collect();
        for path in changed.iter().filter(|p| !wanted.contains_key(*p)) {
            world.reload_from_disk(path);
        }
        self.applied.retain(|path, _| {
            let keep = wanted.contains_key(path);
            if !keep {
                world.close_file(path);
            }
            keep
        });
        for (path, text) in wanted {
            let same = self
                .applied
                .get(&path)
                .is_some_and(|old| Arc::ptr_eq(old, &text) || **old == *text);
            // Caminho já validado pelo núcleo; recusa aqui só deixa o arquivo como no disco.
            if !same && world.set_open_file(&path, &text).is_ok() {
                self.applied.insert(path, text);
            }
        }
        Ok(world)
    }
}

impl Compiler for ProjectCompiler {
    type Input = CompileInput;
    type Output = Compiled;

    fn compile(&mut self, input: CompileInput) -> Outcome<Compiled> {
        match self.sync(input) {
            Ok(world) => {
                let (outcome, world) = world.compile_with_snapshot();
                Outcome {
                    output: outcome.output.map(|document| Compiled { document, world }),
                    diagnostics: outcome.diagnostics,
                }
            }
            Err(diag) => Outcome {
                output: None,
                diagnostics: vec![diag],
            },
        }
    }
}
