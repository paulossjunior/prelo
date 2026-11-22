# D-18 — Autenticação no GitHub por OAuth

- **Status:** aceita · novembro de 2026 · decisão do dono em 2026-11-22 ("vamos usar o OAuth"); a forma concreta abaixo tem pontos que ainda dependem do dono (ver Pendências)
- **Contexto:** D-05 usa o remoto git do grupo como canal para quem não está online junto. Para fazer pull e push pelo app (RF-32, RF-33, RF-40), o núcleo precisa de credenciais no GitHub. A release R2 (`1.1.0`) é a primeira a sincronizar pelo git. As opções eram: chave SSH da própria pessoa, token pessoal de acesso (PAT) guardado no Keychain, ou OAuth. O app não deve carregar segredo no binário, não deve expor credencial à webview (a interface não tem capabilities `http` nem `shell`, conforme [contracts/ipc.md da 003](../../specs/003-editor-local/contracts/ipc.md)) e deve tratar artigos inéditos com o menor acesso possível (Princípio II, RNF-02).
- **Decisão:** autenticar no GitHub por OAuth, com esta forma:
  1. **GitHub App, não OAuth App clássico.** O Prelo usa tokens *user-to-server* de um GitHub App. Permissões por repositório: só `Contents: read & write` e `Metadata: read`. A pessoa instala o app apenas nos repositórios que escolher. Os tokens de acesso expiram em 8 h e são renovados por *refresh token*.
  2. **Login por OAuth Device Flow (RFC 8628).** O app mostra um código e abre `https://github.com/login/device` no navegador; a pessoa digita o código e autoriza. Não há *client secret* no binário nem servidor HTTP local de *callback*. Se a experiência do device flow for recusada, a alternativa é Authorization Code com PKCE e redirecionamento *loopback* (ver Alternativas).
  3. **Tokens só no cofre de credenciais do sistema.** No macOS, o Keychain; nos outros sistemas, o cofre equivalente, por uma crate de licença permissiva como `keyring` (MIT/Apache-2.0, dentro da política do `deny.toml`). O token nunca vai para a pasta do projeto, para `.prelo/`, para logs nem para a webview.
  4. **Só o núcleo Rust fala com a rede.** Login, renovação de token e git (pull/push) acontecem no núcleo. A webview recebe apenas o estado (por exemplo, "conectado como `@login`" ou "login necessário"), nunca o token.
  5. **Git por HTTPS com o token.** O gitoxide autentica no remoto com usuário `x-access-token` e o token de acesso como senha.
  6. **GitLab fica fora desta decisão.** O GitLab também oferece device flow; a autenticação nele será uma decisão futura.
- **Alternativas consideradas:**
  - Chave SSH da pessoa: depende de configuração prévia (gerar chave, cadastrar no GitHub, `ssh-agent`); experiência ruim para quem não é desenvolvedor; a chave costuma dar acesso a todos os repositórios da conta.
  - PAT guardado no Keychain: criação manual no site do GitHub, token de vida longa e, no PAT clássico, escopos amplos; a pessoa precisa saber quais permissões marcar.
  - OAuth App clássico: exige o escopo `repo`, que dá leitura e escrita em todos os repositórios privados da conta, e os tokens não expiram.
  - Authorization Code com PKCE: fluxo padrão de app nativo, mas precisa de servidor HTTP *loopback* no núcleo ou de esquema de URL próprio registrado no sistema, o que aumenta a superfície de ataque. Fica como plano B se o device flow for recusado.
- **Consequências:**
  - (+) Nenhuma senha nem segredo do Prelo dentro do app; o login acontece no site do GitHub.
  - (+) Menor privilégio: só conteúdo e metadados, só nos repositórios escolhidos pela pessoa.
  - (+) A pessoa revoga o acesso nas configurações do GitHub a qualquer momento; tokens de acesso expiram em 8 h.
  - (+) A webview não vê o token, mantendo a regra de IPC sem `http` nem `shell`.
  - (−) O dono do projeto precisa registrar e manter um GitHub App (nome, *client ID*, permissões, device flow habilitado). É uma ação fora do código (ver Pendências).
  - (−) O login exige rede até `github.com`; offline, o app continua guardando local e agendando a sincronização (RF-34), mas não faz o primeiro login.
  - (−) O device flow faz *polling* até a pessoa autorizar e tem limites de taxa do GitHub; o núcleo precisa respeitar o intervalo e os erros de "aguarde" devolvidos pelo GitHub.
  - (−) O núcleo precisa renovar o token expirado e tratar *refresh token* inválido ou revogado pedindo novo login, sem perder a fila de Salvar (Princípio III).
  - (−) Novos testes de segurança (Princípio VI): token ausente de logs, de `.prelo/` e das respostas de IPC; recusa de token expirado ou revogado.
  - (−) O GitHub passa a guardar o repositório do artigo, o que afeta a redação de RNF-02 ("nenhum servidor guarda o conteúdo do artigo"). Isso já é uma consequência de D-05; a questão Q-05 do plano de releases continua aberta.

## Pendências para o dono

1. Registrar o GitHub App: decidir se ele pertence a uma conta pessoal ou a uma organização. TODO(github-app-dono): quem é o dono do GitHub App do Prelo?
2. Confirmar o conjunto de permissões: só `Contents: read & write` e `Metadata: read`. TODO(github-app-permissoes): a R2 precisa de alguma outra permissão?
3. Confirmar device flow como fluxo de login, ou trocar por Authorization Code com PKCE. TODO(github-login-fluxo): device flow ou PKCE?

---

Índice de decisões: [adrs.md](../adrs.md)
