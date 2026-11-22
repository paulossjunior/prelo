# Prelo · Registros de Decisão de Arquitetura (ADRs)

Versão 0.1 · novembro de 2026

Este arquivo é o índice das decisões de arquitetura. Cada decisão fica num arquivo próprio em
[`adrs/`](adrs/), no formato: status, contexto, decisão, alternativas consideradas e
consequências. Os IDs D-xx são os mesmos citados em [requisitos.md](requisitos.md),
[arquitetura.md](arquitetura.md) e na constituição do projeto. Decisão nova entra como
`adrs/D-NN-<slug>.md`, com o próximo número livre, e ganha uma linha nesta tabela.

Status possíveis: **proposta**, **aceita**, **substituída por D-xx**.

## Índice

| ID | Título | Status | Arquivo |
|---|---|---|---|
| <a id="d-01-app-desktop-com-tauri"></a>D-01 | App desktop com Tauri | Aceita | [D-01-tauri.md](adrs/D-01-tauri.md) |
| <a id="d-02-sugestões-como-atributos-do-crdt-fora-do-typ"></a>D-02 | Sugestões como atributos do CRDT, fora do `.typ` | Aceita | [D-02-sugestoes-no-crdt.md](adrs/D-02-sugestoes-no-crdt.md) |
| <a id="d-03-comentários-são-resolvidos-não-aceitos"></a>D-03 | Comentários são resolvidos, não aceitos | Aceita | [D-03-comentarios-resolvidos.md](adrs/D-03-comentarios-resolvidos.md) |
| <a id="d-04-conexão-p2p-com-iroh-sem-servidor-obrigatório"></a>D-04 | Conexão P2P com iroh, sem servidor obrigatório | Aceita | [D-04-p2p-iroh.md](adrs/D-04-p2p-iroh.md) |
| <a id="d-05-git-como-canal-para-quem-não-está-online-junto"></a>D-05 | Git como canal para quem não está online junto | Aceita | [D-05-git-assincrono.md](adrs/D-05-git-assincrono.md) |
| <a id="d-06-offline-só-comenta-e-sugere"></a>D-06 | Offline só comenta e sugere | Aceita | [D-06-offline-comenta-sugere.md](adrs/D-06-offline-comenta-sugere.md) |
| <a id="d-07-edição-liberada-só-com-arquivos-sincronizados"></a>D-07 | Edição liberada só com arquivos sincronizados | Aceita | [D-07-edicao-sincronizada.md](adrs/D-07-edicao-sincronizada.md) |
| <a id="d-08-salvar-s-sincroniza-automaticamente"></a>D-08 | Salvar (⌘S) sincroniza automaticamente | Aceita | [D-08-salvar-sincroniza.md](adrs/D-08-salvar-sincroniza.md) |
| <a id="d-09-aceitar-e-rejeitar-sugestões-bloqueados-offline"></a>D-09 | Aceitar e rejeitar sugestões bloqueados offline | Aceita | [D-09-aceitar-rejeitar-offline.md](adrs/D-09-aceitar-rejeitar-offline.md) |
| <a id="d-10-compilador-typst-embutido-no-núcleo"></a>D-10 | Compilador Typst embutido no núcleo | Aceita | [D-10-typst-embutido.md](adrs/D-10-typst-embutido.md) |
| <a id="d-11-mpl-20-só-como-exceção-por-dependência"></a>D-11 | MPL-2.0 só como exceção por dependência | Aceita | [D-11-excecao-mpl.md](adrs/D-11-excecao-mpl.md) |
| <a id="d-12-transporte-de-arquivos-binários-entre-coautores"></a>D-12 | Transporte de arquivos binários entre coautores | Aceita | [D-12-binarios-entre-coautores.md](adrs/D-12-binarios-entre-coautores.md) |
| <a id="d-13-arquivos-fora-do-projeto"></a>D-13 | Arquivos fora do projeto | Aceita | [D-13-arquivos-fora-do-projeto.md](adrs/D-13-arquivos-fora-do-projeto.md) |
| <a id="d-14-edição-concorrente-de-bibliografia"></a>D-14 | Edição concorrente de bibliografia | Aceita | [D-14-bibliografia-concorrente.md](adrs/D-14-bibliografia-concorrente.md) |

As âncoras `adrs.md#d-xx-...` das versões anteriores deste arquivo continuam apontando para a
linha da decisão nesta tabela.
