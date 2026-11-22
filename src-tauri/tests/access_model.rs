//! Testes do `PathAccessModel` do Prelo (T018; contracts/file-access.md passos 1 e 9,
//! RF-08, RF-51, data-model.md "Diagnostic" e "Cache de conteúdo").
//!
//! O modelo recebe do VFS do Tinymist o caminho absoluto (raiz + caminho virtual), devolve o
//! texto do overlay de arquivos abertos sem tocar o disco, passa todo o resto pela regra de
//! escopo (`fs/scope.rs`) e guarda os bytes num cache por sha256. Recusas viram
//! `FileError` cuja mensagem é a do contrato, citando o caminho relativo à raiz, nunca o real,
//! e são mapeadas para `Diagnostic.kind = access`.

mod support;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use prelo_lib::diagnostic::{Kind, Severity};
use prelo_lib::fs::access_model::{access_diagnostic, AccessModel};
use prelo_lib::fs::scope::{Refusal, Scope};
use support::*;
use tinymist_vfs::{Bytes, PathAccessModel};

struct Case {
    project: TempProject,
    outside: Outside,
    model: AccessModel,
    opens: AccessLog,
}

impl Case {
    fn new() -> Self {
        let project = TempProject::empty();
        project.write("main.typ", "= Artigo");
        project.write("capitulos/cap1.typ", "== Capítulo 1");
        let outside = Outside::new();
        let opens = AccessLog::default();
        let log = opens.clone();
        let scope = Scope::new(project.root())
            .unwrap()
            .with_open_observer(Arc::new(move |p: &Path| log.record(p)));
        Self {
            project,
            outside,
            model: AccessModel::new(scope),
            opens,
        }
    }

    /// Caminho que o VFS do Tinymist entrega: raiz canônica + caminho virtual.
    fn abs(&self, rel: &str) -> PathBuf {
        self.project.root().join(rel)
    }

    fn read(&self, rel: &str) -> Result<Bytes, String> {
        self.model
            .content(&self.abs(rel))
            .map_err(|e| e.to_string())
    }
}

fn text(bytes: &Bytes) -> &str {
    std::str::from_utf8(bytes.as_slice()).unwrap()
}

#[test]
fn le_arquivo_dentro_da_raiz_pela_regra_de_escopo() {
    let case = Case::new();
    assert_eq!(text(&case.read("capitulos/cap1.typ").unwrap()), "== Capítulo 1");
    assert_eq!(case.opens.all(), vec![case.abs("capitulos/cap1.typ")]);
}

#[test]
fn overlay_devolve_texto_em_memoria_sem_abrir_o_disco() {
    let case = Case::new();
    case.model
        .set_overlay("capitulos/cap1.typ", "== Editado, não salvo")
        .unwrap();
    // Arquivo novo, ainda não gravado, também vem do overlay.
    case.model.set_overlay("novo.typ", "= Novo").unwrap();
    assert_eq!(
        text(&case.read("capitulos/cap1.typ").unwrap()),
        "== Editado, não salvo"
    );
    assert_eq!(text(&case.read("novo.typ").unwrap()), "= Novo");
    assert!(case.opens.all().is_empty(), "overlay não toca o disco");
    assert!(case.model.is_open("capitulos/cap1.typ"));
    assert!(case.model.is_open("/capitulos/./cap1.typ"));
}

#[test]
fn overlay_e_compartilhado_entre_clones() {
    // O World guarda um clone do modelo; o núcleo atualiza o overlay pelo outro.
    let case = Case::new();
    let world_side = case.model.clone();
    case.model.set_overlay("main.typ", "= Mudou").unwrap();
    assert_eq!(
        text(&world_side.content(&case.abs("main.typ")).unwrap()),
        "= Mudou"
    );
}

#[test]
fn fechar_arquivo_volta_a_ler_do_disco() {
    let case = Case::new();
    case.model.set_overlay("main.typ", "= Em memória").unwrap();
    case.model.remove_overlay("main.typ");
    assert!(!case.model.is_open("main.typ"));
    assert_eq!(text(&case.read("main.typ").unwrap()), "= Artigo");
    assert_eq!(case.opens.all().len(), 1);
}

