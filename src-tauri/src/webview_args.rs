//! Argumentos do WebView2 (Windows). Quando o app define argumentos próprios, o WebView2
//! ignora a variável WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS, que o msedgedriver usa para ligar
//! a porta de depuração do WebDriver. Em build de debug, o Prelo junta a variável aos
//! argumentos; em release, nunca (porta de depuração não se liga por ambiente).

/// Os mesmos argumentos que o wry passa por padrão no Windows.
pub const DEFAULT_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection";

pub fn browser_args(extra: Option<&str>) -> String {
    match extra.map(str::trim).filter(|e| !e.is_empty()) {
        Some(extra) => format!("{DEFAULT_ARGS} {extra}"),
        None => DEFAULT_ARGS.to_string(),
    }
}

/// Valor da variável de ambiente, só em build de debug.
pub fn extra_from_env(debug_build: bool, env: Option<String>) -> Option<String> {
    if debug_build {
        env
    } else {
        None
    }
}

/// Argumentos para a janela principal neste processo.
pub fn current() -> String {
    let env = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").ok();
    browser_args(extra_from_env(cfg!(debug_assertions), env).as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sem_extra_usa_os_padroes_do_tauri() {
        assert_eq!(browser_args(None), DEFAULT_ARGS);
        assert_eq!(browser_args(Some("   ")), DEFAULT_ARGS);
    }

    #[test]
    fn extra_e_acrescentado_depois_dos_padroes() {
        let args = browser_args(Some("--remote-debugging-port=0"));
        assert!(args.starts_with(DEFAULT_ARGS));
        assert!(args.ends_with(" --remote-debugging-port=0"));
    }

    #[test]
    fn variavel_so_vale_em_build_de_debug() {
        let env = Some("--remote-debugging-port=0".to_string());
        assert_eq!(
            extra_from_env(true, env.clone()).as_deref(),
            Some("--remote-debugging-port=0")
        );
        assert_eq!(
            extra_from_env(false, env),
            None,
            "release não pode ligar depuração por ambiente"
        );
        assert_eq!(extra_from_env(true, None), None);
    }
}
