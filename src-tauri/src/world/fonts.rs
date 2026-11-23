//! Fontes do World (D-15, research R6; FR-016; contracts/file-access.md "Fontes").
//!
//! Só dois conjuntos entram na compilação: as fontes embutidas de `src-tauri/fonts/`
//! (Libertinus 7.051, OFL-1.1) e os arquivos `fonts/**/*.{ttf,otf,ttc,otc}` do projeto. Nenhuma
//! fonte do sistema é consultada: sem `fontdb`, sem varredura de pastas do sistema. Os arquivos
//! do projeto são lidos pelo [`AccessModel`] (regra de escopo, FR-021): link para fora da raiz é
//! recusado e vira diagnóstico, como qualquer outra leitura. Fonte pedida que não existe nesses
//! dois conjuntos sai na fonte padrão, com o aviso do Typst que cita o nome pedido.
//!
//! [`ProjectFonts`] confere `fonts/` a cada compilação (nome, tamanho e data de cada arquivo,
//! sem ler conteúdo) e só remonta o resolvedor quando algo mudou; arquivo novo entra na
//! compilação seguinte sem depender do observador de arquivos.
//!
//! O Typst pede "New Computer Modern Math" para matemática e "DejaVu Sans Mono" para `raw`,
//! que o Prelo não embute. Os padrões do Prelo (Libertinus Math e Libertinus Mono) entram como
//! regras `show … : set text(font: …)` nos estilos padrão da biblioteca do World, sem escrever
//! nada no `.typ` (Princípio I). Por serem os estilos mais externos, qualquer regra do próprio
//! documento vence.

use std::path::Path;
use std::sync::{Arc, LazyLock};
use std::time::SystemTime;

use serde::Serialize;
use tinymist_world::font::memory::MemoryFontSearcher;
use tinymist_world::font::FontResolverImpl;
use tinymist_world::vfs::PathAccessModel;
use typst::foundations::{Bytes, NativeElement, Recipe, Selector, Styles, Transformation};
use typst::math::EquationElem;
use typst::syntax::Span;
use typst::text::{FontFamily, FontInfo, FontList, RawElem, TextElem};

use crate::diagnostic::{Diagnostic, Kind, Severity};
use crate::fs::access_model::{access_diagnostic, AccessModel};
use crate::fs::scope::Refusal;

/// Família padrão da matemática.
pub const MATH_FAMILY: &str = "Libertinus Math";
/// Família padrão de `raw` (código).
pub const RAW_FAMILY: &str = "Libertinus Mono";

/// Pasta de fontes do projeto, relativa à raiz (D-15; spec, Assumptions).
pub const PROJECT_DIR: &str = "fonts";
/// Extensões lidas em `fonts/` (sem diferenciar maiúsculas).
pub const EXTENSIONS: [&str; 4] = ["ttf", "otf", "ttc", "otc"];
/// Profundidade máxima de subpastas em `fonts/`.
const MAX_DEPTH: usize = 16;
/// Máximo de arquivos de fonte lidos de `fonts/`.
const MAX_FILES: usize = 2_000;

