//! Mudanças externas nos arquivos abertos (specs/003-editor-local/research.md R10, FR-008,
//! data-model.md#openfile; Princípio III).
//!
//! O observador (`notify`) acompanha a raiz do projeto, sem seguir links, e só repassa caminhos
//! relativos dentro dela que não sejam ocultos: o temporário da gravação atômica
//! (`.prelo-save-*.tmp`), `.prelo/` e `.git/` ficam de fora. Cada evento é só um aviso: o
//! núcleo relê o arquivo pela regra de escopo e decide pelo conteúdo, comparando com
//! `OpenFile.lastSavedHash`. Por isso o evento da própria gravação (o `rename` do temporário) é
//! ignorado, eventos repetidos não fazem mal e nenhuma decisão depende da ordem ou da hora em
//! que o sistema entrega os eventos.
//!
//! - conteúdo igual ao último lido ou gravado pelo núcleo → ignorado;
//! - sem alteração local pendente → recarrega (versão nova), recompila e avisa a interface com
//!   o evento `external_change_reloaded { path, version }` (o editor busca o texto novo);
//! - com alteração local pendente → conflito: evento `external_change_conflict`, nada é gravado
//!   por cima do disco nem descartado até `resolve_external_change` (manter a minha / usar a do
//!   disco).

use std::path::{Component, Path};

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use super::save::{sha256, OpenFile};
use super::scope::MAX_FILE_BYTES;

/// Código de erro de gravar um arquivo com conflito pendente (e nome do evento do contrato).
pub const CONFLICT: &str = "external_change_conflict";
/// Evento do contrato: o arquivo aberto foi recarregado do disco sem alteração local.
pub const RELOADED: &str = "external_change_reloaded";
const NOT_UTF8: &str = "not_utf8";
const TOO_LARGE: &str = "too_large";

/// O que a leitura do disco depois de um evento fez com o arquivo aberto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskChange {
    /// Nada a fazer: o conteúdo é o que o núcleo já conhece (inclusive a própria gravação), o
    /// arquivo não está aberto ou já há conflito esperando a escolha.
    Ignored,
    /// Sem alteração local: o texto do disco entrou no lugar (versão nova); recompilar.
    Reloaded,
    /// Com alteração local (ou disco ilegível como texto): avisar a interface e esperar.
    Conflict,
}

/// Escolha de `resolve_external_change`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Keep {
    Mine,
    Disk,
}

impl OpenFile {
    /// `true` enquanto uma mudança externa espera a escolha da pessoa.
    pub fn in_conflict(&self) -> bool {
        self.conflict
    }

    /// Decide o que fazer com `bytes`, o conteúdo atual do disco.
    pub fn disk_changed(&mut self, bytes: Vec<u8>) -> DiskChange {
        if self.conflict {
            return DiskChange::Ignored;
        }
        let hash = sha256(&bytes);
        if hash == self.last_saved_hash {
            return DiskChange::Ignored;
        }
        if hash == sha256(self.text.as_bytes()) {
            // O disco ficou igual ao texto em memória: nada a perder dos dois lados.
            self.last_saved_hash = hash;
            self.dirty = false;
            return DiskChange::Ignored;
        }
        if self.dirty {
            self.conflict = true;
            return DiskChange::Conflict;
        }
        match self.replace_with_disk(bytes) {
            Ok(_) => DiskChange::Reloaded,
            Err(_) => self.mark_conflict(),
        }
    }

    /// O disco mudou para algo que não dá para mostrar (grande demais): nunca gravar por cima
    /// sem a escolha da pessoa.
    pub fn mark_conflict(&mut self) -> DiskChange {
        if self.conflict {
            return DiskChange::Ignored;
        }
        self.conflict = true;
        DiskChange::Conflict
    }

    /// "Manter a minha": o texto em memória fica e volta a ser gravado (sujo, inclusive pelo
    /// salvamento automático), substituindo a versão externa.
    pub fn keep_mine(&mut self) {
        if self.conflict {
            self.conflict = false;
            self.dirty = true;
        }
    }

