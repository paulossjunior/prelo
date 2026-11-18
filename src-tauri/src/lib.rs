#[cfg(feature = "self-test")]
mod self_test;

use tauri::{Builder, Runtime};

/// Builder com os comandos IPC do contrato (specs/001-app-skeleton/contracts/ipc.md).
/// Genérico no runtime para os testes de contrato usarem o mock runtime do Tauri.
pub fn builder<R: Runtime>() -> Builder<R> {
    let builder = Builder::<R>::new();
    #[cfg(feature = "self-test")]
    let builder = builder.invoke_handler(tauri::generate_handler![self_test::self_test_report]);
    builder
}

/// Inicia o app.
pub fn run() {
    builder::<tauri::Wry>()
        .run(tauri::generate_context!())
        .expect("falha ao iniciar o Prelo");
}
