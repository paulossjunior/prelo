//! Desempenho da prévia (T075; SC-001, SC-002; research R13 "Desempenho").
//!
//! Mede, pela mesma cadeia do app (agenda de produção + [`ProjectCompiler`] + [`PreviewHub`]),
//! o tempo de cada edição até a prévia estar pronta para mostrar a mudança:
//!
//! - **edição → `PreviewUpdate`**: `submit` da edição até o evento `Ok` dela chegar ao assinante;
//! - **edição → SVG pronto**: o mesmo, mais o SVG de cada página em `changed` que a interface
//!   pediria com `page_svg`: a prévia é virtualizada e só pede as páginas visíveis ± 2 (research
//!   R4). Com a edição na página 1 e duas páginas na tela, são as páginas 1 a 4
//!   ([`VISIBLE`]). É o número que vale para SC-001/SC-002. O tempo para gerar o SVG de todas as
//!   páginas alteradas também é impresso, só como referência.
//!
//! Dois casos: a fixture `artigo-40p` (40 páginas, texto e matemática) e a mesma fixture com
//! duas imagens geradas no teste de 4000×3000 px e mais de 10 MB cada (ruído PNG; nada com
//! direito autoral, nada versionado), uma delas na página editada (SC-002: a edição de texto não
//! toca a imagem, mas a página que muda a contém).
//!
//! Só roda com a feature `perf` e em `--release` (em depuração o Typst é dezenas de vezes mais
//! lento e o número não diz nada): job "perf" do CI, `cargo test --release --features perf
//! --test perf_preview -- --nocapture`. A suíte normal (`npm run verify`) não compila este
//! arquivo. Limite: 250 ms de p95 no CI (guarda de regressão; `PRELO_PERF_LIMIT_MS` muda),
//! meta de 100 ms no computador de referência (research.md, "Medições de desempenho").
#![cfg(feature = "perf")]

mod support;

use std::io::Write;
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};

use flate2::write::ZlibEncoder;
use flate2::{Compression, Crc};
use prelo_lib::compile::preview::{PreviewHub, PreviewUpdate};
use prelo_lib::compile::project::{CompileInput, ProjectCompiler};
use prelo_lib::compile::scheduler::Scheduler;
use support::TempProject;

/// Edições medidas em cada caso (depois do aquecimento).
const EDITS: usize = 60;
/// Edições de aquecimento, fora da conta (cache incremental e imagens decodificadas).
const WARMUP: usize = 5;
/// Guarda de regressão do CI (research R13).
const CI_LIMIT_MS: u64 = 250;
/// Prazo de cada evento; passar dele é falha, não medição.
const WAIT: Duration = Duration::from_secs(60);
/// Páginas cujo SVG a interface pede depois de uma edição na página 1: as duas visíveis ± 2.
const VISIBLE: std::ops::Range<usize> = 0..4;
/// Parágrafo da fixture onde as edições acontecem (página 1).
const ANCHOR: &str = "Parágrafo editável usado pela medição de desempenho da prévia.";
/// Tamanho das imagens grandes (spec, "Imagem grande de referência").
const BIG_W: u32 = 4000;
const BIG_H: u32 = 3000;
const BIG_MIN_BYTES: usize = 10 * 1024 * 1024;

fn limit() -> Duration {
    let ms = std::env::var("PRELO_PERF_LIMIT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(CI_LIMIT_MS);
    Duration::from_millis(ms)
}

/// PNG RGB de ruído pseudoaleatório (determinístico pela `seed`), sem compressão útil: o
/// arquivo fica perto dos 36 MB brutos.
fn noise_png(width: u32, height: u32, seed: u64) -> Vec<u8> {
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut crc = Crc::new();
        crc.update(kind);
        crc.update(data);
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        out.extend_from_slice(&crc.sum().to_be_bytes());
    }
    let mut state = seed | 1;
    let row = width as usize * 3 + 1;
    let mut raw = Vec::with_capacity(row * height as usize);
    for y in 0..height as usize {
        // Cada linha começa com o byte do filtro "None" (0).
        raw.resize(y * row + 1, 0);
        for _ in 0..width {
            // xorshift64*
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            let v = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
            raw.extend_from_slice(&v.to_le_bytes()[..3]);
        }
    }
    let mut z = ZlibEncoder::new(Vec::new(), Compression::fast());
    z.write_all(&raw).unwrap();
    let idat = z.finish().unwrap();

    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // 8 bits, RGB
    chunk(&mut png, b"IHDR", &ihdr);
    chunk(&mut png, b"IDAT", &idat);
    chunk(&mut png, b"IEND", &[]);
    png
}

/// Projeto aberto na cadeia de produção da prévia.
struct Live {
    _project: TempProject,
    main: String,
    scheduler: Scheduler<ProjectCompiler>,
    hub: Arc<PreviewHub>,
    updates: Receiver<PreviewUpdate>,
}

impl Live {
    fn open(project: TempProject) -> Self {
        let root = project.root();
        let main = std::fs::read_to_string(root.join("main.typ")).unwrap();
        let hub = Arc::new(PreviewHub::default());
        let generation = hub.reset();
        let (tx, updates) = mpsc::channel();
        hub.subscribe(Arc::new(move |u: &PreviewUpdate| {
            let _ = tx.send(u.clone());
        }));
        let (scheduler, compilations) =
            Scheduler::new(move || ProjectCompiler::new(root.clone(), "main.typ"));
        hub.spawn_pump(generation, compilations);
        Self {
            _project: project,
            main,
            scheduler,
            hub,
            updates,
        }
    }

    fn submit(&self, text: &str) -> u64 {
        self.scheduler.submit(CompileInput {
            main: "main.typ".into(),
            files: vec![("main.typ".into(), Arc::from(text))],
        })
    }

