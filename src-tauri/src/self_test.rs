//! Autoteste ponta a ponta (T012, T031). Só existe com a feature `self-test`; nunca em release.
//! Contratos: specs/001-app-skeleton/contracts/ipc.md e specs/003-editor-local/contracts/ipc.md,
//! `self_test_report`, `self_test_open_project` e `self_test_write_external`.
//!
//! Encerramento forçado (T077; SC-009): com `PRELO_SELF_TEST_CRASH=write` e
//! `PRELO_SELF_TEST_CRASH_DIR=<pasta>`, a fixture é copiada para `<pasta>/artigo-basico` (fora
//! de pasta temporária: o processo vai ser morto) e a interface digita marcas, cada uma avisada
//! por `self_test_mark` quando o núcleo já tem o texto. Com `PRELO_SELF_TEST_CRASH=reopen`, a
//! mesma cópia é reaberta e `self_test_reopened` avisa o texto que o editor mostrou. O roteiro
//! (scripts/self-test.mjs, scripts/forced-kill.mjs) mata e confere.

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
    /// Problemas de layout e violações da CSP medidos na webview real (src/layout-check.ts).
    #[serde(default)]
    pub layout_problems: Vec<String>,
    /// Mudança externa sem alteração local (FR-008, T084): o editor passou a mostrar o texto
    /// gravado no disco por "outro programa" (`self_test_write_external`).
    #[serde(default)]
    pub external_reloaded: bool,
    /// Linha (1-based) do erro no texto externo e linha do diagnóstico no editor recarregado.
    #[serde(default)]
    pub external_error_line: u32,
    #[serde(default)]
    pub external_diagnostic_line: Option<u32>,
}

/// Cópia temporária da fixture: o roteiro edita e o salvamento automático grava nela, nunca
/// no repositório. Apagada quando o estado do app é descartado.
#[derive(Default)]
pub struct FixtureCopy(Mutex<Option<tempfile::TempDir>>);

/// Modo do roteiro de encerramento forçado (T077).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CrashMode {
    /// Digitar marcas até o processo ser morto.
    Write,
    /// Reabrir a cópia e avisar o texto mostrado.
    Reopen,
}

impl CrashMode {
    /// Lê `PRELO_SELF_TEST_CRASH`; qualquer outro valor é o autoteste normal.
    pub fn parse(value: Option<&str>) -> Option<Self> {
        match value? {
            "write" => Some(Self::Write),
            "reopen" => Some(Self::Reopen),
            _ => None,
        }
    }

    fn from_env() -> Option<(Self, PathBuf)> {
        let mode = Self::parse(std::env::var("PRELO_SELF_TEST_CRASH").ok().as_deref())?;
        let dir = std::env::var_os("PRELO_SELF_TEST_CRASH_DIR").filter(|d| !d.is_empty())?;
        Some((mode, PathBuf::from(dir)))
    }
}

/// Modo do roteiro de encerramento forçado, se houver (a interface escolhe o roteiro).
#[tauri::command]
pub fn self_test_crash_mode() -> Option<CrashMode> {
    CrashMode::from_env().map(|(mode, _)| mode)
}

/// Raiz do projeto do roteiro de encerramento forçado dentro de `dir`. `Write` copia a fixture
/// se a cópia ainda não existir; `Reopen` exige a cópia.
pub fn crash_project(mode: CrashMode, dir: &Path) -> Result<PathBuf, String> {
    let root = dir.join("artigo-basico");
    match mode {
        CrashMode::Write if !root.exists() => {
            copy_fixture(Path::new(FIXTURE), &root).map_err(|e| format!("copiar a fixture: {e}"))?
        }
        CrashMode::Reopen if !root.is_dir() => {
            return Err(format!("cópia ausente para reabrir: {}", root.display()))
        }
        _ => {}
    }
    Ok(root)
}

/// `text` tem a marca `n` numa linha própria (`marca-1` não casa com `marca-10`).
pub fn has_mark(text: &str, n: u32) -> bool {
    let mark = format!("marca-{n}");
    text.lines().any(|line| line.trim() == mark)
}

/// Prazo para o texto da marca chegar ao núcleo depois de digitada.
const MARK_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

