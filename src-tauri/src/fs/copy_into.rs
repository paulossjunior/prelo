//! Cópia de arquivo de fora para dentro do projeto (D-13, FR-023; contracts/ipc.md
//! "Arquivo externo, plugins e pacotes"; T059).
//!
//! O Prelo não lê nada fora da raiz. Quando a compilação recusa um caminho que, como foi
//! escrito no `.typ`, sai da raiz por `..` (`#bibliography("../comum/refs.bib")`), o núcleo
//! oferece trazer o arquivo para o projeto:
//!
//! 1. [`CopyOffers`] guarda, a cada compilação, os caminhos pedidos que saem da raiz e os
//!    `.typ` que os pedem, e avisa a interface uma vez por caminho
//!    ([`CopyIntoProjectOffer`], só o caminho pedido, nunca o real);
//! 2. a pessoa escolhe o arquivo no diálogo nativo do núcleo (a webview nunca manda o caminho
//!    de origem);
//! 3. [`read_source`] lê a origem escolhida (arquivo comum, até 256 MB) e [`write_new`] grava
//!    a cópia na pasta do `.typ` que a pediu, em criação exclusiva pela regra de escopo
//!    ([`crate::fs::create`]): nunca sobrescreve;
//! 4. [`literal_changes`] troca, no `.typ`, o literal pedido pelo caminho da cópia.
//!
//! Link simbólico dentro do projeto que aponta para fora não gera oferta: o caminho pedido já
//! fica dentro da raiz, e a cópia cairia no lugar do link.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::Path;

use serde::Serialize;
use typst::syntax::ast::Str;
use typst::syntax::{LinkedNode, SyntaxKind};

use super::create::{self, NewItem};
use super::save::Change;
use super::scope::{Refusal, Scope, MAX_FILE_BYTES};
use crate::compile::scheduler::{Compilation, Status};
use crate::diagnostic::{Diagnostic, Kind};

/// Nome do evento da oferta.
pub const COPY_OFFER: &str = "copy_into_project_offer";

pub const NOT_OFFERED: &str = "not_offered";
const INVALID_NAME: &str = "invalid_name";
const NOT_FOUND: &str = "not_found";
const TOO_LARGE: &str = "too_large";
const WRITE_FAILED: &str = "write_failed";

/// Evento `copy_into_project_offer`: só o caminho como foi escrito no `.typ`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CopyIntoProjectOffer {
    pub requested: String,
}

/// Arquivo cujo texto o núcleo mudou para apontar para a cópia, com a versão nova.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdatedFile {
    pub path: String,
    pub version: u64,
}

/// Resposta de `copy_into_project`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CopiedIntoProject {
    /// A cópia, relativa à raiz.
    pub path: String,
    pub updated: Vec<UpdatedFile>,
}

/// Ofertas do projeto aberto.
#[derive(Debug, Default)]
pub struct CopyOffers {
    /// Geração da prévia (projeto aberto); compilação de outra geração é ignorada.
    generation: u64,
    /// Caminho pedido → `.typ` que o pedem (relativos à raiz), da última compilação.
    current: BTreeMap<String, BTreeSet<String>>,
    /// Caminhos já avisados à interface neste projeto.
    announced: HashSet<String>,
}

impl CopyOffers {
    /// Projeto novo (ou compilação religada): nenhuma oferta.
    pub fn reset(&mut self, generation: u64) {
        *self = Self {
            generation,
            ..Self::default()
        };
    }

    /// Ofertas de uma compilação que terminou; devolve as que ainda não foram avisadas.
    /// Compilação abandonada não diz nada sobre os caminhos e mantém as ofertas.
    pub fn update<O>(
        &mut self,
        generation: u64,
        compilation: &Compilation<O>,
    ) -> Vec<CopyIntoProjectOffer> {
        if generation != self.generation || compilation.status == Status::Abandonada {
            return Vec::new();
        }
        self.current = offers_in(&compilation.diagnostics);
        self.current
            .keys()
            .filter(|requested| self.announced.insert((*requested).clone()))
            .map(|requested| CopyIntoProjectOffer {
                requested: requested.clone(),
            })
            .collect()
    }

