//! Testes do serviço de linguagem (T029; D-17; FR-006, FR-011; contracts/ipc.md).
//!
//! - autocompletar de função, variável e rótulo pelo `typst-ide`, inclusive com texto ainda não
//!   salvo, pelo mesmo estado que a prévia compila (FR-011);
//! - *hover* de função pelo `typst-ide` (`tooltip`);
//! - *tokens* de realce do `typst-syntax` (`LinkedNode` + `highlight`) em markup, código e
//!   matemática.
//!
//! Deslocamentos de entrada e saída são unidades UTF-16 do texto atual (as do CodeMirror): os
//! textos têm acento e emoji antes do ponto testado, para bytes e UTF-16 divergirem.

mod support;

use std::sync::mpsc;
use std::time::Duration;

use prelo_lib::commands::{AppState, Change};
use prelo_lib::lang_service::{self, Completion, Token};
use prelo_lib::world::ProjectWorld;
use support::TempProject;
use typst::World;

/// Comprimento em unidades UTF-16.
fn u16len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// Trecho `[from, to)` em unidades UTF-16.
fn slice16(text: &str, from: usize, to: usize) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    String::from_utf16(&units[from..to]).expect("intervalo em fronteira de caractere")
}

fn find<'a>(items: &'a [Completion], label: &str) -> Option<&'a Completion> {
    items.iter().find(|c| c.label == label)
}

fn labels(items: &[Completion]) -> Vec<&str> {
    items.iter().map(|c| c.label.as_str()).collect()
}

/// Projeto de um arquivo só, aberto no núcleo como a interface faria.
fn open_single(text: &str) -> (TempProject, AppState) {
    let project = TempProject::empty();
    project.write("main.typ", text);
    let state = AppState::default();
    state.open_project(&project.root()).expect("abrir projeto");
    state.open_file("main.typ").expect("abrir arquivo");
    (project, state)
}

/// Troca o texto inteiro do arquivo aberto (versão `version`), sem salvar.
fn replace_all(state: &AppState, old: &str, new: &str, version: u64) -> u64 {
    state
        .edit_file(
            "main.typ",
            version,
            &[Change {
                from: 0,
                to: u16len(old),
                insert: new.into(),
            }],
        )
        .expect("editar")
}

// ------------------------------------------------------------------ autocompletar

#[test]
fn completa_funcao_da_biblioteca_depois_de_acento_e_emoji() {
    let text = "Introdução 😀 çã #hea";
    let (_p, state) = open_single(text);
    let items = state.complete("main.typ", u16len(text)).expect("complete");
    let heading = find(&items, "heading").unwrap_or_else(|| panic!("{:?}", labels(&items)));
    assert_eq!(heading.kind.as_deref(), Some("function"));
    assert!(heading.detail.as_deref().is_some_and(|d| !d.is_empty()));
}

#[test]
fn completa_variavel_do_texto_ainda_nao_salvo() {
    let disk = "= Início\n";
    let (project, state) = open_single(disk);
    let edited = "= Início\n#let meu-titulo = \"Ação\"\nOlá 😀 #meu-t";
    replace_all(&state, disk, edited, 0);

    let items = state
        .complete("main.typ", u16len(edited))
        .expect("complete");
    let var = find(&items, "meu-titulo").unwrap_or_else(|| panic!("{:?}", labels(&items)));
    assert!(var.kind.is_some());
    // O texto não foi salvo: o disco continua como antes.
    assert_eq!(
        std::fs::read_to_string(project.path("main.typ")).unwrap(),
        disk
    );
}

#[test]
fn completa_funcao_definida_em_outro_arquivo_aberto() {
    let project = TempProject::from_fixture("artigo-basico");
    let state = AppState::default();
    state.open_project(&project.root()).unwrap();
    let utils = state.open_file("utils.typ").unwrap().text;
    let new_utils = format!("{utils}#let nota-ção(x) = x\n");
    state
        .edit_file(
            "utils.typ",
            0,
            &[Change {
                from: u16len(&utils),
                to: u16len(&utils),
                insert: "#let nota-ção(x) = x\n".into(),
            }],
        )
        .unwrap();
    assert_eq!(state.open_file("utils.typ").unwrap().text, new_utils);

    let main = state.open_file("main.typ").unwrap().text;
    let edited = format!("#import \"utils.typ\": *\n{main}\n#nota");
    state
        .edit_file(
            "main.typ",
            0,
            &[Change {
                from: 0,
                to: u16len(&main),
                insert: edited.clone(),
            }],
        )
        .unwrap();
    let items = state.complete("main.typ", u16len(&edited)).unwrap();
    let f = find(&items, "nota-ção").unwrap_or_else(|| panic!("{:?}", labels(&items)));
    assert_eq!(f.kind.as_deref(), Some("function"));
}

#[test]
fn completa_rotulo_do_documento_da_previa() {
    let text = "= Seção 😀 <secao-a>\n\nTexto.\n\n= Conclusão <fim>\n\nVer @";
    let project = TempProject::empty();
    project.write("main.typ", text);
    let state = AppState::default();
    let (tx, rx) = mpsc::channel();
    state.open_project(&project.root()).unwrap();
    state.subscribe_preview(move |u| {
        let _ = tx.send(u.ok);
    });
    // Rótulos vêm do documento compilado: espera a prévia ficar pronta.
    let ok = rx.recv_timeout(Duration::from_secs(60)).expect("prévia");
    assert!(ok, "a prévia deveria compilar");
    state.open_file("main.typ").unwrap();

    let items = state.complete("main.typ", u16len(text)).expect("complete");
    let label = find(&items, "secao-a").unwrap_or_else(|| panic!("{:?}", labels(&items)));
    assert_eq!(label.kind.as_deref(), Some("label"));
    assert!(find(&items, "fim").is_some());
}

