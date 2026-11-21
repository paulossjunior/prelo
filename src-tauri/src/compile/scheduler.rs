//! Agenda de compilação (specs/003-editor-local/research.md R9; data-model.md "Compilation").
//!
//! Uma thread de compilação alimentada por fila de tamanho 1: a edição mais nova substitui a
//! pendente. Cada compilação tem limite de tempo (10 s em produção); estourado, ela vira
//! `Status::Abandonada` com diagnóstico, a thread presa é largada (o Typst não tem
//! cancelamento) e a próxima edição compila numa thread nova com um World novo. Resultado de
//! compilação mais antiga que a última entregue é descartado (FR-015).

use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use crate::diagnostic::{Diagnostic, Kind};

/// Limite de tempo de uma compilação (FR-015, research R9).
pub const COMPILE_TIMEOUT: Duration = Duration::from_secs(10);

/// O que a agenda compila. Em produção, o World do `tinymist-world`; nos testes, um falso.
pub trait Compiler: Send + 'static {
    /// Estado da edição a compilar.
    type Input: Send + 'static;
    /// Resultado bem-sucedido (páginas).
    type Output: Send + 'static;

    fn compile(&mut self, input: Self::Input) -> Outcome<Self::Output>;
}

/// Resposta do compilador: `output` presente quando compilou; diagnósticos em qualquer caso.
#[derive(Debug)]
pub struct Outcome<O> {
    pub output: Option<O>,
    pub diagnostics: Vec<Diagnostic>,
}

/// `Compilation.status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok,
    ComErros,
    Abandonada,
}

/// `Compilation` entregue à interface.
#[derive(Debug)]
pub struct Compilation<O> {
    /// Crescente; atribuído quando a edição entra na fila.
    pub compile_id: u64,
    pub status: Status,
    /// Só em `Status::Ok`.
    pub output: Option<O>,
    pub diagnostics: Vec<Diagnostic>,
    pub duration_ms: u64,
}

/// Diagnóstico de compilação abandonada por tempo.
pub fn timeout_diagnostic(limit: Duration) -> Diagnostic {
    let _ = limit;
    Diagnostic::error(Kind::Typst, String::new())
}

/// Agenda de compilação. Ao ser largada, as threads ociosas terminam; uma thread presa numa
/// compilação termina quando a compilação terminar, e o resultado é descartado.
pub struct Scheduler<C: Compiler> {
    _marker: std::marker::PhantomData<C>,
}

impl<C: Compiler> Scheduler<C> {
    /// Cria a agenda com o limite de produção. `new_compiler` monta um World novo; é chamado
    /// na criação e a cada compilação abandonada.
    pub fn new<F>(new_compiler: F) -> (Self, Receiver<Compilation<C::Output>>)
    where
        F: Fn() -> C + Send + Sync + 'static,
    {
        Self::with_timeout(new_compiler, COMPILE_TIMEOUT)
    }

    /// Como [`Scheduler::new`], com outro limite de tempo (testes).
    pub fn with_timeout<F>(
        new_compiler: F,
        timeout: Duration,
    ) -> (Self, Receiver<Compilation<C::Output>>)
    where
        F: Fn() -> C + Send + Sync + 'static,
    {
        let _ = (new_compiler, timeout);
        let (_tx, rx) = mpsc::channel();
        (
            Self {
                _marker: std::marker::PhantomData,
            },
            rx,
        )
    }

    /// Põe uma edição na fila, substituindo a pendente, e devolve o `compileId` dela. Não
    /// bloqueia, mesmo com uma compilação em andamento ou presa.
    pub fn submit(&self, input: C::Input) -> u64 {
        let _ = input;
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Severity;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Condvar, Mutex};
    use std::thread::{self, ThreadId};
    use std::time::Instant;

    /// Folga para esperar um evento que deve acontecer.
    const WAIT: Duration = Duration::from_secs(5);
    /// Janela em que um evento que não deve acontecer é esperado.
    const QUIET: Duration = Duration::from_millis(300);
    /// Limite curto para os testes de abandono.
    const SHORT: Duration = Duration::from_millis(200);

    /// Portão que segura uma compilação falsa até ser aberto.
    #[derive(Clone, Default)]
    struct Gate(Arc<(Mutex<bool>, Condvar)>);

    impl Gate {
        fn open(&self) {
            *self.0 .0.lock().unwrap() = true;
            self.0 .1.notify_all();
        }
        fn wait(&self) {
            let mut open = self.0 .0.lock().unwrap();
            while !*open {
                open = self.0 .1.wait(open).unwrap();
            }
        }
    }

