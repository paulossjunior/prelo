//! Prévia (specs/003-editor-local/research.md R4; contracts/ipc.md "Compilação e prévia";
//! data-model.md "Compilation").
//!
//! Cada compilação entregue pela agenda vira um [`PreviewUpdate`]: número, tamanho e hash de
//! todas as páginas, e em `changed` só as páginas novas ou alteradas desde a última `Ok`. A
//! interface pede o SVG de cada página por `page_svg(index, hash)`; hash que não é o da página
//! atual responde `stale_page`. Compilação com erro ou abandonada mantém as páginas da última
//! `Ok` (FR-014) e só traz diagnósticos.

use std::sync::Arc;

use serde::Serialize;
use typst_layout::{Page, PagedDocument};

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

/// Hash de uma página, em hexadecimal: muda quando qualquer coisa desenhada nela muda.
pub fn page_hash(_page: &Page) -> String {
    todo!("T035")
}

/// Estado da prévia: o último documento `Ok` e as páginas dele.
#[derive(Default)]
pub struct Preview {
    document: Option<Arc<PagedDocument>>,
    pages: Vec<PageInfo>,
}

impl Preview {
    /// Aplica uma compilação entregue pela agenda e devolve o evento para a interface.
    pub fn apply(&mut self, _compilation: Compilation<PagedDocument>) -> PreviewUpdate {
        let _ = (&self.document, &self.pages, Status::Ok);
        todo!("T035")
    }

    /// SVG da página `index`, se `hash` for o atual; senão `stale_page`.
    pub fn page_svg(&self, _index: usize, _hash: &str) -> Result<String, &'static str> {
        todo!("T035")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Kind;
    use crate::world::ProjectWorld;

    /// Três páginas pequenas; `b` é o texto da segunda.
    fn three_pages(b: &str) -> String {
        format!(
            "#set page(width: 100pt, height: 80pt)\nA\n#pagebreak()\n{b}\n#pagebreak()\nC\n"
        )
    }

    fn compile(text: &str) -> PagedDocument {
        let dir = tempfile::tempdir().expect("pasta temporária");
        std::fs::write(dir.path().join("main.typ"), text).unwrap();
        let mut world = ProjectWorld::open(dir.path(), "main.typ").expect("World");
        let outcome = world.compile();
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
        let fewer = preview.apply(ok(
            5,
            compile("#set page(width: 100pt, height: 80pt)\nA\n"),
        ));
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
            ["changed", "compileId", "diagnostics", "durationMs", "ok", "pages"]
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
}
