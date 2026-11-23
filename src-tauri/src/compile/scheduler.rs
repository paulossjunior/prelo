//! Agenda de compilação (specs/003-editor-local/research.md R9; data-model.md "Compilation").
//!
//! Uma thread de compilação alimentada por fila de tamanho 1: a edição mais nova substitui a
//! pendente. Cada compilação tem limite de tempo (10 s em produção); estourado, ela vira
//! `Status::Abandonada` com diagnóstico, a thread presa é largada (o Typst não tem
//! cancelamento) e a próxima edição compila numa thread nova com um World novo. Resultado de
//! compilação mais antiga que a última entregue é descartado (FR-015).
//!
//! Threads abandonadas (research R9, T078): uma thread presa só termina quando a compilação
//! dela terminar, e um plugin aprovado que não retorna nunca termina. Para que edições
//! seguidas sobre um projeto assim não acumulem threads (cada uma ocupando um núcleo), a agenda
//! mantém no máximo [`MAX_COMPILE_THREADS`] threads de compilação vivas, contando as presas.
//! Atingido o limite, a agenda fica suspensa: cada edição é respondida na hora com um
//! diagnóstico ([`suspended_diagnostic`]) e a mais nova fica pendente; quando uma thread presa
//! termina, uma thread nova compila essa edição.

use std::panic::{self, AssertUnwindSafe};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use crate::diagnostic::{Diagnostic, Kind};

/// Limite de tempo de uma compilação (FR-015, research R9).
pub const COMPILE_TIMEOUT: Duration = Duration::from_secs(10);

/// Máximo de threads de compilação vivas por agenda, contando as abandonadas que seguem presas
/// (research R9, T078): uma ativa e até duas presas.
pub const MAX_COMPILE_THREADS: usize = 3;

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
    let limit = if limit.subsec_millis() == 0 {
        format!("{} s", limit.as_secs())
    } else {
        format!("{} ms", limit.as_millis())
    };
    let mut diag = Diagnostic::error(Kind::Typst, format!("compilação excedeu {limit}"));
    diag.hints
        .push("a prévia volta a compilar na próxima edição".to_owned());
    diag
}

/// Diagnóstico de compilação interrompida por erro interno (pânico) do compilador.
pub fn panic_diagnostic() -> Diagnostic {
    let mut diag = Diagnostic::error(
        Kind::Typst,
        "erro interno do compilador ao processar o projeto",
    );
    diag.hints.push(
        "a prévia volta a compilar na próxima edição; se o erro se repetir, desfaça a última \
         alteração"
            .to_owned(),
    );
    diag
}

/// Diagnóstico de edição que não compilou porque a agenda está suspensa: `stuck` compilações
/// abandonadas ainda não terminaram.
pub fn suspended_diagnostic(stuck: usize) -> Diagnostic {
    let mut diag = Diagnostic::error(
        Kind::Typst,
        format!(
            "compilação suspensa: {stuck} compilações anteriores excederam o limite de tempo e \
             ainda não terminaram"
        ),
    );
    diag.hints.push(
        "a prévia volta a compilar quando uma delas terminar; se o projeto usa um plugin que \
         não retorna, desfaça a chamada e reabra o Prelo"
            .to_owned(),
    );
    diag
}

/// Compilação em andamento, vista pelo vigia.
struct Running {
    id: u64,
    started: Instant,
}

/// Estado compartilhado entre `submit`, a thread de compilação e o vigia.
struct State<I, O> {
    /// Fila de tamanho 1.
    pending: Option<(u64, I)>,
    running: Option<Running>,
    /// Último `compileId` atribuído.
    last_id: u64,
    /// Último `compileId` entregue; resultado com id igual ou menor é descartado.
    last_delivered: u64,
    /// Geração da thread de compilação válida; muda a cada abandono.
    generation: u64,
    /// Threads de compilação vivas (a ativa e as abandonadas que seguem presas).
    threads: usize,
    /// Sem thread ativa: o limite de threads foi atingido e todas estão presas.
    suspended: bool,
    shutdown: bool,
    tx: Sender<Compilation<O>>,
}

impl<I, O> State<I, O> {
    /// Entrega se `compile_id` for mais novo que o último entregue; senão descarta.
    fn deliver(&mut self, compilation: Compilation<O>) {
        if compilation.compile_id > self.last_delivered {
            self.last_delivered = compilation.compile_id;
            // Receptor largado: ninguém mais quer resultados; ignorar.
            let _ = self.tx.send(compilation);
        }
    }
}

type NewCompiler<C> = Box<dyn Fn() -> C + Send + Sync>;