    /// "Usar a do disco": o texto passa a ser `bytes`, limpo, numa versão nova. Erro
    /// (`not_utf8`, `too_large`) → nada muda, conflito mantido.
    pub fn keep_disk(&mut self, bytes: Vec<u8>) -> Result<u64, &'static str> {
        self.replace_with_disk(bytes)?;
        self.conflict = false;
        Ok(self.version)
    }

    fn replace_with_disk(&mut self, bytes: Vec<u8>) -> Result<(), &'static str> {
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err(TOO_LARGE);
        }
        let hash = sha256(&bytes);
        let text = String::from_utf8(bytes).map_err(|_| NOT_UTF8)?;
        self.text = text.into();
        // Versão nova: edição ainda baseada no texto anterior vira `version_mismatch`.
        self.version += 1;
        self.dirty = false;
        self.last_saved_hash = hash;
        Ok(())
    }
}

/// Caminho relativo à raiz (com `/`) de um caminho de evento, ou `None` se estiver fora da
/// raiz, tiver `.`/`..`, não for UTF-8 ou passar por algo oculto (temporário da gravação,
/// `.prelo/`, `.git/`).
pub fn relative_event_path(root: &Path, path: &Path) -> Option<String> {
    let inside = path.strip_prefix(root).ok()?;
    let mut parts = Vec::new();
    for component in inside.components() {
        match component {
            Component::Normal(name) => {
                let name = name.to_str()?;
                if name.starts_with('.') {
                    return None;
                }
                parts.push(name);
            }
            _ => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// Eventos que podem ter mudado o conteúdo de um arquivo (acesso e remoção não mudam o que dá
/// para reler).
fn may_change_content(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Any | EventKind::Create(_) | EventKind::Modify(_) | EventKind::Other
    )
}

/// Observador da raiz de um projeto; parar = soltar.
pub struct ProjectWatcher {
    _inner: RecommendedWatcher,
}

impl std::fmt::Debug for ProjectWatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProjectWatcher")
    }
}

