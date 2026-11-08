//! Programa Painel do dia da Central da Educação (Secretaria Municipal de Educação de Vila Rica, MT).
//!
//! Uma janela pequena que mostra o Painel do dia publicado (central-da-educacao.vercel.app/painel), sempre por cima
//! das outras janelas, aberta quando o computador liga, com ícone perto do relógio e atualização sozinha. O conteúdo
//! vem do site e se atualiza na hora; o programa em si só muda quando sai uma versão nova (assinada por nós).
//!
//! Segurança: a página só pode pedir ao programa para mostrar notificação (capabilities/remoto.json). Links para o
//! resto da Central e para outros sites são abertos no navegador pelo próprio programa, nunca pela página.

use std::{fs, path::PathBuf, thread, time::Duration};

use serde::{Deserialize, Serialize};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, LogicalPosition, LogicalSize, Manager, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;

const SITE: &str = "central-da-educacao.vercel.app";
const PAINEL: &str = "https://central-da-educacao.vercel.app/painel?compacto&app=1";
const CENTRAL: &str = "https://central-da-educacao.vercel.app/hoje";
const JANELA: &str = "painel";
/// De quanto em quanto tempo o programa procura versão nova
const INTERVALO_ATUALIZACAO: Duration = Duration::from_secs(6 * 60 * 60);

/// O que o programa lembra entre uma abertura e outra (o tamanho e o lugar da janela ficam com o window-state)
#[derive(Serialize, Deserialize, Default)]
struct Preferencias {
    ja_abriu: bool,
    por_cima: Option<bool>,
}

fn arquivo_preferencias(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|pasta| pasta.join("preferencias.json"))
}

fn ler_preferencias(app: &AppHandle) -> Preferencias {
    arquivo_preferencias(app)
        .and_then(|caminho| fs::read_to_string(caminho).ok())
        .and_then(|texto| serde_json::from_str(&texto).ok())
        .unwrap_or_default()
}

fn salvar_preferencias(app: &AppHandle, preferencias: &Preferencias) {
    if let Some(caminho) = arquivo_preferencias(app) {
        if let Some(pasta) = caminho.parent() {
            let _ = fs::create_dir_all(pasta);
        }
        if let Ok(texto) = serde_json::to_string(preferencias) {
            let _ = fs::write(caminho, texto);
        }
    }
}

/// Páginas que abrem dentro do programa: a tela de abertura, o painel e a entrada (login, código, senha)
fn fica_no_programa(url: &Url) -> bool {
    let host = url.host_str().unwrap_or_default();
    // Tela de abertura do próprio programa (macOS: tauri://localhost; Windows: http://tauri.localhost)
    if url.scheme() == "tauri" || host == "tauri.localhost" || url.scheme() == "about" {
        return true;
    }
    if url.scheme() != "https" || host != SITE {
        return false;
    }
    let caminho = url.path();
    ["/painel", "/entrar", "/primeiro-acesso", "/recuperar", "/api/", "/_next/"]
        .iter()
        .any(|inicio| caminho.starts_with(inicio))
}

/// Toda navegação passa por aqui. O que não é do painel abre no navegador e a janela fica onde estava.
fn decidir_navegacao(app: &AppHandle, url: &Url) -> bool {
    #[cfg(debug_assertions)]
    eprintln!("[painel] navegar: {url}");
    let pediu_navegador = url.query_pairs().any(|(chave, _)| chave == "no-navegador");
    if fica_no_programa(url) && !pediu_navegador {
        return true;
    }
    // Depois de entrar (ou de criar a senha), a Central manda para /hoje: no programa, o lugar é o painel
    if url.host_str() == Some(SITE) && url.path() == "/hoje" && !pediu_navegador {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            if let (Some(janela), Ok(painel)) = (app.get_webview_window(JANELA), Url::parse(PAINEL)) {
                let _ = janela.navigate(painel);
            }
        });
        return false;
    }
    // O resto da Central e os outros sites: no navegador (sem o marcador "no-navegador")
    let mut destino = url.clone();
    let resto: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(chave, _)| chave != "no-navegador")
        .map(|(chave, valor)| (chave.into_owned(), valor.into_owned()))
        .collect();
    if resto.is_empty() {
        destino.set_query(None);
    } else {
        destino.query_pairs_mut().clear().extend_pairs(resto);
    }
    if matches!(destino.scheme(), "https" | "http" | "mailto") {
        let _ = tauri_plugin_opener::open_url(destino.as_str(), None::<&str>);
    }
    false
}