    /// `.typ` que pedem `requested`, se é uma oferta em aberto.
    pub fn referrers(&self, requested: &str) -> Option<&BTreeSet<String>> {
        self.current.get(requested)
    }

    /// Oferta atendida.
    pub fn remove(&mut self, requested: &str) {
        self.current.remove(requested);
    }

    /// Caminhos com oferta em aberto, em ordem.
    pub fn list(&self) -> Vec<String> {
        self.current.keys().cloned().collect()
    }
}

/// Caminhos recusados por ficarem fora do projeto que, como escritos, saem da raiz por `..`,
/// com os `.typ` que os pedem. Diagnósticos de arquivo de pacote (`@preview/…`) ficam fora.
pub fn offers_in(diagnostics: &[Diagnostic]) -> BTreeMap<String, BTreeSet<String>> {
    let mut offers: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for diag in diagnostics {
        if diag.kind != Kind::Access {
            continue;
        }
        let Some(referrer) = diag.path.as_deref().filter(|p| !p.starts_with('@')) else {
            continue;
        };
        let Some(requested) = outside_requested(&diag.message) else {
            continue;
        };
        if escapes_root(referrer, requested) {
            offers
                .entry(requested.to_owned())
                .or_default()
                .insert(referrer.to_owned());
        }
    }
    offers
}

/// Caminho citado na mensagem de `ForaDoProjeto` (contracts/file-access.md).
fn outside_requested(message: &str) -> Option<&str> {
    let probe = Refusal::OutsideProject {
        requested: String::new(),
    }
    .to_string();
    let (prefix, suffix) = probe.split_once("``")?;
    let prefix = format!("{prefix}`");
    let suffix = format!("`{suffix}");
    message
        .strip_prefix(prefix.as_str())?
        .strip_suffix(suffix.as_str())
        .filter(|r| !r.is_empty())
}

/// `requested`, relativo à pasta de `referrer` (ou à raiz com `/` inicial), sai da raiz por
/// `..`.
pub fn escapes_root(referrer: &str, requested: &str) -> bool {
    let mut depth: Vec<&str> = if requested.starts_with('/') {
        Vec::new()
    } else {
        let mut dir: Vec<&str> = referrer.split('/').filter(|s| !s.is_empty()).collect();
        dir.pop();
        dir
    };
    for part in requested.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => {
                if depth.pop().is_none() {
                    return true;
                }
            }
            other => depth.push(other),
        }
    }
    false
}

/// Pasta (relativa à raiz, `""` na raiz) de um caminho relativo com `/`.
pub fn parent_dir(relative: &str) -> &str {
    relative.rsplit_once('/').map_or("", |(dir, _)| dir)
}

/// Onde a cópia de `requested` fica: na pasta do primeiro `.typ` que o pede, com o nome do
/// último componente. Nome fora das regras de FR-035 → `invalid_name`.
pub fn destination(referrers: &BTreeSet<String>, requested: &str) -> Result<String, &'static str> {
    let name = requested
        .rsplit(['/', '\\'])
        .next()
        .filter(|n| create::is_valid_name(n))
        .ok_or(INVALID_NAME)?;
    let dir = referrers.iter().next().map_or("", |r| parent_dir(r));
    Ok(if dir.is_empty() {
        name.to_owned()
    } else {
        format!("{dir}/{name}")
    })
}

/// Lê a origem escolhida pela pessoa no diálogo nativo: arquivo comum (link seguido, como no
/// diálogo), até 256 MB. Pasta, FIFO ou dispositivo → `not_found`.
pub fn read_source(source: &Path) -> Result<Vec<u8>, &'static str> {
    let meta = std::fs::metadata(source).map_err(|_| NOT_FOUND)?;
    if !meta.is_file() {
        return Err(NOT_FOUND);
    }
    if meta.len() > MAX_FILE_BYTES {
        return Err(TOO_LARGE);
    }
    let file = std::fs::File::open(source).map_err(|_| NOT_FOUND)?;
    // Confere no descritor aberto: o caminho pode ter virado outra coisa depois do `metadata`.
    if !file.metadata().map_err(|_| NOT_FOUND)?.is_file() {
        return Err(NOT_FOUND);
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| NOT_FOUND)?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(TOO_LARGE);
    }
    Ok(bytes)
}

