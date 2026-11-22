fn main() {
    // No Windows, o tauri-build embute o manifesto (Common Controls v6) só no executável do app.
    // Binários de teste que linkam o Tauri falham ao abrir com STATUS_ENTRYPOINT_NOT_FOUND sem
    // ele; por isso o mesmo manifesto é embutido nos alvos de teste.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        let manifest = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("windows-app-manifest.xml");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
            manifest.display()
        );
    }
    embed_typst_version();
    tauri_build::build()
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
