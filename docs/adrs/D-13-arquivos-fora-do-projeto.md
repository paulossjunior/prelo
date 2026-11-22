# D-13 — Arquivos fora do projeto

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

---

Índice de decisões: [adrs.md](../adrs.md)