/// Observa `root` (canônica) recursivamente, sem seguir links, e chama `on_change` com o
/// caminho relativo de cada arquivo possivelmente alterado. `on_change` roda na thread do
/// observador: deve só repassar (o núcleo processa em outra thread).
pub fn watch(
    root: &Path,
    on_change: impl Fn(String) + Send + 'static,
) -> notify::Result<ProjectWatcher> {
    let base = root.to_owned();
    let handler = move |result: notify::Result<Event>| {
        let Ok(event) = result else {
            return;
        };
        if !may_change_content(&event.kind) {
            return;
        }
        for path in &event.paths {
            if let Some(relative) = relative_event_path(&base, path) {
                on_change(relative);
            }
        }
    };
    let config = Config::default().with_follow_symlinks(false);
    let mut inner = RecommendedWatcher::new(handler, config)?;
    inner.watch(root, RecursiveMode::Recursive)?;
    Ok(ProjectWatcher { _inner: inner })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::AppState;
    use crate::fs::save::{write_atomic, Change};
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    fn change(from: usize, to: usize, insert: &str) -> Change {
        Change {
            from,
            to,
            insert: insert.into(),
        }
    }

    fn opened(text: &str) -> OpenFile {
        OpenFile::from_bytes(text.as_bytes().to_vec()).unwrap()
    }

    // ------------------------------------------------ decisão por conteúdo (OpenFile)

    #[test]
    fn propria_gravacao_e_ignorada() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("main.typ");
        std::fs::write(&path, "a").unwrap();
        let mut file = opened("a");
        file.edit(0, &[change(1, 1, "b")]).unwrap();
        file.save(&path).unwrap();
        let disk = std::fs::read(&path).unwrap();
        assert_eq!(file.disk_changed(disk), DiskChange::Ignored);
        assert_eq!(file.text(), "ab");
        assert_eq!(file.version(), 1, "própria gravação não muda a versão");
        assert!(!file.is_dirty());
        assert!(!file.in_conflict());
    }

    #[test]
    fn evento_da_propria_gravacao_depois_de_nova_edicao_nao_e_conflito() {
        // A pessoa grava e volta a digitar antes de o evento do `rename` chegar.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("main.typ");
        let mut file = opened("a");
        file.edit(0, &[change(1, 1, "b")]).unwrap();
        file.save(&path).unwrap();
        file.edit(1, &[change(2, 2, "c")]).unwrap();
        assert_eq!(
            file.disk_changed(std::fs::read(&path).unwrap()),
            DiskChange::Ignored
        );
        assert_eq!(file.text(), "abc");
        assert!(file.is_dirty(), "a edição nova continua por gravar");
    }

    #[test]
    fn mudanca_externa_sem_alteracao_local_recarrega_em_versao_nova() {
        let mut file = opened("antigo");
        assert_eq!(file.disk_changed(b"novo".to_vec()), DiskChange::Reloaded);
        assert_eq!(file.text(), "novo");
        assert_eq!(file.version(), 1);
        assert!(!file.is_dirty());
        assert_eq!(file.last_saved_hash(), sha256(b"novo"));
        // O mesmo evento repetido não recarrega de novo.
        assert_eq!(file.disk_changed(b"novo".to_vec()), DiskChange::Ignored);
        assert_eq!(file.version(), 1);
    }

    #[test]
    fn mudanca_externa_com_alteracao_local_e_conflito_sem_descartar_nada() {
        let mut file = opened("a");
        file.edit(0, &[change(1, 1, " minha")]).unwrap();
        assert_eq!(file.disk_changed(b"deles".to_vec()), DiskChange::Conflict);
        assert!(file.in_conflict());
        assert_eq!(file.text(), "a minha", "o texto local não é descartado");
        assert!(file.is_dirty());
        // Repetições do evento não avisam de novo nem mudam nada.
        assert_eq!(file.disk_changed(b"outra".to_vec()), DiskChange::Ignored);
        assert_eq!(file.text(), "a minha");
        // Editar continua possível durante o conflito.
        assert_eq!(file.edit(1, &[change(0, 0, ">")]), Ok(2));
    }

    #[test]
    fn conflito_suspende_toda_gravacao() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("main.typ");
        std::fs::write(&path, "deles").unwrap();
        let mut file = opened("a");
        file.edit(0, &[change(1, 1, "b")]).unwrap();
        assert_eq!(file.disk_changed(b"deles".to_vec()), DiskChange::Conflict);
        assert!(
            !file.autosave_due(Instant::now() + Duration::from_secs(3600)),
            "salvamento automático não grava por cima da versão externa"
        );
        assert_eq!(file.save(&path), Err(CONFLICT));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "deles");
        assert!(file.is_dirty());
    }

    #[test]
    fn disco_igual_ao_texto_local_limpa_sem_conflito() {
        let mut file = opened("a");
        file.edit(0, &[change(1, 1, "b")]).unwrap();
        assert_eq!(file.disk_changed(b"ab".to_vec()), DiskChange::Ignored);
        assert!(!file.is_dirty());
        assert!(!file.in_conflict());
        assert_eq!(file.version(), 1);
    }

    #[test]
    fn disco_que_nao_e_utf8_nunca_e_sobrescrito_sem_escolha() {
        let mut file = opened("a");
        assert_eq!(file.disk_changed(vec![0x63, 0xe7]), DiskChange::Conflict);
        assert_eq!(file.text(), "a");
        assert!(file.in_conflict());
        assert_eq!(file.keep_disk(vec![0x63, 0xe7]), Err("not_utf8"));
        assert!(
            file.in_conflict(),
            "falha ao usar a do disco mantém o conflito"
        );
    }

    #[test]
    fn manter_a_minha_volta_a_gravar_por_cima_do_disco() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("main.typ");
        std::fs::write(&path, "deles").unwrap();
        let mut file = opened("a");
        file.edit(0, &[change(1, 1, "b")]).unwrap();
        file.disk_changed(b"deles".to_vec());
        file.keep_mine();
        assert!(!file.in_conflict());
        assert!(file.is_dirty());
        assert_eq!(file.version(), 1);
        file.save(&path).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "ab");
    }

    #[test]
    fn usar_a_do_disco_troca_o_texto_e_limpa() {
        let mut file = opened("a");
        file.edit(0, &[change(1, 1, "b")]).unwrap();
        file.disk_changed(b"deles".to_vec());
        assert_eq!(file.keep_disk(b"deles 2".to_vec()), Ok(2));
        assert_eq!(file.text(), "deles 2");
        assert!(!file.is_dirty());
        assert!(!file.in_conflict());
        assert_eq!(file.last_saved_hash(), sha256(b"deles 2"));
    }

    // ------------------------------------------------ filtro de caminhos do observador

    #[test]
    fn caminhos_do_observador_ficam_dentro_da_raiz_e_sem_ocultos() {
        let root = Path::new("/proj");
        let rel = |p: &str| relative_event_path(root, Path::new(p));
        assert_eq!(rel("/proj/main.typ").as_deref(), Some("main.typ"));
        assert_eq!(
            rel("/proj/capitulos/intro.typ").as_deref(),
            Some("capitulos/intro.typ")
        );
        assert_eq!(
            rel("/proj/.prelo-save-abc.tmp"),
            None,
            "temporário da gravação"
        );
        assert_eq!(rel("/proj/capitulos/.prelo-save-x.tmp"), None);
        assert_eq!(rel("/proj/.prelo/project.json"), None);
        assert_eq!(rel("/proj/.git/index"), None);
        assert_eq!(rel("/outro/main.typ"), None, "fora da raiz");
        assert_eq!(
            rel("/projeto/main.typ"),
            None,
            "prefixo de nome não é a raiz"
        );
        assert_eq!(rel("/proj/../etc/passwd"), None);
        assert_eq!(rel("/proj"), None, "a própria raiz");
    }

    // ------------------------------------------------ núcleo (AppState)

    fn project() -> (tempfile::TempDir, PathBuf, AppState) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.typ"), "= Título\n").unwrap();
        let state = AppState::default();
        state.open_project(dir.path()).unwrap();
        let main = std::fs::canonicalize(dir.path()).unwrap().join("main.typ");
        (dir, main, state)
    }

    #[test]
    fn nucleo_ignora_o_evento_da_propria_gravacao() {
        let (_dir, main, state) = project();
        state.open_file("main.typ").unwrap();
        state
            .edit_file("main.typ", 0, &[change(0, 0, "x")])
            .unwrap();
        state.save_file("main.typ").unwrap();
        assert_eq!(state.handle_disk_change("main.typ"), DiskChange::Ignored);
        let opened = state.open_file("main.typ").unwrap();
        assert_eq!((opened.text.as_str(), opened.version), ("x= Título\n", 1));
        assert_eq!(std::fs::read_to_string(main).unwrap(), "x= Título\n");
    }

    #[test]
    fn nucleo_recarrega_e_recompila_sem_alteracao_local() {
        let (_dir, main, state) = project();
        state.open_file("main.typ").unwrap();
        let (tx, rx) = mpsc::channel();
        state.subscribe_preview(move |u| {
            let _ = tx.send(u.compile_id);
        });
        // Primeira compilação (abertura) entregue antes da mudança.
        let first = rx.recv_timeout(Duration::from_secs(60)).unwrap();
        std::fs::write(&main, "= Outro\n").unwrap();
        assert_eq!(state.handle_disk_change("main.typ"), DiskChange::Reloaded);
        let opened = state.open_file("main.typ").unwrap();
        assert_eq!((opened.text.as_str(), opened.version), ("= Outro\n", 1));
        let next = rx.recv_timeout(Duration::from_secs(60)).unwrap();
        assert!(next > first, "recompilou depois de recarregar");
        // Edição baseada no texto anterior é recusada.
        assert_eq!(
            state.edit_file("main.typ", 0, &[change(0, 0, "x")]),
            Err("version_mismatch".into())
        );
    }

    #[test]
    fn nucleo_com_alteracao_local_avisa_conflito_e_espera_a_escolha() {
        let (_dir, main, state) = project();
        state.open_file("main.typ").unwrap();
        state
            .edit_file("main.typ", 0, &[change(0, 0, "minha ")])
            .unwrap();
        std::fs::write(&main, "deles\n").unwrap();
        assert_eq!(state.handle_disk_change("main.typ"), DiskChange::Conflict);
        // Nada é gravado por cima do disco nem descartado da memória.
        assert!(state
            .autosave(Instant::now() + Duration::from_secs(3600))
            .is_empty());
        assert_eq!(state.save_file("main.typ"), Err(CONFLICT.into()));
        assert_eq!(std::fs::read_to_string(&main).unwrap(), "deles\n");
        assert_eq!(
            state.open_file("main.typ").unwrap().text,
            "minha = Título\n"
        );
        // Fechar com conflito pendente: o primeiro pedido é recusado (a pessoa decide).
        assert!(!state.flush_for_close());
        assert_eq!(std::fs::read_to_string(&main).unwrap(), "deles\n");
    }

    #[test]
    fn resolver_usando_a_do_disco() {
        let (_dir, main, state) = project();
        state.open_file("main.typ").unwrap();
        state
            .edit_file("main.typ", 0, &[change(0, 0, "minha ")])
            .unwrap();
        std::fs::write(&main, "deles\n").unwrap();
        state.handle_disk_change("main.typ");
        let opened = state
            .resolve_external_change("main.typ", Keep::Disk)
            .unwrap();
        assert_eq!((opened.text.as_str(), opened.version), ("deles\n", 2));
        assert_eq!(state.save_file("main.typ").map(|_| ()), Ok(()));
        assert_eq!(std::fs::read_to_string(&main).unwrap(), "deles\n");
    }

    #[test]
    fn resolver_mantendo_a_minha() {
        let (_dir, main, state) = project();
        state.open_file("main.typ").unwrap();
        state
            .edit_file("main.typ", 0, &[change(0, 0, "minha ")])
            .unwrap();
        std::fs::write(&main, "deles\n").unwrap();
        state.handle_disk_change("main.typ");
        let opened = state
            .resolve_external_change("main.typ", Keep::Mine)
            .unwrap();
        assert_eq!(
            (opened.text.as_str(), opened.version),
            ("minha = Título\n", 1)
        );
        state.save_file("main.typ").unwrap();
        assert_eq!(std::fs::read_to_string(&main).unwrap(), "minha = Título\n");
    }

    #[test]
    fn resolver_valida_o_caminho_e_exige_arquivo_aberto() {
        let (_dir, _main, state) = project();
        assert_eq!(
            state.resolve_external_change("/etc/passwd", Keep::Disk),
            Err("path_outside_project".into())
        );
        assert_eq!(
            state.resolve_external_change("../fora.typ", Keep::Mine),
            Err("path_outside_project".into())
        );
        assert_eq!(
            state.resolve_external_change("main.typ", Keep::Disk),
            Err("not_found".into()),
            "arquivo não aberto"
        );
    }

    #[test]
    fn evento_de_arquivo_nao_aberto_ou_fora_da_raiz_e_ignorado() {
        let (_dir, _main, state) = project();
        assert_eq!(state.handle_disk_change("main.typ"), DiskChange::Ignored);
        assert_eq!(state.handle_disk_change("../fora.typ"), DiskChange::Ignored);
        assert_eq!(
            state.handle_disk_change("nao-existe.typ"),
            DiskChange::Ignored
        );
        assert_eq!(
            AppState::default().handle_disk_change("main.typ"),
            DiskChange::Ignored
        );
    }

    #[cfg(unix)]
    #[test]
    fn link_para_fora_da_raiz_nunca_e_lido() {
        let fora = tempfile::tempdir().unwrap();
        std::fs::write(fora.path().join("segredo.typ"), "segredo").unwrap();
        let (dir, _main, state) = project();
        std::fs::write(dir.path().join("cap.typ"), "cap").unwrap();
        state.open_file("cap.typ").unwrap();
        // O arquivo aberto vira link para fora depois de aberto.
        std::fs::remove_file(dir.path().join("cap.typ")).unwrap();
        std::os::unix::fs::symlink(fora.path().join("segredo.typ"), dir.path().join("cap.typ"))
            .unwrap();
        assert_eq!(state.handle_disk_change("cap.typ"), DiskChange::Ignored);
        assert_eq!(
            state.open_file("cap.typ").map(|o| o.text),
            Err("path_outside_project".into()),
            "o link para fora nunca vira texto do editor"
        );
    }

    // ------------------------------------------------ observador real (espera limitada)

    #[test]
    fn observador_entrega_mudanca_externa_e_nao_o_temporario() {
        let dir = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(dir.path()).unwrap();
        std::fs::write(root.join("main.typ"), "a").unwrap();
        let (tx, rx) = mpsc::channel();
        let _watcher = watch(&root, move |p| {
            let _ = tx.send(p);
        })
        .unwrap();
        // Gravação atômica: cria `.prelo-save-*.tmp` e renomeia para `main.typ`.
        write_atomic(&root.join("main.typ"), b"b").unwrap();
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut seen = Vec::new();
        while !seen.iter().any(|p: &String| p == "main.typ") {
            let left = deadline.saturating_duration_since(Instant::now());
            match rx.recv_timeout(left) {
                Ok(p) => seen.push(p),
                Err(_) => panic!("nenhum evento de main.typ em 20 s; vistos: {seen:?}"),
            }
        }
        assert!(
            seen.iter()
                .all(|p| !p.starts_with('.') && !p.contains("/.")),
            "temporário repassado: {seen:?}"
        );
    }
}