/// Grava `bytes` num arquivo novo em `relative` (criação exclusiva pela regra de escopo; nunca
/// sobrescreve). Devolve o caminho normalizado. Falha depois de criar apaga o arquivo vazio.
pub fn write_new(scope: &Scope, relative: &str, bytes: &[u8]) -> Result<String, &'static str> {
    let created = create::create(scope, relative, NewItem::File).map_err(|e| e.code())?;
    let written = scope
        .resolve(&created)
        .map_err(|_| WRITE_FAILED)
        .and_then(|path| {
            // O criado ainda precisa estar dentro da raiz (troca por link entre criar e abrir).
            if !path.starts_with(scope.root()) {
                return Err(WRITE_FAILED);
            }
            let mut file = OpenOptions::new()
                .write(true)
                .open(&path)
                .map_err(|_| WRITE_FAILED)?;
            file.write_all(bytes).map_err(|_| WRITE_FAILED)?;
            file.sync_all().map_err(|_| WRITE_FAILED)?;
            Ok(())
        });
    if let Err(code) = written {
        if let Ok(path) = scope.resolve(&created) {
            let _ = std::fs::remove_file(path);
        }
        return Err(code);
    }
    Ok(created)
}

/// Literal que o `.typ` em `referrer` passa a usar para chegar à cópia `created`: só o nome se
/// estão na mesma pasta; senão o caminho a partir da raiz (`/…`).
pub fn literal_for(referrer: &str, created: &str) -> String {
    if parent_dir(referrer) == parent_dir(created) {
        created.rsplit('/').next().unwrap_or(created).to_owned()
    } else {
        format!("/{created}")
    }
}

/// Mudanças (em unidades UTF-16, em ordem) que trocam cada literal de string com valor
/// `requested` em `text` (código Typst) por `"replacement"`.
pub fn literal_changes(text: &str, requested: &str, replacement: &str) -> Vec<Change> {
    let root = typst::syntax::parse(text);
    let mut ranges = Vec::new();
    collect_literals(&LinkedNode::new(&root), requested, &mut ranges);
    ranges.sort_by_key(|r: &std::ops::Range<usize>| r.start);
    let units = |byte: usize| text[..byte].encode_utf16().count();
    ranges
        .into_iter()
        .map(|r| Change {
            from: units(r.start),
            to: units(r.end),
            insert: format!("\"{}\"", escape(replacement)),
        })
        .collect()
}

fn collect_literals(node: &LinkedNode<'_>, requested: &str, out: &mut Vec<std::ops::Range<usize>>) {
    if node.kind() == SyntaxKind::Str {
        if node
            .cast::<Str>()
            .is_some_and(|s| s.get().as_str() == requested)
        {
            out.push(node.range());
        }
        return;
    }
    for child in node.children() {
        collect_literals(&child, requested, out);
    }
}

