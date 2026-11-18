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
    builder.invoke_handler(tauri::generate_handler![
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
