# Prelo Constitution

## Core Principles

### I. Typst Puro no Repositório

- O `.typ` versionado no git MUST ser sempre Typst puro, compilável fora do Prelo.
- Sugestões, comentários, participantes e versões MUST ficar em `.prelo/`, nunca como
  marcação dentro do `.typ`.
- O texto gravado no git MUST ser o texto aceito: inserções pendentes ignoradas, exclusões
  pendentes mantidas.

Motivo: quem não usa o Prelo continua vendo um repositório Typst normal (RNF-05, D-02).

### II. Privacidade sem Servidor Obrigatório

- Nenhum servidor MUST guardar o conteúdo do artigo.
- Todo tráfego entre computadores MUST ser criptografado de ponta a ponta, inclusive via relay.
- Recursos MUST funcionar sem servidor próprio do Prelo: rede local (mDNS), P2P (iroh) e git
  do grupo cobrem a colaboração. Relay serve só para conectar.
- Identidade MUST ser chave criptográfica local, sem conta nem senha.

Motivo: textos inéditos, LGPD e custo zero de infraestrutura (RNF-02, D-04, D-05).

### III. Nenhum Trabalho Perdido

- Fechar o app, travar ou perder a conexão MUST NOT perder trabalho.
- Ações offline (comentar, responder, resolver, sugerir) MUST ir para fila persistida no disco
  antes de serem confirmadas ao usuário.
- Fechar o app MUST executar um último Salvar.
- Comentário ou sugestão sobre trecho que mudou MUST NOT ser descartado em silêncio; o usuário
  decide se mantém no trecho novo ou exclui.

Motivo: confiança na ferramenta depende disso (RNF-04, RF-30, RF-36, RF-37).

### IV. Edição Só sobre Versão Sincronizada

- Editar direto, aceitar e rejeitar sugestões MUST ficar bloqueados até o app receber tudo o
  que está disponível nos pares online e no git remoto.
- Offline, só comentar e sugerir MUST ser permitido.
- A ordem de sincronização MUST ser sempre: receber, enviar a fila, gravar no git, liberar.
- Salvar (⌘S) MUST sincronizar: receber, enviar e gravar commit; sem conexão, guardar local e
  agendar.

Motivo: evita editar sobre versão antiga e sobrescrever trabalho alheio (D-06, D-07, D-08, D-09).

### V. Revisão Explícita e Reversível

- Toda mudança de texto vinda de revisor ou do modo Sugerindo MUST ser sugestão com autor,
  hora e id, até alguém aceitar ou rejeitar.
- Sugestões MUST ser atributos do CRDT; comentários MUST ancorar por posição relativa do Yjs.
- Aceitar e rejeitar MUST poder ser desfeitos.
- Sugestão que quebra a compilação MUST ter Aceitar desativado.
- Comentários são resolvidos, não aceitos; excluir conversa MUST pedir confirmação.
- Permissões MUST seguir a tabela de papéis (dono, coautor, revisor, leitor) de docs/requisitos.md §1.

Motivo: modelo de revisão no estilo do Overleaf, sem estado ambíguo (RF-10 a RF-17, D-03).

### VI. Testes Primeiro e Segurança Testada (NÃO NEGOCIÁVEL)

- Todo código MUST ser escrito em TDD: teste escrito → teste falha (vermelho) → implementação
  mínima passa (verde) → refatoração com testes verdes. Código de produção sem teste que o
  exercite MUST NOT ser aceito.
- Bug corrigido MUST vir com teste que reproduz o bug e falhava antes da correção.
- Toda feature MUST ter testes de segurança junto com os funcionais, cobrindo no que se aplicar:
  - criptografia de ponta a ponta do tráfego entre pares e via relay (Princípio II);
  - identidade por chave: entrada sem chave aprovada, chave trocada ou código de convite
    reutilizado, expirado ou adulterado MUST ser recusado;
  - permissões por papel: cada ação proibida na tabela de papéis MUST ter teste de recusa;
  - entrada não confiável: atualizações CRDT de pares, links `prelo://`, arquivos `.prelo/`,
    `.typ`, `.bib` e pacotes MUST ser validados, com testes de entrada malformada;
  - dependências: auditoria de vulnerabilidades conhecidas (ex.: `cargo audit`, `npm audit`)
    MUST passar sem achados de severidade alta ou crítica não justificados.
