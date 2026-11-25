# Prelo · Especificação

Versão 0.1 · novembro de 2026 · rascunho para discussão

Prelo é um editor desktop para escrever artigos científicos em [Typst](https://typst.app) a várias mãos. Ele junta, numa só ferramenta, o que hoje está espalhado: edição com prévia instantânea, sugestões que se aceitam ou rejeitam, comentários em conversa, trabalho offline e histórico no git.

Este documento é a visão geral. O detalhe está em:

| Documento | Conteúdo |
|---|---|
| [requisitos.md](requisitos.md) | Papéis, requisitos funcionais (RF) e não funcionais (RNF), regras de revisão e de sincronização, fluxos, PDF de diferenças |
| [arquitetura.md](arquitetura.md) | Componentes, arquivos do repositório, conexão sem servidor, modelo de revisão, sincronização, acesso a arquivos pelo compilador, riscos técnicos |
| [adrs.md](adrs.md) | Índice das decisões de arquitetura D-01 a D-14; cada decisão, com contexto, alternativas e consequências, num arquivo em [adrs/](adrs/) |

O nome vem da prensa tipográfica. Um texto "no prelo" é um texto prestes a ser publicado, que é a fase que a ferramenta atende.

---

## Sumário

1. [Problema](#1-problema)
2. [Alternativas existentes](#2-alternativas-existentes)
3. [Escopo](#3-escopo)
4. [Plano de construção](#4-plano-de-construção)
5. [Pendências do produto](#5-pendências-do-produto)
6. [Fontes](#6-fontes)

---

## 1. Problema

O Typst compila em milissegundos e tem uma sintaxe mais limpa que a do LaTeX. Para um artigo com coautores, orientadores e revisores, porém, o trabalho que mais consome tempo é a revisão: alguém sugere uma frase, outro comenta uma figura, o autor principal aceita parte e rejeita o resto. Hoje esse ciclo acontece em ferramentas que não juntam tudo isso para Typst:

- O **Overleaf** tem controle de alterações e papel de revisor, mas só funciona com LaTeX.
- O **Typst web app** tem edição em tempo real e comentários (no plano Pro), sem controle de alterações documentado, e funciona só online.
- **VS Code + Tinymist + git** funciona offline e tem ótima prévia, mas a revisão vira diff do git e comentário no código-fonte.

## 2. Alternativas existentes

Levantamento feito em novembro de 2026 a partir da documentação de cada ferramenta. "Não verificado" indica que a documentação não confirma o recurso.

| Ferramenta | Typst | Tempo real | Comentários | Controle de alterações | Offline / desktop | Git | Auto-hospedagem |
|---|---|---|---|---|---|---|---|
| Overleaf | Não | Sim | Sim, em todos os planos | Sim, premium; papel "pode revisar" | Não | Sim, planos pagos | Community Edition, sem controle de alterações |
| Typst web app | Sim | Sim | Sim, se o dono for Pro; sem menções | Não documentado | Não | Não verificado | Não |
| VS Code + Tinymist + git | Sim | Via Live Share | Só no código | Só diff do git | Sim | Sim | Não se aplica |
| TeXlyre | Sim, e LaTeX | Sim | Não verificado | Não verificado | Local-first, no navegador | Não verificado | Sim |
| Collabst · TINO | Sim | Sim | Collabst: sim | Não verificado | Web | Não verificado | Sim |
| **Prelo** | Sim | Sim | Sim, com menções e conversa | Sim, aceitar e rejeitar | Sim, app desktop | Sim, nativo | Não precisa de servidor |

Nenhuma opção documentada junta Typst, controle de alterações no estilo do Overleaf, trabalho offline e git. Esse é o espaço do Prelo.

## 3. Escopo

O escopo abaixo é o do produto completo. Ele é entregue em cinco releases, uma por fase do [plano de construção](#4-plano-de-construção): R1 a R5, versões `1.0.0` a `1.4.0` (da `1.1.0` em diante, propostas). O conteúdo de cada release está em [release-plan.md](release-plan.md).

A primeira versão estável, `1.0.0`, é a fase 1 (editor local): editar e compilar um arquivo Typst com o compilador embutido e gerar o PDF (decisão do dono, 2026-11-21). Comentários, git, tempo real, sugestões e papéis chegam nas releases seguintes.

**Dentro do escopo**

- Projetos Typst com vários arquivos (`.typ`, `.bib`, figuras).
- Edição por várias pessoas, ao mesmo tempo ou em horários diferentes.
- Sugestões, comentários, histórico e versões nomeadas.
- Integração com repositórios git (GitHub, GitLab ou qualquer remoto).
- Exportação de PDF final, PDF com marcações e PDF de diferenças.

**Fora do escopo das releases R1 a R5**

- Edição de LaTeX.
- Versão web ou mobile.
- Gerenciamento de referências além do `.bib` (Zotero etc. ficam por arquivo).
- Submissão direta a revistas.

## 4. Plano de construção

1. **Editor local**: Tauri, CodeMirror 6, Tinymist, prévia SVG e sincronização código ↔ prévia.
2. **Comentários e histórico**: `.prelo/review.json`, commit e push pelo app, versões nomeadas. Colaboração assíncrona via git, com Salvar sincronizando pelo git.
3. **Tempo real sem servidor**: documento Yjs, mDNS na rede local, iroh pela internet, convite por código, cursores e presença. Fila offline e edição liberada só após sincronizar.
4. **Modo sugestão**: sugestões como atributos, aceitar e rejeitar, prévia com marcações, aviso de sugestão que não compila, PDF de diferenças.
5. **Papéis e convites**: revisor e leitor, aprovação de entrada, remoção de participantes, notificações.

## 5. Pendências do produto

- **Nome e marca.** Verificar se "Prelo" já é marca registrada (INPI e buscas gerais) antes de divulgar.
- ~~**Licença do próprio Prelo.**~~ Resolvida em 2026-11-19: `MIT OR Apache-2.0` (`LICENSE-MIT`, `LICENSE-APACHE`).

Riscos técnicos estão em [arquitetura.md §7](arquitetura.md#7-riscos-técnicos-e-questões-em-aberto).

## 6. Fontes

- Overleaf: [Track changes](https://docs.overleaf.com/collaborating/track-changes), [Reviewing and reviewers](https://www.overleaf.com/learn/how-to/Reviewing_and_reviewers_on_Overleaf), [Commenting](https://docs.overleaf.com/collaborating/commenting)
- Typst: [Comentários no web app](https://typst.app/docs/web-app/comments/), [Blog: folders e review-only shares](https://typst.app/blog/2025/folders-reviews), [Fórum: menções](https://forum.typst.app/t/how-can-i-mention-other-users-in-comments/4048)
- Alternativas: [Collabst](https://forum.typst.app/t/introducing-collabst-a-self-hosted-collaborative-workspace-for-typst/8856), [TINO](https://forum.typst.app/t/tino-a-collaborative-self-hosted-editing-platform-around-typst/8914), [TeXlyre](https://texlyre.github.io/texlyre/)
- Tecnologia: [Yjs + CodeMirror](https://docs.yjs.dev/ecosystem/editor-bindings/codemirror), [typst-preview / Tinymist](https://github.com/Enter-tainer/typst-preview), [iroh](https://www.iroh.computer/), [y-webrtc](https://github.com/yjs/y-webrtc), [libp2p](https://libp2p.io/)
