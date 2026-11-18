mod app_info;
#[cfg(feature = "self-test")]
mod self_test;
mod webview_args;

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

/// Inicia o app. A janela principal sai do tauri.conf.json (`create: false`) e é criada aqui
/// para receber os argumentos do WebView2 calculados em `webview_args`.
pub fn run() {
    configure(Builder::default())
        .setup(|app| {
            let config = app
                .config()
                .app
                .windows
                .iter()
                .find(|w| w.label == "main")
                .cloned()
                .ok_or("janela \"main\" ausente em tauri.conf.json")?;
            tauri::WebviewWindowBuilder::from_config(app.handle(), &config)?
                .additional_browser_args(&webview_args::current())
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("falha ao iniciar o Prelo");
}
