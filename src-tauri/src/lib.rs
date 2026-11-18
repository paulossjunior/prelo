/// Monta o app Tauri. Comandos IPC são registrados nas histórias seguintes.
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("falha ao iniciar o Prelo");
}
