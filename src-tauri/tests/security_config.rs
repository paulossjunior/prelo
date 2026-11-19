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
    let text =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("não leu {}: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("JSON inválido em {}: {e}", path.display()))
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
            !csp.split_whitespace()
                .any(|t| t.trim_matches(['\'', ';']) == bad),
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

// --- Lacunas apontadas pelo QA na validação da US1 (contracts/ipc.md, plan.md "Constraints") ---

/// Permissões que a janela pode receber: só as necessárias para chamar os comandos do contrato
/// (contracts/ipc.md: "falha em qualquer permissão fora desta lista"; plan.md: "nenhuma permissão
/// de janela além do IPC do contrato"). Comandos do app não exigem permissão `core:*`.
const ALLOWED_PERMISSIONS: &[&str] = &["allow-get-app-info", "allow-self-test-report"];

/// Fontes que a CSP pode citar. Qualquer origem de rede além do IPC do Tauri abriria canal para
/// fora do app (STRIDE: divulgação) e anularia `default-src 'self'`.
const ALLOWED_CSP_SOURCES: &[&str] = &["'self'", "'none'", "ipc:", "http://ipc.localhost"];

fn csp_directives(csp: &str) -> Vec<(String, Vec<String>)> {
    csp.split(';')
        .map(str::trim)
        .filter(|d| !d.is_empty())
        .map(|d| {
            let mut parts = d.split_whitespace();
            let name = parts.next().unwrap_or_default().to_ascii_lowercase();
            (name, parts.map(str::to_owned).collect())
        })
        .collect()
}

#[test]
fn csp_connect_src_so_permite_o_ipc() {
    let csp = csp(&tauri_conf());
    let directives = csp_directives(&csp);
    let connect = directives
        .iter()
        .find(|(name, _)| name == "connect-src")
        .unwrap_or_else(|| panic!("CSP sem connect-src explícito: {csp}"));
    for source in &connect.1 {
        assert!(
            ["ipc:", "http://ipc.localhost"].contains(&source.as_str()),
            "connect-src permite origem fora do IPC: {source} ({csp})"
        );
    }
}

#[test]
fn csp_nenhuma_diretiva_libera_origem_remota() {
    let csp = csp(&tauri_conf());
    for (name, sources) in csp_directives(&csp) {
        for source in sources {
            assert!(
                ALLOWED_CSP_SOURCES.contains(&source.as_str()),
                "{name} libera fonte fora da lista {ALLOWED_CSP_SOURCES:?}: {source} ({csp})"
            );
        }
    }
}

#[test]
fn capabilities_so_contem_permissoes_do_contrato() {
    let dir = manifest_dir().join("capabilities");
    for entry in fs::read_dir(&dir).expect("pasta capabilities ausente") {
        let path = entry.expect("entrada ilegível").path();
        // .toml/.json5 também são lidos pelo Tauri e escapariam dos testes acima.
        assert!(
            path.extension().is_some_and(|x| x == "json"),
            "{}: capability em formato não verificado",
            path.display()
        );
        let cap = read_json(&path);
        for perm in cap["permissions"].as_array().into_iter().flatten() {
            let id = perm
                .as_str()
                .or_else(|| perm["identifier"].as_str())
                .unwrap_or_default();
            assert!(
                ALLOWED_PERMISSIONS.contains(&id),
                "{}: permissão fora do contrato: {id:?}",
                path.display()
            );
        }
    }
}

#[test]
fn tauri_conf_nao_declara_capabilities_embutidas() {
    let conf = tauri_conf();
    // Capability declarada como objeto dentro de tauri.conf.json não passaria pelos testes
    // de capabilities/; só referência por identificador é aceita.
    if let Some(caps) = conf["app"]["security"]["capabilities"].as_array() {
        for cap in caps {
            assert!(
                cap.is_string(),
                "capability embutida em tauri.conf.json: {cap}"
            );
        }
    }
}

#[test]
fn self_test_nao_compila_em_release() {
    // O comando `self_test_report` encerra o app e só pode existir em build de depuração
    // (contracts/ipc.md). A trava precisa estar no código, não só na convenção.
    let lib = fs::read_to_string(manifest_dir().join("src/lib.rs")).expect("não leu src/lib.rs");
    let guard = "#[cfg(all(feature = \"self-test\", not(debug_assertions)))]\ncompile_error!(";
    assert!(
        lib.contains(guard),
        "src/lib.rs precisa barrar `--features self-test` em release com compile_error!"
    );
}
