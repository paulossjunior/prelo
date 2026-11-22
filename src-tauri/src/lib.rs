mod app_info;
pub mod commands;
pub mod compile;
pub mod diagnostic;
pub mod fs;
pub mod lang_service;
pub mod project;
#[cfg(feature = "self-test")]
mod self_test;

// O autoteste encerra o app pelo IPC; nunca pode ir para uma build de release.
#[cfg(all(feature = "self-test", not(debug_assertions)))]
compile_error!("a feature `self-test` só é permitida em build de depuração");
mod webview_args;
pub mod world;

use tauri::{Builder, Emitter, Manager, Runtime};

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
        // Diálogos nativos chamados só pelo núcleo (`open_project_dialog`, `export_pdf_dialog`). Os comandos
        // `plugin:dialog|*` ficam fora do alcance da webview: nenhuma capability concede
        // `dialog:` (tests/security_config.rs, tests/security_ipc.rs).
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            app_info::get_app_info,
            commands::list_tree,
            commands::choose_main_file,
            commands::create_file,
            commands::create_dir,
            project::open_project_dialog,
            project::update_project_compiler_version,
            project::compiler_info,
            commands::open_file,
            commands::edit_file,
            commands::save_file,
            commands::resolve_external_change,
            commands::complete,
            commands::semantic_tokens,
            commands::jump_from_cursor,
            commands::subscribe_preview,
            commands::page_svg,
            compile::export::export_pdf_dialog
        ])
}

/// Variante de teste: acrescenta `self_test_report` e `self_test_open_project` (nunca em
/// release).
#[cfg(feature = "self-test")]
pub fn configure_with_state<R: Runtime>(
    builder: Builder<R>,
    state: commands::AppState,
) -> Builder<R> {
    builder
        // Diálogos nativos chamados só pelo núcleo (`open_project_dialog`, `export_pdf_dialog`). Os comandos
        // `plugin:dialog|*` ficam fora do alcance da webview: nenhuma capability concede
        // `dialog:` (tests/security_config.rs, tests/security_ipc.rs).
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            app_info::get_app_info,
            commands::list_tree,
            commands::choose_main_file,
            commands::create_file,
            commands::create_dir,
            project::open_project_dialog,
            project::update_project_compiler_version,
            project::compiler_info,
            commands::open_file,
            commands::edit_file,
            commands::save_file,
            commands::resolve_external_change,
            commands::complete,
            commands::semantic_tokens,
            commands::jump_from_cursor,
            commands::subscribe_preview,
            commands::page_svg,
            compile::export::export_pdf_dialog,
            self_test::self_test_report,
            self_test::self_test_open_project
        ])
        .manage(self_test::FixtureCopy::default())
}

/// Intervalo em que o núcleo confere se há arquivo para o salvamento automático.
const AUTOSAVE_TICK: std::time::Duration = std::time::Duration::from_millis(250);

/// Salvamento automático (FR-007): confere a cada [`AUTOSAVE_TICK`] os arquivos ociosos há 2 s.
fn spawn_autosave<R: Runtime>(app: tauri::AppHandle<R>) {
    std::thread::Builder::new()
        .name("prelo-autosave".into())
        .spawn(move || loop {
            std::thread::sleep(AUTOSAVE_TICK);
            for path in app
                .state::<commands::AppState>()
                .autosave(std::time::Instant::now())
            {
                eprintln!("prelo: salvamento automático falhou: {path}");
            }
        })
        .expect("thread de salvamento automático");
}

/// Mudanças externas (FR-008): processa os caminhos avisados pelo observador e emite
/// `external_change_conflict` quando há alteração local pendente.
fn spawn_disk_changes<R: Runtime>(app: tauri::AppHandle<R>) {
    let Some(changes) = app.state::<commands::AppState>().take_disk_changes() else {
        return;
    };
    std::thread::Builder::new()
        .name("prelo-disk-changes".into())
        .spawn(move || {
            for path in changes {
                let state = app.state::<commands::AppState>();
                if state.handle_disk_change(&path) == commands::DiskChange::Conflict {
                    let payload = serde_json::json!({ "path": path });
                    if let Err(e) = app.emit(fs::watch::CONFLICT, payload) {
                        eprintln!("prelo: aviso de mudança externa não entregue: {e}");
                    }
                }
            }
        })
        .expect("thread de mudanças externas");
}

/// Último Salvar antes de fechar (Princípio III). `false` → manter a janela/o app aberto.
fn may_close<R: Runtime>(app: &tauri::AppHandle<R>) -> bool {
    let ok = app.state::<commands::AppState>().flush_for_close();
    if !ok {
        eprintln!("prelo: falha ao gravar antes de fechar; texto mantido em memória");
    }
    ok
}

/// Inicia o app. A janela principal sai do tauri.conf.json (`create: false`) e é criada aqui
/// para receber os argumentos do WebView2 calculados em `webview_args`.
pub fn run() {
    configure(Builder::default())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if !may_close(window.app_handle()) {
                    api.prevent_close();
                }
            }
        })
        .setup(|app| {
            spawn_autosave(app.handle().clone());
            spawn_disk_changes(app.handle().clone());
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
        .build(tauri::generate_context!())
        .expect("falha ao iniciar o Prelo")
        .run(|app, event| {
            // ⌘Q / Ctrl+Q encerram sem passar por `CloseRequested`.
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                if !may_close(app) {
                    api.prevent_exit();
                }
            }
        });
}
