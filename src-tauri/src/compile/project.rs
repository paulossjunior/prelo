//! Compilação do projeto aberto pela agenda (research R2, R9): o [`ProjectWorld`] atrás do
//! trait [`Compiler`].
//!
//! Cada edição leva o estado inteiro (arquivo principal e texto de todos os arquivos abertos),
//! não só a diferença: depois de uma compilação abandonada a agenda monta um World novo, e ele
//! precisa de tudo. Os textos são `Arc<str>`, então enviar o estado não copia texto; só o
//! arquivo que mudou é reaplicado ao World, o que mantém o cache incremental do Typst.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use typst_layout::PagedDocument;

use super::scheduler::{Compiler, Outcome};
use crate::diagnostic::Diagnostic;
use crate::world::ProjectWorld;

/// Estado da edição a compilar.
#[derive(Debug, Clone)]
pub struct CompileInput {
    /// Arquivo principal, relativo à raiz.
    pub main: String,
    /// Arquivos abertos no editor (caminho relativo à raiz já validado → texto não salvo).
    pub files: Vec<(String, Arc<str>)>,
}

/// World de um projeto, recriado quando o arquivo principal muda.
pub struct ProjectCompiler {
    root: PathBuf,
    main: String,
    world: Result<ProjectWorld, Diagnostic>,
    /// Textos já aplicados ao World como arquivos abertos.
    applied: HashMap<String, Arc<str>>,
}

impl ProjectCompiler {
    pub fn new(root: PathBuf, main: &str) -> Self {
        let world = ProjectWorld::open(&root, main);
        Self {
            root,
            main: main.to_owned(),
            world,
            applied: HashMap::new(),
        }
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
            *self = Self::new(self.root.clone(), &input.main);
        }
        let world = match &mut self.world {
            Ok(world) => world,
            Err(diag) => return Err(diag.clone()),
        };
        let wanted: HashMap<String, Arc<str>> = input.files.into_iter().collect();
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
    type Output = PagedDocument;

    fn compile(&mut self, input: CompileInput) -> Outcome<PagedDocument> {
        match self.sync(input) {
            Ok(world) => world.compile(),
            Err(diag) => Outcome {
                output: None,
                diagnostics: vec![diag],
            },
        }
    }
}
