//! Arquivo aberto no editor e sua gravação (specs/003-editor-local/data-model.md#openfile,
//! research R10; FR-007, FR-009; Princípio III).
//!
//! O núcleo guarda o texto de cada arquivo aberto ([`OpenFile`]). Gravar é atômico: o texto vai
//! para um arquivo temporário na mesma pasta, passa por `fsync` e só então substitui o original
//! por `rename`; qualquer falha deixa o original intacto e o arquivo continua `dirty`, com o
//! texto em memória. O salvamento automático vale [`AUTOSAVE_DELAY`] depois da última edição
//! (ou da última tentativa que falhou, para não insistir sem pausa num disco cheio).

use std::fs::File;
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::scope::MAX_FILE_BYTES;

/// Ociosidade depois da última edição que dispara o salvamento automático (FR-007, SC-009).
pub const AUTOSAVE_DELAY: Duration = Duration::from_secs(2);

const BAD_RANGE: &str = "bad_range";
const VERSION_MISMATCH: &str = "version_mismatch";
const NOT_UTF8: &str = "not_utf8";
const TOO_LARGE: &str = "too_large";
const WRITE_FAILED: &str = "write_failed";

/// `{ from, to, insert }` de `edit_file`, em unidades UTF-16 (as do CodeMirror) do texto
/// anterior à edição; mudanças em ordem crescente e sem sobreposição.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub from: usize,
    pub to: usize,
    pub insert: String,
}

/// Aplica as mudanças de uma vez; qualquer intervalo inválido (fora do texto, invertido, fora de
/// ordem, sobreposto ou no meio de um par substituto UTF-16) → `bad_range` sem aplicar nada.
pub fn apply_changes(text: &str, changes: &[Change]) -> Result<String, &'static str> {
    let mut out = String::with_capacity(text.len());
    // Cursor de conversão UTF-16 → byte, só avança (mudanças em ordem crescente).
    let mut chars = text.char_indices().peekable();
    let mut units = 0usize;
    let mut to_byte = |target: usize| -> Result<usize, &'static str> {
        while units < target {
            let (_, c) = chars.next().ok_or(BAD_RANGE)?;
            units += c.len_utf16();
        }
        if units != target {
            return Err(BAD_RANGE);
        }
        Ok(chars.peek().map_or(text.len(), |&(i, _)| i))
    };
    let mut last_units = 0usize;
    let mut last_byte = 0usize;
    for change in changes {
        if change.from > change.to || change.from < last_units {
            return Err(BAD_RANGE);
        }
        let from = to_byte(change.from)?;
        let to = to_byte(change.to)?;
        out.push_str(&text[last_byte..from]);
        out.push_str(&change.insert);
        last_units = change.to;
        last_byte = to;
    }
    out.push_str(&text[last_byte..]);
    Ok(out)
}

/// Arquivo aberto no editor: overlay em memória do que está no disco.
///
/// Estados (data-model.md): `Limpo → (edição) → Sujo → (2 s ociosos, ⌘S ou fechar) → Gravando →
/// Limpo`; `Gravando → (falha) → Sujo`.
#[derive(Debug)]
pub struct OpenFile {
    /// Compartilhado com a compilação sem cópia.
    pub(super) text: Arc<str>,
    pub(super) version: u64,
    pub(super) dirty: bool,
    pub(super) last_saved_hash: [u8; 32],
    /// Desde quando o arquivo está ocioso: última edição ou última gravação que falhou.
    idle_since: Instant,
    /// Mudança externa à espera da escolha da pessoa ([`super::watch`]): enquanto `true`, nada
    /// é gravado por cima do disco.
    pub(super) conflict: bool,
}