    /// Edição falsa: `tag` identifica a edição; `gate` segura a compilação; `fail` devolve
    /// erro de compilação.
    struct Edit {
        tag: &'static str,
        gate: Option<Gate>,
        fail: bool,
    }

    fn quick(tag: &'static str) -> Edit {
        Edit {
            tag,
            gate: None,
            fail: false,
        }
    }

    fn held(tag: &'static str, gate: &Gate) -> Edit {
        Edit {
            tag,
            gate: Some(gate.clone()),
            fail: false,
        }
    }

    #[derive(Debug)]
    struct Done {
        tag: &'static str,
        world: usize,
        thread: ThreadId,
    }

    /// World falso: avisa quando começa a compilar e guarda qual instância compilou.
    struct FakeWorld {
        id: usize,
        started: Arc<Mutex<mpsc::Sender<(&'static str, ThreadId)>>>,
    }

    impl Compiler for FakeWorld {
        type Input = Edit;
        type Output = Done;

        fn compile(&mut self, input: Edit) -> Outcome<Done> {
            let thread = thread::current().id();
            let _ = self.started.lock().unwrap().send((input.tag, thread));
            if let Some(gate) = &input.gate {
                gate.wait();
            }
            if input.fail {
                return Outcome {
                    output: None,
                    diagnostics: vec![Diagnostic::error(Kind::Typst, "erro falso")],
                };
            }
            Outcome {
                output: Some(Done {
                    tag: input.tag,
                    world: self.id,
                    thread,
                }),
                diagnostics: Vec::new(),
            }
        }
    }

    struct Harness {
        scheduler: Scheduler<FakeWorld>,
        results: Receiver<Compilation<Done>>,
        started: Receiver<(&'static str, ThreadId)>,
        worlds: Arc<AtomicUsize>,
    }

    impl Harness {
        fn new(timeout: Duration) -> Self {
            let (tx, started) = mpsc::channel();
            let tx = Arc::new(Mutex::new(tx));
            let worlds = Arc::new(AtomicUsize::new(0));
            let counter = Arc::clone(&worlds);
            let (scheduler, results) = Scheduler::with_timeout(
                move || FakeWorld {
                    id: counter.fetch_add(1, Ordering::SeqCst),
                    started: Arc::clone(&tx),
                },
                timeout,
            );
            Self {
                scheduler,
                results,
                started,
                worlds,
            }
        }

        fn next(&self) -> Compilation<Done> {
            self.results
                .recv_timeout(WAIT)
                .expect("nenhuma compilação entregue no prazo")
        }

        fn wait_started(&self, tag: &str) -> ThreadId {
            let (got, thread) = self
                .started
                .recv_timeout(WAIT)
                .expect("compilação não começou no prazo");
            assert_eq!(got, tag, "começou outra edição");
            thread
        }

        fn assert_quiet(&self) {
            if let Ok(extra) = self.results.recv_timeout(QUIET) {
                panic!("entrega inesperada: {extra:?}");
            }
        }
    }

    #[test]
    fn production_timeout_is_ten_seconds() {
        assert_eq!(COMPILE_TIMEOUT, Duration::from_secs(10));
        let diag = timeout_diagnostic(COMPILE_TIMEOUT);
        assert_eq!(diag.message, "compilação excedeu 10 s");
        assert_eq!(diag.severity, Severity::Error);
    }

    #[test]
    fn compiles_an_edit_and_delivers_ok() {
        let h = Harness::new(WAIT);
        let id = h.scheduler.submit(quick("a"));
        let c = h.next();
        assert_eq!(c.compile_id, id);
        assert_eq!(c.status, Status::Ok);
        assert_eq!(c.output.expect("saída").tag, "a");
        assert!(c.diagnostics.is_empty());
    }

    #[test]
    fn failed_compile_is_com_erros_without_output() {
        let h = Harness::new(WAIT);
        let id = h.scheduler.submit(Edit {
            tag: "ruim",
            gate: None,
            fail: true,
        });
        let c = h.next();
        assert_eq!(c.compile_id, id);
        assert_eq!(c.status, Status::ComErros);
        assert!(c.output.is_none());
        assert_eq!(c.diagnostics.len(), 1);
        assert_eq!(c.diagnostics[0].message, "erro falso");
    }

    #[test]
    fn compile_ids_increase() {
        let h = Harness::new(WAIT);
        let a = h.scheduler.submit(quick("a"));
        assert_eq!(h.next().compile_id, a);
        let b = h.scheduler.submit(quick("b"));
        assert!(b > a, "compileId deve crescer: {a} depois {b}");
        assert_eq!(h.next().compile_id, b);
    }

