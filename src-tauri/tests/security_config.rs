//! Testes de segurança da configuração da janela (T008, FR-007).
//! Contrato: specs/001-app-skeleton/contracts/ipc.md, "Permissões da janela".

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// Prefixos de permissão proibidos na janela principal.
const FORBIDDEN_PREFIXES: &[&str] = &["fs:", "shell:", "http:", "dialog:", "opener:"];

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_json(path: &Path) -> Value {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("não leu {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("JSON inválido em {}: {e}", path.display()))
}

fn tauri_conf() -> Value {
    read_json(&manifest_dir().join("tauri.conf.json"))
}

fn main_window(conf: &Value) -> &Value {
    conf["app"]["windows"]
        .as_array()
        .and_then(|ws| ws.iter().find(|w| w["label"] == "main"))
        .expect("janela com label \"main\" ausente em tauri.conf.json")
}

fn csp(conf: &Value) -> String {
    match &conf["app"]["security"]["csp"] {
        Value::String(s) => s.clone(),
        Value::Object(map) => map
            .iter()
            .map(|(k, v)| format!("{k} {}", v.as_str().unwrap_or_default()))
            .collect::<Vec<_>>()
            .join("; "),
        other => panic!("CSP ausente ou nula em app.security.csp: {other}"),
    }
}

#[test]
fn janela_principal_se_chama_prelo() {
    let conf = tauri_conf();
    assert_eq!(main_window(&conf)["title"], "Prelo");
}

#[test]
fn csp_restringe_a_origem_propria() {
    let csp = csp(&tauri_conf());
    assert!(
        csp.contains("default-src 'self'"),
        "CSP precisa de default-src 'self': {csp}"
    );
}

#[test]
fn csp_nao_permite_codigo_inseguro() {
    let csp = csp(&tauri_conf());
    for bad in ["unsafe-inline", "unsafe-eval", "unsafe-hashes", "*"] {
        assert!(
            !csp.split_whitespace().any(|t| t.trim_matches(['\'', ';']) == bad),
            "CSP contém {bad}: {csp}"
        );
    }
}

#[test]
fn capabilities_nao_concedem_permissoes_de_sistema() {
    let dir = manifest_dir().join("capabilities");
    let files: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("pasta capabilities ausente")
        .map(|e| e.expect("entrada ilegível").path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    assert!(!files.is_empty(), "nenhum arquivo em capabilities/");

    for file in files {
        let cap = read_json(&file);
        let perms = cap["permissions"]
            .as_array()
            .unwrap_or_else(|| panic!("{}: permissions ausente", file.display()));
        for perm in perms {
            let id = perm
                .as_str()
                .or_else(|| perm["identifier"].as_str())
                .unwrap_or_default();
            assert!(
                !FORBIDDEN_PREFIXES.iter().any(|p| id.starts_with(p)),
                "{}: permissão proibida {id}",
                file.display()
            );
        }
    }
}

#[test]
fn capabilities_valem_so_para_a_janela_principal() {
    let cap = read_json(&manifest_dir().join("capabilities/default.json"));
    assert_eq!(cap["windows"], serde_json::json!(["main"]));
    assert!(
        cap.get("remote").is_none(),
        "capabilities não podem liberar URLs remotas"
    );
}
