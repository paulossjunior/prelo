//! Informação do app para a interface (FR-002). Contrato: contracts/ipc.md, `get_app_info`.
//! Campos: data-model.md `AppInfo.version`, `AppInfo.os`, `AppInfo.arch`.

use serde::Serialize;
use serde_json::Value;
use tauri::ipc::{InvokeBody, Request};
use tauri::{AppHandle, Runtime};

/// Sistema em execução; outro valor é erro `unsupported_os`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Macos,
    Windows,
    Linux,
}

impl Os {
    pub fn from_name(name: &str) -> Result<Self, String> {
        match name {
            "macos" => Ok(Self::Macos),
            "windows" => Ok(Self::Windows),
            "linux" => Ok(Self::Linux),
            _ => Err("unsupported_os".to_string()),
        }
    }

    pub fn current() -> Result<Self, String> {
        Self::from_name(std::env::consts::OS)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Macos => "macos",
            Self::Windows => "windows",
            Self::Linux => "linux",
        }
    }
}

/// Serializa como no contrato: `"macos"`, `"windows"`, `"linux"`.
impl Serialize for Os {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppInfo {
    pub version: String,
    pub os: Os,
    pub arch: String,
}

impl AppInfo {
    pub fn new(version: &str) -> Result<Self, String> {
        if !is_semver(version) {
            return Err("invalid_version".to_string());
        }
        Ok(Self {
            version: version.to_string(),
            os: Os::current()?,
            arch: std::env::consts::ARCH.to_string(),
        })
    }
}

/// MAJOR.MINOR.PATCH numéricos, com pré-release opcional após `-`.
fn is_semver(version: &str) -> bool {
    let core = version.split('-').next().unwrap_or_default();
    let parts: Vec<&str> = core.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

/// Sem argumentos: qualquer corpo além de objeto vazio ou nulo é recusado (FR-003).
fn has_no_arguments(body: &InvokeBody) -> bool {
    match body {
        InvokeBody::Json(Value::Null) => true,
        InvokeBody::Json(Value::Object(map)) => map.is_empty(),
        _ => false,
    }
}

#[tauri::command]
pub fn get_app_info<R: Runtime>(
    app: AppHandle<R>,
    request: Request<'_>,
) -> Result<AppInfo, String> {
    if !has_no_arguments(request.body()) {
        return Err("invalid_arguments".to_string());
    }
    AppInfo::new(&app.package_info().version.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conf_version() -> String {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tauri.conf.json");
        let conf: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        conf["version"].as_str().unwrap().to_string()
    }

    #[test]
    fn versao_igual_ao_pacote_e_ao_tauri_conf() {
        let info = AppInfo::new(env!("CARGO_PKG_VERSION")).unwrap();
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(info.version, conf_version());
    }

    #[test]
    fn sistema_mapeado_para_os_tres_valores_do_contrato() {
        assert_eq!(Os::from_name("macos"), Ok(Os::Macos));
        assert_eq!(Os::from_name("windows"), Ok(Os::Windows));
        assert_eq!(Os::from_name("linux"), Ok(Os::Linux));
        let json = serde_json::to_value([Os::Macos, Os::Windows, Os::Linux]).unwrap();
        assert_eq!(json, serde_json::json!(["macos", "windows", "linux"]));
    }

    #[test]
    fn sistema_desconhecido_e_erro_unsupported_os() {
        assert_eq!(Os::from_name("freebsd"), Err("unsupported_os".to_string()));
        assert_eq!(Os::from_name(""), Err("unsupported_os".to_string()));
    }

    #[test]
    fn arquitetura_preenchida() {
        let info = AppInfo::new("0.1.0").unwrap();
        assert_eq!(info.arch, std::env::consts::ARCH);
        assert!(!info.arch.is_empty());
    }

    #[test]
    fn versao_que_nao_e_semver_e_recusada() {
        assert!(AppInfo::new("").is_err());
        assert!(AppInfo::new("um.dois").is_err());
    }
}
