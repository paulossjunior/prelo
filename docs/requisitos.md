# Prelo · Especificação de Requisitos

Versão 0.1 · novembro de 2026 · rascunho para discussão

Este documento é a fonte canônica dos requisitos do Prelo (RF-xx e RNF-xx). Contexto e escopo
estão em [especificacao.md](especificacao.md); como o sistema atende aos requisitos está em
[arquitetura.md](arquitetura.md); o porquê das escolhas está em [adrs.md](adrs.md).

**Como este documento está organizado.** Papéis, RNFs, regras e fluxos ficam em seções próprias.
Os requisitos funcionais (§2) estão agrupados por **épico → história de usuário (US) → RF**,
na ordem das releases de [release-plan.md](release-plan.md), com link para a issue de cada
épico e história. Os IDs e o texto dos RFs não dependem desse agrupamento. O estado de épicos e
histórias (aberto, em andamento, fechado) não é registrado aqui: a fonte de verdade são as
issues do GitHub, resumidas em [backlog.md](backlog.md).

---

## Sumário

1. [Papéis e permissões](#1-papéis-e-permissões)
2. [Requisitos funcionais](#2-requisitos-funcionais), por épico e história
3. [Requisitos não funcionais](#3-requisitos-não-funcionais)
4. [Regras de revisão](#4-regras-de-revisão)
5. [Regras de offline e sincronização](#5-regras-de-offline-e-sincronização)
6. [Fluxos](#6-fluxos)
7. [PDF de diferenças](#7-pdf-de-diferenças)

---

## 1. Papéis e permissões

| Papel | Ler | Comentar | Sugerir | Editar direto | Aceitar / rejeitar sugestões | Convidar e mudar papéis |
|---|---|---|---|---|---|---|
| Dono | Sim | Sim | Sim | Sim | Sim | Sim |
| Coautor | Sim | Sim | Sim | Sim | Sim | Não |
| Revisor | Sim | Sim | Sim (todas as edições viram sugestões) | Não | Não | Não |
| Leitor | Sim | Não | Não | Não | Não | Não |

Regras sobre comentários:

| Ação | Quem pode |
|---|---|
| Resolver comentário | Todos, menos leitor |
| Reabrir comentário resolvido | Todos, menos leitor |
| Apagar a própria mensagem | Autor da mensagem |
| Excluir conversa inteira | Quem abriu a conversa e o dono |
| Excluir todos os resolvidos | Dono |

## 2. Requisitos funcionais

Os RFs estão agrupados por **épico** (um por release, na ordem de
[release-plan.md](release-plan.md)) e, dentro do épico, por **história de usuário (US)**. Onde
ainda não há histórias (R2 a R5), os RFs aparecem direto sob o épico planejado, agrupados pelo
tema original (Escrita, Revisão, Colaboração, Offline e sincronização, Entrega). IDs e texto
dos RFs não mudaram; só a posição. RF dividido entre releases aparece uma vez, na primeira
release, com a nota "parte … na R…" (alocação de
[release-plan.md §5](release-plan.md#5-rastreabilidade-rfrnf--release)). A
[tabela de verificação](#verificação-rf--épico--us) no fim da seção lista todos os RFs.

Os requisitos de escrita usam RF-01 a RF-09 e continuam em RF-50, porque RF-10 já abre a faixa de revisão.

### Épico: Publicar instaladores no GitHub Releases por tag de versão — [#35](https://github.com/paulossjunior/prelo/issues/35)

Release R0 Fundação · versão `0.1.0`. Inclui também o esqueleto do app,
[#1](https://github.com/paulossjunior/prelo/issues/1).

Nenhum RF. Esta release atende RNF-01 e RNF-06 (D-01). Histórias:
[#149](https://github.com/paulossjunior/prelo/issues/149),
[#150](https://github.com/paulossjunior/prelo/issues/150),
[#152](https://github.com/paulossjunior/prelo/issues/152),
[#153](https://github.com/paulossjunior/prelo/issues/153).

### Épico: Construir o editor local com prévia e compilador Typst embutido — [#38](https://github.com/paulossjunior/prelo/issues/38)

Release R1 Editor local · versão `1.0.0` · spec `specs/003-editor-local/spec.md`. Histórias e
RFs conforme a spec 003; #154, #155 e #158 bloqueiam a `1.0.0`.

#### US1 — Escrever com prévia ao vivo [#154](https://github.com/paulossjunior/prelo/issues/154)

- **RF-01** Editor de código Typst com realce de sintaxe, autocompletar e diagnósticos fornecidos pelo compilador Typst embutido (`typst-ide` e `typst-syntax` oficiais, D-17).
- **RF-02** Prévia ao lado do editor, atualizada enquanto se digita (meta: menos de 100 ms em artigos de até 40 páginas).
- **RF-06** O Prelo compila Typst com compilador embutido, sem exigir `typst` instalado na máquina. A mesma compilação gera prévia, diagnósticos e todos os PDFs exportados.
  - Nota: também em US5 ([#158](https://github.com/paulossjunior/prelo/issues/158)), para o PDF exportado.
- **RF-07** A versão do compilador embutido aparece no app e fica registrada em `.prelo/project.json`, para que todos os coautores compilem com a mesma versão.
  - Nota: também em US6 ([#159](https://github.com/paulossjunior/prelo/issues/159)), no registro criado com o projeto novo.

#### US2 — Projeto confinado à própria pasta [#155](https://github.com/paulossjunior/prelo/issues/155)

- **RF-08** O compilador embutido só lê **arquivos regulares** cujo caminho real fica dentro da raiz do projeto, depois de resolver `..` e links simbólicos. Caminho que sairia da raiz, link simbólico que aponta para fora dela e arquivo especial (dispositivo, FIFO, socket) são recusados sem ler o conteúdo, com diagnóstico que cita o caminho pedido e o motivo. A regra vale para `#include`, `#import`, `image`, `bibliography`, estilo `.csl`, `read`, funções de dados (`json`, `yaml`, `toml`, `csv`, `xml`, `cbor`), `plugin` e para o autocompletar de caminhos do Tinymist. Caminho começando com `/` é relativo à raiz do projeto, como no Typst, e é permitido. Arquivos de pacotes seguem a mesma regra, com a pasta do pacote no cache como raiz. Ver [arquitetura.md §6](arquitetura.md#6-acesso-a-arquivos-pelo-compilador).
  - Origem: Princípio VI (o `.typ` vem de outros coautores e é entrada não confiável), RF-05, RF-06.
  - Verificação: testes de segurança com projetos de fixture contendo `#image("../../fora.png")`, `#read("/../fora.txt")`, `#include` de caminho subindo além da raiz, link simbólico versionado apontando para fora, link simbólico interno apontando para dentro (aceito) e FIFO na raiz. Cada caso recusado deve gerar o diagnóstico e nenhuma chamada de leitura fora da raiz; o autocompletar não deve listar nada fora da raiz.

#### US3 — Bibliografia e figuras do projeto [#156](https://github.com/paulossjunior/prelo/issues/156)

- **RF-05** Suporte a `.bib` e figuras dentro do projeto.
- **RF-09** A bibliografia aceita arquivos BibLaTeX `.bib` e Hayagriva `.yml`/`.yaml`, um ou vários, e estilos de citação CSL personalizados (`.csl`), além dos estilos embutidos do Typst. Todos seguem RF-08. Arquivo malformado gera diagnóstico com arquivo e linha, sem encerrar o app.
  - Origem: RF-05, RF-06; Princípio VI (entrada malformada).
  - Verificação: projetos de fixture com `.bib`, `.yml` e `.csl` compilam no Prelo e geram o mesmo PDF que o `typst` de linha de comando na versão registrada em `.prelo/project.json`; fixtures malformadas (chave não fechada, YAML inválido, XML de CSL inválido) geram diagnóstico e o app continua respondendo.
- **RF-50** Figuras aceitam os formatos de imagem suportados pela versão do compilador registrada em `.prelo/project.json`: PNG, JPEG, GIF (sempre estática) e SVG; PDF e WebP a partir do Typst 0.14.0. TODO(versão-typst): definir a versão do Typst embutida na primeira release; se for anterior à 0.14.0, PDF e WebP ficam fora.
  - Origem: RF-05, RF-07.
  - Verificação: um projeto de fixture com uma figura de cada formato compila e mostra todas na prévia e no PDF; arquivo com extensão de imagem e conteúdo inválido gera diagnóstico sem encerrar o app.
- **RF-51** Uma edição que não altera os bytes de uma imagem não a decodifica de novo: o núcleo reaproveita a imagem já carregada enquanto o conteúdo do arquivo não mudar. A meta de RF-02 vale também para artigos com imagens grandes, quando a edição não toca a imagem.
  - Origem: RF-02, RNF-03.
  - Verificação: teste que conta decodificações de imagem ao longo de uma sequência de edições de texto (deve ser uma por imagem); medição do tempo de prévia após edição de texto num artigo de fixture com imagens grandes. TODO(imagem-grande): definir o tamanho de referência da imagem grande (pixels e bytes) e se há meta para a primeira compilação depois que a imagem chega ou muda.

#### US4 — Navegar entre código e prévia [#157](https://github.com/paulossjunior/prelo/issues/157)

- **RF-03** Sincronização entre código e prévia: clicar na prévia leva à linha do código, e vice-versa.
- **RF-04** Árvore de arquivos e estrutura de seções (outline) com contagem de pendências de revisão por seção.
  - Nota: na R1, sem a contagem de pendências; parte contagem de comentários na R2 e de sugestões na R4.

#### US5 — Exportar o PDF [#158](https://github.com/paulossjunior/prelo/issues/158)

- **RF-42** Exportar PDF final e `.typ` limpos, sem marcações nem comentários.
  - Nota: na R1, só o PDF final; parte `.typ` limpo na R4 (proposta, release-plan Q-03).

Esta história também cobre RF-06 (ver US1).

#### US6 — Criar um projeto novo [#159](https://github.com/paulossjunior/prelo/issues/159)

Sem RF próprio. Cobre RF-07 (ver US1): o projeto novo já nasce com a versão do compilador
registrada em `.prelo/project.json`.

### Épico: Comentários e histórico — (épico a criar)

Release R2 · versão `1.1.0` (proposta). Sem histórias ainda.

#### Revisão

- **RF-16** Comentários ancorados num trecho, com respostas, menções (@nome), resolver, reabrir e excluir.
- **RF-17** Aba "Resolvidos" com as conversas resolvidas, de onde se pode reabrir ou excluir.

#### Offline e sincronização

- **RF-32** Ao abrir o projeto, o app verifica versões com quem está online e com o git remoto. Se houver mudanças, recebe-as antes de liberar a edição, mantendo o editor em modo leitura.
  - Nota: na R2, só com o git remoto; parte dos pares na R3.
- **RF-33** **Salvar (⌘S) sincroniza automaticamente**: recebe as mudanças dos outros, envia o trabalho local e grava um commit no git.
- **RF-34** Salvar sem conexão guarda no computador e agenda a sincronização para quando a conexão voltar.
- **RF-36** Ao fechar o app, ele faz um último Salvar.
- **RF-38** Após sincronizar, mostrar um resumo do que mudou ("Bruno fez 2 commits, Ana deixou 1 comentário").
  - Nota: na R2, só o que veio do git; parte dos pares na R3.

#### Entrega

- **RF-40** Commit e push pelo próprio app.
- **RF-41** Versões nomeadas no histórico ("Submissão 1", "Revisão R1").
- **RF-43** Exportar PDF com comentários na margem (para orientadores).
  - Nota: alocação na R2 é proposta (release-plan Q-03).

### Épico: Tempo real sem servidor — (épico a criar)

Release R3 · versão `1.2.0` (proposta). Sem histórias ainda.

#### Escrita

- **RF-52** Arquivo referenciado que o projeto já conhece mas que ainda não chegou ao computador (**pendente**) não impede a prévia. Imagem pendente aparece como marcador do mesmo tamanho pedido no `.typ`, com o nome do arquivo. Bibliografia, `.typ` incluído ou dado pendente mantém na prévia a última compilação bem-sucedida, com aviso "Aguardando <arquivo>". Arquivo que o projeto não conhece gera o diagnóstico normal de arquivo não encontrado. Quando o arquivo chega, a prévia recompila sem ação do usuário.
  - Origem: RF-02, RF-05; fluxo 6.1, passo 7 (o texto chega antes das figuras); [arquitetura.md §7](arquitetura.md#7-riscos-técnicos-e-questões-em-aberto) ("Sincronização lenta").
  - Verificação: teste de integração com dois pares em que o texto chega e a figura é retida; a prévia do receptor mostra o marcador e o aviso, e recompila ao liberar a figura. O app sabe que um arquivo é "pendente" pelo manifesto de arquivos de D-12 (aceita em 2026-11-18). TODO(marcador-tamanho): imagem pendente sem `width`/`height` no `.typ` não tem tamanho conhecido; definir o tamanho do marcador.
- **RF-53** Exportar o PDF final (RF-42) fica bloqueado enquanto houver arquivo pendente referenciado pelo artigo. O app lista os arquivos que faltam.
  - Origem: RF-42 (PDF final limpo, sem marcações) e Princípio I (o `.typ` precisa compilar fora do Prelo, onde não existe marcador).
  - Verificação: com uma figura pendente, a ação Exportar PDF final aparece desativada e lista o arquivo; ao chegar a figura, a ação é liberada. TODO(export-revisao): decidir se o PDF com comentários (RF-43) e o PDF de diferenças (RF-44) podem sair com marcador.

#### Colaboração

- **RF-20** Edição simultânea com cursores e presença visíveis.
- **RF-21** Convite por código de uso único, com papel, validade e opção de exigir aprovação do dono.
  - Nota: na R3, o código de uso único; parte papel, validade e aprovação do dono na R5.
- **RF-22** Identidade local por chave criptográfica, sem conta nem senha. O dono confere o código da chave antes de aprovar uma entrada.
  - Nota: na R3, a identidade por chave; parte conferência do código da chave pelo dono na R5.
- **RF-23** Conexão direta entre computadores, sem servidor obrigatório (ver [arquitetura.md §3](arquitetura.md#3-conexão-sem-servidor)).

#### Offline e sincronização

- **RF-30** Offline, é possível comentar, responder, resolver e sugerir. Essas ações vão para uma fila persistida no disco.
  - Nota: na R3, comentar, responder e resolver; parte sugerir offline na R4.
- **RF-31** Editar direto e aceitar ou rejeitar sugestões só são liberados com os arquivos sincronizados.
  - Nota: na R3, editar direto; parte aceitar e rejeitar sugestões na R4.
- **RF-35** Ao reconectar, a ordem é sempre: receber, enviar a fila, gravar no git, liberar a edição.
- **RF-37** Comentário ou sugestão feito offline sobre um trecho que mudou nesse meio tempo não é descartado. O app pergunta se deve mantê-lo no trecho novo ou excluí-lo.

### Épico: Modo sugestão — (épico a criar)

Release R4 · versão `1.3.0` (proposta). Sem histórias ainda. Recebe também as partes R4 de
RF-04 (contagem de sugestões), RF-30 (sugerir offline), RF-31 (aceitar e rejeitar só
sincronizado) e RF-42 (`.typ` limpo).

#### Escrita

- **RF-54** Erro causado só por arquivo pendente não conta como "sugestão que quebra a compilação": não desativa o Aceitar (RF-15).
  - Origem: RF-15, Princípio V.
  - Verificação: com uma figura pendente, uma sugestão válida de texto continua com Aceitar ativo; uma sugestão que apaga um `]` continua com Aceitar desativado.

#### Revisão

- **RF-10** Três modos de trabalho: **Editando** (alterações entram direto), **Sugerindo** (alterações viram sugestões) e **Lendo** (só leitura e comentários).
- **RF-11** Sugestões de inserção, exclusão e substituição, marcadas no código e na prévia com a cor do autor.
- **RF-12** Aceitar ou rejeitar uma sugestão pelo cartão no painel de revisão ou clicando no trecho marcado. Atalhos: ⌘↵ aceita, ⌘⌫ rejeita, ⌥↓ vai para a próxima.
- **RF-13** Aceitar ou rejeitar em lote, com filtro por pessoa e por seção, e confirmação na própria tela mostrando a contagem.
- **RF-14** Desfazer um aceite ou rejeição (⌘Z ou botão Desfazer no aviso).
- **RF-15** Testar cada sugestão em segundo plano. Uma sugestão que quebra a compilação aparece com aviso e o botão Aceitar desativado.
- **RF-18** Prévia em dois modos: com marcações ou só com o texto aceito.
- **RF-19** Ao aceitar sugestões, propor um commit com mensagem pronta (ex.: "Accept 3 suggestions from Ana in Methodological Approach").

#### Entrega

- **RF-44** Exportar PDF de diferenças entre duas versões (ver [seção 7](#7-pdf-de-diferenças)).
- **RF-45** Tela "Preparar submissão": lista sugestões pendentes e comentários abertos, e permite excluir todos os resolvidos de uma vez.
  - Nota: alocação na R4 é proposta (release-plan Q-03).

### Épico: Papéis e convites — (épico a criar)

Release R5 · versão `1.4.0` (proposta). Sem histórias ainda. Recebe também as partes R5 de
RF-21 (papel, validade e aprovação) e RF-22 (conferência do código da chave pelo dono), além
da [tabela de papéis](#1-papéis-e-permissões).

#### Colaboração

- **RF-24** Remover uma pessoa ou mudar seu papel a qualquer momento.

### Verificação: RF → Épico → US

Lista de conferência: cada RF aparece uma única vez acima. O estado de épicos e histórias fica
nas issues do GitHub ([backlog.md](backlog.md)).

| RF | Release · épico | US | Outras partes |
|---|---|---|---|
| RF-01 | R1 Editor local · [#38](https://github.com/paulossjunior/prelo/issues/38) | US1 [#154](https://github.com/paulossjunior/prelo/issues/154) | — |
| RF-02 | R1 Editor local · [#38](https://github.com/paulossjunior/prelo/issues/38) | US1 [#154](https://github.com/paulossjunior/prelo/issues/154) | — |
| RF-03 | R1 Editor local · [#38](https://github.com/paulossjunior/prelo/issues/38) | US4 [#157](https://github.com/paulossjunior/prelo/issues/157) | — |
| RF-04 | R1 Editor local · [#38](https://github.com/paulossjunior/prelo/issues/38) | US4 [#157](https://github.com/paulossjunior/prelo/issues/157) | R2 e R4 (contagem de pendências) |
| RF-05 | R1 Editor local · [#38](https://github.com/paulossjunior/prelo/issues/38) | US3 [#156](https://github.com/paulossjunior/prelo/issues/156) | — |
| RF-06 | R1 Editor local · [#38](https://github.com/paulossjunior/prelo/issues/38) | US1 [#154](https://github.com/paulossjunior/prelo/issues/154) | também US5 #158 |
| RF-07 | R1 Editor local · [#38](https://github.com/paulossjunior/prelo/issues/38) | US1 [#154](https://github.com/paulossjunior/prelo/issues/154) | também US6 #159 |
| RF-08 | R1 Editor local · [#38](https://github.com/paulossjunior/prelo/issues/38) | US2 [#155](https://github.com/paulossjunior/prelo/issues/155) | — |
| RF-09 | R1 Editor local · [#38](https://github.com/paulossjunior/prelo/issues/38) | US3 [#156](https://github.com/paulossjunior/prelo/issues/156) | — |
| RF-10 | R4 Modo sugestão · épico a criar | sem US ainda | — |
| RF-11 | R4 Modo sugestão · épico a criar | sem US ainda | — |
| RF-12 | R4 Modo sugestão · épico a criar | sem US ainda | — |
| RF-13 | R4 Modo sugestão · épico a criar | sem US ainda | — |
| RF-14 | R4 Modo sugestão · épico a criar | sem US ainda | — |
| RF-15 | R4 Modo sugestão · épico a criar | sem US ainda | — |
| RF-16 | R2 Comentários e histórico · épico a criar | sem US ainda | — |
| RF-17 | R2 Comentários e histórico · épico a criar | sem US ainda | — |
| RF-18 | R4 Modo sugestão · épico a criar | sem US ainda | — |
| RF-19 | R4 Modo sugestão · épico a criar | sem US ainda | — |
| RF-20 | R3 Tempo real sem servidor · épico a criar | sem US ainda | — |
| RF-21 | R3 Tempo real sem servidor · épico a criar | sem US ainda | R5 (papel, validade, aprovação) |
| RF-22 | R3 Tempo real sem servidor · épico a criar | sem US ainda | R5 (conferência pelo dono) |
| RF-23 | R3 Tempo real sem servidor · épico a criar | sem US ainda | — |
| RF-24 | R5 Papéis e convites · épico a criar | sem US ainda | — |
| RF-30 | R3 Tempo real sem servidor · épico a criar | sem US ainda | R4 (sugerir offline) |
| RF-31 | R3 Tempo real sem servidor · épico a criar | sem US ainda | R4 (aceitar e rejeitar) |
| RF-32 | R2 Comentários e histórico · épico a criar | sem US ainda | R3 (pares) |
| RF-33 | R2 Comentários e histórico · épico a criar | sem US ainda | — |
| RF-34 | R2 Comentários e histórico · épico a criar | sem US ainda | — |
| RF-35 | R3 Tempo real sem servidor · épico a criar | sem US ainda | — |
| RF-36 | R2 Comentários e histórico · épico a criar | sem US ainda | — |
| RF-37 | R3 Tempo real sem servidor · épico a criar | sem US ainda | — |
| RF-38 | R2 Comentários e histórico · épico a criar | sem US ainda | R3 (pares) |
| RF-40 | R2 Comentários e histórico · épico a criar | sem US ainda | — |
| RF-41 | R2 Comentários e histórico · épico a criar | sem US ainda | — |
| RF-42 | R1 Editor local · [#38](https://github.com/paulossjunior/prelo/issues/38) | US5 [#158](https://github.com/paulossjunior/prelo/issues/158) | R4 (`.typ` limpo, proposta) |
| RF-43 | R2 Comentários e histórico · épico a criar | sem US ainda | — |
| RF-44 | R4 Modo sugestão · épico a criar | sem US ainda | — |
| RF-45 | R4 Modo sugestão · épico a criar | sem US ainda | — |
| RF-50 | R1 Editor local · [#38](https://github.com/paulossjunior/prelo/issues/38) | US3 [#156](https://github.com/paulossjunior/prelo/issues/156) | — |
| RF-51 | R1 Editor local · [#38](https://github.com/paulossjunior/prelo/issues/38) | US3 [#156](https://github.com/paulossjunior/prelo/issues/156) | — |
| RF-52 | R3 Tempo real sem servidor · épico a criar | sem US ainda | — |
| RF-53 | R3 Tempo real sem servidor · épico a criar | sem US ainda | — |
| RF-54 | R4 Modo sugestão · épico a criar | sem US ainda | — |

## 3. Requisitos não funcionais

- **RNF-01 Plataformas**: macOS, Windows e Linux.
- **RNF-02 Privacidade**: nenhum servidor guarda o conteúdo do artigo. Todo tráfego entre computadores é criptografado de ponta a ponta. Isso atende a textos inéditos e à LGPD.
- **RNF-03 Desempenho**: prévia em menos de 100 ms após uma edição; sincronização de um Salvar em cerca de 1 s em rede comum.
- **RNF-04 Robustez**: fechar o app ou perder a conexão nunca perde trabalho. A fila offline fica no disco.
- **RNF-05 Interoperabilidade**: o `.typ` no repositório é sempre Typst puro, compilável fora do Prelo.
- **RNF-06 Licenças**: só dependências com licenças permissivas (Apache-2.0, MIT). Arquivos de fonte embutidos no app também podem usar OFL-1.1 (D-15); pela emenda proposta de 2026-11-23, também GUST Font License, GPL-3.0+ com Font Exception e Distribution Exception e Bitstream Vera, só para as fontes que o `typst` CLI embute.

## 4. Regras de revisão

Regras observáveis pelo usuário. A representação interna está em
[arquitetura.md §4](arquitetura.md#4-modelo-de-revisão).

| Ação | Inserção sugerida | Exclusão sugerida |
|---|---|---|
| Aceitar | O texto inserido fica | O texto marcado é apagado |
| Rejeitar | O texto inserido é apagado | O texto marcado fica |

- Uma substituição é uma exclusão e uma inserção tratadas como uma única sugestão.
- Uma exclusão sugerida mantém o texto visível, marcado como excluído, até alguém decidir.
- Um comentário não muda o texto, então não é aceito nem rejeitado. Ele é **resolvido** quando o pedido foi atendido (D-03).
- O PDF final e o git recebem só o **texto aceito**: sem inserções pendentes, com o texto das exclusões pendentes.

## 5. Regras de offline e sincronização

1. **Comentar e sugerir funcionam offline.** As ações vão para uma fila no disco.
2. **Editar direto só com os arquivos sincronizados.** Ao abrir, reconectar ou salvar, o app recebe primeiro as mudanças dos outros e mantém o editor em leitura até terminar.
3. **Salvar (⌘S) sincroniza automaticamente**: recebe, envia e grava no git. Sem conexão, guarda no computador.
4. **Fechar faz um último Salvar**, para que quem abrir depois receba o trabalho pelo git mesmo com o computador do autor desligado.

O que cada estado permite (o diagrama de estados está em
[arquitetura.md §5](arquitetura.md#5-sincronização)):

| Estado | Ler | Comentar | Sugerir | Editar direto | Aceitar sugestões |
|---|---|---|---|---|---|
| Verificando / Recebendo | Sim | Espera | Espera | Bloqueado | Bloqueado |
| Sincronizado | Sim | Sim | Sim | Sim | Sim |
| Offline | Sim | Sim, na fila | Sim, na fila | Bloqueado | Bloqueado |

**O que "sincronizado" significa.** O app considera os arquivos sincronizados quando recebeu tudo o que está disponível em **quem está online** e no **git remoto**. Mudanças feitas offline por alguém que ainda não reconectou chegam depois; por isso Salvar e fechar sempre gravam no git.

## 6. Fluxos

Os fluxos abaixo estão desenhados tela a tela no protótipo ([`prototype/index.html`](../prototype/index.html)).

### 6.1 Entrada no projeto

1. **Dono** clica em Compartilhar e escolhe papel, validade e se exige aprovação.
2. **Dono** recebe um código de uso único (com QR) e o envia por canal privado. O código contém a chave do documento.
3. **Convidado** instala o Prelo, escolhe nome e cor; o app cria a chave dele.
4. **Convidado** cola o código ou clica no link `prelo://join/…`.
5. **Convidado**: o app valida o convite e conecta (rede local → internet direta → relay). Se ninguém estiver online, ele espera um aviso ou baixa pelo GitHub.
6. **Dono** vê o pedido de entrada, confere o código da chave e aprova.
7. **Convidado** recebe o texto primeiro, depois figuras e histórico.
8. **Convidado** está no projeto, com cursores e presença visíveis.

### 6.2 Revisão

1. Aceitar ou rejeitar uma sugestão pelo cartão ou pelo trecho marcado.
2. Aceitar em lote filtrando por pessoa e seção.
3. Sugestão que quebra a compilação: aviso e Aceitar desativado.
4. Depois de aceitar: desfazer disponível, histórico registra, commit proposto.
5. Resolver um comentário: responder e clicar em Resolver.
6. Reabrir ou excluir a partir da aba Resolvidos.
7. Excluir uma conversa aberta pelo menu ⋯, com confirmação no cartão.
8. Preparar submissão: listar pendências e excluir resolvidos.

### 6.3 Offline e sincronização

1. Ao abrir: verificação de versões e recebimento das mudanças.
2. Editor em leitura enquanto recebe.
3. Sincronizado: resumo do que mudou e edição liberada.
4. Offline: comentar e sugerir; modo Editando travado.
5. Fila de envio com hora e linha de cada item.
6. Reconectar: receber, enviar a fila, gravar no git, liberar.
7. Trecho que mudou enquanto offline: o app pergunta onde manter o item.
8. Salvar (⌘S): sincroniza na hora.
9. Fechar: último Salvar.

## 7. PDF de diferenças

Revistas costumam pedir, na resubmissão, uma versão com as mudanças marcadas. No LaTeX isso se faz com `latexdiff`; para Typst não há ferramenta equivalente consolidada.

O Prelo compara duas versões nomeadas ("Submissão 1" e "Revisão R1"), aplica a diferença como sugestões temporárias e compila o resultado com marcações. A resposta aos revisores pode citar as mudanças pelo número de linha desse PDF.
