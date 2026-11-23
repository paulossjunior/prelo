# D-12 — Transporte de arquivos binários entre coautores

- **Status:** aceita · 2026-11-18
- **Contexto:** o documento Yjs cobre texto, sugestões e comentários (D-02). Imagens, PDFs usados como figura, `.csl` e outros arquivos não textuais não têm transporte definido entre pares nem fora do git. O fluxo de entrada já supõe que "o texto chega primeiro, depois figuras e histórico" ([requisitos.md §6.1](../requisitos.md#61-entrada-no-projeto), passo 7), e o protótipo mostra "Figuras · 118 de 131 arquivos" vindo direto do computador do dono, criptografado. A prévia precisa saber se um arquivo ausente é pendente ou inexistente (RF-52), o que exige uma lista de arquivos do projeto conhecida antes dos bytes.
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

---

Índice de decisões: [adrs.md](../adrs.md)
