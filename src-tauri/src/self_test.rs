//! Autoteste ponta a ponta (T012). Só existe com a feature `self-test`; nunca em release.
//! Contrato: specs/001-app-skeleton/contracts/ipc.md, `self_test_report`.

use tauri::{AppHandle, Manager, Runtime};

/// Recebe o texto que a interface renderizou. Sucesso: fecha a janela principal, e o app
/// encerra com código 0 pelo caminho normal de fechamento (FR-005). Falha: sai com código 1.
#[tauri::command]
pub fn self_test_report<R: Runtime>(app: AppHandle<R>, rendered: String, elapsed_ms: u64) {
    match check(&rendered) {
        Ok(()) => {
            eprintln!("self-test: ok ({elapsed_ms} ms): {rendered}");
            match app.get_webview_window("main") {
                Some(window) => {
                    if let Err(e) = window.close() {
                        eprintln!("self-test: falha ao fechar a janela: {e}");
                        app.exit(1);
                    }
                }
                None => {
                    eprintln!("self-test: janela principal ausente");
                    app.exit(1);
                }
            }
        }
        Err(reason) => {
            eprintln!("self-test: FALHA: {reason}");
            app.exit(1);
        }
    }
}

fn check(rendered: &str) -> Result<(), String> {
    if rendered.contains("Prelo") {
        Ok(())
    } else {
        Err(format!(
            "texto renderizado não contém \"Prelo\": {rendered:?}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::check;

    #[test]
    fn aceita_texto_com_prelo() {
        assert!(check("Prelo").is_ok());
    }

    #[test]
    fn recusa_texto_sem_prelo() {
        assert!(check("").is_err());
    }
}
