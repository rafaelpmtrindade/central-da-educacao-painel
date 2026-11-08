# Painel do dia · Central da Educação

Programa de computador (Windows e Mac) da **Central da Educação**, da Secretaria Municipal de Educação de Vila Rica (MT).
Mostra o Painel do dia (saídas, pedidos, comunicados e agenda) numa janela pequena, **sempre por cima das outras
janelas**, aberta **quando o computador liga**, com **avisos** (notificação e som) e **atualização sozinha**.

O conteúdo vem do sistema publicado (https://central-da-educacao.vercel.app) e se atualiza na hora. Este repositório
tem só o programa (a "casca") e os instaladores; o código do sistema é privado.

## Instalar

Baixe o instalador da versão mais nova em **Releases**:

- **Windows**: `Painel.do.dia_x.y.z_x64-setup.exe`. Na primeira vez o Windows pode avisar "O Windows protegeu o
  computador": clique em **Mais informações** → **Executar assim mesmo**. Não precisa de senha de administrador.
- **Mac**: `Painel.do.dia_x.y.z_universal.dmg`. Arraste para Aplicativos. Na primeira vez o Mac avisa que não pode
  verificar o programa: em **Ajustes do Sistema → Privacidade e Segurança**, clique em **Abrir Mesmo Assim**.

Depois de instalado, ele abre sozinho a cada vez que o computador liga. Entre com o seu e-mail e senha da Central uma
vez; o login fica guardado.

## Usar

- **Ícone perto do relógio** (Windows) ou na barra de cima (Mac): clique para mostrar o painel; o menu tem "Sempre por
  cima das outras janelas", "Abrir quando o computador ligar", "Abrir a Central no navegador", "Procurar atualização"
  e "Sair".
- Fechar a janela só esconde o painel: ele continua perto do relógio.
- Avisos (pedido para decidir, resposta, saída cancelada, comunicado importante, lembrete da saída): escolha quais
  em **Meu perfil → Avisos**, na Central.

## Atualizar

Sozinho: o programa procura versão nova ao abrir e a cada 6 horas e pergunta antes de atualizar. Cada versão é
assinada; o programa recusa qualquer atualização que não seja nossa.

## Publicar uma versão nova (VRDOIS)

1. Mude a versão em `package.json`, `src-tauri/Cargo.toml` e `src-tauri/tauri.conf.json`.
2. `git tag vX.Y.Z && git push --tags`. O GitHub monta os instaladores de Windows e Mac e publica a versão com o
   arquivo de atualização (`.github/workflows/publicar.yml`).

Para montar no Mac: `npm install` e `npm run montar` (precisa do Rust: https://rustup.rs).

A chave de assinatura das atualizações fica em `~/.tauri/central-da-educacao-painel.key` no Mac do desenvolvedor (a
senha no Chaveiro: `central-da-educacao-painel:TAURI_SIGNING_PRIVATE_KEY_PASSWORD`) e nos segredos deste repositório.
**Sem ela, os programas instalados não aceitam mais atualização**: guarde uma cópia no Cofre VRDOIS.