/// Escapa `\` e `"` para um literal de string do Typst.
fn escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn access(path: &str, requested: &str) -> Diagnostic {
        let mut d = Diagnostic::error(
            Kind::Access,
            Refusal::OutsideProject {
                requested: requested.into(),
            }
            .to_string(),
        );
        d.path = Some(path.into());
        d
    }

    #[test]
    fn so_caminho_que_sai_da_raiz_por_ponto_ponto_e_oferecido() {
        let offers = offers_in(&[
            access("main.typ", "../comum/refs.bib"),
            access("cap/intro.typ", "../../comum/refs.bib"),
            access("cap/intro.typ", "../refs.bib"),
            access("main.typ", "link.bib"),
            access("@preview/x:1.0.0/lib.typ", "../../y.bib"),
            Diagnostic::error(
                Kind::Typst,
                "acesso recusado: `../a.bib` fica fora da pasta do projeto",
            ),
        ]);
        let keys: Vec<_> = offers.keys().cloned().collect();
        assert_eq!(keys, vec!["../../comum/refs.bib", "../comum/refs.bib"]);
        assert_eq!(
            offers["../comum/refs.bib"].iter().collect::<Vec<_>>(),
            vec!["main.typ"]
        );
    }

    #[test]
    fn escapa_da_raiz() {
        assert!(escapes_root("main.typ", "../x.bib"));
        assert!(escapes_root("a/b.typ", "../../x.bib"));
        assert!(!escapes_root("a/b.typ", "../x.bib"));
        assert!(escapes_root("a/b.typ", "/../x.bib"));
        assert!(!escapes_root("main.typ", "/x.bib"));
        assert!(!escapes_root("main.typ", "a/../x.bib"));
        assert!(escapes_root("main.typ", "a/../../x.bib"));
    }

    #[test]
    fn destino_na_pasta_do_typ_com_nome_valido() {
        let refs: BTreeSet<String> = ["cap/intro.typ".to_owned()].into();
        assert_eq!(
            destination(&refs, "../../c/refs.bib"),
            Ok("cap/refs.bib".into())
        );
        let root: BTreeSet<String> = ["main.typ".to_owned()].into();
        assert_eq!(destination(&root, "../c/refs.bib"), Ok("refs.bib".into()));
        assert_eq!(destination(&root, "../c/.oculto"), Err("invalid_name"));
        assert_eq!(destination(&root, "../c/.."), Err("invalid_name"));
        assert_eq!(destination(&root, "../c/"), Err("invalid_name"));
        assert_eq!(destination(&root, "../c/CON.bib"), Err("invalid_name"));
    }

    #[test]
    fn literal_relativo_ou_pela_raiz() {
        assert_eq!(literal_for("main.typ", "refs.bib"), "refs.bib");
        assert_eq!(literal_for("cap/a.typ", "cap/refs.bib"), "refs.bib");
        assert_eq!(literal_for("outro/b.typ", "cap/refs.bib"), "/cap/refs.bib");
    }

    #[test]
    fn troca_so_literais_com_o_valor_pedido_em_utf16() {
        let text = "= Introdução\n#bibliography(\"../c/r.bib\")\n// \"../c/r.bib\" em comentário\n#let x = \"../c/r.bib\"\n#let y = \"../c/r.bibx\"\n";
        let changes = literal_changes(text, "../c/r.bib", "r.bib");
        assert_eq!(changes.len(), 2);
        let applied = crate::fs::save::apply_changes(text, &changes).unwrap();
        assert!(applied.contains("#bibliography(\"r.bib\")"));
        assert!(applied.contains("#let x = \"r.bib\""));
        assert!(applied.contains("// \"../c/r.bib\" em comentário"));
        assert!(applied.contains("\"../c/r.bibx\""));
    }

    #[test]
    fn oferta_avisada_uma_vez_e_ignorada_em_outra_geracao() {
        let mut offers = CopyOffers::default();
        offers.reset(2);
        let compilation = |status| Compilation::<()> {
            compile_id: 1,
            status,
            output: None,
            diagnostics: vec![access("main.typ", "../r.bib")],
            duration_ms: 0,
        };
        assert_eq!(offers.update(1, &compilation(Status::ComErros)), vec![]);
        assert!(offers.list().is_empty());
        let first = offers.update(2, &compilation(Status::ComErros));
        assert_eq!(
            first,
            vec![CopyIntoProjectOffer {
                requested: "../r.bib".into()
            }]
        );
        assert!(offers.update(2, &compilation(Status::ComErros)).is_empty());
        assert_eq!(offers.list(), vec!["../r.bib"]);
        let fixed = Compilation::<()> {
            compile_id: 2,
            status: Status::Ok,
            output: Some(()),
            diagnostics: vec![],
            duration_ms: 0,
        };
        let abandoned = Compilation::<()> {
            status: Status::Abandonada,
            ..compilation(Status::Abandonada)
        };
        offers.update(2, &abandoned);
        assert_eq!(offers.list(), vec!["../r.bib"], "abandonada não muda nada");
        offers.update(2, &fixed);
        assert!(offers.list().is_empty());
    }
}
