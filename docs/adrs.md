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
| [D-11](#d-11-mpl-20-só-como-exceção-por-dependência) | MPL-2.0 só como exceção por dependência | Aceita |
| [D-12](#d-12-transporte-de-arquivos-binários-entre-coautores) | Transporte de arquivos binários entre coautores | Aceita |
| [D-13](#d-13-arquivos-fora-do-projeto) | Arquivos fora do projeto | Aceita |
| [D-14](#d-14-edição-concorrente-de-bibliografia) | Edição concorrente de bibliografia | Aceita |
| [D-15](#d-15-fontes-embutidas--pasta-do-projeto-licença-ofl-11-aceita-só-para-arquivos-de-fonte) | Fontes: embutidas + pasta do projeto; licença OFL-1.1 aceita só para arquivos de fonte | Aceita |
| [D-16](#d-16-cc-by-40-e-w3c-só-como-exceção-da-crate-typst-assets) | CC-BY-4.0 e W3C só como exceção da crate `typst-assets` | Aceita |

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
  - (−) Edições feitas fora do app chegam como diff do git e precisam virar sugestões de um autor "externo"; o tratamento de trechos alterados nos dois lados ainda está em aberto ([arquitetura.md §7](arquitetura.md#7-riscos-técnicos-e-questões-em-aberto)).

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

## D-11 MPL-2.0 só como exceção por dependência

- **Status:** aceita · 2026-11-18
- **Contexto:** RNF-06 exige dependências com licenças permissivas (Apache-2.0, MIT). A árvore de dependências do Tauri (D-01) costuma incluir crates sob MPL-2.0, como `cssparser` e `selectors`. MPL-2.0 é copyleft por arquivo: usar sem modificar não impõe obrigações ao código do Prelo.
- **Decisão:** MPL-2.0 não entra na lista geral de licenças permitidas. Cada crate MPL-2.0 só é aceita como exceção nomeada em `src-tauri/deny.toml`, com motivo, e apenas se usada sem modificação. Nova exceção exige PR.
- **Alternativas consideradas:**
  - Recusar MPL-2.0: bloquearia o Tauri e contradiria D-01.
  - Incluir MPL-2.0 na lista geral: permitiria novas dependências copyleft sem revisão.
- **Consequências:**
  - (+) O Tauri pode ser usado sem relaxar a regra para o resto do projeto.
  - (−) Cada atualização que traga nova crate MPL-2.0 falha na auditoria até alguém registrar a exceção.
  - (−) Modificar uma dessas crates (fork ou patch) obrigaria publicar os arquivos alterados sob MPL-2.0.

## D-12 Transporte de arquivos binários entre coautores

- **Status:** aceita · 2026-11-18
- **Contexto:** o documento Yjs cobre texto, sugestões e comentários (D-02). Imagens, PDFs usados como figura, `.csl` e outros arquivos não textuais não têm transporte definido entre pares nem fora do git. O fluxo de entrada já supõe que "o texto chega primeiro, depois figuras e histórico" ([requisitos.md §6.1](requisitos.md#61-entrada-no-projeto), passo 7), e o protótipo mostra "Figuras · 118 de 131 arquivos" vindo direto do computador do dono, criptografado. A prévia precisa saber se um arquivo ausente é pendente ou inexistente (RF-52), o que exige uma lista de arquivos do projeto conhecida antes dos bytes.
- **Decisão:** manifesto de arquivos num `Y.Map` do documento Yjs (caminho → hash do conteúdo, tamanho, autor, hora) e bytes transferidos sob demanda por iroh, de qualquer par online que os tenha, com verificação do hash ao receber. Salvar e fechar continuam gravando os arquivos no commit (D-05), para quem não está online junto.
- **Alternativas consideradas:**
  - Só git: binários viajam apenas por commit, push e pull. Simples e já previsto em D-05, mas durante a edição em tempo real uma figura nova só aparece para o outro coautor depois que o autor salva e o outro recebe; exige remoto git mesmo para quem está online junto; não dá a lista de arquivos pendentes antes do pull.
  - Bytes dentro do documento Yjs (`Uint8Array` num `Y.Map`): um único canal, mas o documento CRDT cresce com cada versão de cada figura, a sincronização inicial transfere tudo junto e o texto deixa de chegar antes das figuras.
  - Git LFS: exige servidor LFS e cotas do provedor; não serve à conexão direta entre pares.
- **Consequências:**
  - (+) Atende o fluxo 6.1 e o protótipo: texto primeiro, figuras depois, sem travar a edição (D-07).
  - (+) O manifesto permite distinguir pendente de inexistente (RF-52) e bloquear o PDF final só quando necessário (RF-53).
  - (+) Tráfego criptografado de ponta a ponta pela mesma conexão iroh (Princípio II, D-04); o hash valida bytes vindos de outro par (Princípio VI).
  - (−) Um componente a mais no núcleo (protocolo de blobs sobre iroh). TODO(licença-blobs): confirmar a licença da biblioteca escolhida frente a RNF-06.
  - (−) Duas escritas simultâneas no mesmo caminho do `Y.Map` ficam com uma só versão no manifesto. Para não perder trabalho (Princípio III), a outra versão **deveria** ser guardada e o usuário avisado. TODO(conflito-binario): como apresentar a versão perdedora.
  - (−) "Sincronizado" passa a conviver com arquivos ainda pendentes; o Princípio IV fala em receber tudo antes de liberar a edição, e D-07 já limita o bloqueio ao texto. TODO(estado-arquivos): confirmar com o dono que arquivos pendentes não bloqueiam a edição.
  - Em aberto: TODO(permissoes-arquivos): quem pode adicionar, trocar ou remover arquivos (revisor? offline?), já que a tabela de papéis só fala de texto. TODO(limite-arquivo): limite de tamanho por arquivo, se houver.

## D-13 Arquivos fora do projeto

- **Status:** aceita · 2026-11-18
- **Contexto:** pesquisadores costumam manter um `.bib` único para vários artigos, fora da pasta de cada um. RF-05 fala em `.bib` e figuras "dentro do projeto", e RF-08 recusa qualquer leitura fora da raiz, porque o `.typ` é entrada não confiável (Princípio VI). Um caminho fora do projeto também não existe no computador dos outros coautores nem em quem compila o repositório fora do Prelo (Princípio I).
- **Decisão:** não permitir arquivos fora do projeto. O app oferece copiar o arquivo para dentro do projeto quando o `.typ` aponta para fora, e a cópia passa a ser versionada e sincronizada como os demais arquivos.
- **Alternativas consideradas:**
  - Raízes extras declaradas em `.prelo/project.json`: o dono autoriza pastas fora do projeto. Quebra RF-08 como está escrito, depende de caminhos que só existem na máquina de quem configurou e torna o `.typ` não compilável fora do Prelo sem reproduzir a mesma estrutura de pastas.
  - Cópia com vínculo: o app copia para dentro do projeto e guarda em `.prelo/` o caminho de origem, com um comando "Atualizar do original". Mantém o projeto autocontido, mas o caminho de origem só vale para um computador e a atualização é manual.
  - Submódulo git com o `.bib` compartilhado: padrão do git e compilável fora do Prelo, mas exige que todos entendam submódulos e que o Prelo os suporte em commit, push e pull. TODO(gitoxide-submodulos): verificar suporte a submódulos no gitoxide.
- **Consequências:**
  - (+) Projeto autocontido: o mesmo `.typ` compila no Prelo, no `typst` de linha de comando e no computador de qualquer coautor (Princípio I, RNF-05).
  - (+) Nenhuma exceção à regra de escopo (RF-08).
  - (−) O `.bib` compartilhado vira várias cópias que divergem com o tempo; corrigir uma entrada exige repetir a correção em cada artigo.
  - (−) Pacotes `@local`, que também ficam fora do projeto, deixam de funcionar no Prelo ou precisam ser copiados para o projeto. TODO(pacotes-local): confirmar.
  - A cópia com vínculo pode entrar depois sem invalidar esta escolha.

## D-14 Edição concorrente de bibliografia

- **Status:** aceita · 2026-11-18
- **Contexto:** `.bib` e `.yml` (RF-09) são texto e podem ser editados por dois coautores ao mesmo tempo, como o `.typ`. O documento Yjs hoje cobre o texto do artigo (D-02); a arquitetura não diz se a bibliografia entra nele. Ferramentas externas, como exportação automática de gerenciadores de referência, costumam reescrever o arquivo inteiro.
- **Decisão:** `.bib` e `.yml` como `Y.Text` no documento Yjs, com as mesmas regras do `.typ` (edição direta só sincronizado, sugestões, comentários). `.csl` e binários como arquivo inteiro, pelo transporte de D-12.
- **Alternativas consideradas:**
  - Arquivo inteiro, última gravação vence: simples, mas uma das duas edições simultâneas some, o que contraria o Princípio III.
  - Arquivo inteiro com detecção de conflito: guarda o hash da versão base e, se as duas versões mudaram, pede ao usuário que escolha ou junte. Não perde trabalho, mas exige uma tela de conflito e interrompe quem edita referências com frequência.
  - Fusão estruturada por entrada (pela chave de citação): o resultado é sempre BibLaTeX válido entrada a entrada, mas exige analisar e regravar o arquivo, o que pode perder comentários e formatação, e não serve para `.yml` sem um segundo analisador.
- **Consequências:**
  - (+) Um único modelo para todo texto do projeto: CRDT, fila offline, sugestões e comentários já especificados (D-02, D-06, D-07).
  - (+) Edições em entradas diferentes se fundem sem conflito.
  - (+) Mudança de revisor em referência vira sugestão, como no texto (Princípio V).
  - (−) Duas pessoas editando a mesma entrada ao mesmo tempo podem gerar BibLaTeX inválido; o erro aparece como diagnóstico (RF-09), não é evitado.
  - (−) Reescrita do arquivo inteiro por ferramenta externa chega como diff e cai no mesmo problema em aberto da edição fora do app ([arquitetura.md §7](arquitetura.md#7-riscos-técnicos-e-questões-em-aberto)).
  - (−) TODO(bib-grande): medir o custo de um `.bib` com milhares de entradas como `Y.Text` no carregamento e na sincronização.

## D-15 Fontes: embutidas + pasta do projeto; licença OFL-1.1 aceita só para arquivos de fonte

- **Status:** aceita · 2026-11-19 · decisor: dono do projeto (clarificação Q1 da feature 003)
- **Contexto:** o `.typ` pede fontes pelo nome; quem decide quais existem é o World do núcleo ([arquitetura.md §6.5](arquitetura.md#65-fontes-e-pacotes)). D-10 quer que todos os coautores compilem igual, o que fontes do sistema impedem. Quase todas as fontes livres têm licença própria, em geral SIL OFL-1.1, que RNF-06 não cita. As fontes que o Typst traz embutidas (crate `typst-assets` 0.15.1) têm licenças variadas: Libertinus Serif sob OFL-1.1; New Computer Modern sob GUST Font License (LPPL) e, no caso de NewCM10-Regular, GPL-3.0 com Font Exception; DejaVu Sans Mono sob a licença Bitstream Vera.
- **Decisão:**
  - O World entrega só (a) as fontes embutidas no app e (b) os arquivos de fonte da pasta `fonts/` do projeto, que segue RF-08. Fontes instaladas no sistema nunca são lidas.
  - Arquivos de fonte embutidos no app podem ter licença OFL-1.1, além das licenças permitidas por RNF-06. A exceção vale só para arquivos de fonte: código e demais dependências sob OFL-1.1 continuam proibidos.
  - Fontes da pasta `fonts/` do projeto são conteúdo do artigo, como figuras: o app não as distribui nem as audita; a licença de cada uma é responsabilidade de quem a adiciona.
- **Alternativas consideradas:**
  - Só fontes embutidas: reprodutível, mas o artigo fica limitado ao que o app traz, e modelos de revista com fonte própria não funcionam.
  - Embutidas + fontes do sistema: cômodo, mas prévia e PDF mudam de um computador para outro, o que anula parte de D-10.
  - Aceitar também GUST/LPPL e GPL com Font Exception para fontes embutidas: usaria o conjunto do Typst sem mudança, mas amplia a regra de licenças para termos copyleft que o projeto evita (RNF-06, D-11).
- **Consequências:**
  - (+) Todos os coautores, na mesma versão do Prelo, compilam com o mesmo conjunto de fontes.
  - (+) Modelos com fonte própria funcionam com a fonte copiada para `fonts/`, e o `.typ` compila fora do Prelo com `typst compile --font-path fonts` (Princípio I).
  - (−) As fontes do `typst-assets` sob GUST, GPL e Bitstream Vera não são embutidas, porque a exceção é só para OFL-1.1. O Prelo embute substitutas sob OFL-1.1, e o padrão do Typst para matemática (New Computer Modern Math) e para código (DejaVu Sans Mono) fica diferente do `typst` de linha de comando; quem precisar delas as coloca em `fonts/`.
  - (−) A comparação de PDF com o `typst` de linha de comando precisa usar o mesmo conjunto de fontes (`--ignore-system-fonts --ignore-embedded-fonts --font-path`).
  - (−) A OFL-1.1 exige distribuir o aviso de copyright e o texto da licença junto com as fontes; o app precisa exibi-los.
  - (−) A auditoria de licenças por crate não enxerga arquivos de fonte embutidos; é preciso uma verificação própria desses arquivos.

## D-16 CC-BY-4.0 e W3C só como exceção da crate `typst-assets`

- **Status:** aceita · 2026-11-19 · decisor: dono do projeto
- **Contexto:** o compilador embutido (D-10) depende da crate `typst-assets`, declarada Apache-2.0. Mesmo sem a feature `fonts`, ela compila para dentro do binário dados com licenças próprias, listadas no `NOTICE` da crate (versão 0.15.1):
  - perfis de cor ICC (`files/icc/`), sob CC0-1.0;
  - fontes Foxit (`files/fonts/Foxit*.pfb`), usadas ao embutir PDFs como imagem, sob BSD-3-Clause;
  - tabelas de HTML (`files/html/data.rs`) derivadas das especificações do WHATWG, sob CC-BY-4.0 (as partes incorporadas em código-fonte, sob BSD-3-Clause);
  - tabelas de HTML e MathML (`files/html/data.rs`, `files/mathml/data.rs`) derivadas de especificações do W3C (WAI-ARIA 1.1, Referrer Policy, MathML Core), sob a licença de software e documento do W3C (SPDX `W3C-20150513`).

  CC0-1.0 e BSD-3-Clause já estão na lista de RNF-06; CC-BY-4.0 e W3C não. Sem esses dados não há `typst` e, portanto, não há compilação embutida.
- **Decisão:** CC-BY-4.0 e W3C-20150513 são aceitas só como exceção nomeada da crate `typst-assets` em `src-tauri/deny.toml` (`[[licenses.exceptions]]`), no mesmo padrão de D-11. Não entram na lista geral de licenças permitidas. A exceção é coberta por teste: só `typst-assets` pode tê-la, e as duas licenças não podem aparecer na lista geral. Os avisos de atribuição exigidos pelas duas licenças (texto do `NOTICE` da crate) acompanham o app na tela "Sobre", junto das licenças das fontes (D-15).
- **Alternativas consideradas:**
  - Incluir CC-BY-4.0 e W3C na lista geral: outras dependências poderiam trazer essas licenças sem revisão.
  - *Fork* do `typst-assets` sem esses dados: quebra recursos do Typst (exportação HTML, MathML, atributos ARIA) e obriga a manter um *fork* a cada versão do Typst.
  - Recusar a crate: impossível sem abandonar o compilador embutido (D-10).
- **Consequências:**
  - (+) O compilador embutido fica, sem relaxar a regra para o resto do projeto.
  - (+) Nova crate com CC-BY-4.0 ou W3C falha na auditoria até alguém registrar outra decisão.
  - (−) O `cargo-deny` lê só a licença declarada da crate (Apache-2.0); a exceção documenta e trava o escopo, e a verificação dos arquivos embutidos (D-15) lista os dados e suas licenças.
  - (−) Cada atualização do `typst-assets` exige reler o `NOTICE` e ajustar a exceção se surgir licença nova.
  - (−) O app precisa exibir os avisos de atribuição de CC-BY-4.0 e W3C.