fn mostrar_painel(app: &AppHandle) {
    if let Some(janela) = app.get_webview_window(JANELA) {
        let _ = janela.unminimize();
        let _ = janela.show();
        let _ = janela.set_focus();
    }
}

fn mensagem(app: &AppHandle, titulo: &str, texto: String, tipo: MessageDialogKind) {
    app.dialog().message(texto).title(titulo).kind(tipo).blocking_show();
}

/// Procura versão nova. Se houver, pergunta e atualiza: baixa, instala e abre de novo, sem reinstalar à mão.
fn procurar_atualizacao(app: &AppHandle, manual: bool) {
    let Ok(atualizador) = app.updater() else { return };
    match tauri::async_runtime::block_on(atualizador.check()) {
        Ok(Some(atualizacao)) => {
            let versao = atualizacao.version.clone();
            let aceitou = app
                .dialog()
                .message(format!(
                    "Saiu a versão {versao} do Painel do dia.\n\nAtualizar agora? O painel fecha e abre de novo em instantes."
                ))
                .title("Atualização do Painel do dia")
                .kind(MessageDialogKind::Info)
                .buttons(MessageDialogButtons::OkCancelCustom("Atualizar agora".into(), "Depois".into()))
                .blocking_show();
            if !aceitou {
                return;
            }
            match tauri::async_runtime::block_on(atualizacao.download_and_install(|_, _| {}, || {})) {
                Ok(()) => app.restart(),
                Err(_) => mensagem(
                    app,
                    "Atualização do Painel do dia",
                    "Não deu para atualizar agora. O painel continua funcionando; ele tenta de novo mais tarde.".into(),
                    MessageDialogKind::Warning,
                ),
            }
        }
        Ok(None) if manual => mensagem(
            app,
            "Painel do dia",
            format!("Você já está com a versão mais nova ({}).", app.package_info().version),
            MessageDialogKind::Info,
        ),
        Err(_) if manual => mensagem(
            app,
            "Painel do dia",
            "Não deu para procurar atualização agora. Confira a internet e tente de novo.".into(),
            MessageDialogKind::Warning,
        ),
        _ => {}
    }
}

/// Primeira vez: janela na altura da tela (menos a barra de tarefas), encostada à direita
fn ajustar_primeira_vez(janela: &tauri::WebviewWindow) {
    if let Ok(Some(monitor)) = janela.current_monitor() {
        let tela = monitor.size().to_logical::<f64>(monitor.scale_factor());
        let altura = (tela.height - 96.0).clamp(560.0, 960.0);
        let _ = janela.set_size(LogicalSize::new(400.0, altura));
        let _ = janela.set_position(LogicalPosition::new((tela.width - 416.0).max(0.0), 16.0));
    }
}