/// Avisa no stderr (`self-test: marca <n>`) quando o arquivo aberto `path` já tem a marca `n`
/// no núcleo: a partir daí, só o salvamento automático separa a marca do disco.
#[tauri::command(async)]
pub fn self_test_mark(state: State<'_, AppState>, path: String, n: u32) -> Result<(), String> {
    let deadline = std::time::Instant::now() + MARK_WAIT;
    loop {
        let opened = state.open_file(&path)?;
        if has_mark(&opened.text, n) {
            eprintln!("self-test: marca {n}");
            return Ok(());
        }
        if std::time::Instant::now() > deadline {
            return Err(format!("marca-{n} não chegou ao núcleo em {MARK_WAIT:?}"));
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

/// Avisa o texto que o editor mostrou ao reabrir (`self-test: reaberto <json>`) e encerra.
#[tauri::command]
pub fn self_test_reopened<R: Runtime>(app: AppHandle<R>, text: String) {
    match serde_json::to_string(&text) {
        Ok(json) => {
            eprintln!("self-test: reaberto {json}");
            app.exit(0);
        }
        Err(e) => {
            eprintln!("self-test: FALHA: texto reaberto: {e}");
            app.exit(1);
        }
    }
}

/// Abre uma cópia nova da fixture `artigo-basico` como projeto, sem diálogo nativo. No roteiro
/// de encerramento forçado, abre a cópia de `PRELO_SELF_TEST_CRASH_DIR` ([`crash_project`]).
#[tauri::command(async)]
pub fn self_test_open_project(
    state: State<'_, AppState>,
    copy: State<'_, FixtureCopy>,
) -> Result<ProjectInfo, String> {
    if let Some((mode, dir)) = CrashMode::from_env() {
        let root = crash_project(mode, &dir)?;
        return state
            .open_project(&root)
            .map_err(|e| format!("abrir a cópia: {e}"));
    }
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

/// Grava `text` no `main.typ` da cópia temporária da fixture como outro programa faria
/// (temporário oculto + `rename`), para o roteiro conferir a mudança externa (FR-008, T084).
/// Só escreve dentro da cópia aberta por [`self_test_open_project`].
#[tauri::command(async)]
pub fn self_test_write_external(copy: State<'_, FixtureCopy>, text: String) -> Result<(), String> {
    let guard = copy
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = guard.as_ref().ok_or("nenhuma fixture aberta")?;
    let root = dir.path().join("artigo-basico");
    let tmp = root.join(".prelo-self-test-external.tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("gravar a mudança externa: {e}"))?;
    std::fs::rename(&tmp, root.join("main.typ"))
        .map_err(|e| format!("gravar a mudança externa: {e}"))
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
    // FR-008 (T084): o arquivo aberto mudou no disco sem alteração local → o editor mostra o
    // texto novo e o diagnóstico cai numa linha que o editor tem.
    if !scenario.external_reloaded {
        return Err("mudança externa sem alteração local não chegou ao editor".into());
    }
    if scenario.external_error_line == 0
        || scenario.external_diagnostic_line != Some(scenario.external_error_line)
    {
        return Err(format!(
            "mudança externa: erro na linha {} do disco, diagnóstico na linha {:?} do editor",
            scenario.external_error_line, scenario.external_diagnostic_line
        ));
    }
    if !scenario.compiler_shown.contains(EMBEDDED_TYPST) {
        return Err(format!(
            "versão do compilador não aparece no app: {:?}",
            scenario.compiler_shown
        ));
    }
    // O jsdom não aplica CSP nem faz layout: só a webview real vê o editor sem tema base.
    if !scenario.layout_problems.is_empty() {
        return Err(format!(
            "layout no app real: {}",
            scenario.layout_problems.join("; ")
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        check, copy_fixture, crash_project, find_on_path, has_mark, CrashMode, Scenario,
        MAX_ELAPSED_MS,
    };
    use std::path::Path;

    #[test]
    fn modo_de_encerramento_forcado_so_com_valor_conhecido() {
        assert_eq!(CrashMode::parse(Some("write")), Some(CrashMode::Write));
        assert_eq!(CrashMode::parse(Some("reopen")), Some(CrashMode::Reopen));
        assert_eq!(CrashMode::parse(Some("")), None);
        assert_eq!(CrashMode::parse(Some("WRITE")), None);
        assert_eq!(CrashMode::parse(None), None);
        assert_eq!(
            serde_json::to_string(&CrashMode::Reopen).unwrap(),
            "\"reopen\""
        );
    }

    #[test]
    fn copia_para_escrever_e_reabre_a_mesma_copia() {
        let dir = tempfile::tempdir().unwrap();
        assert!(crash_project(CrashMode::Reopen, dir.path()).is_err());
        let root = crash_project(CrashMode::Write, dir.path()).unwrap();
        assert!(root.join("main.typ").is_file());
        // A cópia editada não é sobrescrita pela fixture ao abrir de novo.
        std::fs::write(root.join("main.typ"), "editado").unwrap();
        assert_eq!(crash_project(CrashMode::Write, dir.path()).unwrap(), root);
        assert_eq!(crash_project(CrashMode::Reopen, dir.path()).unwrap(), root);
        assert_eq!(
            std::fs::read_to_string(root.join("main.typ")).unwrap(),
            "editado"
        );
    }

    #[test]
    fn marca_so_em_linha_propria() {
        assert!(has_mark("= A\nmarca-1\n", 1));
        assert!(has_mark("= A\n  marca-10  ", 10));
        assert!(!has_mark("= A\nmarca-10\n", 1));
        assert!(!has_mark("texto marca-1 no meio", 1));
    }

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
            layout_problems: Vec::new(),
            external_reloaded: true,
            external_error_line: 131,
            external_diagnostic_line: Some(131),
        }
    }

    #[test]
    fn mudanca_externa_que_nao_chegou_ao_editor_e_recusada_com_a_descricao() {
        let mut s = ok_scenario();
        s.external_reloaded = false;
        let err = check(&expected(), V, 10, &s, None).unwrap_err();
        assert!(err.contains("mudança externa"), "{err}");
    }

    #[test]
    fn layout_problems_e_opcional_no_json_e_vazio_por_padrao() {
        let s: Scenario = serde_json::from_str(
            r#"{"projectOpened":true,"previewChanged":true,"errorLine":3,"diagnosticLine":3,
               "editorMarked":true,"compilerShown":"Typst"}"#,
        )
        .unwrap();
        assert!(s.layout_problems.is_empty());
    }

    #[test]
    fn recusa_problema_de_layout_no_app_real_com_a_descricao() {
        let mut s = ok_scenario();
        s.layout_problems = vec!["CSP bloqueou style-src-elem: inline".into()];
        let err = check(&expected(), V, 10, &s, None).unwrap_err();
        assert!(err.contains("CSP bloqueou style-src-elem"), "{err}");
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
            Box::new(|s| s.layout_problems = vec!["números de linha não estão numa coluna".into()]),
            Box::new(|s| s.external_reloaded = false),
            Box::new(|s| s.external_diagnostic_line = None),
            Box::new(|s| s.external_diagnostic_line = Some(16)),
            Box::new(|s| {
                s.external_error_line = 0;
                s.external_diagnostic_line = Some(0);
            }),
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
