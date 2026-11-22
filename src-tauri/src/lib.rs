mod app_info;
pub mod commands;
pub mod compile;
pub mod diagnostic;
pub mod fs;
pub mod project;
#[cfg(feature = "self-test")]
mod self_test;

// O autoteste encerra o app pelo IPC; nunca pode ir para uma build de release.
#[cfg(all(feature = "self-test", not(debug_assertions)))]
compile_error!("a feature `self-test` só é permitida em build de depuração");
mod webview_args;
pub mod world;

use tauri::{Builder, Runtime};

/// Registra os comandos IPC do contrato (specs/001-app-skeleton/contracts/ipc.md e
/// specs/003-editor-local/contracts/ipc.md) com o estado padrão do núcleo.
/// Recebe o builder de fora para os testes de contrato usarem `tauri::test::mock_builder()`.
pub fn configure<R: Runtime>(builder: Builder<R>) -> Builder<R> {
    configure_with_state(builder, commands::AppState::default())
}

/// Como [`configure`], com o estado do núcleo dado (os testes trocam o motor por um falso).
#[cfg(not(feature = "self-test"))]
pub fn configure_with_state<R: Runtime>(
    builder: Builder<R>,
    state: commands::AppState,
) -> Builder<R> {
    builder
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            app_info::get_app_info,
            commands::list_tree,
            commands::choose_main_file,
            commands::open_file,
            commands::edit_file,
            commands::save_file,
            commands::resolve_external_change,
            commands::complete,
            commands::semantic_tokens,
            commands::jump_from_cursor,
            commands::subscribe_preview,
            commands::page_svg
        ])
}

/// Variante de teste: acrescenta `self_test_report` (nunca em release).
#[cfg(feature = "self-test")]
pub fn configure_with_state<R: Runtime>(
    builder: Builder<R>,
    state: commands::AppState,
) -> Builder<R> {
    builder
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            app_info::get_app_info,
            commands::list_tree,
            commands::choose_main_file,
            commands::open_file,
            commands::edit_file,
            commands::save_file,
            commands::resolve_external_change,
            commands::complete,
            commands::semantic_tokens,
            commands::jump_from_cursor,
            commands::subscribe_preview,
            commands::page_svg,
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