macro_rules! embedded {
    ($($file:literal),* $(,)?) => {
        &[$(($file, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/", $file)))),*]
    };
}

/// Arquivos de fonte embutidos (nome, bytes). Licenças em `src-tauri/fonts/licenses.toml`.
pub static EMBEDDED: &[(&str, &[u8])] = embedded![
    "LibertinusMath-Regular.otf",
    "LibertinusMono-Regular.otf",
    "LibertinusSans-Bold.otf",
    "LibertinusSans-Italic.otf",
    "LibertinusSans-Regular.otf",
    "LibertinusSerif-Bold.otf",
    "LibertinusSerif-BoldItalic.otf",
    "LibertinusSerif-Italic.otf",
    "LibertinusSerif-Regular.otf",
    "LibertinusSerif-Semibold.otf",
    "LibertinusSerif-SemiboldItalic.otf",
];

/// Licenças das fontes embutidas, como o `scripts/audit.mjs` as confere.
const LICENSES_TOML: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/licenses.toml"));

/// `FontLicense` (data-model.md "Fontes"; `compiler_info.fonts`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FontLicense {
    pub family: String,
    pub file: String,
    pub license: String,
    pub copyright: String,
}

static LICENSES: LazyLock<Vec<FontLicense>> = LazyLock::new(|| parse_licenses(LICENSES_TOML));

/// Licenças das fontes embutidas, uma por arquivo (tela "Sobre", FR-016).
pub fn licenses() -> &'static [FontLicense] {
    &LICENSES
}

/// Lê as tabelas `[[font]]` de `licenses.toml`: só chaves com texto entre aspas, que é tudo o
/// que o arquivo usa (o `audit.mjs` rejeita o resto).
fn parse_licenses(text: &str) -> Vec<FontLicense> {
    let mut fonts = Vec::new();
    let mut current: Option<FontLicense> = None;
    let empty = || FontLicense {
        family: String::new(),
        file: String::new(),
        license: String::new(),
        copyright: String::new(),
    };
    for line in text.lines().map(str::trim) {
        if line == "[[font]]" {
            fonts.extend(current.replace(empty()));
            continue;
        }
        let (Some(font), Some((key, value))) = (current.as_mut(), line.split_once('=')) else {
            continue;
        };
        let value = value.trim();
        let Some(value) = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) else {
            continue;
        };
        let value = value.replace("\\\"", "\"").replace("\\\\", "\\");
        match key.trim() {
            "family" => font.family = value,
            "file" => font.file = value,
            "license" => font.license = value,
            "copyright" => font.copyright = value,
            _ => {}
        }
    }
    fonts.extend(current);
    fonts
}

fn embedded_searcher() -> MemoryFontSearcher {
    let mut searcher = MemoryFontSearcher::new();
    for (_, data) in EMBEDDED {
        searcher.add_memory_font(Bytes::new(*data));
    }
    searcher
}

static RESOLVER: LazyLock<Arc<FontResolverImpl>> =
    LazyLock::new(|| Arc::new(embedded_searcher().build()));

/// Resolvedor com só as fontes embutidas. Montado uma vez e compartilhado entre Worlds.
pub fn resolver() -> Arc<FontResolverImpl> {
    RESOLVER.clone()
}

/// Arquivo de fonte visto em `fonts/`: caminho relativo à raiz, tamanho e data, do próprio
/// arquivo (link não é seguido para isso).
type Entry = (String, u64, Option<SystemTime>);

/// Fontes de `fonts/` do projeto vistas na última conferência.
#[derive(Debug, Default)]
pub struct ProjectFonts {
    seen: Option<Vec<Entry>>,
    diagnostics: Vec<Diagnostic>,
}

impl ProjectFonts {
    /// Confere `fonts/`. Se algo mudou desde a última vez, relê os arquivos pelo modelo de
    /// acesso e devolve o resolvedor novo (embutidas primeiro, depois as do projeto, a mesma
    /// ordem de `--font-path` no `typst` CLI). Sem mudança, `None`.
    pub fn refresh(&mut self, access: &AccessModel) -> Option<Arc<FontResolverImpl>> {
        let mut diagnostics = Vec::new();
        let entries = list(access, &mut diagnostics);
        if self.seen.as_ref() == Some(&entries) {
            return None;
        }
        let was_default = self.seen.as_ref().is_none_or(Vec::is_empty);
        self.seen = Some(entries);
        let entries = self.seen.as_deref().unwrap_or_default();
        if entries.is_empty() {
            self.diagnostics = diagnostics;
            return (!was_default).then(resolver);
        }

        let mut searcher = embedded_searcher();
        for (relative, _, _) in entries {
            let Ok(path) = access.lexical_path(relative) else {
                continue;
            };
            match access.content(&path) {
                Ok(bytes) if FontInfo::iter(&bytes).next().is_some() => {
                    searcher.add_memory_font(bytes);
                }
                Ok(_) => diagnostics.push(Diagnostic {
                    severity: Severity::Warning,
                    path: Some(relative.clone()),
                    range: None,
                    line: None,
                    message: format!("arquivo de fonte inválido: `{relative}` (ignorado)"),
                    hints: Vec::new(),
                    kind: Kind::Font,
                }),
                Err(error) => {
                    let refusal = access
                        .refusal_for_message(&error.to_string())
                        .unwrap_or(Refusal::NotFound {
                            requested: relative.clone(),
                        })
                        .with_requested(relative.clone());
                    let mut diag = access_diagnostic(&refusal);
                    diag.severity = Severity::Warning;
                    diagnostics.push(diag);
                }
            }
        }
        self.diagnostics = diagnostics;
        Some(Arc::new(searcher.build()))
    }