#[test]
fn sugestoes_nao_trazem_sintaxe_de_snippet() {
    let text = "Olá #";
    let (_p, state) = open_single(text);
    let items = state.complete("main.typ", u16len(text)).unwrap();
    assert!(!items.is_empty());
    for c in &items {
        for s in std::iter::once(&c.label).chain(c.apply.as_ref()) {
            assert!(!s.contains("${"), "snippet em {c:?}");
        }
    }
}

#[test]
fn deslocamento_fora_do_texto_nao_derruba() {
    let text = "Olá #hea";
    let (_p, state) = open_single(text);
    assert_eq!(state.complete("main.typ", 10_000), Ok(Vec::new()));
    // No meio de um par substituto (emoji): sem sugestão, sem pânico.
    let text2 = "😀";
    let (_p2, state2) = open_single(text2);
    assert_eq!(state2.complete("main.typ", 1), Ok(Vec::new()));
}

#[test]
fn caminho_fora_do_projeto_e_recusado_antes_do_motor() {
    let (_p, state) = open_single("#hea");
    assert_eq!(
        state.complete("../fora.typ", 0),
        Err("path_outside_project".into())
    );
    assert_eq!(
        state.semantic_tokens("/etc/passwd"),
        Err("path_outside_project".into())
    );
}

// ------------------------------------------------------------------ hover

#[test]
fn hover_de_funcao_mostra_a_documentacao() {
    let text = "Introdução 😀\n#heading[Olá]\n";
    let project = TempProject::empty();
    project.write("main.typ", text);
    let world = ProjectWorld::open(project.root(), "main.typ").expect("World");
    let world = world.snapshot();
    let source = world.source(world.main()).expect("fonte");
    // Cursor no meio de `heading` (em UTF-16).
    let offset = u16len("Introdução 😀\n#hea");
    let hover = lang_service::hover(&world, None, &source, offset).expect("hover");
    assert!(hover.text.to_lowercase().contains("heading"), "{hover:?}");
}

#[test]
fn hover_em_texto_comum_nao_mostra_nada() {
    let text = "Só texto comum.\n";
    let project = TempProject::empty();
    project.write("main.typ", text);
    let world = ProjectWorld::open(project.root(), "main.typ")
        .unwrap()
        .snapshot();
    let source = world.source(world.main()).unwrap();
    assert_eq!(lang_service::hover(&world, None, &source, 3), None);
}

// ------------------------------------------------------------------ realce

fn token_at<'a>(tokens: &'a [Token], text: &str, kind: &str, piece: &str) -> Option<&'a Token> {
    tokens
        .iter()
        .find(|t| t.kind == kind && slice16(text, t.from, t.to) == piece)
}

#[test]
fn tokens_de_markup_codigo_e_matematica_em_utf16() {
    let text = "😀 Introdução\n= Título é\n*forte* _ênfase_ <rótulo> @rótulo\n\
                #let ação = 42\n#text(\"olá\")\n$ a + b^2 $\n// comentário\n";
    let (_p, state) = open_single(text);
    let tokens = state.semantic_tokens("main.typ").expect("tokens");

    for (kind, piece) in [
        ("heading", "= Título é"),
        ("strong", "*forte*"),
        ("emph", "_ênfase_"),
        ("label", "<rótulo>"),
        ("ref", "@rótulo"),
        ("key", "let"),
        ("num", "42"),
        ("func", "text"),
        ("str", "\"olá\""),
        ("math-delim", "$"),
        ("comment", "// comentário"),
    ] {
        assert!(
            token_at(&tokens, text, kind, piece).is_some(),
            "falta {kind} em {piece:?}: {tokens:?}"
        );
    }
    let len = u16len(text);
    for t in &tokens {
        assert!(t.from < t.to && t.to <= len, "token inválido {t:?}");
        assert!(!t.kind.is_empty() && !t.kind.starts_with("typ-"), "{t:?}");
    }
}

#[test]
fn tokens_seguem_o_texto_nao_salvo() {
    let disk = "Texto\n";
    let (_p, state) = open_single(disk);
    assert!(token_at(
        &state.semantic_tokens("main.typ").unwrap(),
        disk,
        "key",
        "let"
    )
    .is_none());
    let edited = "Ação 😀\n#let x = 1\n";
    replace_all(&state, disk, edited, 0);
    let tokens = state.semantic_tokens("main.typ").unwrap();
    assert!(
        token_at(&tokens, edited, "key", "let").is_some(),
        "{tokens:?}"
    );
}

#[test]
fn tokens_de_arquivo_que_nao_e_typ_vem_vazios() {
    let project = TempProject::empty();
    project.write("main.typ", "= Oi\n");
    project.write("refs.bib", "@article{chave, title = {= Não é Typst}}\n");
    let state = AppState::default();
    state.open_project(&project.root()).unwrap();
    assert_eq!(state.semantic_tokens("refs.bib"), Ok(Vec::new()));
}

#[test]
fn formato_json_segue_o_contrato_da_interface() {
    let c = Completion {
        label: "heading".into(),
        kind: Some("function".into()),
        detail: None,
        info: None,
        apply: None,
    };
    assert_eq!(
        serde_json::to_value(&c).unwrap(),
        serde_json::json!({ "label": "heading", "kind": "function" })
    );
    let t = Token {
        from: 1,
        to: 3,
        kind: "key".into(),
    };
    assert_eq!(
        serde_json::to_value(&t).unwrap(),
        serde_json::json!({ "from": 1, "to": 3, "kind": "key" })
    );
}
