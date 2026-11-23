//! Prévia (specs/003-editor-local/research.md R4; contracts/ipc.md "Compilação e prévia";
//! data-model.md "Compilation").
//!
//! Cada compilação entregue pela agenda vira um [`PreviewUpdate`]: número, tamanho e hash de
//! todas as páginas, e em `changed` só as páginas novas ou alteradas desde a última `Ok`. A
//! interface pede o SVG de cada página por `page_svg(index, hash)`; hash que não é o da página
//! atual responde `stale_page`. Compilação com erro ou abandonada mantém as páginas da última
//! `Ok` (FR-014) e só traz diagnósticos.
//!
//! [`PreviewHub`] liga a agenda à interface: guarda a prévia do projeto aberto, entrega cada
//! evento ao assinante (o `tauri::ipc::Channel` de `subscribe_preview`) e descarta o que vier
//! da agenda de um projeto já fechado.
//!
//! Imagens (T058; RF-51, research R4): o `typst-svg` embute cada imagem como `data:` URL. Para
//! não reenviar uma imagem grande a cada edição na mesma página, o SVG de `page_svg` cita cada
//! uma por `prelo-asset://<sha256>` (sha256 dos bytes servidos). O conteúdo de cada hash sai
//! pelo protocolo `prelo-asset` ([`asset_response`]) e por `preview_asset` (como `data:` URL,
//! que a interface devolve ao SVG: um SVG mostrado como `<img>` não carrega recurso de fora).
//! Só respondem os hashes de imagens do documento atual cujo SVG já foi pedido; o resto é 404.

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;

use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::http::{header, Method, Request, Response, StatusCode};
use typst::layout::{Frame, FrameItem};
use typst::visualize::Image;
use typst_layout::{Page, PagedDocument};
use typst_svg::{SvgOptions, WebImage};

use super::scheduler::{Compilation, Status};
use crate::diagnostic::Diagnostic;

/// Erro de `page_svg` com hash que não é o da página atual (ou página inexistente).
pub const STALE_PAGE: &str = "stale_page";

/// `{ index, hash, widthPt, heightPt }` de cada página.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub index: usize,
    pub hash: String,
    pub width_pt: f64,
    pub height_pt: f64,
}

/// Evento `PreviewUpdate` (contracts/ipc.md).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewUpdate {
    pub compile_id: u64,
    pub ok: bool,
    pub pages: Vec<PageInfo>,
    pub changed: Vec<usize>,
    pub diagnostics: Vec<Diagnostic>,
    pub duration_ms: u64,
}

/// Hash de uma página, em hexadecimal: muda quando qualquer coisa desenhada nela muda. Os
/// quadros do Typst guardam o próprio hash (`LazyHash`), então o custo é pequeno.
pub fn page_hash(page: &Page) -> String {
    format!("{:032x}", typst::utils::hash128(page))
}

/// SVG de uma página (`typst-svg`). Imagens vêm embutidas como `data:` URL; a interface mostra
/// o SVG como `<img>`, nunca como marcação no DOM (research R4).
pub fn render_svg(page: &Page) -> String {
    typst_svg::svg(page, &SvgOptions::default())
}

/// Última compilação, para exportar: `compileId` e documento se foi `Ok`, senão os
/// diagnósticos dela.
pub type LastCompilation = Result<(u64, Arc<PagedDocument>), Vec<Diagnostic>>;

/// Estado da prévia: o último documento `Ok` e as páginas dele.
#[derive(Default)]
pub struct Preview {
    document: Option<Arc<PagedDocument>>,
    pages: Vec<PageInfo>,
    /// Último evento entregue, para quem assinar depois.
    last: Option<PreviewUpdate>,
    /// Imagens do documento atual servidas por hash.
    assets: Assets,
}

/// Imagem servida por `prelo-asset`: o conteúdo como o `typst-svg` o embutiria.
struct Asset {
    sha256: String,
    image: WebImage,
}

impl Asset {
    fn mime(&self) -> &'static str {
        self.image.format.mime()
    }
}

/// Imagens que a prévia pode servir: só as do documento `Ok` atual (`live`, pelo hash do
/// Typst de cada imagem) cujo SVG já foi pedido.
#[derive(Default)]
struct Assets {
    live: HashSet<u128>,
    by_image: HashMap<u128, Arc<Asset>>,
    by_sha: HashMap<String, Arc<Asset>>,
}

