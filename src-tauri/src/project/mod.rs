//! Projeto aberto (specs/003-editor-local; T032).
//!
//! - [`record`]: o registro `.prelo/project.json` (entrada não confiável).
//! - [`EMBEDDED_TYPST`]: versão do compilador embutido, gerada no build a partir do
//!   `Cargo.lock` (research R1).
//! - Comandos IPC de projeto que não recebem caminho: `open_project_dialog` (diálogo nativo
//!   mostrado pelo núcleo; a webview não tem permissão de diálogo nem de arquivo),
//!   `create_project_dialog` (US6; o projeto mínimo vem de [`create`]),
//!   `update_project_compiler_version` e `compiler_info`. `list_tree` e `choose_main_file`
//!   ficam em [`crate::commands`], junto da validação de caminho.
//!
//! O estado do projeto aberto (registro ausente criado ao abrir, estado `VersaoDiferente`,
//! aviso de principal ausente) mora em [`crate::commands::AppState`].

pub mod create;
pub mod record;

use serde::Serialize;
use serde_json::Value;
use tauri::ipc::Request;
use tauri::{Runtime, State, Window};
use tauri_plugin_dialog::DialogExt;

use crate::commands::{no_args, AppState, ProjectInfo};

/// Versão do compilador embutido: a de `typst` no `Cargo.lock` (build.rs; T026 confere).
pub const EMBEDDED_TYPST: &str = env!("PRELO_EMBEDDED_TYPST");

/// Aviso de `ProjectInfo.warnings`: o arquivo principal (registrado ou `main.typ`) não existe;
/// a interface pede a escolha (`choose_main_file`). Até lá, a prévia mostra o diagnóstico de
/// arquivo não encontrado.
pub const MAIN_FILE_MISSING: &str = "main_file_missing";

/// Resposta de `compiler_info` (contracts/ipc.md).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CompilerInfo {
    pub embedded: String,
    /// Versão do registro do projeto aberto; `null` sem projeto ou com registro inválido.
    pub registered: Option<String>,
    /// `FontLicense[]` das fontes embutidas; preenchido com T052 (US3).
    pub fonts: Vec<Value>,
}

/// Pasta escolhida pela pessoa no diálogo nativo do núcleo. Abre o projeto (cria o registro
/// ausente) e devolve `ProjectInfo`; cancelado → `null`.
#[tauri::command(async)]
pub fn open_project_dialog<R: Runtime>(
    window: Window<R>,
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<Option<ProjectInfo>, String> {
    no_args(&request)?;
    let Some(folder) = window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title("Abrir projeto")
        .blocking_pick_folder()
    else {
        return Ok(None);
    };
    let root = folder.into_path().map_err(|_| "not_found".to_owned())?;
    state
        .open_project(&root)
        .map(Some)
        .map_err(|_| "not_found".into())
}

/// "Novo projeto" (US6; FR-002): pasta escolhida no diálogo nativo do núcleo; se vazia, ganha o
/// projeto mínimo ([`create`]) e abre. Cancelado → `null`; pasta com arquivos →
/// `folder_not_empty`, sem escrever nada.
#[tauri::command(async)]
pub fn create_project_dialog<R: Runtime>(
    window: Window<R>,
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<Option<ProjectInfo>, String> {
    no_args(&request)?;
    let Some(folder) = window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title("Novo projeto: escolha uma pasta vazia")
        .set_can_create_directories(true)
        .blocking_pick_folder()
    else {
        return Ok(None);
    };
    let folder = folder.into_path().map_err(|_| "not_found".to_owned())?;
    state.create_project(&folder).map(Some)
}

/// "Atualizar o projeto para a versão embutida" (FR-012): única ação que muda
/// `compiler.typst` no registro.
#[tauri::command(async)]
pub fn update_project_compiler_version(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<ProjectInfo, String> {
    no_args(&request)?;
    state.update_project_compiler_version()
}

#[tauri::command(async)]
pub fn compiler_info(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<CompilerInfo, String> {
    no_args(&request)?;
    Ok(state.compiler_info())
}