#[test]
fn overlay_recusa_caminho_fora_da_raiz() {
    let case = Case::new();
    for requested in ["../fora.typ", "/../fora.typ", "capitulos/../../fora.typ", ""] {
        assert!(
            case.model.set_overlay(requested, "x").is_err(),
            "{requested:?} aceito no overlay"
        );
    }
    let fora = case.outside.root().join("segredo.txt");
    assert!(case.model.set_overlay(fora.to_str().unwrap(), "x").is_ok());
    // Caminho do sistema vira relativo à raiz (`/` inicial = raiz, como no Typst): não
    // cria overlay fora do projeto.
    assert!(case.model.content(&fora).is_err());
}

#[test]
fn caminho_absoluto_fora_da_raiz_e_recusado_sem_abrir() {
    let case = Case::new();
    let err = case
        .model
        .content(&case.outside.secret())
        .expect_err("fora da raiz")
        .to_string();
    assert!(err.contains("fica fora da pasta do projeto"), "{err}");
    assert!(
        !err.contains(case.outside.root().to_str().unwrap()),
        "mensagem revela caminho real: {err}"
    );
    assert!(case.opens.all().is_empty());
}

#[test]
fn link_simbolico_para_fora_vira_recusa_com_caminho_relativo() {
    let case = Case::new();
    symlink(&case.outside.secret(), &case.project.path("dados.csv")).unwrap();
    let err = case.read("dados.csv").expect_err("link para fora");
    assert_eq!(
        err,
        "failed to load file (acesso recusado: `/dados.csv` fica fora da pasta do projeto)"
    );
    assert_eq!(
        case.opens.outside(&case.project.root()),
        Vec::<PathBuf>::new()
    );
}

#[test]
fn inexistente_nao_revela_caminho_real() {
    let case = Case::new();
    let err = case.read("nada.typ").expect_err("não existe");
    assert_eq!(
        err,
        "failed to load file (arquivo não encontrado: `/nada.typ`)"
    );
    assert!(!err.contains(case.project.root().to_str().unwrap()));
}

#[test]
fn recusa_e_reconhecida_pela_mensagem_do_typst() {
    let case = Case::new();
    let err = case.read("nada.typ").unwrap_err();
    assert_eq!(
        case.model.refusal_for_message(&err),
        Some(Refusal::NotFound {
            requested: "/nada.typ".into()
        })
    );
    assert_eq!(case.model.refusal_for_message("unknown variable: x"), None);
}

#[test]
fn recusa_vira_diagnostico_de_acesso() {
    let refusal = Refusal::OutsideProject {
        requested: "../../fora.png".into(),
    };
    let diag = access_diagnostic(&refusal);
    assert_eq!(diag.kind, Kind::Access);
    assert_eq!(diag.severity, Severity::Error);
    assert_eq!(diag.path.as_deref(), Some("../../fora.png"));
    assert_eq!(
        diag.message,
        "acesso recusado: `../../fora.png` fica fora da pasta do projeto"
    );
}

#[test]
fn cache_por_conteudo_reaproveita_bytes_iguais() {
    let case = Case::new();
    case.project.write("figuras/a.png", "mesmos bytes");
    case.project.write("figuras/b.png", "mesmos bytes");
    let a = case.read("figuras/a.png").unwrap();
    let a2 = case.read("figuras/a.png").unwrap();
    let b = case.read("figuras/b.png").unwrap();
    // Mesmo conteúdo → mesma alocação: o Typst reaproveita a imagem decodificada (RF-51).
    assert_eq!(a.as_slice().as_ptr(), a2.as_slice().as_ptr());
    assert_eq!(a.as_slice().as_ptr(), b.as_slice().as_ptr());
    assert_eq!(case.model.cached_entries(), 1);
    // Toda leitura passa de novo pela regra de escopo: o cache não pula a conferência.
    assert_eq!(case.opens.all().len(), 3);

    // Conteúdo novo → entrada nova.
    case.project.write("figuras/a.png", "bytes novos");
    assert_eq!(text(&case.read("figuras/a.png").unwrap()), "bytes novos");
    assert_eq!(case.model.cached_entries(), 2);
}

#[test]
fn reset_limpa_o_cache_de_conteudo() {
    let mut case = Case::new();
    case.read("main.typ").unwrap();
    assert_eq!(case.model.cached_entries(), 1);
    case.model.reset();
    assert_eq!(case.model.cached_entries(), 0);
}

#[test]
fn recusa_nao_entra_no_cache() {
    let case = Case::new();
    symlink(&case.outside.secret(), &case.project.path("fora.txt")).unwrap();
    assert!(case.read("fora.txt").is_err());
    assert_eq!(case.model.cached_entries(), 0);
}
