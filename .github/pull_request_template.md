## Resumo para humanos

<!-- 2 a 4 frases, sem jargão (ou com uma explicação curta entre parênteses):
o que muda para quem usa ou mantém o Prelo, e por quê. -->

## Rastreabilidade

<!-- Links completos para as issues, com o título. Sem história: "Fundação — habilita US1…USn".
Sem épico: "Processo — sem épico" e de onde veio o pedido. -->

- **Épico:** [#N](https://github.com/paulossjunior/prelo/issues/N) título
- **História:** [#N](https://github.com/paulossjunior/prelo/issues/N) USn título
- **Tasks:**
  - [#N](https://github.com/paulossjunior/prelo/issues/N) T0xx título
- **Spec:** [`specs/NNN-nome/tasks.md`](https://github.com/paulossjunior/prelo/blob/dev/specs/NNN-nome/tasks.md)

## O que mudou

<!-- Técnico. Um item por arquivo ou área. -->

- `caminho/arquivo`: ...

## Como foi testado

<!-- Vermelho → verde (commits ou contagens), modo e resultado do verify, checagens manuais. -->

- Vermelho: ...
- Verde: ...
- `npm run verify -- --quick`: ...

## Decisões para revisão

<!-- Numeradas. Cada uma: a opção escolhida e por quê. "Nenhuma" se não houver. -->

1. ...

## Fora deste PR / próximos passos

- ...

<!-- Base = main (branch padrão): "Closes #N", uma por issue.
Outra base (dev, feature/*): a linha abaixo, com todas as issues. -->
Refs #N (fechar à mão após o merge: a base não é a branch padrão)
