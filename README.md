# Prelo

Editor desktop para escrever artigos científicos em [Typst](https://typst.app) a várias mãos: sugestões que se aceitam ou rejeitam, comentários em conversa, edição em tempo real sem servidor, trabalho offline e histórico no git.

> **Estado:** especificação e protótipo de interface. Ainda não há código do app.

## Conteúdo

| Caminho | O que é |
|---|---|
| [`docs/especificacao.md`](docs/especificacao.md) | Especificação: problema, alternativas, requisitos, arquitetura, modelo de revisão, offline e sincronização, fluxos, decisões e plano |
| [`prototype/index.html`](prototype/index.html) | Protótipo clicável da interface e telas dos fluxos. Abra no navegador |

## Ideias principais

- **Typst puro no git.** O `.typ` do repositório é sempre compilável fora do Prelo. Sugestões e comentários ficam em `.prelo/`.
- **Revisão no estilo do Overleaf.** Modos Editando, Sugerindo e Lendo; aceitar e rejeitar por sugestão ou em lote; comentários com respostas, menções, resolver e excluir.
- **Sem servidor obrigatório.** Conexão direta entre os computadores dos coautores (mDNS na rede local, iroh pela internet). O git cobre quem não está online ao mesmo tempo.
- **Offline com regra clara.** Offline se comenta e sugere. Editar direto só com os arquivos sincronizados.
- **Salvar sincroniza.** ⌘S recebe as mudanças dos outros, envia as suas e grava um commit.

## Tecnologia prevista

Tauri · Rust · CodeMirror 6 · Yjs / yrs · Typst + Tinymist · iroh · gitoxide

## Nome

Prelo é a prensa tipográfica. Um texto "no prelo" está prestes a ser publicado, que é a fase do artigo que a ferramenta atende.