impl OpenFile {
    /// Arquivo lido do disco, limpo e na versão 0. Bytes que não são UTF-8 → `not_utf8`.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, &'static str> {
        let last_saved_hash = sha256(&bytes);
        let text = String::from_utf8(bytes).map_err(|_| NOT_UTF8)?;
        Ok(Self {
            text: text.into(),
            version: 0,
            dirty: false,
            last_saved_hash,
            idle_since: Instant::now(),
            conflict: false,
        })
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// O texto, compartilhado (a compilação lê o mesmo `Arc` sem copiar).
    pub fn shared_text(&self) -> Arc<str> {
        Arc::clone(&self.text)
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    /// `true` desde a última edição até gravar com sucesso.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// sha256 do último conteúdo lido ou gravado pelo próprio núcleo (T034 usa para ignorar o
    /// evento do observador causado pela própria gravação).
    pub fn last_saved_hash(&self) -> [u8; 32] {
        self.last_saved_hash
    }

    /// Aplica `changes` se `version` é a atual; devolve a nova versão. Versão antiga →
    /// `version_mismatch`, intervalo inválido → `bad_range`, texto acima do limite →
    /// `too_large`; em todos os erros nada muda.
    pub fn edit(&mut self, version: u64, changes: &[Change]) -> Result<u64, &'static str> {
        if version != self.version {
            return Err(VERSION_MISMATCH);
        }
        let text = apply_changes(&self.text, changes)?;
        if text.len() as u64 > MAX_FILE_BYTES {
            return Err(TOO_LARGE);
        }
        self.text = text.into();
        self.version += 1;
        self.dirty = true;
        self.idle_since = Instant::now();
        Ok(self.version)
    }

    /// Hora de salvar sozinho: sujo, sem conflito com o disco e ocioso há pelo menos
    /// [`AUTOSAVE_DELAY`] em `now`.
    pub fn autosave_due(&self, now: Instant) -> bool {
        self.dirty
            && !self.conflict
            && now.saturating_duration_since(self.idle_since) >= AUTOSAVE_DELAY
    }

    /// Grava o texto em `path` de forma atômica. Limpo → nada a gravar. Falha → `write_failed`,
    /// texto e `dirty` mantidos. Com conflito pendente → `external_change_conflict` sem tocar o
    /// disco (gravar descartaria a versão externa antes da escolha; Princípio III).
    pub fn save(&mut self, path: &Path) -> Result<(), &'static str> {
        self.save_with(path, io::Write::write_all)
    }

    /// Como [`save`](Self::save), com quem preenche o temporário (os testes simulam falha).
    fn save_with(
        &mut self,
        path: &Path,
        fill: impl FnOnce(&mut File, &[u8]) -> io::Result<()>,
    ) -> Result<(), &'static str> {
        if !self.dirty {
            return Ok(());
        }
        if self.conflict {
            return Err(super::watch::CONFLICT);
        }
        let bytes = self.text.as_bytes();
        match write_atomic_with(path, |file| fill(file, bytes)) {
            Ok(()) => {
                self.last_saved_hash = sha256(bytes);
                self.dirty = false;
                Ok(())
            }
            Err(_) => {
                // Continua sujo; o salvamento automático espera outra pausa antes de tentar.
                self.idle_since = Instant::now();
                Err(WRITE_FAILED)
            }
        }
    }
}

/// Grava `bytes` em `path` de forma atômica (FR-009): temporário na mesma pasta, `fsync`,
/// permissões do original e `rename`.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    write_atomic_with(path, |file| io::Write::write_all(file, bytes))
}

fn write_atomic_with(
    path: &Path,
    fill: impl FnOnce(&mut File) -> io::Result<()>,
) -> io::Result<()> {
    let dir = match path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    };
    // Oculto (começa com `.`): a árvore do projeto não mostra o temporário.
    let mut temp = tempfile::Builder::new()
        .prefix(".prelo-save-")
        .suffix(".tmp")
        .tempfile_in(dir)?;
    fill(temp.as_file_mut())?;
    // O temporário nasce com 0600; o arquivo gravado mantém as permissões do original.
    if let Ok(meta) = std::fs::metadata(path) {
        temp.as_file().set_permissions(meta.permissions())?;
    }
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|e| e.error)?;
    // O `rename` só é durável depois do `fsync` da pasta (Unix; no Windows não se abre pasta).
    #[cfg(unix)]
    if let Ok(d) = File::open(dir) {
        let _ = d.sync_all();
    }
    Ok(())
}

pub(super) fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// `SystemTime` em RFC 3339, UTC, com milissegundos (`savedAt` do contrato).
pub fn format_utc(time: SystemTime) -> String {
    let since = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = since.as_secs();
    let (days, rem) = (secs / 86_400, secs % 86_400);
    let (year, month, day) = civil_from_days(days as i64);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60,
        since.subsec_millis()
    )
}

