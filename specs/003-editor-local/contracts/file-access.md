# Contrato: acesso a arquivos pelo compilador

Implementa RF-08, FR-021 a FR-024, FR-016 a FR-019 e docs/arquitetura.md §6.2. Vale para
**todo** byte que o compilador, o autocompletar, o resolvedor de fontes e o registro de
pacotes leem do disco. Nenhum outro código do núcleo lê arquivo do projeto para entregar ao
compilador.

## Entrada

`read(raiz, caminho_virtual) -> Result<Bytes, Recusa>`, onde `raiz` é:

- a raiz do projeto, canonicalizada ao abrir; ou
- a pasta `nome/versão` do pacote no cache (R7), canonicalizada ao resolver o pacote.

`caminho_virtual` vem do Typst (`FileId`). Começar com `/` significa "a partir da raiz".

## Passos, nesta ordem

1. Se o arquivo está aberto no editor (overlay), devolve o texto em memória. Fim.
2. Junta `raiz` e `caminho_virtual`.
3. Canonicaliza (resolve `..` e links simbólicos; no Windows também junções). Falha →
   `NaoEncontrado`.
4. Caminho canônico fora de `raiz` → `ForaDoProjeto`.
5. Abre sem bloquear (`O_NONBLOCK`; `O_NOFOLLOW` no último componente em Unix).
6. No descritor aberto: não é arquivo regular → `ArquivoEspecial`; caminho real do descritor
   fora de `raiz` → `ForaDoProjeto` (troca de link entre 3 e 5).
7. Tamanho > 256 MB → `GrandeDemais`.
8. Lê. Bytes começam com `\0asm` e não há aprovação para `(raiz do projeto, sha256)` →
   `PluginNaoAprovado` + evento `plugin_approval_needed` (R8).
9. Guarda no cache por conteúdo (RF-51) e devolve.

## Recusas e diagnóstico

| Recusa | Mensagem no diagnóstico (pt-BR) | Conteúdo lido? |
|---|---|---|
| `NaoEncontrado` | "arquivo não encontrado: `<caminho pedido>`" | Não |
| `ForaDoProjeto` | "acesso recusado: `<caminho pedido>` fica fora da pasta do projeto" | Não |
| `ArquivoEspecial` | "acesso recusado: `<caminho pedido>` não é um arquivo comum" | Não |
| `GrandeDemais` | "arquivo grande demais: `<caminho pedido>` (limite 256 MB)" | Não |
| `PluginNaoAprovado` | "plugin `<caminho pedido>` aguardando sua confirmação" | Sim, só para calcular o hash; não é entregue |
| `PacoteAdulterado` | "o pacote `@preview/<nome>:<versão>` foi alterado depois do download" | Não é entregue |
| `PacoteIndisponivel` | "pacote `@preview/<nome>:<versão>` não está no computador e não há conexão" | — |
| `NamespaceNaoSuportado` | "pacotes `@<namespace>` não são suportados; copie o pacote para o projeto" | — |

O diagnóstico cita sempre o caminho **pedido** no `.typ`, nunca o caminho real resolvido
(não revela a estrutura de pastas fora do projeto).

## Autocompletar de caminhos

Toda sugestão de caminho passa pelos passos 2–4 antes de ir para a interface; sugestão
recusada some da lista. Links simbólicos para fora não são listados.

## Fontes

O resolvedor de fontes lê `fonts/**/*.{ttf,otf,ttc,otc}` do projeto por esta mesma função.
Fontes do sistema nunca são consultadas.

## Pacotes (download)

Extração aceita só entradas do tipo arquivo regular ou pasta, com caminho relativo, sem `..`,
dentro dos limites de R7. Qualquer outra entrada → pacote inteiro recusado e pasta temporária
apagada. Nada é gravado fora de `<cache>/packages/`.

## Testes obrigatórios (FR-030)

Cada linha da tabela de recusas tem ao menos um teste com fixture. O `PathAccessModel` de
teste conta toda abertura de arquivo; os testes de escape conferem **zero** aberturas fora
da raiz.
