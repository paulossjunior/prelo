//! Testes de contrato da ponte interface → núcleo (T014, T015).
//! Contrato: specs/001-app-skeleton/contracts/ipc.md.

use serde_json::{json, Value};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, MockRuntime, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::{App, WebviewWindow, WebviewWindowBuilder};

fn app() -> (App<MockRuntime>, WebviewWindow<MockRuntime>) {
    let app = prelo_lib::configure(mock_builder())
        .build(mock_context(noop_assets()))
        .expect("app de teste não montou");
    let window = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("janela de teste não abriu");
    (app, window)
}

fn invoke(window: &WebviewWindow<MockRuntime>, cmd: &str, body: InvokeBody) -> Result<Value, Value> {
    let url = if cfg!(windows) { "http://tauri.localhost" } else { "tauri://localhost" };
    get_ipc_response(
        window,
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: url.parse().unwrap(),
            body,
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .map(|b| b.deserialize::<Value>().expect("resposta não é JSON"))
}

fn get_app_info_ok(window: &WebviewWindow<MockRuntime>) {
    let info = invoke(window, "get_app_info", InvokeBody::default()).expect("get_app_info falhou");
    let obj = info.as_object().expect("AppInfo deve ser objeto");
    let mut keys: Vec<_> = obj.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["arch", "os", "version"]);
    assert!(["macos", "windows", "linux"].contains(&info["os"].as_str().unwrap()));
    assert!(!info["arch"].as_str().unwrap().is_empty());
    assert_eq!(info["version"].as_str().unwrap().split('.').count(), 3);
}

#[test]
fn get_app_info_responde_no_formato_do_contrato() {
    let (_app, window) = app();
    get_app_info_ok(&window);
}

#[test]
fn comando_nao_registrado_e_rejeitado_e_nucleo_segue_funcionando() {
    let (_app, window) = app();
    assert!(invoke(&window, "read_file", InvokeBody::default()).is_err());
    assert!(invoke(&window, "", InvokeBody::default()).is_err());
    get_app_info_ok(&window);
}

#[test]
fn argumentos_extras_sao_rejeitados() {
    let (_app, window) = app();
    let res = invoke(&window, "get_app_info", InvokeBody::Json(json!({ "path": "/etc/passwd" })));
    assert!(res.is_err(), "argumento extra deveria ser recusado: {res:?}");
    get_app_info_ok(&window);
}

#[test]
fn corpo_de_tipo_errado_e_rejeitado() {
    let (_app, window) = app();
    for body in [
        InvokeBody::Json(json!("texto")),
        InvokeBody::Json(json!([1, 2])),
        InvokeBody::Raw(vec![0xff, 0x00, 0x13]),
    ] {
        assert!(invoke(&window, "get_app_info", body).is_err());
    }
    get_app_info_ok(&window);
}

/// T015: sem a feature `self-test`, o comando de autoteste não existe (nunca em release).
#[cfg(not(feature = "self-test"))]
#[test]
fn self_test_report_nao_existe_sem_a_feature() {
    let (_app, window) = app();
    let body = InvokeBody::Json(json!({ "rendered": "Prelo", "elapsedMs": 1 }));
    assert!(invoke(&window, "self_test_report", body).is_err());
    get_app_info_ok(&window);
}
