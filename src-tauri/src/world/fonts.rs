//! Fontes do World (D-15, research R6).
//!
//! Nesta etapa, só as fontes embutidas de `src-tauri/fonts/` (Libertinus 7.051, OFL-1.1).
//! Nenhuma fonte do sistema é consultada: sem `fontdb`, sem varredura de pastas. As fontes de
//! `fonts/` do projeto entram com T056, lidas pelo `PathAccessModel`.
//!
//! O Typst pede "New Computer Modern Math" para matemática e "DejaVu Sans Mono" para `raw`,
//! que o Prelo não embute. Os padrões do Prelo (Libertinus Math e Libertinus Mono) entram como
//! regras `show … : set text(font: …)` nos estilos padrão da biblioteca do World, sem escrever
//! nada no `.typ` (Princípio I). Por serem os estilos mais externos, qualquer regra do próprio
//! documento vence.

use std::sync::{Arc, LazyLock};

use tinymist_world::font::memory::MemoryFontSearcher;
use tinymist_world::font::FontResolverImpl;
use typst::foundations::{Bytes, NativeElement, Recipe, Selector, Styles, Transformation};
use typst::math::EquationElem;
use typst::syntax::Span;
use typst::text::{FontFamily, FontList, RawElem, TextElem};

/// Família padrão da matemática.
pub const MATH_FAMILY: &str = "Libertinus Math";
/// Família padrão de `raw` (código).
pub const RAW_FAMILY: &str = "Libertinus Mono";

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

static RESOLVER: LazyLock<Arc<FontResolverImpl>> = LazyLock::new(|| {
    let mut searcher = MemoryFontSearcher::new();
    for (_, data) in EMBEDDED {
        searcher.add_memory_font(Bytes::new(*data));
    }
    Arc::new(searcher.build())
});

/// Resolvedor com só as fontes embutidas. Montado uma vez e compartilhado entre Worlds.
pub fn resolver() -> Arc<FontResolverImpl> {
    RESOLVER.clone()
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