/// Dias desde 1970-01-01 → (ano, mês, dia) no calendário gregoriano (algoritmo de H. Hinnant).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(from: usize, to: usize, insert: &str) -> Change {
        Change {
            from,
            to,
            insert: insert.into(),
        }
    }

    fn sha(bytes: &[u8]) -> [u8; 32] {
        sha256(bytes)
    }

    /// Pasta com `main.typ` = "= Título\n" e o arquivo aberto a partir do disco.
    fn opened() -> (tempfile::TempDir, std::path::PathBuf, OpenFile) {
        let dir = tempfile::tempdir().expect("pasta temporária");
        let path = dir.path().join("main.typ");
        std::fs::write(&path, "= Título\n").unwrap();
        let file = OpenFile::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        (dir, path, file)
    }

    fn entries(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn arquivo_aberto_comeca_limpo_na_versao_zero() {
        let (_dir, _path, file) = opened();
        assert_eq!(file.text(), "= Título\n");
        assert_eq!(file.version(), 0);
        assert!(!file.is_dirty());
        assert_eq!(file.last_saved_hash(), sha("= Título\n".as_bytes()));
        assert!(!file.autosave_due(Instant::now() + AUTOSAVE_DELAY * 10));
    }

    #[test]
    fn arquivo_nao_utf8_e_recusado() {
        assert_eq!(
            OpenFile::from_bytes(vec![0x63, 0xe7, 0x0a]).unwrap_err(),
            "not_utf8"
        );
    }

    #[test]
    fn edicao_incrementa_versao_e_suja() {
        let (_dir, _path, mut file) = opened();
        assert_eq!(file.edit(0, &[change(0, 0, "x")]), Ok(1));
        assert!(file.is_dirty());
        assert_eq!(file.edit(1, &[]), Ok(2));
        assert_eq!(file.text(), "x= Título\n");
    }

    #[test]
    fn edicao_com_versao_antiga_e_version_mismatch_sem_mudar_nada() {
        let (_dir, _path, mut file) = opened();
        file.edit(0, &[change(0, 0, "a")]).unwrap();
        assert_eq!(file.edit(0, &[change(0, 0, "b")]), Err("version_mismatch"));
        assert_eq!(file.edit(7, &[change(0, 0, "b")]), Err("version_mismatch"));
        assert_eq!(file.text(), "a= Título\n");
        assert_eq!(file.version(), 1);
    }

    #[test]
    fn intervalo_invalido_nao_suja_nem_muda_versao() {
        let (_dir, _path, mut file) = opened();
        assert_eq!(file.edit(0, &[change(0, 99, "")]), Err("bad_range"));
        assert_eq!(file.version(), 0);
        assert!(!file.is_dirty());
        assert_eq!(file.text(), "= Título\n");
    }

    #[test]
    fn salvar_grava_no_disco_e_limpa() {
        let (dir, path, mut file) = opened();
        file.edit(0, &[change(0, 0, "x")]).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "= Título\n",
            "editar não grava; só salvar grava"
        );
        file.save(&path).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "x= Título\n");
        assert!(!file.is_dirty());
        assert_eq!(file.version(), 1, "salvar não muda a versão");
        assert_eq!(file.last_saved_hash(), sha("x= Título\n".as_bytes()));
        assert_eq!(entries(dir.path()), ["main.typ"], "sem temporário sobrando");
        // Edição depois de salvar volta a sujar.
        file.edit(1, &[change(0, 1, "")]).unwrap();
        assert!(file.is_dirty());
    }

    #[test]
    fn falha_no_meio_da_gravacao_deixa_o_original_intacto_e_mantem_dirty() {
        let (dir, path, mut file) = opened();
        file.edit(0, &[change(0, 0, "texto novo e mais longo ")])
            .unwrap();
        let result = file.save_with(&path, |f, bytes| {
            // Grava metade e "o disco enche".
            io::Write::write_all(f, &bytes[..bytes.len() / 2])?;
            Err(io::Error::other("disco cheio"))
        });
        assert_eq!(result, Err("write_failed"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "= Título\n");
        assert!(file.is_dirty(), "falha mantém o arquivo sujo");
        assert_eq!(file.text(), "texto novo e mais longo = Título\n");
        assert_eq!(file.last_saved_hash(), sha("= Título\n".as_bytes()));
        assert_eq!(entries(dir.path()), ["main.typ"], "temporário removido");
        // A próxima tentativa grava.
        file.save(&path).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "texto novo e mais longo = Título\n"
        );
        assert!(!file.is_dirty());
    }

    #[cfg(unix)]
    #[test]
    fn pasta_sem_permissao_de_escrita_e_write_failed_sem_perder_o_texto() {
        use std::os::unix::fs::PermissionsExt;
        let (dir, path, mut file) = opened();
        file.edit(0, &[change(0, 0, "x")]).unwrap();
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();
        let result = file.save(&path);
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(result, Err("write_failed"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "= Título\n");
        assert!(file.is_dirty());
        assert_eq!(file.text(), "x= Título\n");
    }

    #[cfg(unix)]
    #[test]
    fn gravacao_preserva_as_permissoes_do_original() {
        use std::os::unix::fs::PermissionsExt;
        let (_dir, path, mut file) = opened();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
        file.edit(0, &[change(0, 0, "x")]).unwrap();
        file.save(&path).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o640);
    }

    #[test]
    fn salvamento_automatico_dois_segundos_depois_da_ultima_edicao() {
        let (_dir, path, mut file) = opened();
        let before = Instant::now();
        file.edit(0, &[change(0, 0, "x")]).unwrap();
        let after = Instant::now();
        assert!(!file.autosave_due(before + AUTOSAVE_DELAY / 2));
        assert!(!file.autosave_due(before + AUTOSAVE_DELAY - Duration::from_millis(1)));
        assert!(file.autosave_due(after + AUTOSAVE_DELAY));
        // Nova edição reinicia a contagem.
        let t1 = Instant::now();
        file.edit(1, &[change(0, 0, "y")]).unwrap();
        assert!(!file.autosave_due(t1 + AUTOSAVE_DELAY - Duration::from_millis(1)));
        assert!(file.autosave_due(Instant::now() + AUTOSAVE_DELAY));
        // Depois de salvar não há o que salvar.
        file.save(&path).unwrap();
        assert!(!file.autosave_due(Instant::now() + AUTOSAVE_DELAY * 10));
    }

    #[test]
    fn falha_reinicia_a_espera_do_salvamento_automatico() {
        let (_dir, path, mut file) = opened();
        file.edit(0, &[change(0, 0, "x")]).unwrap();
        let due = Instant::now() + AUTOSAVE_DELAY;
        assert!(file.autosave_due(due));
        let before_fail = Instant::now();
        let _ = file.save_with(&path, |_, _| Err(io::Error::other("falhou")));
        assert!(
            !file.autosave_due(before_fail + AUTOSAVE_DELAY - Duration::from_millis(1)),
            "não insiste sem pausa"
        );
        assert!(file.autosave_due(Instant::now() + AUTOSAVE_DELAY));
    }

    #[test]
    fn salvar_arquivo_limpo_nao_toca_o_disco() {
        let (dir, path, mut file) = opened();
        std::fs::remove_file(&path).unwrap();
        file.save(&path).unwrap();
        assert!(entries(dir.path()).is_empty());
    }

    #[test]
    fn write_atomic_cria_e_substitui() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("novo.typ");
        write_atomic(&path, b"um").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"um");
        write_atomic(&path, b"dois").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"dois");
        assert_eq!(entries(dir.path()), ["novo.typ"]);
    }

    #[test]
    fn saved_at_em_rfc3339_utc() {
        assert_eq!(format_utc(UNIX_EPOCH), "1970-01-01T00:00:00.000Z");
        assert_eq!(
            format_utc(UNIX_EPOCH + Duration::from_millis(1_700_000_000_123)),
            "2023-11-14T22:13:20.123Z"
        );
        // 29 de fevereiro em ano bissexto.
        assert_eq!(
            format_utc(UNIX_EPOCH + Duration::from_secs(951_782_400)),
            "2000-02-29T00:00:00.000Z"
        );
    }
}
