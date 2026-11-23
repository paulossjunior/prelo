mod app_info;
pub mod commands;
pub mod compile;
pub mod diagnostic;
pub mod fs;
pub mod lang_service;
pub mod project;
#[cfg(feature = "self-test")]
mod self_test;
pub mod trust;

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
        // Diálogos nativos chamados só pelo núcleo (`open_project_dialog`,
        // `create_project_dialog`, `export_pdf_dialog`, `copy_into_project`). Os comandos
        // `plugin:dialog|*` ficam fora do alcance da webview: nenhuma capability concede
        // `dialog:` (tests/security_config.rs, tests/security_ipc.rs).
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .register_uri_scheme_protocol(compile::preview::ASSET_SCHEME, asset_protocol)
        .invoke_handler(tauri::generate_handler![
            app_info::get_app_info,
            commands::list_tree,
            commands::choose_main_file,
            commands::create_file,
            commands::create_dir,
            project::open_project_dialog,
            project::create_project_dialog,
            project::update_project_compiler_version,
            project::compiler_info,
            commands::open_file,
            commands::edit_file,
            commands::save_file,
            commands::resolve_external_change,
            commands::complete,
            commands::semantic_tokens,
            commands::jump_from_cursor,
            commands::jump_from_click,
            commands::outline,
            commands::compile_now,
            commands::subscribe_preview,
            commands::page_svg,
            commands::preview_asset,
            commands::decide_plugin,
            commands::redownload_package,
            commands::copy_into_project,
            compile::export::export_pdf_dialog
        ])
}

/// Variante de teste: acrescenta `self_test_report`, `self_test_open_project` e
/// `self_test_write_external` (nunca em release).
#[cfg(feature = "self-test")]
pub fn configure_with_state<R: Runtime>(
    builder: Builder<R>,
    state: commands::AppState,
) -> Builder<R> {
    builder
        // Diálogos nativos chamados só pelo núcleo (`open_project_dialog`,
        // `create_project_dialog`, `export_pdf_dialog`). Os comandos `plugin:dialog|*` ficam
        // fora do alcance da webview: nenhuma capability concede `dialog:`
        // (tests/security_config.rs, tests/security_ipc.rs).
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .register_uri_scheme_protocol(compile::preview::ASSET_SCHEME, asset_protocol)
        .invoke_handler(tauri::generate_handler![
            app_info::get_app_info,
            commands::list_tree,
            commands::choose_main_file,
            commands::create_file,
            commands::create_dir,
            project::open_project_dialog,
            project::create_project_dialog,
            project::update_project_compiler_version,
            project::compiler_info,
            commands::open_file,
            commands::edit_file,
            commands::save_file,
            commands::resolve_external_change,
            commands::complete,
            commands::semantic_tokens,
            commands::jump_from_cursor,
            commands::jump_from_click,
            commands::outline,
            commands::compile_now,
            commands::subscribe_preview,
            commands::page_svg,
            commands::preview_asset,
            commands::decide_plugin,
            commands::redownload_package,
            commands::copy_into_project,
            compile::export::export_pdf_dialog,
            self_test::self_test_report,
            self_test::self_test_open_project,
            self_test::self_test_write_external,
            self_test::self_test_crash_mode,
            self_test::self_test_mark,
            self_test::self_test_reopened
        ])
        .manage(self_test::FixtureCopy::default())
}

