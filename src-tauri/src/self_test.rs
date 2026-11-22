//! Autoteste ponta a ponta (T012, T031). Só existe com a feature `self-test`; nunca em release.
//! Contratos: specs/001-app-skeleton/contracts/ipc.md e specs/003-editor-local/contracts/ipc.md,
//! `self_test_report` e `self_test_open_project`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Deserialize;
use tauri::{AppHandle, Manager, Runtime, State};

use crate::app_info::Os;
use crate::commands::{AppState, ProjectInfo, EMBEDDED_TYPST};

/// Meta de SC-002: versão e sistema visíveis em até 3 s.
pub const MAX_ELAPSED_MS: u64 = 3000;

/// Fixture aberta pelo autoteste e pelo e2e (US1, Independent Test).
const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/artigo-basico");

/// O que a interface observou ao percorrer US1 (src/self-test.ts, `ScenarioResult`).
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scenario {
    pub project_opened: bool,
    pub preview_changed: bool,
    pub error_line: u32,
    pub diagnostic_line: Option<u32>,
    pub editor_marked: bool,
    pub compiler_shown: String,
    pub failure: Option<String>,
}

/// Cópia temporária da fixture: o roteiro edita e o salvamento automático grava nela, nunca
/// no repositório. Apagada quando o estado do app é descartado.
#[derive(Default)]
pub struct FixtureCopy(Mutex<Option<tempfile::TempDir>>);

/// Abre uma cópia nova da fixture `artigo-basico` como projeto, sem diálogo nativo.
#[tauri::command(async)]
pub fn self_test_open_project(
    state: State<'_, AppState>,
    copy: State<'_, FixtureCopy>,
) -> Result<ProjectInfo, String> {
    let dir = tempfile::Builder::new()
        .prefix("prelo-self-test-")
        .tempdir()
        .map_err(|e| format!("pasta temporária: {e}"))?;
    let root = dir.path().join("artigo-basico");
    copy_fixture(Path::new(FIXTURE), &root).map_err(|e| format!("copiar a fixture: {e}"))?;
    let info = state
        .open_project(&root)
        .map_err(|e| format!("abrir a fixture: {e}"))?;
    *copy
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(dir);
    Ok(info)
}

/// Copia `src` (arquivos e pastas) para `dest`.
pub fn copy_fixture(src: &Path, dest: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let to = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_fixture(&entry.path(), &to)?;
        } else {
            std::fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}

/// Primeiro executável `name` em `path` (formato da variável PATH), se houver.
pub fn find_on_path(name: &str, path: Option<OsString>) -> Option<PathBuf> {
    let path = path?;
    let exe = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };
    std::env::split_paths(&path)
        .map(|dir| dir.join(&exe))
        .find(|candidate| candidate.is_file())
}

