mod app_info;
#[cfg(feature = "self-test")]
mod self_test;

use tauri::{Builder, Runtime};

/// Registra os comandos IPC do contrato (specs/001-app-skeleton/contracts/ipc.md).
/// Recebe o builder de fora para os testes de contrato usarem `tauri::test::mock_builder()`.
#[cfg(not(feature = "self-test"))]
pub fn configure<R: Runtime>(builder: Builder<R>) -> Builder<R> {
    builder.invoke_handler(tauri::generate_handler![app_info::get_app_info])
}

/// Variante de teste: acrescenta `self_test_report` (nunca em release).
#[cfg(feature = "self-test")]
pub fn configure<R: Runtime>(builder: Builder<R>) -> Builder<R> {
    builder
        .on_page_load(|webview, payload| {
            // Diagnóstico do autoteste: mostra se a página chegou a carregar.
            eprintln!(
                "self-test: página {:?} em {} ({})",
                payload.event(),
                payload.url(),
                webview.label()
            );
        })
        .invoke_handler(tauri::generate_handler![
            app_info::get_app_info,
            self_test::self_test_report
        ])
}

/// Inicia o app.
pub fn run() {
    configure(Builder::default())
        .run(tauri::generate_context!())
        .expect("falha ao iniciar o Prelo");
}
