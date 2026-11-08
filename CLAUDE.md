# Painel do dia (programa desktop da Central da Educação)

Completa o padrão geral (`~/.claude/CLAUDE.md`). O sistema em si é `rafaelpmtrindade/central-da-educacao` (privado);
este repositório (público, para os instaladores e as atualizações serem baixados sem login) tem só a casca Tauri 2.

- **O que faz**: abre `https://central-da-educacao.vercel.app/painel?app=1&tamanho=<pequeno|medio|grande>`. Dois
  jeitos (menu do ícone; padrão widget): **widget** (sem bordas, fora da barra de tarefas, always-on-bottom, todas as
  áreas de trabalho, arrastar pela faixa de cima) e **por cima** (bordas, always-on-top). Três tamanhos (padrão médio):
  pequeno 340 px, médio 420 px, grande 480 px (alturas conforme a tela). Abre ao ligar (autostart), uma instância só,
  lembra lugar, fechar = esconder, no Mac sem ícone no Dock; procura atualização ao abrir e a cada 1 h e atualiza sozinho (baixa, instala, reabre e avisa com notificação). Tela de abertura local (`abertura/`) confere a
  internet antes de abrir o painel.
- **Identificação**: na primeira abertura cria `preferencias.aparelho` (uuid v4 aleatório, 32 letras e números) e
  manda `&versao=…&sistema=windows|mac&aparelho=…` na URL da abertura e do painel; a Central registra em
  `programas_painel` (Administração → Implantação). O menu mostra "Versão X · a mais nova / atualizando / não deu
  para conferir / não deu para atualizar".
- **Segurança**: a página remota só tem notificação, arrastar a janela e dois comandos nossos: `ajustar_altura`
  (altura da janela segue o conteúdo, dentro do limite do tamanho; se a pessoa puxou a borda, a altura dela fica em
  `preferencias.altura` e o pedido é ignorado até ela escolher um tamanho no menu; mudança de tamanho feita pelo
  programa é separada da feita pela pessoa por `UltimoAjuste`, 2 s) e `abrir_menu` (botão ⋯ abre o menu do ícone)
  (`src-tauri/capabilities/remoto.json`, manifesto em `build.rs`). Janela transparente (macOSPrivateApi): o painel
  desenha o cartão arredondado. Aparência (sistema/clara/escura) aplicada à página a cada carregamento. Navegação:
  painel e telas de entrada ficam no programa; o resto da Central e outros sites abrem no navegador pelo próprio
  programa (`decidir_navegacao` em `src-tauri/src/lib.rs`). A página pede "abrir no navegador" com `?no-navegador`.
- **Atualização**: plugin updater, chave em `~/.tauri/central-da-educacao-painel.key` (senha no Chaveiro
  `central-da-educacao-painel:TAURI_SIGNING_PRIVATE_KEY_PASSWORD`), segredos `TAURI_SIGNING_PRIVATE_KEY` e
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` no GitHub. Endpoint: `releases/latest/download/latest.json`.
- **Publicar**: subir a versão nos 3 arquivos e `git tag vX.Y.Z && git push --tags` (Actions monta Windows e Mac).
- **Comandos**: `npm install`, `npm run montar` (Mac local), `cargo check` em `src-tauri/`.
- **Sem assinatura paga**: Windows mostra o aviso do SmartScreen na instalação; Mac pede "Abrir Mesmo Assim" uma vez.