struct Shared<C: Compiler> {
    state: Mutex<State<C::Input, C::Output>>,
    /// Acorda a thread de compilação (edição nova, encerramento).
    work: Condvar,
    /// Acorda o vigia (compilação começou ou terminou, encerramento).
    watch: Condvar,
    new_compiler: NewCompiler<C>,
    timeout: Duration,
}

impl<C: Compiler> Shared<C> {
    fn lock(&self) -> MutexGuard<'_, State<C::Input, C::Output>> {
        // Nada sob a trava entra em pânico; se entrar, o estado continua coerente.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Agenda de compilação. Ao ser largada, as threads ociosas terminam; uma thread presa numa
/// compilação termina quando a compilação terminar, e o resultado é descartado.
pub struct Scheduler<C: Compiler> {
    shared: Arc<Shared<C>>,
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
        let (tx, rx) = mpsc::channel();
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                pending: None,
                running: None,
                last_id: 0,
                last_delivered: 0,
                generation: 0,
                threads: 0,
                suspended: false,
                shutdown: false,
                tx,
            }),
            work: Condvar::new(),
            watch: Condvar::new(),
            new_compiler: Box::new(new_compiler),
            timeout,
        });
        spawn_worker(&shared, &mut shared.lock(), 0);
        let watchdog = Arc::clone(&shared);
        thread::Builder::new()
            .name("prelo-compile-watchdog".to_owned())
            .spawn(move || watch(&watchdog))
            .expect("falha ao criar a thread do vigia de compilação");
        (Self { shared }, rx)
    }

    /// Põe uma edição na fila, substituindo a pendente, e devolve o `compileId` dela. Não
    /// bloqueia, mesmo com uma compilação em andamento ou presa. Com a agenda suspensa, a
    /// edição é respondida na hora com [`suspended_diagnostic`] e fica pendente.
    pub fn submit(&self, input: C::Input) -> u64 {
        let mut state = self.shared.lock();
        state.last_id += 1;
        let id = state.last_id;
        state.pending = Some((id, input));
        if state.suspended {
            let stuck = state.threads;
            state.deliver(suspended(id, stuck));
        }
        drop(state);
        self.shared.work.notify_all();
        id
    }

    /// Compilação forçada ("Compilar", FR-014a): ainda não implementada (T101).
    pub fn compile_now(&self, input: C::Input) -> Result<u64, Suspended> {
        let _ = input;
        Err(Suspended { stuck: 0 })
    }

    /// Threads de compilação vivas: a ativa e as abandonadas que seguem presas. Nunca passa de
    /// [`MAX_COMPILE_THREADS`].
    pub fn compile_threads(&self) -> usize {
        self.shared.lock().threads
    }
}

/// `compile_now` recusado: a agenda está suspensa, com `stuck` threads presas (o limite).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Suspended {
    pub stuck: usize,
}

/// Resposta a uma edição com a agenda suspensa.
fn suspended<O>(compile_id: u64, stuck: usize) -> Compilation<O> {
    Compilation {
        compile_id,
        status: Status::ComErros,
        output: None,
        diagnostics: vec![suspended_diagnostic(stuck)],
        duration_ms: 0,
    }
}

impl<C: Compiler> Drop for Scheduler<C> {
    fn drop(&mut self) {
        self.shared.lock().shutdown = true;
        self.shared.work.notify_all();
        self.shared.watch.notify_all();
    }
}

/// Cria a thread de compilação da geração `generation`, com um World novo. `state` é o estado
/// já travado de `shared`, onde a thread entra na conta.
fn spawn_worker<C: Compiler>(
    shared: &Arc<Shared<C>>,
    state: &mut State<C::Input, C::Output>,
    generation: u64,
) {
    state.threads += 1;
    let shared = Arc::clone(shared);
    thread::Builder::new()
        .name(format!("prelo-compile-{generation}"))
        .spawn(move || {
            let exit = Exit(Arc::clone(&shared));
            let compiler = (shared.new_compiler)();
            work(&shared, compiler, generation);
            drop(exit);
        })
        .expect("falha ao criar a thread de compilação");
}

/// Saída de uma thread de compilação (normal ou por pânico): sai da conta e, se a agenda estava
/// suspensa, põe uma thread nova para a edição pendente.
struct Exit<C: Compiler>(Arc<Shared<C>>);

