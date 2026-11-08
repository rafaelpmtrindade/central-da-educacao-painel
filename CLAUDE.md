# Painel do dia (programa desktop da Central da Educação)

Completa o padrão geral (`~/.claude/CLAUDE.md`). O sistema em si é `rafaelpmtrindade/central-da-educacao` (privado);
este repositório (público, para os instaladores e as atualizações serem baixados sem login) tem só a casca Tauri 2.

- **O que faz**: janela de 400 px que abre `https://central-da-educacao.vercel.app/painel?compacto&app=1`, sempre por
  cima, abre ao ligar (autostart), ícone na bandeja com menu, fechar = esconder, uma instância só, lembra tamanho e
  lugar, procura atualização ao abrir e a cada 6 h (pergunta antes). Tela de abertura local (`abertura/`) confere a
  internet antes de abrir o painel.
- **Segurança**: a página remota só tem a permissão de notificação (`src-tauri/capabilities/remoto.json`). Navegação:
  painel e telas de entrada ficam no programa; o resto da Central e outros sites abrem no navegador pelo próprio
  programa (`decidir_navegacao` em `src-tauri/src/lib.rs`). A página pede "abrir no navegador" com `?no-navegador`.
- **Atualização**: plugin updater, chave em `~/.tauri/central-da-educacao-painel.key` (senha no Chaveiro
  `central-da-educacao-painel:TAURI_SIGNING_PRIVATE_KEY_PASSWORD`), segredos `TAURI_SIGNING_PRIVATE_KEY` e
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` no GitHub. Endpoint: `releases/latest/download/latest.json`.
- **Publicar**: subir a versão nos 3 arquivos e `git tag vX.Y.Z && git push --tags` (Actions monta Windows e Mac).
- **Comandos**: `npm install`, `npm run montar` (Mac local), `cargo check` em `src-tauri/`.
- **Sem assinatura paga**: Windows mostra o aviso do SmartScreen na instalação; Mac pede "Abrir Mesmo Assim" uma vez.
