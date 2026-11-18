//! Informação do app para a interface (FR-002). Contrato: contracts/ipc.md, `get_app_info`.
//! Implementação em T018; por ora só os testes (TDD, T013).

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