impl<C: Compiler> Drop for Exit<C> {
    fn drop(&mut self) {
        let shared = &self.0;
        let mut state = shared.lock();
        state.threads -= 1;
        if state.suspended && !state.shutdown && state.threads < MAX_COMPILE_THREADS {
            state.suspended = false;
            // A edição pendente já foi respondida como suspensa com o id dela; o resultado
            // de verdade precisa de um id mais novo para ser entregue.
            if state.pending.is_some() {
                state.last_id += 1;
                let id = state.last_id;
                if let Some(job) = state.pending.as_mut() {
                    job.0 = id;
                }
            }
            let generation = state.generation;
            spawn_worker(shared, &mut state, generation);
        }
    }
}

/// Laço da thread de compilação: pega a edição pendente, compila sem a trava e entrega. Sai no
/// encerramento ou quando foi abandonada (geração velha).
fn work<C: Compiler>(shared: &Shared<C>, mut compiler: C, generation: u64) {
    loop {
        let (id, input) = {
            let mut state = shared.lock();
            loop {
                if state.shutdown || state.generation != generation {
                    return;
                }
                if let Some(job) = state.pending.take() {
                    state.running = Some(Running {
                        id: job.0,
                        started: Instant::now(),
                    });
                    break job;
                }
                state = shared
                    .work
                    .wait(state)
                    .unwrap_or_else(PoisonError::into_inner);
            }
        };
        shared.watch.notify_all();

        let started = Instant::now();
        // Pânico do compilador com entrada malformada (FR-024) vira diagnóstico na hora, sem
        // esperar o vigia. O World pode ter ficado incoerente: a próxima edição usa um novo.
        let result = panic::catch_unwind(AssertUnwindSafe(|| compiler.compile(input)));
        let duration_ms = millis(started.elapsed());
        let (outcome, broken) = match result {
            Ok(outcome) => (outcome, false),
            Err(_) => (
                Outcome {
                    output: None,
                    diagnostics: vec![panic_diagnostic()],
                },
                true,
            ),
        };

        let mut state = shared.lock();
        if state.generation != generation {
            // Abandonada pelo vigia e já entregue como `Abandonada`: descarta e termina.
            return;
        }
        state.running = None;
        let status = if outcome.output.is_some() {
            Status::Ok
        } else {
            Status::ComErros
        };
        state.deliver(Compilation {
            compile_id: id,
            status,
            output: outcome.output,
            diagnostics: outcome.diagnostics,
            duration_ms,
        });
        drop(state);
        shared.watch.notify_all();
        if broken {
            compiler = (shared.new_compiler)();
        }
    }
}

/// Vigia: abandona a compilação que passar do limite e põe uma thread nova no lugar.
fn watch<C: Compiler>(shared: &Arc<Shared<C>>) {
    let mut state = shared.lock();
    loop {
        if state.shutdown {
            return;
        }
        let Some(running) = &state.running else {
            state = shared
                .watch
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
            continue;
        };
        let elapsed = running.started.elapsed();
        if elapsed < shared.timeout {
            state = shared
                .watch
                .wait_timeout(state, shared.timeout - elapsed)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
            continue;
        }
        let id = running.id;
        state.running = None;
        state.generation += 1;
        let generation = state.generation;
        state.deliver(Compilation {
            compile_id: id,
            status: Status::Abandonada,
            output: None,
            diagnostics: vec![timeout_diagnostic(shared.timeout)],
            duration_ms: millis(elapsed),
        });
        // A thread presa continua na conta até terminar (`Exit`).
        if state.threads < MAX_COMPILE_THREADS {
            spawn_worker(shared, &mut state, generation);
        } else {
            state.suspended = true;
            let stuck = state.threads;
            if let Some(pending) = state.pending.as_ref().map(|job| job.0) {
                state.deliver(suspended(pending, stuck));
            }
        }
    }
}