/// Recebe o texto que a interface renderizou e o roteiro do editor. Sucesso: fecha a janela
/// principal, e o app encerra com código 0 pelo caminho normal de fechamento (FR-005).
/// Falha: sai com código 1.
#[tauri::command]
pub fn self_test_report<R: Runtime>(
    app: AppHandle<R>,
    rendered: String,
    elapsed_ms: u64,
    scenario: Scenario,
) {
    let version = app.package_info().version.to_string();
    let typst = find_on_path("typst", std::env::var_os("PATH"));
    match check(&rendered, &version, elapsed_ms, &scenario, typst) {
        Ok(()) => {
            eprintln!("self-test: ok ({elapsed_ms} ms): {rendered}; editor: {scenario:?}");
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

fn check(
    rendered: &str,
    version: &str,
    elapsed_ms: u64,
    scenario: &Scenario,
    typst_on_path: Option<PathBuf>,
) -> Result<(), String> {
    let expected = format!("Prelo {version} · {}", Os::current()?.as_str());
    if rendered != expected {
        return Err(format!("esperado {expected:?}, renderizado {rendered:?}"));
    }
    if elapsed_ms > MAX_ELAPSED_MS {
        return Err(format!("levou {elapsed_ms} ms; limite {MAX_ELAPSED_MS} ms"));
    }
    // SC-006: a prévia e os diagnósticos não podem depender de um `typst` instalado.
    if let Some(found) = typst_on_path {
        return Err(format!(
            "`typst` está no PATH ({}); o autoteste roda sem ele (SC-006)",
            found.display()
        ));
    }
    if let Some(failure) = &scenario.failure {
        return Err(format!("roteiro do editor: {failure}"));
    }
    if !scenario.project_opened {
        return Err("o arquivo principal da fixture não abriu no editor".into());
    }
    if !scenario.preview_changed {
        return Err("a prévia não mudou depois da edição".into());
    }
    if scenario.diagnostic_line != Some(scenario.error_line) {
        return Err(format!(
            "erro escrito na linha {}, diagnóstico na linha {:?}",
            scenario.error_line, scenario.diagnostic_line
        ));
    }
    if !scenario.editor_marked {
        return Err("o editor não marcou o erro".into());
    }
    if !scenario.compiler_shown.contains(EMBEDDED_TYPST) {
        return Err(format!(
            "versão do compilador não aparece no app: {:?}",
            scenario.compiler_shown
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{check, copy_fixture, find_on_path, Scenario, MAX_ELAPSED_MS};
    use std::path::Path;

    const V: &str = env!("CARGO_PKG_VERSION");

    fn expected() -> String {
        format!(
            "Prelo {V} · {}",
            crate::app_info::Os::current().unwrap().as_str()
        )
    }

    fn ok_scenario() -> Scenario {
        Scenario {
            project_opened: true,
            preview_changed: true,
            error_line: 30,
            diagnostic_line: Some(30),
            editor_marked: true,
            compiler_shown: format!("Typst {}", crate::commands::EMBEDDED_TYPST),
            failure: None,
        }
    }

    #[test]
    fn aceita_texto_exato_dentro_do_tempo() {
        assert!(check(&expected(), V, MAX_ELAPSED_MS, &ok_scenario(), None).is_ok());
    }

    #[test]
    fn recusa_texto_diferente() {
        let s = ok_scenario();
        assert!(check("Prelo", V, 10, &s, None).is_err());
        assert!(check(&format!("{} extra", expected()), V, 10, &s, None).is_err());
        assert!(check("Carregando…", V, 10, &s, None).is_err());
    }

    #[test]
    fn recusa_acima_de_3_segundos() {
        assert!(check(&expected(), V, MAX_ELAPSED_MS + 1, &ok_scenario(), None).is_err());
    }

    #[test]
    fn recusa_roteiro_do_editor_incompleto() {
        let e = expected();
        type Change = Box<dyn Fn(&mut Scenario)>;
        let cases: Vec<Change> = vec![
            Box::new(|s| s.project_opened = false),
            Box::new(|s| s.preview_changed = false),
            Box::new(|s| s.diagnostic_line = None),
            Box::new(|s| s.diagnostic_line = Some(29)),
            Box::new(|s| s.editor_marked = false),
            Box::new(|s| s.compiler_shown = "Typst 0.0.0".into()),
            Box::new(|s| s.failure = Some("prévia não chegou".into())),
        ];
        for change in cases {
            let mut s = ok_scenario();
            change(&mut s);
            assert!(check(&e, V, 10, &s, None).is_err(), "deveria recusar {s:?}");
        }
    }

    #[test]
    fn recusa_typst_no_path_sc_006() {
        let err = check(
            &expected(),
            V,
            10,
            &ok_scenario(),
            Some("/usr/bin/typst".into()),
        )
        .unwrap_err();
        assert!(err.contains("PATH"), "{err}");
    }

    #[test]
    fn acha_executavel_no_path() {
        let dir = tempfile::tempdir().unwrap();
        let name = if cfg!(windows) { "typst.exe" } else { "typst" };
        std::fs::write(dir.path().join(name), b"").unwrap();
        let path = std::env::join_paths([dir.path()]).unwrap();
        assert_eq!(
            find_on_path("typst", Some(path)),
            Some(dir.path().join(name))
        );
        let empty = tempfile::tempdir().unwrap();
        let path = std::env::join_paths([empty.path()]).unwrap();
        assert_eq!(find_on_path("typst", Some(path)), None);
        assert_eq!(find_on_path("typst", None), None);
    }

    #[test]
    fn copia_a_fixture_inteira_para_pasta_temporaria() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/artigo-basico");
        let dest = tempfile::tempdir().unwrap();
        copy_fixture(&src, dest.path()).unwrap();
        assert!(dest.path().join("main.typ").is_file());
        assert!(dest.path().join("capitulos/metodo.typ").is_file());
        assert_eq!(
            std::fs::read(src.join("main.typ")).unwrap(),
            std::fs::read(dest.path().join("main.typ")).unwrap()
        );
    }
}