fn montar_bandeja(app: &tauri::App, por_cima: bool) -> tauri::Result<()> {
    let mostrar = MenuItem::with_id(app, "mostrar", "Mostrar o painel", true, None::<&str>)?;
    let item_por_cima =
        CheckMenuItem::with_id(app, "por_cima", "Sempre por cima das outras janelas", true, por_cima, None::<&str>)?;
    let ligado = app.autolaunch().is_enabled().unwrap_or(false);
    let item_iniciar =
        CheckMenuItem::with_id(app, "iniciar", "Abrir quando o computador ligar", true, ligado, None::<&str>)?;
    let central = MenuItem::with_id(app, "central", "Abrir a Central no navegador", true, None::<&str>)?;
    let atualizar = MenuItem::with_id(app, "atualizar", "Procurar atualização", true, None::<&str>)?;
    let versao = MenuItem::with_id(
        app,
        "versao",
        format!("Versão {}", app.package_info().version),
        false,
        None::<&str>,
    )?;
    let sair = MenuItem::with_id(app, "sair", "Sair do Painel do dia", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &mostrar,
            &PredefinedMenuItem::separator(app)?,
            &item_por_cima,
            &item_iniciar,
            &PredefinedMenuItem::separator(app)?,
            &central,
            &atualizar,
            &versao,
            &PredefinedMenuItem::separator(app)?,
            &sair,
        ],
    )?;

    let icone = app.default_window_icon().cloned().expect("ícone do programa");
    TrayIconBuilder::with_id("principal")
        .icon(icone)
        .tooltip("Painel do dia · Central da Educação")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, evento| match evento.id().as_ref() {
            "mostrar" => mostrar_painel(app),
            "por_cima" => {
                let ligar = item_por_cima.is_checked().unwrap_or(true);
                if let Some(janela) = app.get_webview_window(JANELA) {
                    let _ = janela.set_always_on_top(ligar);
                }
                let mut preferencias = ler_preferencias(app);
                preferencias.por_cima = Some(ligar);
                salvar_preferencias(app, &preferencias);
            }
            "iniciar" => {
                let gerenciador = app.autolaunch();
                let _ = if item_iniciar.is_checked().unwrap_or(false) {
                    gerenciador.enable()
                } else {
                    gerenciador.disable()
                };
            }
            "central" => {
                let _ = tauri_plugin_opener::open_url(CENTRAL, None::<&str>);
            }
            "atualizar" => {
                let app = app.clone();
                thread::spawn(move || procurar_atualizacao(&app, true));
            }
            "sair" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|bandeja, evento| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = evento {
                mostrar_painel(bandeja.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Abrir de novo só traz para a frente o painel que já está aberto
        .plugin(tauri_plugin_single_instance::init(|app, _argumentos, _pasta| mostrar_painel(app)))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let mut preferencias = ler_preferencias(&handle);
            let por_cima = preferencias.por_cima.unwrap_or(true);

            let para_navegacao = handle.clone();
            let janela = WebviewWindowBuilder::new(app, JANELA, WebviewUrl::App("index.html".into()))
                .title("Painel do dia · Central da Educação")
                .inner_size(400.0, 820.0)
                .min_inner_size(360.0, 480.0)
                .always_on_top(por_cima)
                .on_navigation(move |url| decidir_navegacao(&para_navegacao, url))
                .on_page_load(|_janela, carga| {
                    #[cfg(debug_assertions)]
                    eprintln!("[painel] página {:?}: {}", carga.event(), carga.url());
                    let _ = carga;
                })
                .build()?;

            // Fechar a janela só esconde: o painel continua perto do relógio (para sair: menu do ícone)
            let para_fechar = janela.clone();
            janela.on_window_event(move |evento| {
                if let WindowEvent::CloseRequested { api, .. } = evento {
                    api.prevent_close();
                    let _ = para_fechar.hide();
                }
            });

            if !preferencias.ja_abriu {
                ajustar_primeira_vez(&janela);
                // Na primeira vez, já fica para abrir quando o computador ligar (dá para desligar no menu do ícone)
                let _ = handle.autolaunch().enable();
                preferencias.ja_abriu = true;
                preferencias.por_cima = Some(por_cima);
                salvar_preferencias(&handle, &preferencias);
            }

            montar_bandeja(app, por_cima)?;

            // Procura atualização pouco depois de abrir e depois de tempos em tempos
            let para_atualizar = handle.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(20));
                loop {
                    procurar_atualizacao(&para_atualizar, false);
                    thread::sleep(INTERVALO_ATUALIZACAO);
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("não deu para abrir o Painel do dia");
}