/// Protocolo `prelo-asset` (contracts/ipc.md; T058): imagens da prévia atual por sha256,
/// 404 para qualquer outra coisa ([`compile::preview::asset_response`]).
fn asset_protocol<R: Runtime>(
    ctx: tauri::UriSchemeContext<'_, R>,
    request: tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<Vec<u8>> {
    ctx.app_handle()
        .state::<commands::AppState>()
        .preview_asset_response(&request)
}

/// Salvamento automático (FR-007, SC-009): confere a cada
/// [`AUTOSAVE_TICK`](crate::fs::save::AUTOSAVE_TICK) os arquivos com
/// texto por gravar há [`AUTOSAVE_DELAY`](crate::fs::save::AUTOSAVE_DELAY).
fn spawn_autosave<R: Runtime>(app: tauri::AppHandle<R>) {
    std::thread::Builder::new()
        .name("prelo-autosave".into())
        .spawn(move || loop {
            std::thread::sleep(fs::save::AUTOSAVE_TICK);
            for path in app
                .state::<commands::AppState>()
                .autosave(std::time::Instant::now())
            {
                eprintln!("prelo: salvamento automático falhou: {path}");
            }
        })
        .expect("thread de salvamento automático");
}

/// Mudanças externas (FR-008): processa os caminhos avisados pelo observador e avisa a
/// interface ([`notify_disk_change`]).
fn spawn_disk_changes<R: Runtime>(app: tauri::AppHandle<R>) {
    let Some(changes) = app.state::<commands::AppState>().take_disk_changes() else {
        return;
    };
    std::thread::Builder::new()
        .name("prelo-disk-changes".into())
        .spawn(move || {
            for path in changes {
                notify_disk_change(&app, &path);
            }
        })
        .expect("thread de mudanças externas");
}

/// Relê `path` depois de um aviso do observador e emite o evento do contrato
/// (specs/003-editor-local/contracts/ipc.md): `external_change_conflict { path }` com alteração
/// local pendente; `external_change_reloaded { path, version }` quando o núcleo recarregou o
/// texto do disco sem alteração local (a interface busca o texto com `open_file`).
fn notify_disk_change<R: Runtime>(app: &tauri::AppHandle<R>, path: &str) {
    let state = app.state::<commands::AppState>();
    let (event, payload) = match state.handle_disk_change(path) {
        commands::DiskChange::Conflict => {
            (fs::watch::CONFLICT, serde_json::json!({ "path": path }))
        }
        commands::DiskChange::Reloaded => {
            let Some(version) = state.file_version(path) else {
                return;
            };
            (
                fs::watch::RELOADED,
                serde_json::json!({ "path": path, "version": version }),
            )
        }
        commands::DiskChange::Ignored => return,
    };
    if let Err(e) = app.emit(event, payload) {
        eprintln!("prelo: aviso de mudança externa não entregue: {e}");
    }
}

/// Decisões de plugin e cache de pacotes nos dados locais do app; pedidos de confirmação
/// viram o evento `plugin_approval_needed` (contracts/ipc.md).
fn connect_trust<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
    let dir = app.path().app_local_data_dir()?;
    let state = app.state::<commands::AppState>();
    state.trust().set_data_dir(dir);
    let handle = app.clone();
    state
        .trust()
        .set_listener(std::sync::Arc::new(move |request| {
            if let Err(e) = handle.emit(trust::PLUGIN_APPROVAL_NEEDED, request) {
                eprintln!("prelo: pedido de confirmação de plugin não entregue: {e}");
            }
        }));
    // Ofertas de cópia para o projeto (D-13): `copy_into_project_offer { requested }`.
    let handle = app.clone();
    state.set_copy_offer_listener(std::sync::Arc::new(move |offer| {
        if let Err(e) = handle.emit(fs::copy_into::COPY_OFFER, offer) {
            eprintln!("prelo: oferta de cópia para o projeto não entregue: {e}");
        }
    }));
    Ok(())
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
            connect_trust(app.handle())?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;
    use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};
    use tauri::Listener;

    /// App de teste com a fixture mínima aberta e `main.typ` aberto no editor.
    fn app() -> (
        tauri::App<MockRuntime>,
        tempfile::TempDir,
        std::path::PathBuf,
    ) {
        let app = configure(mock_builder())
            .build(mock_context(noop_assets()))
            .expect("app de teste não montou");
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.typ"), "= Título\n").unwrap();
        let state = app.state::<commands::AppState>();
        state.open_project(dir.path()).unwrap();
        state.open_file("main.typ").unwrap();
        let main = std::fs::canonicalize(dir.path()).unwrap().join("main.typ");
        (app, dir, main)
    }

    /// Eventos `name` emitidos pelo núcleo, com o conteúdo em JSON.
    fn events(app: &tauri::App<MockRuntime>, name: &str) -> mpsc::Receiver<serde_json::Value> {
        let (tx, rx) = mpsc::channel();
        app.listen_any(name, move |e| {
            let _ = tx.send(serde_json::from_str(e.payload()).unwrap());
        });
        rx
    }

    const WAIT: Duration = Duration::from_secs(5);
    const NONE: Duration = Duration::from_millis(200);

    #[test]
    fn mudanca_externa_sem_alteracao_local_avisa_a_interface_com_a_versao_nova() {
        let (app, _dir, main) = app();
        let reloaded = events(&app, fs::watch::RELOADED);
        let conflict = events(&app, fs::watch::CONFLICT);
        std::fs::write(&main, "= Outro\n\nTexto novo.\n").unwrap();
        notify_disk_change(app.handle(), "main.typ");
        assert_eq!(
            reloaded
                .recv_timeout(WAIT)
                .expect("external_change_reloaded não veio"),
            serde_json::json!({ "path": "main.typ", "version": 1 })
        );
        assert!(
            conflict.recv_timeout(NONE).is_err(),
            "sem alteração local não há conflito"
        );
        let opened = app
            .state::<commands::AppState>()
            .open_file("main.typ")
            .unwrap();
        assert_eq!(opened.text, "= Outro\n\nTexto novo.\n");
    }

    #[test]
    fn mudanca_externa_com_alteracao_local_so_avisa_o_conflito() {
        let (app, _dir, main) = app();
        let reloaded = events(&app, fs::watch::RELOADED);
        let conflict = events(&app, fs::watch::CONFLICT);
        let change = commands::Change {
            from: 0,
            to: 0,
            insert: "minha ".into(),
        };
        app.state::<commands::AppState>()
            .edit_file("main.typ", 0, &[change])
            .unwrap();
        std::fs::write(&main, "deles\n").unwrap();
        notify_disk_change(app.handle(), "main.typ");
        assert_eq!(
            conflict
                .recv_timeout(WAIT)
                .expect("external_change_conflict não veio"),
            serde_json::json!({ "path": "main.typ" })
        );
        assert!(
            reloaded.recv_timeout(NONE).is_err(),
            "conflito não recarrega"
        );
    }

    #[test]
    fn evento_sem_mudanca_de_conteudo_nao_avisa_nada() {
        let (app, _dir, _main) = app();
        let reloaded = events(&app, fs::watch::RELOADED);
        let conflict = events(&app, fs::watch::CONFLICT);
        // O disco continua igual ao que o núcleo leu (ex.: a própria gravação).
        notify_disk_change(app.handle(), "main.typ");
        notify_disk_change(app.handle(), "outro.typ");
        assert!(reloaded.recv_timeout(NONE).is_err());
        assert!(conflict.recv_timeout(NONE).is_err());
    }
}