    fn wait(&self, id: u64) -> PreviewUpdate {
        loop {
            let u = self
                .updates
                .recv_timeout(WAIT)
                .unwrap_or_else(|_| panic!("compilação {id} não chegou em {WAIT:?}"));
            if u.compile_id >= id {
                assert!(u.ok, "compilação {id} falhou: {:?}", u.diagnostics);
                return u;
            }
        }
    }

    /// SVG de cada página em `changed` dentro de `pages`, como a interface pede.
    fn render_changed(&self, update: &PreviewUpdate, pages: std::ops::Range<usize>) -> usize {
        let mut bytes = 0;
        for &index in update.changed.iter().filter(|i| pages.contains(i)) {
            let hash = &update.pages[index].hash;
            bytes += self.hub.page_svg(index, hash).expect("page_svg").len();
        }
        bytes
    }

    /// Texto com a `n`-ésima edição: o parágrafo âncora ganha `n` palavras.
    fn edited(&self, n: usize) -> String {
        let words: String = (0..n).map(|i| format!(" edição{i}")).collect();
        self.main.replacen(ANCHOR, &format!("{ANCHOR}{words}"), 1)
    }
}

struct Report {
    update: Vec<Duration>,
    svg: Vec<Duration>,
    /// SVG de todas as páginas alteradas, não só as visíveis (referência).
    all_svg: Vec<Duration>,
    pages: usize,
    max_changed: usize,
}

fn p(samples: &[Duration], q: f64) -> Duration {
    let mut sorted = samples.to_vec();
    sorted.sort();
    let rank = ((q * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len());
    sorted[rank - 1]
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn measure(live: &Live) -> Report {
    assert!(live.main.contains(ANCHOR), "âncora ausente do main.typ");
    let first = live.wait(live.submit(&live.main));
    let pages = first.pages.len();
    live.render_changed(&first, 0..pages);

    let mut update = Vec::with_capacity(EDITS);
    let mut svg = Vec::with_capacity(EDITS);
    let mut all_svg = Vec::with_capacity(EDITS);
    let mut max_changed = 0;
    for n in 1..=WARMUP + EDITS {
        let text = live.edited(n);
        let started = Instant::now();
        let id = live.submit(&text);
        let u = live.wait(id);
        let got_update = started.elapsed();
        assert!(!u.changed.is_empty(), "edição {n} não mudou nenhuma página");
        assert!(u.changed.contains(&0), "a página editada não mudou");
        live.render_changed(&u, VISIBLE);
        let got_svg = started.elapsed();
        // Referência: o resto das páginas alteradas, como se todas estivessem na tela.
        live.render_changed(&u, VISIBLE.end..u.pages.len());
        let got_all = started.elapsed();
        if n > WARMUP {
            update.push(got_update);
            svg.push(got_svg);
            all_svg.push(got_all);
            max_changed = max_changed.max(u.changed.len());
        }
    }
    Report {
        update,
        svg,
        all_svg,
        pages,
        max_changed,
    }
}

fn check(case: &str, report: &Report) {
    let limit = limit();
    let p95_update = p(&report.update, 0.95);
    let p95_svg = p(&report.svg, 0.95);
    println!(
        "perf_preview [{case}]: {} páginas, {EDITS} edições, até {} página(s) em `changed`; \
         edição→PreviewUpdate p50 {:.1} ms, p95 {:.1} ms, máx {:.1} ms; \
         edição→SVG pronto (páginas visíveis) p50 {:.1} ms, p95 {:.1} ms, máx {:.1} ms \
         (limite {} ms); SVG de todas as alteradas p95 {:.1} ms",
        report.pages,
        report.max_changed,
        ms(p(&report.update, 0.5)),
        ms(p95_update),
        ms(p(&report.update, 1.0)),
        ms(p(&report.svg, 0.5)),
        ms(p95_svg),
        ms(p(&report.svg, 1.0)),
        limit.as_millis(),
        ms(p(&report.all_svg, 0.95)),
    );
    assert!(
        report.pages >= 40,
        "a fixture deveria ter 40 páginas: {}",
        report.pages
    );
    assert!(
        p95_svg <= limit,
        "[{case}] p95 edição→SVG pronto {:.1} ms acima de {} ms (SC-001/SC-002)",
        ms(p95_svg),
        limit.as_millis()
    );
}

#[test]
fn p95_artigo_40_paginas() {
    let live = Live::open(TempProject::from_fixture("artigo-40p"));
    let report = measure(&live);
    check("artigo-40p", &report);
}

#[test]
fn p95_artigo_40_paginas_com_imagens_grandes() {
    let project = TempProject::from_fixture("artigo-40p");
    for (i, seed) in [(1, 0x5EED_0001_u64), (2, 0x5EED_0002)] {
        let png = noise_png(BIG_W, BIG_H, seed);
        assert!(
            png.len() >= BIG_MIN_BYTES,
            "imagem pequena: {} bytes",
            png.len()
        );
        project.write(&format!("grande-{i}.png"), &png);
    }
    let main = std::fs::read_to_string(project.path("main.typ")).unwrap();
    // Uma na página editada (logo depois do parágrafo âncora) e outra no fim do artigo.
    let main = main
        .replacen(
            ANCHOR,
            &format!("{ANCHOR}\n\n#image(\"grande-1.png\", width: 60%)\n"),
            1,
        )
        .replace(
            "#corpo(secoes: 29)",
            "#corpo(secoes: 29)\n\n#image(\"grande-2.png\", width: 60%)",
        );
    project.write("main.typ", &main);
    let live = Live::open(project);
    let report = measure(&live);
    check("artigo-40p + imagens 4000×3000", &report);
}