- `tasks.md` MUST listar as tarefas de teste (funcionais e de segurança) antes das tarefas de
  implementação que elas cobrem.

Motivo: textos inéditos e conexão direta entre computadores tornam falha de segurança e
regressão caras; TDD mantém o comportamento especificado verificável desde o início.

## Restrições Técnicas

- Stack: Tauri (shell), Rust (núcleo), CodeMirror 6, Yjs/yrs, typst + Tinymist, iroh + mDNS,
  gitoxide. Trocar componente MUST registrar nova decisão D-xx em docs/adrs.md.
- Compilação embutida: o Prelo MUST compilar Typst com o compilador embutido no núcleo, sem
  depender de `typst` instalado. Prévia, diagnósticos e PDFs MUST vir da mesma compilação, na
  versão registrada em `.prelo/project.json` (RF-06, RF-07, D-10). O `.typ` continua
  compilável fora do Prelo (Princípio I).
- Plataformas: macOS, Windows e Linux MUST ser suportados (RNF-01).
- Desempenho: prévia MUST atualizar em menos de 100 ms após edição em artigos de até 40
  páginas; Salvar SHOULD sincronizar em cerca de 1 s em rede comum (RF-02, RNF-03).
- Licenças: dependências MUST ter licença permissiva (Apache-2.0, MIT ou compatível) (RNF-06).
- Escopo das releases R1–R5 (1.x): LaTeX, web, mobile e submissão a revistas ficam fora
  (docs/especificacao.md §3; versões em docs/release-plan.md).

## Fluxo de Desenvolvimento

- `docs/requisitos.md` é a fonte dos requisitos (RF/RNF) e `docs/adrs.md` das decisões
  (D-xx). Cada feature
  MUST citar os requisitos que atende.
- Construção MUST seguir a ordem do plano (docs/especificacao.md §4): editor local → comentários e git →
  tempo real → modo sugestão → papéis e convites. Pular fase exige justificativa no plan.md.
- Sincronização, merge CRDT, fila offline e geração do texto aceito MUST ter testes
  automatizados de integração, incluindo cenários de dois pares e de reconexão (Princípio VI).
- Gitflow: `feature/NNN-nome` sai da `dev` e volta a ela por pull request; a `dev` vai para a
  `main` por pull request quando houver versão a publicar; a tag `vX.Y.Z` na `main` dispara a
  release. `main` e `dev` MUST receber mudanças só por pull request, inclusive do dono.
- Push MUST acontecer só com a verificação verde, no modo que o destino exige:
  - para a `main` ou de tag: a suíte completa (`npm run verify`), com testes unitários, de
    integração, de segurança, ponta a ponta e auditoria de dependências;
  - de `feature/*` e `dev`: o modo rápido (`npm run verify -- --quick`), que é a suíte completa
    menos o build e os testes ponta a ponta.
  Teste falhando, pulado sem justificativa escrita ou desativado bloqueia o push nos dois modos.
  Vale para pessoas e agentes; `--no-verify` e equivalentes MUST NOT ser usados para contornar.
- CI: pull request e push para a `dev` MUST passar o verify no macOS; pull request e push para a
  `main` e a release MUST passar o verify em macOS, Windows e Linux. Assim o suporte aos três
  sistemas (RNF-01) é garantido antes de qualquer versão chegar à `main`.
- Toda PR MUST verificar conformidade com os princípios I–VI; violação MUST ser justificada na
  seção de complexidade do plan.md.

## Governance

- Esta constituição prevalece sobre outras práticas do projeto.
- Emendas MUST ser feitas via `/speckit-constitution`, com Sync Impact Report e atualização de
  docs/adrs.md quando mudarem decisões D-xx.
- Versão segue SemVer: MAJOR remove ou redefine princípio; MINOR adiciona princípio ou seção;
  PATCH esclarece redação.
- Revisões de plan.md e PRs MUST checar os princípios I–VI no Constitution Check.
- Licença: o Prelo é distribuído sob `MIT OR Apache-2.0` (`LICENSE-MIT` e `LICENSE-APACHE`
  na raiz; decisão do dono em 2026-11-19). Contribuições entram sob a mesma licença
  (inbound = outbound), sem CLA nem DCO. `package.json` e `src-tauri/Cargo.toml` MUST declarar
  o mesmo identificador SPDX.

**Version**: 1.3.1 | **Ratified**: 2026-11-18 | **Last Amended**: 2026-11-21