    /// Fila de 1: com uma compilação em andamento, a edição nova substitui a pendente, e
    /// `submit` não espera a compilação.
    #[test]
    fn newer_edit_replaces_pending_one() {
        let h = Harness::new(WAIT);
        let gate = Gate::default();
        let first = h.scheduler.submit(held("a", &gate));
        h.wait_started("a");

        let before = Instant::now();
        let replaced = h.scheduler.submit(quick("b"));
        let latest = h.scheduler.submit(quick("c"));
        assert!(before.elapsed() < Duration::from_secs(1), "submit bloqueou");
        assert!(first < replaced && replaced < latest);

        gate.open();
        let c = h.next();
        assert_eq!((c.compile_id, c.output.unwrap().tag), (first, "a"));
        let c = h.next();
        assert_eq!(c.status, Status::Ok);
        assert_eq!((c.compile_id, c.output.unwrap().tag), (latest, "c"));
        h.assert_quiet();
        assert!(
            h.started.try_recv().is_err(),
            "a edição substituída não pode compilar"
        );
    }

    /// Compilação acima do limite vira `Abandonada` com diagnóstico, sem esperar o compilador.
    #[test]
    fn compile_over_timeout_is_abandoned_with_diagnostic() {
        let h = Harness::new(SHORT);
        let gate = Gate::default();
        let id = h.scheduler.submit(held("preso", &gate));
        h.wait_started("preso");

        let c = h.next();
        assert_eq!(c.compile_id, id);
        assert_eq!(c.status, Status::Abandonada);
        assert!(c.output.is_none());
        assert_eq!(c.diagnostics.len(), 1);
        assert_eq!(c.diagnostics[0].severity, Severity::Error);
        assert!(
            c.diagnostics[0].message.starts_with("compilação excedeu"),
            "{}",
            c.diagnostics[0].message
        );
        assert!(c.duration_ms >= SHORT.as_millis() as u64);
        gate.open();
    }

    /// Depois do abandono, a próxima edição compila numa thread nova com World novo, enquanto
    /// a thread antiga continua presa.
    #[test]
    fn next_edit_after_abandon_compiles_on_new_thread() {
        let h = Harness::new(SHORT);
        let gate = Gate::default();
        h.scheduler.submit(held("preso", &gate));
        let stuck_thread = h.wait_started("preso");
        assert_eq!(h.next().status, Status::Abandonada);

        let id = h.scheduler.submit(quick("seguinte"));
        let c = h.next();
        assert_eq!(c.compile_id, id);
        assert_eq!(c.status, Status::Ok);
        let done = c.output.unwrap();
        assert_eq!(done.tag, "seguinte");
        assert_ne!(done.thread, stuck_thread, "compilou na thread presa");
        assert_eq!(done.world, 1, "deve usar um World novo");
        assert_eq!(h.worlds.load(Ordering::SeqCst), 2);
        gate.open();
    }

    /// Edição que chegou enquanto a compilação estava presa compila logo após o abandono.
    #[test]
    fn pending_edit_compiles_after_abandon() {
        let h = Harness::new(SHORT);
        let gate = Gate::default();
        h.scheduler.submit(held("preso", &gate));
        let stuck_thread = h.wait_started("preso");
        let pending = h.scheduler.submit(quick("pendente"));

        assert_eq!(h.next().status, Status::Abandonada);
        let c = h.next();
        assert_eq!(c.compile_id, pending);
        let done = c.output.unwrap();
        assert_eq!(done.tag, "pendente");
        assert_ne!(done.thread, stuck_thread);
        gate.open();
    }

    /// O resultado da compilação abandonada, quando enfim chega, é mais antigo que o último
    /// entregue e é descartado.
    #[test]
    fn result_older_than_last_delivered_is_discarded() {
        let h = Harness::new(SHORT);
        let gate = Gate::default();
        let old = h.scheduler.submit(held("antigo", &gate));
        h.wait_started("antigo");
        let c = h.next();
        assert_eq!((c.compile_id, c.status), (old, Status::Abandonada));

        let new = h.scheduler.submit(quick("novo"));
        let c = h.next();
        assert_eq!((c.compile_id, c.status), (new, Status::Ok));

        gate.open();
        h.assert_quiet();
    }

    /// Mesmo sem compilação mais nova, a compilação já entregue como abandonada não é
    /// entregue de novo quando termina.
    #[test]
    fn abandoned_result_is_never_delivered_twice() {
        let h = Harness::new(SHORT);
        let gate = Gate::default();
        h.scheduler.submit(held("antigo", &gate));
        h.wait_started("antigo");
        assert_eq!(h.next().status, Status::Abandonada);
        gate.open();
        h.assert_quiet();
    }
}
