//! Argumentos do WebView2 (Windows). Quando o app define argumentos próprios, o WebView2
//! ignora a variável WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS, que o msedgedriver usa para ligar
//! a porta de depuração do WebDriver. Em build de debug, o Prelo junta a variável aos
//! argumentos; em release, nunca (porta de depuração não se liga por ambiente).
//! Implementação em seguida; por ora só os testes (TDD).

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
        assert_eq!(extra_from_env(true, env.clone()).as_deref(), Some("--remote-debugging-port=0"));
        assert_eq!(extra_from_env(false, env), None, "release não pode ligar depuração por ambiente");
        assert_eq!(extra_from_env(true, None), None);
    }
}