fn millis(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Severity;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread::ThreadId;

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

    /// Edição que faz o compilador falso entrar em pânico.
    const PANIC: &str = "pânico";

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
            if input.tag == PANIC {
                panic!("pânico falso do compilador");
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
        // Só "a" e "c" começaram; "b" nunca chegou ao compilador.
        h.wait_started("c");
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

    /// Prende `MAX_COMPILE_THREADS` compilações seguidas, cada uma abandonada por tempo.
    fn stick_all(h: &Harness) -> Vec<Gate> {
        let mut gates = Vec::new();
        for i in 0..MAX_COMPILE_THREADS {
            let gate = Gate::default();
            h.scheduler.submit(held("preso", &gate));
            h.wait_started("preso");
            assert_eq!(h.next().status, Status::Abandonada, "abandono {i}");
            assert!(h.scheduler.compile_threads() <= MAX_COMPILE_THREADS);
            gates.push(gate);
        }
        gates
    }

    /// T078: compilações abandonadas em sequência não acumulam threads além do limite.
    #[test]
    fn abandoned_threads_stay_bounded() {
        let h = Harness::new(SHORT);
        let gates = stick_all(&h);
        assert_eq!(h.scheduler.compile_threads(), MAX_COMPILE_THREADS);
        // Mais edições não criam threads novas: cada uma é respondida na hora como suspensa.
        for _ in 0..3 {
            let started = Instant::now();
            let id = h.scheduler.submit(quick("suspensa"));
            let c = h.next();
            assert!(started.elapsed() < Duration::from_secs(1));
            assert_eq!((c.compile_id, c.status), (id, Status::ComErros));
            assert!(
                c.diagnostics[0].message.starts_with("compilação suspensa"),
                "{:?}",
                c.diagnostics
            );
        }
        h.assert_quiet();
        assert_eq!(h.scheduler.compile_threads(), MAX_COMPILE_THREADS);
        assert_eq!(h.worlds.load(Ordering::SeqCst), MAX_COMPILE_THREADS);
        for gate in gates {
            gate.open();
        }
    }

    /// Quando uma thread presa termina, a edição pendente compila numa thread nova, com um id
    /// mais novo que o da resposta "suspensa".
    #[test]
    fn suspended_edit_compiles_when_a_stuck_thread_ends() {
        let h = Harness::new(SHORT);
        let gates = stick_all(&h);
        let suspended = h.scheduler.submit(quick("depois"));
        assert_eq!(h.next().status, Status::ComErros);

        gates[0].open();
        let c = h.next();
        assert_eq!(c.status, Status::Ok, "{:?}", c.diagnostics);
        assert!(c.compile_id > suspended);
        assert_eq!(c.output.unwrap().tag, "depois");
        assert!(h.scheduler.compile_threads() <= MAX_COMPILE_THREADS);
        h.assert_quiet();

        // De volta ao normal: a próxima edição compila.
        let id = h.scheduler.submit(quick("normal"));
        let c = h.next();
        assert_eq!((c.compile_id, c.status), (id, Status::Ok));
        for gate in &gates[1..] {
            gate.open();
        }
    }

    /// Edição que estava pendente quando o limite foi atingido é respondida como suspensa.
    #[test]
    fn pending_edit_is_answered_when_suspension_starts() {
        let h = Harness::new(SHORT);
        let mut gates = Vec::new();
        for _ in 0..MAX_COMPILE_THREADS - 1 {
            let gate = Gate::default();
            h.scheduler.submit(held("preso", &gate));
            h.wait_started("preso");
            assert_eq!(h.next().status, Status::Abandonada);
            gates.push(gate);
        }
        let gate = Gate::default();
        h.scheduler.submit(held("preso", &gate));
        h.wait_started("preso");
        let pending = h.scheduler.submit(quick("pendente"));
        assert_eq!(h.next().status, Status::Abandonada);
        let c = h.next();
        assert_eq!((c.compile_id, c.status), (pending, Status::ComErros));
        gates.push(gate);
        for gate in gates {
            gate.open();
        }
    }

    /// Threads presas saem da conta quando terminam.
    #[test]
    fn threads_leave_the_count_when_they_end() {
        let h = Harness::new(SHORT);
        let gates = stick_all(&h);
        for gate in &gates {
            gate.open();
        }
        // As presas terminam; sobra só a thread ativa criada quando a suspensão acabou.
        let deadline = Instant::now() + WAIT;
        while h.scheduler.compile_threads() > 1 {
            assert!(
                Instant::now() < deadline,
                "threads presas não saíram da conta"
            );
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(h.scheduler.compile_threads(), 1);
    }

    /// FR-014a: sem compilação em andamento, `compile_now` só põe na fila; nada é abandonado e
    /// o World continua o mesmo (cache incremental mantido).
    #[test]
    fn compile_now_when_idle_compiles_on_the_same_world() {
        let h = Harness::new(WAIT);
        let id = h.scheduler.submit(quick("a"));
        assert_eq!(h.next().compile_id, id);

        let id = h.scheduler.compile_now(quick("forçada")).expect("na fila");
        let c = h.next();
        assert_eq!((c.compile_id, c.status), (id, Status::Ok));
        assert_eq!(c.output.unwrap().world, 0, "não deve trocar de World");
        assert_eq!(h.scheduler.compile_threads(), 1);
        h.assert_quiet();
    }

    /// FR-014a: com uma compilação presa (ainda abaixo do limite de tempo), `compile_now` a
    /// abandona na hora e compila numa thread nova, sem esperar o vigia; o resultado da presa,
    /// quando chega, é descartado, e o total de threads fica no limite (T078).
    #[test]
    fn compile_now_abandons_a_stuck_compile_and_compiles_on_a_new_thread() {
        let h = Harness::new(WAIT);
        let gate = Gate::default();
        let stuck = h.scheduler.submit(held("preso", &gate));
        let stuck_thread = h.wait_started("preso");

        let started = Instant::now();
        let id = h.scheduler.compile_now(quick("forçada")).expect("na fila");
        assert!(id > stuck);
        let c = h.next();
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "esperou o vigia"
        );
        assert_eq!((c.compile_id, c.status), (id, Status::Ok));
        let done = c.output.unwrap();
        assert_eq!(done.tag, "forçada");
        assert_ne!(done.thread, stuck_thread, "compilou na thread presa");
        assert_eq!(h.scheduler.compile_threads(), 2);
        assert!(h.scheduler.compile_threads() <= MAX_COMPILE_THREADS);

        gate.open();
        h.assert_quiet();
    }

    /// FR-014a: com o limite de threads atingido por compilações presas, `compile_now` é recusado
    /// sem criar thread nem World; a edição fica pendente e compila quando uma presa termina.
    #[test]
    fn compile_now_at_the_thread_limit_is_refused_without_a_new_thread() {
        let h = Harness::new(SHORT);
        let gates = stick_all(&h);
        assert_eq!(
            h.scheduler.compile_now(quick("forçada")),
            Err(Suspended {
                stuck: MAX_COMPILE_THREADS
            })
        );
        h.assert_quiet();
        assert_eq!(h.scheduler.compile_threads(), MAX_COMPILE_THREADS);
        assert_eq!(h.worlds.load(Ordering::SeqCst), MAX_COMPILE_THREADS);

        gates[0].open();
        let c = h.next();
        assert_eq!(c.status, Status::Ok, "{:?}", c.diagnostics);
        assert_eq!(c.output.unwrap().tag, "forçada");
        assert!(h.scheduler.compile_threads() <= MAX_COMPILE_THREADS);
        for gate in &gates[1..] {
            gate.open();
        }
    }

    /// FR-014a: `compile_now` com o limite já ocupado por presas e uma ativa não cria thread: a
    /// edição espera na fila de 1 como qualquer outra.
    #[test]
    fn compile_now_never_spawns_past_the_limit() {
        let h = Harness::new(WAIT);
        let mut gates = Vec::new();
        for _ in 0..MAX_COMPILE_THREADS {
            let gate = Gate::default();
            h.scheduler
                .compile_now(held("preso", &gate))
                .expect("dentro do limite");
            h.wait_started("preso");
            gates.push(gate);
            assert!(h.scheduler.compile_threads() <= MAX_COMPILE_THREADS);
        }
        assert_eq!(h.scheduler.compile_threads(), MAX_COMPILE_THREADS);
        let id = h
            .scheduler
            .compile_now(quick("forçada"))
            .expect("fica na fila");
        assert_eq!(h.scheduler.compile_threads(), MAX_COMPILE_THREADS);
        assert_eq!(h.worlds.load(Ordering::SeqCst), MAX_COMPILE_THREADS);
        // A ativa (a última presa) termina e a forçada compila em seguida.
        gates.last().unwrap().open();
        let c = loop {
            let c = h.next();
            if c.compile_id == id {
                break c;
            }
        };
        assert_eq!(c.status, Status::Ok);
        assert_eq!(c.output.unwrap().tag, "forçada");
        for gate in &gates {
            gate.open();
        }
    }

    /// Pânico do compilador (erro interno do Typst com entrada malformada) vira diagnóstico na
    /// hora, sem esperar o limite de tempo, e a próxima edição compila num World novo.
    #[test]
    fn compiler_panic_becomes_a_diagnostic_and_the_next_edit_compiles() {
        let h = Harness::new(WAIT);
        let started = Instant::now();
        let id = h.scheduler.submit(quick(PANIC));
        let c = h.next();
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "esperou o limite"
        );
        assert_eq!((c.compile_id, c.status), (id, Status::ComErros));
        let diag = &c.diagnostics[0];
        assert_eq!(diag.kind, Kind::Typst);
        assert_eq!(diag.severity, Severity::Error);
        assert!(diag.message.contains("erro interno"), "{diag:?}");

        let id = h.scheduler.submit(quick("depois"));
        let c = h.next();
        assert_eq!((c.compile_id, c.status), (id, Status::Ok));
        assert_eq!(c.output.expect("saída").world, 1, "World não foi refeito");
        h.assert_quiet();
    }
}