    /// Avisos da última conferência: arquivo malformado (`kind = font`) ou recusado pela regra
    /// de escopo (`kind = access`).
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// Arquivos de fonte sob `fonts/`, em ordem. Não segue link para pasta; link para arquivo
/// entra na lista e a leitura decide (a regra de escopo recusa o que aponta para fora).
fn list(access: &AccessModel, diagnostics: &mut Vec<Diagnostic>) -> Vec<Entry> {
    let mut entries = Vec::new();
    let root = access.scope().root();
    let dir = root.join(PROJECT_DIR);
    let Ok(meta) = std::fs::symlink_metadata(&dir) else {
        return entries;
    };
    if meta.file_type().is_symlink() {
        // `fonts` como link: só vale se a pasta de destino ficar dentro da raiz.
        match access.scope().resolve(PROJECT_DIR) {
            Ok(target) if target.is_dir() => walk(&target, PROJECT_DIR, 0, &mut entries),
            Ok(_) => {}
            Err(refusal) => {
                let mut diag = access_diagnostic(&refusal.with_requested(PROJECT_DIR.into()));
                diag.severity = Severity::Warning;
                diagnostics.push(diag);
            }
        }
    } else if meta.is_dir() {
        walk(&dir, PROJECT_DIR, 0, &mut entries);
    }
    entries.sort();
    entries
}

fn walk(dir: &Path, prefix: &str, depth: usize, out: &mut Vec<Entry>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        if out.len() >= MAX_FILES {
            return;
        }
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let Ok(meta) = std::fs::symlink_metadata(entry.path()) else {
            continue;
        };
        let relative = format!("{prefix}/{name}");
        if meta.is_dir() {
            if depth < MAX_DEPTH {
                walk(&entry.path(), &relative, depth + 1, out);
            }
        } else if is_font_file(&name) && (meta.is_file() || meta.file_type().is_symlink()) {
            out.push((relative, meta.len(), meta.modified().ok()));
        }
    }
}

fn is_font_file(name: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

/// Regras de fonte padrão do Prelo para matemática e `raw`.
pub fn default_styles() -> Styles {
    let mut styles = Styles::new();
    for (selector, family) in [
        (Selector::Elem(EquationElem::ELEM, None), MATH_FAMILY),
        (Selector::Elem(RawElem::ELEM, None), RAW_FAMILY),
    ] {
        let mut set = Styles::new();
        set.set(TextElem::font, FontList(vec![FontFamily::new(family)]));
        styles.push(Recipe::new(
            Some(selector),
            Transformation::Style(set),
            Span::detached(),
        ));
    }
    styles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn licencas_cobrem_cada_fonte_embutida() {
        let files: Vec<_> = licenses().iter().map(|l| l.file.as_str()).collect();
        let mut embedded: Vec<_> = EMBEDDED.iter().map(|(f, _)| *f).collect();
        let mut listed = files.clone();
        embedded.sort();
        listed.sort();
        assert_eq!(listed, embedded);
        assert!(licenses().iter().all(|l| l.license == "OFL-1.1"));
        // Aspas escapadas no copyright.
        assert!(licenses()[0].copyright.contains("\"Linux Libertine\""));
    }

    #[test]
    fn extensoes_de_fonte() {
        for ok in ["a.ttf", "a.OTF", "b.ttc", "c.Otc"] {
            assert!(is_font_file(ok), "{ok}");
        }
        for no in ["a.woff", "a.ttf.txt", "ttf", "a"] {
            assert!(!is_font_file(no), "{no}");
        }
    }
}
