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
    tauri_build::build()
}
