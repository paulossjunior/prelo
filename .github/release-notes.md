## Prelo {{VERSION}}

Instaladores do Prelo {{VERSION}}:

| Sistema | Arquivo |
|---|---|
| macOS (Apple Silicon e Intel) | `Prelo_{{VERSION}}_universal.dmg` |
| Windows (x86_64) | `Prelo_{{VERSION}}_x64-setup.exe` |
| Linux (x86_64) | `Prelo_{{VERSION}}_amd64.AppImage` ou `Prelo_{{VERSION}}_amd64.deb` |

### Instaladores sem assinatura

Os instaladores ainda não são assinados nem notarizados. O sistema vai avisar que não
reconhece o desenvolvedor. Antes de abrir, confira o checksum (abaixo).

**macOS (Gatekeeper)**: abra o `.dmg` e arraste o Prelo para Aplicativos. Na primeira vez,
clique no app com o botão direito (ou Control + clique) e escolha **Abrir**. Se o macOS não
oferecer essa opção, tente abrir o app uma vez, vá em **Ajustes do Sistema > Privacidade e
Segurança** e clique em **Abrir Mesmo Assim**. Se o macOS disser que o app está danificado,
rode no Terminal `xattr -dr com.apple.quarantine /Applications/Prelo.app` e abra de novo.

**Windows (SmartScreen)**: ao rodar o `.exe`, se aparecer "O Windows protegeu o computador",
clique em **Mais informações** e depois em **Executar assim mesmo**.

**Linux**: para o `.AppImage`, `chmod +x Prelo_{{VERSION}}_amd64.AppImage` e execute o
arquivo. Para o `.deb`, `sudo apt install ./Prelo_{{VERSION}}_amd64.deb`.

### Conferir o checksum

O arquivo `SHA256SUMS.txt` tem o SHA-256 de cada instalador. Baixe-o para a mesma pasta do
instalador e rode:

- Linux: `sha256sum -c SHA256SUMS.txt --ignore-missing`
- macOS: `shasum -a 256 -c SHA256SUMS.txt --ignore-missing`
- Windows (PowerShell): `Get-FileHash .\Prelo_{{VERSION}}_x64-setup.exe -Algorithm SHA256` e
  compare o valor com a linha do `.exe` em `SHA256SUMS.txt` (maiúsculas e minúsculas não
  importam).

Se o valor não coincidir, não instale: o arquivo foi alterado ou corrompido.
