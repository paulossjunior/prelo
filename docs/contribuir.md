# Commits e pull requests

Cada commit e cada PR do Prelo diz **por que** a mudança existe, em linguagem que qualquer
pessoa do projeto entenda, e aponta para as issues de onde ela veio (Épico → História → Task,
ver [backlog.md](backlog.md)).

## Commit

```text
<tipo>(<escopo>): <resumo em inglês, até 72 caracteres>

<corpo em pt-BR: 1 a 3 parágrafos curtos explicando por quê, para um humano>

Task: T0xx (#N)
Story: USn (#N)
Epic: #N
Co-Authored-By: <agente> <e-mail>
```

- **Assunto:** em inglês, no imperativo, até 72 caracteres. `tipo`: `feat`, `fix`, `test`,
  `docs`, `refactor`, `chore`, `ci`, `build`, `deps`. `escopo`: a área (`fs`, `compile`, `ui`,
  `release`, `gestor-projeto`...) ou a feature (`003`); pode faltar.
- **Corpo:** em pt-BR. O que muda para quem usa ou mantém o Prelo e por que foi feito assim.
  O *como* fica no diff; cite só o que não é óbvio nele (decisão, risco, alternativa
  descartada).
- **Trailers** (no fim, um por linha, sem linha em branco entre eles):
  - `Task: T0xx (#N)`: a tarefa do `tasks.md` e a issue dela. Uma linha por tarefa.
  - `Story: USn (#N)`: a história; omita se a tarefa não tem (Setup, Foundational, Polish).
  - `Epic: #N`: o épico.
  - `Co-Authored-By:` obrigatório quando um agente faz o commit.
  - Commit sem tarefa (processo, dependência): omita `Task`/`Story`/`Epic` e diga a origem no
    corpo.

Exemplo:

```text
feat(fs): refuse project files outside the root

Um documento Typst pode pedir qualquer arquivo do disco, por exemplo
#read("../../.ssh/id_rsa"). Agora o núcleo só entrega arquivos de dentro
da pasta do projeto; o resto é recusado com uma mensagem que cita o
caminho pedido, sem nunca abrir o arquivo de fora.

Links simbólicos e pastas trocadas no meio da leitura também são
recusados. Resta um risco pequeno, descrito no PR: um arquivo de fora
pode ser aberto (nunca lido) se a troca acontecer no instante exato.

Task: T017 (#83)
Epic: #38
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

## Pull request

O GitHub preenche o corpo com o [modelo](../.github/pull_request_template.md). Preencha todas as
seções:

1. **Resumo para humanos:** 2 a 4 frases sem jargão.
2. **Rastreabilidade:** Épico, História e Tasks com link e título; sem história, "Fundação —
   habilita US1…USn"; sem épico, "Processo — sem épico" e a origem do pedido.
3. **O que mudou**, **Como foi testado** (vermelho → verde, verify), **Decisões para revisão**,
   **Fora deste PR / próximos passos**.
4. **Fechamento:** com base na `main` (branch padrão), `Closes #N`. Com outra base, o GitHub
   não fecha a issue sozinho: `Refs #N (fechar à mão após o merge: a base não é a branch
   padrão)`.

Para achar os links: `gh issue view N` (o corpo da Task traz `Parte de #M`) e
`gh api repos/paulossjunior/prelo/issues/M/sub_issues`.
