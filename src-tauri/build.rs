fn main() {
    embed_typst_version();
    let msvc = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    if !msvc {
        tauri_build::build();
        return;
    }
    // No Windows, o diálogo nativo (rfd, via tauri-plugin-dialog) importa funções do Common
    // Controls v6 (TaskDialogIndirect), que só existem se o binário tiver o manifesto que pede
    // essa versão. O tauri-build põe o manifesto num recurso ligado só aos executáveis
    // (`rustc-link-arg-bins`); o binário de teste da biblioteca (`cargo test --lib`) e os de
    // `tests/` ficavam sem ele e abriam com STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139).
    // `rustc-link-arg-tests` não resolve: vale só para `tests/`, não para os testes da lib.
    // Solução da própria Tauri (examples/api/src-tauri/build.rs): tirar o manifesto do
    // recurso e embuti-lo pelo linker em todo alvo ligado (app, testes, benches, cdylib).
    // O arquivo é o mesmo manifesto padrão do tauri-build.
    let manifest = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("windows-app-manifest.xml");
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    let windows = tauri_build::WindowsAttributes::new_without_app_manifest();
    if let Err(error) =
        tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
    {
        panic!("tauri-build: {error:#}");
    }
}

/// Versão do compilador embutido lida do `Cargo.lock` (research R1): vira a constante
/// `prelo_lib::project::EMBEDDED_TYPST` pela variável `PRELO_EMBEDDED_TYPST`. Assim a versão
/// mostrada no app e gravada em `.prelo/project.json` nunca diverge da que foi compilada.
fn embed_typst_version() {
    let lock =
        std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("Cargo.lock");
    println!("cargo:rerun-if-changed={}", lock.display());
    let text = std::fs::read_to_string(&lock).expect("ler Cargo.lock");
    let mut lines = text.lines();
    let mut version = None;
    while let Some(line) = lines.next() {
        if line.trim() == r#"name = "typst""# {
            version = lines
                .next()
                .and_then(|l| l.trim().strip_prefix("version = \""))
                .and_then(|v| v.strip_suffix('"'))
                .map(str::to_owned);
            break;
        }
    }
    let version = version.expect("pacote `typst` com versão no Cargo.lock");
    println!("cargo:rustc-env=PRELO_EMBEDDED_TYPST={version}");
}
