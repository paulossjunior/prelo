# Prelo · Registros de Decisão de Arquitetura (ADRs)

Versão 0.1 · novembro de 2026

Cada decisão segue o formato: status, contexto, decisão, alternativas consideradas e
consequências. Os IDs D-xx são os mesmos citados em [requisitos.md](requisitos.md),
[arquitetura.md](arquitetura.md) e na constituição do projeto.

Status possíveis: **proposta**, **aceita**, **substituída por D-xx**.

## Índice

| ID | Decisão | Status |
|---|---|---|
| [D-01](#d-01-app-desktop-com-tauri) | App desktop com Tauri | Aceita |
| [D-02](#d-02-sugestões-como-atributos-do-crdt-fora-do-typ) | Sugestões como atributos do CRDT, fora do `.typ` | Aceita |
| [D-03](#d-03-comentários-são-resolvidos-não-aceitos) | Comentários são resolvidos, não aceitos | Aceita |
| [D-04](#d-04-conexão-p2p-com-iroh-sem-servidor-obrigatório) | Conexão P2P com iroh, sem servidor obrigatório | Aceita |
| [D-05](#d-05-git-como-canal-para-quem-não-está-online-junto) | Git como canal para quem não está online junto | Aceita |
| [D-06](#d-06-offline-só-comenta-e-sugere) | Offline só comenta e sugere | Aceita |
| [D-07](#d-07-edição-liberada-só-com-arquivos-sincronizados) | Edição liberada só com arquivos sincronizados | Aceita |
| [D-08](#d-08-salvar-s-sincroniza-automaticamente) | Salvar (⌘S) sincroniza automaticamente | Aceita |
| [D-09](#d-09-aceitar-e-rejeitar-sugestões-bloqueados-offline) | Aceitar e rejeitar sugestões bloqueados offline | Aceita |
| [D-10](#d-10-compilador-typst-embutido-no-núcleo) | Compilador Typst embutido no núcleo | Aceita |

---

## D-01 App desktop com Tauri

- **Status:** aceita · novembro de 2026
- **Contexto:** o Prelo precisa rodar em macOS, Windows e Linux (RNF-01) e funcionar offline. O compilador Typst e o Tinymist são escritos em Rust.
- **Decisão:** shell Tauri, com interface em HTML/TypeScript e núcleo em Rust.
- **Alternativas consideradas:** TODO(D-01): a especificação não registra as alternativas avaliadas (ex.: Electron, app nativo por plataforma).
- **Consequências:**
  - (+) Núcleo na mesma linguagem do Typst e do Tinymist, no mesmo processo.
  - (+) Instaladores leves.
  - (−) A interface depende do WebView de cada sistema, que muda entre plataformas.

## D-02 Sugestões como atributos do CRDT, fora do `.typ`

- **Status:** aceita · novembro de 2026
- **Contexto:** o `.typ` no repositório deve ser Typst puro, compilável fora do Prelo (RNF-05). Sugestões precisam marcar trechos sem alterar esse arquivo.
- **Decisão:** sugestões são atributos de formatação sobre trechos do `Y.Text`; comentários e sugestões persistem em `.prelo/review.json`. Ver [arquitetura.md §4](arquitetura.md#4-modelo-de-revisão).
- **Alternativas consideradas:** marcações dentro do próprio `.typ` (descartada: suja o código-fonte e quebra quem não usa o Prelo).
- **Consequências:**
  - (+) Quem não usa o Prelo vê um repositório Typst normal.
  - (−) Edições feitas fora do app chegam como diff do git e precisam virar sugestões de um autor "externo"; o tratamento de trechos alterados nos dois lados ainda está em aberto ([arquitetura.md §6](arquitetura.md#6-riscos-técnicos-e-questões-em-aberto)).

## D-03 Comentários são resolvidos, não aceitos

- **Status:** aceita · novembro de 2026
- **Contexto:** comentários e sugestões aparecem no mesmo painel de revisão, mas só a sugestão muda o texto.
- **Decisão:** comentários têm os estados aberto e resolvido; não existe aceitar ou rejeitar comentário.
- **Alternativas consideradas:** não registradas na especificação.
- **Consequências:**
  - (+) Resolver indica que o pedido foi atendido, sem ambiguidade sobre o texto.
  - (+) Resolvidos ficam numa aba própria, de onde se reabre ou exclui (RF-17).

## D-04 Conexão P2P com iroh, sem servidor obrigatório

- **Status:** aceita · novembro de 2026
- **Contexto:** artigos inéditos não devem ficar em servidor de terceiros (RNF-02), e o grupo não deve ter custo de infraestrutura. O Yjs já resolve conflitos localmente; falta só os computadores se encontrarem.
- **Decisão:** mDNS na rede local e iroh pela internet, conectando pela chave pública de cada pessoa. Relay iroh só para conectar quando o NAT bloqueia, com tráfego criptografado de ponta a ponta.
- **Alternativas consideradas:**
  - y-webrtc: pronto para Yjs, mas precisa de servidor de sinalização e de TURN em redes fechadas.
  - libp2p: mesma função do iroh, mais complexo.
- **Consequências:**
  - (+) Roda no mesmo núcleo Rust, ao lado do yrs; não depende de endereço IP.
  - (−) Redes que bloqueiam UDP forçam todo o tráfego pelo relay. Falta decidir entre relays públicos do iroh e relay próprio do grupo.

## D-05 Git como canal para quem não está online junto

- **Status:** aceita · novembro de 2026
- **Contexto:** coautores muitas vezes trabalham em horários diferentes, e sem servidor (D-04) não há onde guardar as mudanças de quem saiu.
- **Decisão:** Salvar e fechar gravam commit e fazem push no remoto git do grupo; quem abre depois faz pull.
- **Alternativas consideradas:** servidor de armazenamento próprio (descartado pelo mesmo motivo de D-04).
- **Consequências:**
  - (+) O grupo já usa GitHub ou GitLab; nenhum serviço novo.
  - (−) "Sincronizado" só cobre quem está online e o que já está no git; mudanças de quem ainda não reconectou chegam depois.

## D-06 Offline só comenta e sugere

- **Status:** aceita · novembro de 2026
- **Contexto:** edições diretas feitas offline podem sobrescrever trabalho de outra pessoa sem que ela veja.
- **Decisão:** offline, só comentar, responder, resolver e sugerir; essas ações vão para uma fila no disco (RF-30).
- **Alternativas consideradas:** não registradas na especificação.
- **Consequências:**
  - (+) Tudo o que é feito offline passa por revisão antes de mudar o texto.
  - (−) Quem trabalha offline não pode editar direto; o item sobre trecho que mudou exige decisão do usuário ao reconectar (RF-37).

## D-07 Edição liberada só com arquivos sincronizados

- **Status:** aceita · novembro de 2026
- **Contexto:** editar sobre uma versão antiga gera trabalho em cima de texto que já mudou.
- **Decisão:** ao abrir, reconectar ou salvar, o app recebe primeiro as mudanças e mantém o editor em leitura até terminar (RF-31, RF-32).
- **Alternativas consideradas:** não registradas na especificação.
- **Consequências:**
  - (+) Evita editar sobre versão antiga.
  - (−) Sincronização lenta atrasa a edição. Mitigação: o bloqueio vale só para o texto; figuras terminam depois sem travar.

## D-08 Salvar (⌘S) sincroniza automaticamente

- **Status:** aceita · novembro de 2026
- **Contexto:** receber, enviar e gravar no git em gestos separados deixa espaço para esquecer um deles.
- **Decisão:** ⌘S recebe, envia a fila, grava commit e faz push. Sem conexão, guarda local e agenda (RF-33, RF-34).
- **Alternativas consideradas:** não registradas na especificação.
- **Consequências:**
  - (+) Um único gesto conhecido garante as três etapas.
  - (−) Cada Salvar gera um commit; o histórico do git fica com muitos commits `Prelo sync`.

## D-09 Aceitar e rejeitar sugestões bloqueados offline

- **Status:** aceita · novembro de 2026
- **Contexto:** aceitar ou rejeitar muda o texto, como uma edição direta.
- **Decisão:** essas ações seguem D-07 e só ficam disponíveis com os arquivos sincronizados.
- **Alternativas consideradas:** não registradas na especificação.
- **Consequências:**
  - (+) Regra única: tudo o que muda o texto exige estar sincronizado.
  - (−) Quem revisa offline acumula pendências até reconectar.

## D-10 Compilador Typst embutido no núcleo

- **Status:** aceita · novembro de 2026
- **Contexto:** prévia, diagnósticos e PDFs precisam sair do mesmo compilador e na mesma versão para todos os coautores, sem exigir instalação separada (RF-06, RF-07).
- **Decisão:** typst e Tinymist entram como bibliotecas Rust no núcleo. A versão do compilador fica registrada em `.prelo/project.json`.
- **Alternativas consideradas:** chamar o `typst` instalado na máquina (descartada: versões diferentes entre coautores e dependência de instalação manual).
- **Consequências:**
  - (+) Ninguém precisa instalar `typst`; prévia e PDF final batem.
  - (−) Atualizar o Typst exige nova versão do Prelo, e uma versão nova pode mudar o layout. Falta definir quem decide a troca de versão no projeto.
  - (−) Pacotes `@preview` dependem da internet. Falta definir cache local para compilar offline e quais fontes vêm embutidas.