impl Assets {
    /// Documento novo: esquece as imagens que saíram dele.
    fn track(&mut self, doc: &PagedDocument) {
        self.live.clear();
        for page in doc.pages() {
            for_each_image(&page.frame, &mut |image| {
                self.live.insert(typst::utils::hash128(image));
            });
        }
        let live = &self.live;
        self.by_image.retain(|key, _| live.contains(key));
        self.by_sha = self
            .by_image
            .values()
            .map(|asset| (asset.sha256.clone(), Arc::clone(asset)))
            .collect();
    }

    /// Guarda a imagem `key` se ela ainda está no documento atual.
    fn insert(&mut self, key: u128, asset: &Arc<Asset>) {
        if self.live.contains(&key) {
            self.by_image.insert(key, Arc::clone(asset));
            self.by_sha
                .insert(asset.sha256.clone(), Arc::clone(asset));
        }
    }
}

/// Chama `f` para cada imagem do quadro, inclusive dentro de grupos.
fn for_each_image(frame: &Frame, f: &mut impl FnMut(&Image)) {
    for (_, item) in frame.items() {
        match item {
            FrameItem::Group(group) => for_each_image(&group.frame, f),
            FrameItem::Image(image, _, _) => f(image),
            _ => {}
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}

/// `sha256` em hexadecimal minúsculo (64 dígitos)? Qualquer outra coisa é recusada sem
/// consultar nada.
fn is_sha256(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

impl Preview {
    /// Aplica uma compilação entregue pela agenda e devolve o evento para a interface.
    pub fn apply(&mut self, compilation: Compilation<PagedDocument>) -> PreviewUpdate {
        let Compilation {
            compile_id,
            status,
            output,
            diagnostics,
            duration_ms,
        } = compilation;
        let (ok, changed) = match output {
            Some(doc) if status == Status::Ok => {
                let pages: Vec<PageInfo> = doc
                    .pages()
                    .iter()
                    .enumerate()
                    .map(|(index, page)| PageInfo {
                        index,
                        hash: page_hash(page),
                        width_pt: page.frame.width().to_pt(),
                        height_pt: page.frame.height().to_pt(),
                    })
                    .collect();
                let changed = pages
                    .iter()
                    .filter(|p| self.pages.get(p.index).is_none_or(|old| old.hash != p.hash))
                    .map(|p| p.index)
                    .collect();
                self.pages = pages;
                self.assets.track(&doc);
                self.document = Some(Arc::new(doc));
                (true, changed)
            }
            // `ComErros` e `Abandonada`: mantém as páginas da última `Ok` (FR-014).
            _ => (false, Vec::new()),
        };
        let update = PreviewUpdate {
            compile_id,
            ok,
            pages: self.pages.clone(),
            changed,
            diagnostics,
            duration_ms,
        };
        self.last = Some(update.clone());
        update
    }

    /// Último documento compilado com sucesso.
    pub fn document(&self) -> Option<&Arc<PagedDocument>> {
        self.document.as_ref()
    }

    /// O que a exportação usa (FR-011, FR-029): a última compilação, se foi `Ok`, com o
    /// `compileId` e o documento dela; senão os diagnósticos da última compilação. Sem nenhuma
    /// compilação entregue ainda, `None`.
    pub fn export_source(&self) -> Option<LastCompilation> {
        let last = self.last.as_ref()?;
        Some(match (&self.document, last.ok) {
            (Some(doc), true) => Ok((last.compile_id, Arc::clone(doc))),
            _ => Err(last.diagnostics.clone()),
        })
    }

    /// Evento para quem assina depois de compilar: o último, com todas as páginas em `changed`.
    pub fn replay(&self) -> Option<PreviewUpdate> {
        self.last.clone().map(|mut update| {
            update.changed = (0..update.pages.len()).collect();
            update
        })
    }

    /// Página `index`, se `hash` for o atual; senão `stale_page`. Clonar a página é barato
    /// (os quadros são compartilhados), e o SVG pode ser gerado fora de qualquer trava.
    pub fn page(&self, index: usize, hash: &str) -> Result<Page, &'static str> {
        match (self.pages.get(index), &self.document) {
            (Some(info), Some(doc)) if info.hash == hash => {
                doc.pages().get(index).cloned().ok_or(STALE_PAGE)
            }
            _ => Err(STALE_PAGE),
        }
    }

    /// SVG da página `index`, se `hash` for o atual; senão `stale_page`.
    pub fn page_svg(&self, index: usize, hash: &str) -> Result<String, &'static str> {
        self.page(index, hash).map(|page| render_svg(&page))
    }
}

/// Quem recebe cada [`PreviewUpdate`] (em produção, o canal de `subscribe_preview`).
pub type Sink = Arc<dyn Fn(&PreviewUpdate) + Send + Sync>;

#[derive(Default)]
struct HubState {
    /// Muda a cada projeto aberto; entrega de geração velha é descartada.
    generation: u64,
    preview: Preview,
    sink: Option<Sink>,
}

/// Prévia do projeto aberto e o assinante dela.
#[derive(Default)]
pub struct PreviewHub {
    state: Mutex<HubState>,
}

impl PreviewHub {
    fn lock(&self) -> MutexGuard<'_, HubState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Troca o assinante. Se já houver prévia, ele a recebe logo, com todas as páginas. A
    /// entrega acontece sob a trava, para nunca chegar fora de ordem.
    pub fn subscribe(&self, sink: Sink) {
        let mut state = self.lock();
        if let Some(update) = state.preview.replay() {
            sink(&update);
        }
        state.sink = Some(sink);
    }

    /// Começa a prévia de um projeto novo e devolve a geração dele.
    pub fn reset(&self) -> u64 {
        let mut state = self.lock();
        state.generation += 1;
        state.preview = Preview::default();
        state.generation
    }

    /// Aplica a compilação da geração `generation` e entrega o evento ao assinante.
    pub fn deliver(&self, generation: u64, compilation: Compilation<PagedDocument>) {
        let mut state = self.lock();
        if state.generation != generation {
            return;
        }
        let update = state.preview.apply(compilation);
        if let Some(sink) = &state.sink {
            sink(&update);
        }
    }

    /// Página atual `index` com o hash `hash` (ver [`Preview::page`]).
    pub fn page(&self, index: usize, hash: &str) -> Result<Page, &'static str> {
        self.lock().preview.page(index, hash)
    }

    /// SVG da página atual `index` com o hash `hash`, com cada imagem citada por
    /// `prelo-asset://<sha256>` em vez da `data:` URL. O SVG e o sha256 de uma imagem nova são
    /// calculados fora da trava; o sha256 fica guardado enquanto a imagem estiver no documento.
    pub fn page_svg(&self, index: usize, hash: &str) -> Result<String, &'static str> {
        let page = self.page(index, hash)?;
        let mut svg = render_svg(&page);
        let mut done = HashSet::new();
        let mut images = Vec::new();
        for_each_image(&page.frame, &mut |image| {
            if done.insert(typst::utils::hash128(image)) {
                images.push(image.clone());
            }
        });
        for image in images {
            let key = typst::utils::hash128(&image);
            let known = self.lock().preview.assets.by_image.get(&key).cloned();
            let asset = known.unwrap_or_else(|| {
                let image = WebImage::new(&image);
                let asset = Arc::new(Asset {
                    sha256: sha256_hex(&image.data),
                    image,
                });
                self.lock().preview.assets.insert(key, &asset);
                asset
            });
            // Memorizado pelo Typst: é a mesma `data:` URL que o `typst-svg` escreveu.
            let url = asset.image.to_base64_url();
            svg = svg.replace(url.as_str(), &format!("{ASSET_PREFIX}{}", asset.sha256));
        }
        Ok(svg)
    }

    fn asset(&self, sha256: &str) -> Option<Arc<Asset>> {
        if !is_sha256(sha256) {
            return None;
        }
        self.lock().preview.assets.by_sha.get(sha256).cloned()
    }

    /// Conteúdo `sha256` como `data:` URL, para a interface devolver ao SVG da página
    /// (`preview_asset`). `None` se o hash não é de uma imagem servida da prévia atual.
    pub fn asset_data_url(&self, sha256: &str) -> Option<String> {
        self.asset(sha256)
            .map(|asset| asset.image.to_base64_url().to_string())
    }

    /// Último documento compilado com sucesso (saltos e estrutura, fases seguintes).
    pub fn document(&self) -> Option<Arc<PagedDocument>> {
        self.lock().preview.document().cloned()
    }

    /// Ver [`Preview::export_source`].
    pub fn export_source(&self) -> Option<LastCompilation> {
        self.lock().preview.export_source()
    }

    /// Thread que leva as compilações da agenda à prévia. Termina quando a agenda é largada.
    pub fn spawn_pump(
        self: &Arc<Self>,
        generation: u64,
        compilations: Receiver<Compilation<PagedDocument>>,
    ) {
        let hub = Arc::clone(self);
        thread::Builder::new()
            .name(format!("prelo-preview-{generation}"))
            .spawn(move || {
                for compilation in compilations {
                    hub.deliver(generation, compilation);
                }
            })
            .expect("falha ao criar a thread da prévia");
    }
}

