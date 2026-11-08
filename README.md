# Painel do dia · Central da Educação

Programa de computador (Windows e Mac) da **Central da Educação**, da Secretaria Municipal de Educação de Vila Rica (MT).
Mostra o Painel do dia (saídas, pedidos para aprovar, comunicados e agenda) como um **widget na área de trabalho** (ou,
se preferir, numa janela **sempre por cima das outras**), aberto **quando o computador liga**, com **avisos**
(notificação e som) e **atualização sozinha**.

O conteúdo vem do sistema publicado (https://central-da-educacao.vercel.app) e se atualiza na hora. Este repositório
tem só o programa (a "casca") e os instaladores; o código do sistema é privado.

## Instalar (passo a passo)

**Baixar:** abra https://github.com/rafaelpmtrindade/central-da-educacao-painel/releases/latest e baixe:

- **Windows**: o arquivo que termina em **`_x64-setup.exe`**;
- **Mac**: o arquivo que termina em **`_universal.dmg`**.

### Windows

1. Dê dois cliques no arquivo baixado.
2. Se aparecer **"O Windows protegeu o computador"**: clique em **Mais informações** e depois em **Executar assim
   mesmo**. (Aparece porque o programa não tem a assinatura paga da Microsoft; é só na instalação.)
3. Siga o instalador (**Avançar** → **Instalar** → **Concluir**). Não pede senha de administrador: fica instalado para
   o usuário do Windows que está usando o computador.
4. O Painel do dia abre sozinho no canto de cima, à direita, como widget na área de trabalho.
5. **Entre com o e-mail e a senha da Central** da pessoa que usa o computador. O login fica guardado.
6. Pronto: a partir de agora ele abre sozinho quando o computador liga.

### Mac

1. Abra o arquivo `.dmg` e arraste o **Painel do dia** para **Aplicativos**.
2. Abra o Painel do dia em Aplicativos. Na primeira vez o Mac avisa que não pode verificar o programa: abra **Ajustes
   do Sistema → Privacidade e Segurança**, role até o fim e clique em **Abrir Mesmo Assim**.
3. Entre com o e-mail e a senha da Central. Se o Mac perguntar se o Painel do dia pode mostrar notificações, clique em
   **Permitir**.

### Depois de instalado

- O ícone do brasão fica perto do relógio (Windows; às vezes dentro da setinha **^**) ou na barra de cima (Mac).
  Clique nele para mostrar o painel; o menu traz as opções abaixo.
- Se a pessoa preferir o painel sempre à vista por cima dos outros programas: menu do ícone → **Mostrar por cima das
  janelas**. Para um painel menor ou maior: **Tamanho**.
- Os avisos que a pessoa recebe (pedido para decidir, resposta, saída cancelada, comunicado importante, lembrete da
  saída) se escolhem na Central, em **Meu perfil → Avisos**.

## Usar

- **Ícone perto do relógio** (Windows) ou na barra de cima (Mac): clique para mostrar o painel. O menu tem:
  - **Mostrar como widget na área de trabalho** (vem marcado) ou **Mostrar por cima das janelas**;
  - **Tamanho**: Pequeno, Médio (vem marcado) ou Grande;
  - **Aparência**: Igual ao sistema (vem marcado), Clara ou Escura;
  - **Abrir quando o computador ligar**, **Abrir a Central no navegador**, **Procurar atualização** e **Sair**.
- Widget: preso ao fundo da tela, sem bordas, com cantos arredondados e fora da barra de tarefas; aparece quando a área
  de trabalho está à vista (ou num segundo monitor). Para mudar de lugar, arraste pela faixa de cima (logo e hora); o
  lugar fica guardado. A altura acompanha o conteúdo; o topo e o botão **Abrir a Central** ficam sempre à vista.
- Por cima das janelas, dá para puxar a borda e escolher a altura: ela fica guardada (também ao reiniciar) e o painel
  rola por dentro. Para voltar à altura automática, escolha um tamanho no menu.
- **Que versão está instalada?** Menu do ícone (ou ⋯): a linha **Versão** diz o número e se é a mais nova
  ("a mais nova", "atualizando" ou "não deu para conferir", quando falta internet).
- O mesmo menu do ícone abre pelo botão **⋯** no canto do painel (útil no MacBook, quando o ícone some atrás do
  entalhe da câmera).
- Por cima das janelas: fechar a janela só esconde o painel; ele continua perto do relógio.
- Pedidos para decidir: **Aprovar** e **Recusar** no próprio painel.
- Avisos (pedido para decidir, resposta, saída cancelada, comunicado importante, lembrete da saída): escolha quais
  em **Meu perfil → Avisos**, na Central.

## Atualizar

Sozinho, sem precisar clicar: o programa procura versão nova ao abrir e a cada hora; quando acha, baixa, instala e
abre de novo em poucos segundos, e avisa com uma notificação. Para forçar na hora: menu → **Procurar atualização**.
Cada versão é assinada; o programa recusa qualquer atualização que não seja nossa.

## Publicar uma versão nova (VRDOIS)

1. Mude a versão em `package.json`, `src-tauri/Cargo.toml` e `src-tauri/tauri.conf.json`.
2. `git tag vX.Y.Z && git push --tags`. O GitHub monta os instaladores de Windows e Mac e publica a versão com o
   arquivo de atualização (`.github/workflows/publicar.yml`).

Para montar no Mac: `npm install` e `npm run montar` (precisa do Rust: https://rustup.rs).

A chave de assinatura das atualizações fica em `~/.tauri/central-da-educacao-painel.key` no Mac do desenvolvedor (a
senha no Chaveiro: `central-da-educacao-painel:TAURI_SIGNING_PRIVATE_KEY_PASSWORD`) e nos segredos deste repositório.
**Sem ela, os programas instalados não aceitam mais atualização**: guarde uma cópia no Cofre VRDOIS.
