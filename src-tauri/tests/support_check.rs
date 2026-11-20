//! Confere o apoio de testes (T009): os geradores produzem exatamente as entradas pedidas,
//! inclusive as maliciosas, e o modelo instrumentado registra toda abertura.

mod support;

use std::io::Read;

use support::*;
use tinymist_vfs::PathAccessModel;

fn entries(bytes: &[u8]) -> Vec<(Vec<u8>, tar::EntryType, Vec<u8>)> {
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(bytes));
    archive
        .entries()
        .expect("entradas")
        .map(|e| {
            let mut e = e.expect("entrada");
            let path = e.path_bytes().to_vec();
            let kind = e.header().entry_type();
            let link = e.link_name_bytes().map(|l| l.to_vec()).unwrap_or_default();
            let mut data = Vec::new();
            if kind == tar::EntryType::Regular {
                e.read_to_end(&mut data).expect("dados");
                return (path, kind, data);
            }
            (path, kind, link)
        })
        .collect()
}

#[test]
fn targz_preserva_entradas_maliciosas_cruas() {
    let bytes = make_targz(&[
        TarEntry::file("typst.toml", "[package]"),
        TarEntry::file("../fora.typ", "x"),
        TarEntry::file("/etc/abs.typ", "y"),
        TarEntry::Symlink {
            path: "link".into(),
            target: "/etc/passwd".into(),
        },
        TarEntry::HardLink {
            path: "hard".into(),
            target: "typst.toml".into(),
        },
        TarEntry::CharDevice { path: "dev".into() },
        TarEntry::Fifo {
            path: "fifo".into(),
        },
        TarEntry::Dir {
            path: "src/".into(),
        },
    ]);
    let got = entries(&bytes);
    let names: Vec<&[u8]> = got.iter().map(|(p, _, _)| p.as_slice()).collect();
    assert_eq!(
        names,
        vec![
            b"typst.toml".as_slice(),
            b"../fora.typ",
            b"/etc/abs.typ",
            b"link",
            b"hard",
            b"dev",
            b"fifo",
            b"src/",
        ]
    );
    assert_eq!(got[0].2, b"[package]");
    assert_eq!(got[3].1, tar::EntryType::Symlink);
    assert_eq!(got[3].2, b"/etc/passwd");
    assert_eq!(got[4].1, tar::EntryType::Link);
    assert_eq!(got[5].1, tar::EntryType::Char);
    assert_eq!(got[6].1, tar::EntryType::Fifo);
    assert_eq!(got[7].1, tar::EntryType::Directory);
}

#[test]
fn targz_malformado_nao_descompacta() {
    let bytes = malformed_targz();
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(&bytes[..]));
    let ok = archive
        .entries()
        .map(|mut it| it.all(|e| e.is_ok()))
        .unwrap_or(false);
    assert!(!ok);
}

#[test]
fn projeto_de_fixture_e_link_para_fora() {
    let project = TempProject::from_fixture("artigo-basico");
    assert!(project.path("main.typ").is_file());
    let outside = Outside::new();
    let link = project.path("atalho.txt");
    symlink(&outside.secret(), &link).expect("criar link");
    assert_eq!(link.canonicalize().unwrap(), outside.secret());
}

#[cfg(unix)]
#[test]
fn fifo_criado_e_arquivo_especial() {
    use std::os::unix::fs::FileTypeExt;
    let project = TempProject::empty();
    let fifo = project.path("pipe");
    mkfifo(&fifo).expect("mkfifo");
    assert!(std::fs::symlink_metadata(&fifo)
        .unwrap()
        .file_type()
        .is_fifo());
}

#[test]
fn modelo_instrumentado_registra_cada_abertura() {
    let project = TempProject::empty();
    let inside = project.write("a.typ", "oi");
    let outside = Outside::new();
    let model = RecordingAccess::new(NaiveAccess);
    model.content(&inside).expect("ler dentro");
    assert!(model.log.outside(&project.root()).is_empty());
    let _ = model.content(&outside.secret());
    assert_eq!(model.log.all().len(), 2);
    assert_eq!(model.log.outside(&project.root()), vec![outside.secret()]);
}