/// Nome do protocolo próprio das imagens da prévia (contracts/ipc.md).
pub const ASSET_SCHEME: &str = "prelo-asset";

/// Prefixo das referências a imagem no SVG da prévia: `prelo-asset://<sha256>`.
pub const ASSET_PREFIX: &str = "prelo-asset://";

/// CSP da resposta: se o conteúdo (um SVG, por exemplo) for aberto como documento, não roda
/// script nem carrega nada.
const ASSET_CSP: &str = "default-src 'none'; sandbox";

/// Protocolo `prelo-asset` (contracts/ipc.md): `GET` de `/<sha256>` (macOS e Linux:
/// `prelo-asset://localhost/<sha256>`; Windows: `http://prelo-asset.localhost/<sha256>`; ou o
/// próprio `prelo-asset://<sha256>`) devolve os bytes da imagem. Só hashes de imagens da prévia
/// atual; qualquer outro pedido (hash desconhecido, caminho, `..`, hexadecimal maiúsculo) é
/// 404 sem consultar nada, e o que não for leitura é 405. Nenhum caminho de arquivo é aceito.
pub fn asset_response(hub: &PreviewHub, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let status = |code: StatusCode| {
        Response::builder()
            .status(code)
            .header(header::CACHE_CONTROL, "no-store")
            .body(Vec::new())
            .unwrap_or_default()
    };
    if request.method() != Method::GET {
        return status(StatusCode::METHOD_NOT_ALLOWED);
    }
    let uri = request.uri();
    let path = uri.path().strip_prefix('/').unwrap_or(uri.path());
    let sha256 = if path.is_empty() {
        uri.host().unwrap_or_default()
    } else {
        path
    };
    let Some(asset) = hub.asset(sha256) else {
        return status(StatusCode::NOT_FOUND);
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, asset.mime())
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header(header::CONTENT_SECURITY_POLICY, ASSET_CSP)
        .header(header::CACHE_CONTROL, "no-store")
        .body(asset.image.data.to_vec())
        .unwrap_or_else(|_| status(StatusCode::NOT_FOUND))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Kind;
    use crate::world::ProjectWorld;

    /// Três páginas pequenas; `b` é o texto da segunda.
    fn three_pages(b: &str) -> String {
        format!("#set page(width: 100pt, height: 80pt)\nA\n#pagebreak()\n{b}\n#pagebreak()\nC\n")
    }

    thread_local! {
        /// Um World por teste (cada teste roda na sua thread), como no editor: o hash de uma
        /// página é estável entre compilações do mesmo World. Worlds diferentes (pastas
        /// diferentes) dão hashes diferentes, porque os quadros guardam a origem no código.
        static WORLD: std::cell::RefCell<Option<(tempfile::TempDir, ProjectWorld)>> =
            const { std::cell::RefCell::new(None) };
    }

    /// Compila `text` como `main.typ` (texto não salvo) no World do teste.
    fn compile(text: &str) -> PagedDocument {
        let outcome = WORLD.with_borrow_mut(|slot| {
            let (_, world) = slot.get_or_insert_with(|| {
                let dir = tempfile::tempdir().expect("pasta temporária");
                std::fs::write(dir.path().join("main.typ"), "").unwrap();
                std::fs::write(dir.path().join("figura.png"), PNG).unwrap();
                std::fs::write(dir.path().join("outra.webp"), OTHER_IMAGE).unwrap();
                let world = ProjectWorld::open(dir.path(), "main.typ").expect("World");
                (dir, world)
            });
            world.set_open_file("main.typ", text).expect("overlay");
            world.compile()
        });
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        outcome.output.expect("compilou")
    }

    fn ok(compile_id: u64, doc: PagedDocument) -> Compilation<PagedDocument> {
        Compilation {
            compile_id,
            status: Status::Ok,
            output: Some(doc),
            diagnostics: Vec::new(),
            duration_ms: 7,
        }
    }

    fn failed(compile_id: u64, status: Status) -> Compilation<PagedDocument> {
        Compilation {
            compile_id,
            status,
            output: None,
            diagnostics: vec![Diagnostic::error(Kind::Typst, "erro falso")],
            duration_ms: 3,
        }
    }

    fn hashes(update: &PreviewUpdate) -> Vec<String> {
        update.pages.iter().map(|p| p.hash.clone()).collect()
    }

    #[test]
    fn hash_por_pagina_e_estavel_e_distingue_paginas() {
        let a = compile(&three_pages("B"));
        let b = compile(&three_pages("B"));
        let ha: Vec<_> = a.pages().iter().map(page_hash).collect();
        let hb: Vec<_> = b.pages().iter().map(page_hash).collect();
        assert_eq!(ha, hb, "mesmo texto, mesmos hashes");
        assert_eq!(ha.len(), 3);
        assert_ne!(ha[0], ha[1]);
        assert_ne!(ha[1], ha[2]);
        for h in &ha {
            assert!(
                h.len() >= 16 && h.chars().all(|c| c.is_ascii_hexdigit()),
                "hash em hexadecimal: {h}"
            );
        }
        let c = compile(&three_pages("B mudou"));
        assert_ne!(page_hash(&c.pages()[1]), ha[1]);
        assert_eq!(page_hash(&c.pages()[0]), ha[0]);
    }

    #[test]
    fn primeira_compilacao_traz_todas_as_paginas_com_tamanho() {
        let mut preview = Preview::default();
        let update = preview.apply(ok(1, compile(&three_pages("B"))));
        assert_eq!(update.compile_id, 1);
        assert!(update.ok);
        assert_eq!(update.duration_ms, 7);
        assert_eq!(update.changed, vec![0, 1, 2]);
        assert_eq!(
            update.pages.iter().map(|p| p.index).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        for page in &update.pages {
            assert!((page.width_pt - 100.0).abs() < 0.01, "{page:?}");
            assert!((page.height_pt - 80.0).abs() < 0.01, "{page:?}");
        }
        assert!(update.diagnostics.is_empty());
    }

    #[test]
    fn changed_so_com_paginas_alteradas() {
        let mut preview = Preview::default();
        let first = preview.apply(ok(1, compile(&three_pages("B"))));
        let second = preview.apply(ok(2, compile(&three_pages("B mudou"))));
        assert!(second.ok);
        assert_eq!(second.changed, vec![1]);
        assert_eq!(second.pages[0].hash, first.pages[0].hash);
        assert_ne!(second.pages[1].hash, first.pages[1].hash);

        // Nada mudou: `changed` vazio.
        let same = preview.apply(ok(3, compile(&three_pages("B mudou"))));
        assert!(same.changed.is_empty(), "{:?}", same.changed);
        assert_eq!(hashes(&same), hashes(&second));

        // Página nova no fim entra em `changed`; página removida some de `pages`.
        let more = preview.apply(ok(
            4,
            compile(&format!("{}#pagebreak()\nD\n", three_pages("B mudou"))),
        ));
        assert_eq!(more.changed, vec![3]);
        assert_eq!(more.pages.len(), 4);
        let fewer = preview.apply(ok(5, compile("#set page(width: 100pt, height: 80pt)\nA\n")));
        assert_eq!(fewer.pages.len(), 1);
        assert!(fewer.changed.is_empty(), "{:?}", fewer.changed);
    }

    #[test]
    fn erro_mantem_as_paginas_da_ultima_ok() {
        let mut preview = Preview::default();
        let good = preview.apply(ok(1, compile(&three_pages("B"))));
        for (id, status) in [(2, Status::ComErros), (3, Status::Abandonada)] {
            let bad = preview.apply(failed(id, status));
            assert_eq!(bad.compile_id, id);
            assert!(!bad.ok);
            assert_eq!(bad.pages, good.pages, "mantém a última Ok (FR-014)");
            assert!(bad.changed.is_empty());
            assert_eq!(bad.diagnostics.len(), 1);
            assert_eq!(bad.diagnostics[0].message, "erro falso");
            let svg = preview
                .page_svg(1, &good.pages[1].hash)
                .expect("SVG da última Ok continua disponível");
            assert!(svg.contains("<svg"));
        }
        // Depois do erro, `changed` compara com a última Ok.
        let back = preview.apply(ok(4, compile(&three_pages("B"))));
        assert!(back.ok);
        assert!(back.changed.is_empty(), "{:?}", back.changed);
    }

    #[test]
    fn erro_antes_de_qualquer_ok_nao_tem_paginas() {
        let mut preview = Preview::default();
        let bad = preview.apply(failed(1, Status::ComErros));
        assert!(!bad.ok);
        assert!(bad.pages.is_empty());
        assert!(bad.changed.is_empty());
        assert_eq!(preview.page_svg(0, "00"), Err(STALE_PAGE));
    }

    #[test]
    fn page_svg_devolve_svg_da_pagina_atual() {
        let mut preview = Preview::default();
        let update = preview.apply(ok(1, compile(&three_pages("B"))));
        let svg = preview.page_svg(1, &update.pages[1].hash).expect("SVG");
        assert!(svg.starts_with("<svg") || svg.contains("<svg"), "{svg:.80}");
        assert!(svg.trim_end().ends_with("</svg>"));
        assert_ne!(
            svg,
            preview.page_svg(0, &update.pages[0].hash).unwrap(),
            "cada página tem seu SVG"
        );
    }

    #[test]
    fn page_svg_com_hash_velho_e_stale_page() {
        let mut preview = Preview::default();
        let old = preview.apply(ok(1, compile(&three_pages("B"))));
        let new = preview.apply(ok(2, compile(&three_pages("B mudou"))));
        assert_eq!(preview.page_svg(1, &old.pages[1].hash), Err(STALE_PAGE));
        assert!(preview.page_svg(1, &new.pages[1].hash).is_ok());
        // Página que não mudou continua com o mesmo hash.
        assert!(preview.page_svg(0, &old.pages[0].hash).is_ok());
        // Índice fora do documento e hash inventado.
        assert_eq!(preview.page_svg(9, &new.pages[1].hash), Err(STALE_PAGE));
        assert_eq!(preview.page_svg(0, "nao-e-hash"), Err(STALE_PAGE));
    }

    #[test]
    fn preview_update_serializa_no_formato_do_contrato() {
        let mut preview = Preview::default();
        preview.apply(ok(1, compile(&three_pages("B"))));
        let mut diag = Diagnostic::error(Kind::Typst, "unknown variable: x");
        diag.path = Some("main.typ".into());
        diag.range = Some(3..4);
        diag.line = Some(2);
        let update = preview.apply(Compilation {
            compile_id: 2,
            status: Status::ComErros,
            output: None,
            diagnostics: vec![diag],
            duration_ms: 5,
        });
        let json = serde_json::to_value(&update).unwrap();
        let mut keys: Vec<_> = json.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        assert_eq!(
            keys,
            [
                "changed",
                "compileId",
                "diagnostics",
                "durationMs",
                "ok",
                "pages"
            ]
        );
        let page = &json["pages"][0];
        let mut page_keys: Vec<_> = page.as_object().unwrap().keys().cloned().collect();
        page_keys.sort();
        assert_eq!(page_keys, ["hash", "heightPt", "index", "widthPt"]);
        assert_eq!(
            json["diagnostics"][0],
            serde_json::json!({
                "severity": "error",
                "path": "main.typ",
                "range": { "from": 3, "to": 4 },
                "line": 2,
                "message": "unknown variable: x",
                "hints": [],
                "kind": "typst"
            })
        );
        // Campos opcionais ausentes não aparecem.
        let bare = serde_json::to_value(Diagnostic::error(Kind::Access, "x")).unwrap();
        assert_eq!(
            bare,
            serde_json::json!({ "severity": "error", "message": "x", "hints": [], "kind": "access" })
        );
    }

    // ------------------------------------------------ protocolo `prelo-asset` (T055)

    /// Figuras das fixtures, gravadas na pasta do World de cada teste.
    const PNG: &[u8] = include_bytes!("../../tests/fixtures/imagens/figura.png");
    const OTHER_IMAGE: &[u8] = include_bytes!("../../tests/fixtures/imagens/figura.webp");

    fn sha256_hex(bytes: &[u8]) -> String {
        use sha2::Digest;
        sha2::Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    /// Duas páginas: a primeira com a figura e `text`, a segunda só com texto.
    fn with_image(text: &str) -> String {
        format!(
            "#set page(width: 200pt, height: 200pt)\n#image(\"figura.png\", width: 50pt)\n{text}\n#pagebreak()\nFim\n"
        )
    }

    fn request(uri: &str) -> tauri::http::Request<Vec<u8>> {
        tauri::http::Request::builder()
            .uri(uri)
            .body(Vec::new())
            .unwrap()
    }

    /// Hub com a compilação `doc` entregue; devolve o evento.
    fn hub_with(doc: PagedDocument) -> (PreviewHub, u64, PreviewUpdate) {
        let hub = PreviewHub::default();
        let generation = hub.reset();
        let (tx, rx) = std::sync::mpsc::channel();
        hub.subscribe(Arc::new(move |u: &PreviewUpdate| {
            let _ = tx.send(u.clone());
        }));
        hub.deliver(generation, ok(1, doc));
        let update = rx.try_recv().expect("evento entregue");
        (hub, generation, update)
    }

    fn current_hash(hub: &PreviewHub, index: usize) -> String {
        hub.lock().preview.pages[index].hash.clone()
    }

    fn asset_tokens(svg: &str) -> Vec<String> {
        svg.match_indices(ASSET_PREFIX)
            .map(|(i, _)| svg[i + ASSET_PREFIX.len()..][..64].to_owned())
            .collect()
    }

    #[test]
    fn data_url_de_imagem_vira_prelo_asset_com_o_sha256_dos_bytes() {
        let (hub, _, update) = hub_with(compile(&with_image("Legenda")));
        let svg = hub.page_svg(0, &update.pages[0].hash).expect("SVG");
        assert!(svg.contains("<image"), "{svg:.200}");
        assert!(!svg.contains("data:image/"), "imagem continua embutida no SVG");
        assert_eq!(asset_tokens(&svg), vec![sha256_hex(PNG)]);
        // A referência é o atributo inteiro, sem sobra do `data:`.
        assert!(svg.contains(&format!("\"{ASSET_PREFIX}{}\"", sha256_hex(PNG))));

        // Página sem figura: nenhuma referência.
        let text_only = hub.page_svg(1, &update.pages[1].hash).expect("SVG");
        assert!(asset_tokens(&text_only).is_empty());
    }

    #[test]
    fn protocolo_serve_os_bytes_do_hash_conhecido() {
        let (hub, _, update) = hub_with(compile(&with_image("Legenda")));
        hub.page_svg(0, &update.pages[0].hash).expect("SVG");
        let sha = sha256_hex(PNG);
        // macOS/Linux (`prelo-asset://localhost/<sha>`) e Windows
        // (`http://prelo-asset.localhost/<sha>`).
        for uri in [
            format!("prelo-asset://localhost/{sha}"),
            format!("http://prelo-asset.localhost/{sha}"),
        ] {
            let response = asset_response(&hub, &request(&uri));
            assert_eq!(response.status(), 200, "{uri}");
            assert_eq!(response.body().as_slice(), PNG);
            let header = |name: &str| {
                response
                    .headers()
                    .get(name)
                    .and_then(|v| v.to_str().ok())
                    .map(str::to_owned)
            };
            assert_eq!(header("content-type").as_deref(), Some("image/png"));
            assert_eq!(header("x-content-type-options").as_deref(), Some("nosniff"));
            assert!(
                header("content-security-policy").is_some_and(|c| c.contains("default-src 'none'")),
                "resposta sem CSP própria"
            );
        }
        // O mesmo conteúdo como `data:` URL, para a interface embutir no SVG da página.
        let url = hub.asset_data_url(&sha).expect("data URL");
        assert!(url.starts_with("data:image/png;base64,"), "{url:.40}");
    }

    #[test]
    fn hash_desconhecido_ou_pedido_malformado_responde_404() {
        let (hub, _, update) = hub_with(compile(&with_image("Legenda")));
        hub.page_svg(0, &update.pages[0].hash).expect("SVG");
        let sha = sha256_hex(PNG);
        let unknown = sha256_hex(b"outra coisa");
        for uri in [
            format!("prelo-asset://localhost/{unknown}"),
            // Bytes que existem no projeto mas não foram para a prévia.
            format!("prelo-asset://localhost/{}", sha256_hex(OTHER_IMAGE)),
            format!("prelo-asset://localhost/{}", sha.to_uppercase()),
            format!("prelo-asset://localhost/{}", &sha[..63]),
            format!("prelo-asset://localhost/{sha}/x"),
            format!("prelo-asset://localhost/../{sha}"),
            "prelo-asset://localhost/..%2F..%2Fetc%2Fpasswd".to_owned(),
            "prelo-asset://localhost/main.typ".to_owned(),
            "prelo-asset://localhost/".to_owned(),
        ] {
            let response = asset_response(&hub, &request(&uri));
            assert_eq!(response.status(), 404, "{uri}");
            assert!(response.body().is_empty(), "{uri}");
        }
        assert!(hub.asset_data_url(&unknown).is_none());

        // Só leitura.
        let post = tauri::http::Request::builder()
            .method("POST")
            .uri(format!("prelo-asset://localhost/{sha}"))
            .body(Vec::new())
            .unwrap();
        assert_eq!(asset_response(&hub, &post).status(), 405);
    }

    #[test]
    fn hash_so_vale_enquanto_a_imagem_esta_na_compilacao_atual() {
        let hub = PreviewHub::default();
        let generation = hub.reset();
        hub.deliver(generation, ok(1, compile(&with_image("Legenda"))));
        let page = current_hash(&hub, 0);
        hub.page_svg(0, &page).expect("SVG");
        let sha = sha256_hex(PNG);
        assert!(hub.asset_data_url(&sha).is_some());

        // Compilação nova sem a figura: o hash some.
        hub.deliver(generation, ok(2, compile(&three_pages("B"))));
        assert!(hub.asset_data_url(&sha).is_none());
        let response = asset_response(&hub, &request(&format!("prelo-asset://localhost/{sha}")));
        assert_eq!(response.status(), 404);

        // Projeto novo (reset): nada da prévia anterior.
        hub.deliver(generation, ok(3, compile(&with_image("Legenda"))));
        let page = current_hash(&hub, 0);
        hub.page_svg(0, &page).expect("SVG");
        assert!(hub.asset_data_url(&sha).is_some());
        hub.reset();
        assert!(hub.asset_data_url(&sha).is_none());
    }

    #[test]
    fn editar_texto_na_pagina_da_imagem_nao_reenvia_os_bytes() {
        let hub = PreviewHub::default();
        let generation = hub.reset();
        hub.deliver(generation, ok(1, compile(&with_image("Legenda"))));
        let first_hash = current_hash(&hub, 0);
        let first = hub.page_svg(0, &first_hash).expect("SVG");

        hub.deliver(generation, ok(2, compile(&with_image("Legenda editada"))));
        let second_hash = current_hash(&hub, 0);
        assert_ne!(second_hash, first_hash, "a página da figura mudou");
        let second = hub.page_svg(0, &second_hash).expect("SVG");

        // O SVG da página editada só cita o hash: os bytes da figura não vão de novo.
        assert_eq!(asset_tokens(&second), asset_tokens(&first));
        assert!(!second.contains("data:image/"));
        // Comparado ao SVG com a imagem embutida, fica menor pelo menos o tamanho dela.
        let inline = render_svg(&hub.page(0, &second_hash).unwrap());
        assert!(
            inline.len() >= second.len() + PNG.len(),
            "SVG leva a imagem: {} com, {} sem",
            inline.len(),
            second.len()
        );
        assert!(hub.asset_data_url(&sha256_hex(PNG)).is_some());
    }
}
