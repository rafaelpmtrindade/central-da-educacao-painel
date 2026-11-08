//! Programa Painel do dia da Central da Educação (Secretaria Municipal de Educação de Vila Rica, MT).
//!
//! Uma janela pequena que mostra o Painel do dia publicado (central-da-educacao.vercel.app/painel), sempre por cima
//! das outras janelas, aberta quando o computador liga, com ícone perto do relógio e atualização sozinha. O conteúdo
//! vem do site e se atualiza na hora; o programa em si só muda quando sai uma versão nova (assinada por nós).
//!
//! Segurança: a página só pode pedir ao programa para mostrar notificação (capabilities/remoto.json). Links para o
//! resto da Central e para outros sites são abertos no navegador pelo próprio programa, nunca pela página.

use std::{
    fs,
    path::PathBuf,
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, LogicalPosition, LogicalSize, Manager, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

const SITE: &str = "central-da-educacao.vercel.app";
const CENTRAL: &str = "https://central-da-educacao.vercel.app/hoje";
const JANELA: &str = "painel";
/// De quanto em quanto tempo o programa procura versão nova (o widget fica aberto o dia todo)
const INTERVALO_ATUALIZACAO: Duration = Duration::from_secs(60 * 60);

/// O que o programa lembra entre uma abertura e outra (o tamanho e o lugar da janela ficam com o window-state)
#[derive(Serialize, Deserialize, Default)]
struct Preferencias {
    ja_abriu: bool,
    /// "widget" (preso à área de trabalho, o padrão) ou "por_cima" (janela por cima das outras)
    modo: Option<String>,
    /// "pequeno", "medio" (o padrão) ou "grande"
    tamanho: Option<String>,
    /// "sistema" (o padrão), "clara" ou "escura"
    aparencia: Option<String>,
    /// Versão da última abertura (para avisar quando o programa se atualizou)
    versao: Option<String>,
    /// Identificador aleatório deste programa neste computador (32 letras e números, criado na primeira abertura). Vai
    /// à Central junto com a versão, para o suporte saber quem tem o programa e se está atualizado; não diz nada do
    /// computador nem da pessoa.
    aparelho: Option<String>,
    /// Altura que a pessoa escolheu puxando a borda da janela. Enquanto existir, a altura não segue mais o conteúdo
    /// (só o meio do painel rola); escolher um tamanho no menu volta para a altura automática.
    altura: Option<f64>,
}

/// O último tamanho que o próprio programa deu à janela, e quando. O sistema avisa do mesmo jeito a mudança feita pelo
/// programa e a feita pela pessoa puxando a borda; só conta como da pessoa uma altura diferente da que o programa pediu
/// (restaurar depois de minimizar, mudar a escala da tela ou puxar só a lateral voltam a altura que já era).
struct UltimoAjuste(Mutex<(Instant, f64)>);

const FOLGA_AJUSTE: Duration = Duration::from_secs(2);

fn marcar_ajuste(app: &AppHandle, altura: Option<f64>) {
    if let Some(ultimo) = app.try_state::<UltimoAjuste>() {
        if let Ok(mut guardado) = ultimo.0.lock() {
            *guardado = (Instant::now(), altura.unwrap_or(guardado.1));
        }
    }
}

fn mudar_tamanho(janela: &tauri::WebviewWindow, largura: f64, altura: f64) {
    marcar_ajuste(janela.app_handle(), Some(altura));
    let _ = janela.set_size(LogicalSize::new(largura, altura));
}

/// A altura escolhida à mão só vale no modo "por cima" (o widget não deixa puxar a borda e segue o conteúdo)
fn altura_escolhida(preferencias: &Preferencias) -> Option<f64> {
    preferencias.altura.filter(|_| Modo::ler(preferencias) == Modo::PorCima)
}

/// A pessoa mudou a altura puxando a borda (modo "por cima"): guarda para valer também nas próximas aberturas
fn guardar_altura_escolhida(janela: &tauri::WebviewWindow, fisico: tauri::PhysicalSize<u32>) {
    let app = janela.app_handle();
    let mut preferencias = ler_preferencias(app);
    // Minimizar, maximizar e tela cheia também avisam, e não são uma altura escolhida
    if Modo::ler(&preferencias) != Modo::PorCima
        || janela.is_minimized().unwrap_or(true)
        || janela.is_maximized().unwrap_or(true)
        || janela.is_fullscreen().unwrap_or(true)
    {
        return;
    }
    let altura = fisico.to_logical::<f64>(janela.scale_factor().unwrap_or(1.0)).height.round();
    let do_programa = app
        .try_state::<UltimoAjuste>()
        .and_then(|ultimo| ultimo.0.lock().ok().map(|guardado| guardado.0.elapsed() < FOLGA_AJUSTE || (altura - guardado.1).abs() <= 2.0))
        .unwrap_or(true);
    if do_programa || altura < 240.0 || preferencias.altura == Some(altura) {
        return;
    }
    preferencias.altura = Some(altura);
    salvar_preferencias(app, &preferencias);
}

#[derive(Clone, Copy, PartialEq)]
enum Tamanho {
    Pequeno,
    Medio,
    Grande,
}

impl Tamanho {
    fn ler(preferencias: &Preferencias) -> Tamanho {
        match preferencias.tamanho.as_deref() {
            Some("pequeno") => Tamanho::Pequeno,
            Some("grande") => Tamanho::Grande,
            _ => Tamanho::Medio,
        }
    }
    fn nome(self) -> &'static str {
        match self {
            Tamanho::Pequeno => "pequeno",
            Tamanho::Medio => "medio",
            Tamanho::Grande => "grande",
        }
    }
    /// O painel publicado, montado para este tamanho
    fn endereco(self, app: &AppHandle) -> String {
        format!("https://{SITE}/painel?app=1&tamanho={}{}", self.nome(), identificacao(app))
    }
}

/// Sistema do computador, como a Central registra
const SISTEMA: &str = if cfg!(target_os = "macos") { "mac" } else { "windows" };

/// "&versao=0.4.0&sistema=windows&aparelho=…": o painel manda à Central (Administração → Implantação)
fn identificacao(app: &AppHandle) -> String {
    let aparelho = ler_preferencias(app).aparelho.unwrap_or_default();
    format!("&versao={}&sistema={SISTEMA}&aparelho={aparelho}", app.package_info().version)
}

#[derive(Clone, Copy, PartialEq)]
enum Modo {
    /// Preso à área de trabalho, sem bordas e fora da barra de tarefas (o jeito que vem ao instalar)
    Widget,
    /// Janela com bordas, sempre por cima das outras
    PorCima,
}

impl Modo {
    fn ler(preferencias: &Preferencias) -> Modo {
        match preferencias.modo.as_deref() {
            Some("por_cima") => Modo::PorCima,
            _ => Modo::Widget,
        }
    }
    fn nome(self) -> &'static str {
        match self {
            Modo::Widget => "widget",
            Modo::PorCima => "por_cima",
        }
    }
}

/// Tamanho e lugar conforme a tela, no canto de cima à direita. Pequeno e médio continuam compactos (não ocupam a
/// área de trabalho); grande vai quase até embaixo.
fn tamanho_e_lugar(janela: &tauri::WebviewWindow, tamanho: Tamanho) {
    if let Ok(Some(monitor)) = janela.current_monitor() {
        let tela = monitor.size().to_logical::<f64>(monitor.scale_factor());
        let (largura, altura) = match tamanho {
            Tamanho::Pequeno => (340.0, (tela.height - 160.0).clamp(420.0, 560.0)),
            Tamanho::Medio => (420.0, (tela.height - 160.0).clamp(520.0, 720.0)),
            Tamanho::Grande => (480.0, (tela.height - 96.0).clamp(600.0, 960.0)),
        };
        // A altura que a pessoa escolheu vale até ela escolher um tamanho no menu (sem passar da tela)
        let altura = altura_escolhida(&ler_preferencias(janela.app_handle())).map_or(altura, |escolhida| escolhida.min(tela.height));
        mudar_tamanho(janela, largura, altura);
        let _ = janela.set_position(LogicalPosition::new((tela.width - largura - 24.0).max(0.0), 40.0));
    }
}

/// Aplica o jeito de mostrar. `reposicionar`: ao trocar de jeito (ao abrir, o tamanho e o lugar guardados valem)
fn aplicar_modo(janela: &tauri::WebviewWindow, modo: Modo, tamanho: Tamanho, reposicionar: bool) {
    let widget = modo == Modo::Widget;
    let _ = janela.set_decorations(!widget);
    let _ = janela.set_skip_taskbar(widget);
    let _ = janela.set_resizable(!widget);
    let _ = janela.set_always_on_top(!widget);
    let _ = janela.set_always_on_bottom(widget);
    // No Mac, o widget aparece em todas as áreas de trabalho (Spaces)
    let _ = janela.set_visible_on_all_workspaces(widget);
    if reposicionar {
        tamanho_e_lugar(janela, tamanho);
    }
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
            let painel = Tamanho::ler(&ler_preferencias(&app)).endereco(&app);
            if let (Some(janela), Ok(painel)) = (app.get_webview_window(JANELA), Url::parse(&painel)) {
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

/// Procura versão nova e atualiza sozinho: baixa, instala e abre de novo em poucos segundos, sem pedir clique (como o
/// Chrome). Ao reabrir, avisa com uma notificação. `manual` (menu "Procurar atualização"): também diz quando já está
/// na mais nova ou quando não deu para procurar.
fn procurar_atualizacao(app: &AppHandle, manual: bool) {
    let Ok(atualizador) = app.updater() else { return };
    let resultado = tauri::async_runtime::block_on(atualizador.check());
    mostrar_versao(
        app,
        match &resultado {
            Ok(None) => "a mais nova",
            Ok(Some(_)) => "atualizando",
            Err(_) => "não deu para conferir",
        },
    );
    match resultado {
        Ok(Some(atualizacao)) => {
            if tauri::async_runtime::block_on(atualizacao.download_and_install(|_, _| {}, || {})).is_ok() {
                app.restart();
            }
            // Não instalou (sem permissão, internet caiu no meio): o menu não pode ficar dizendo "atualizando"
            mostrar_versao(app, "não deu para atualizar");
            if manual {
                mensagem(
                    app,
                    "Atualização do Painel do dia",
                    "Não deu para atualizar agora. O painel continua funcionando; ele tenta de novo mais tarde.".into(),
                    MessageDialogKind::Warning,
                );
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

/// O item de versão do menu diz se o programa está em dia (conferido a cada hora e no "Procurar atualização")
fn mostrar_versao(app: &AppHandle, situacao: &str) {
    if let Some(itens) = app.try_state::<ItensDoMenu>() {
        let _ = itens.versao.set_text(format!("Versão {} · {situacao}", app.package_info().version));
    }
}

/// Depois de uma atualização: avisa com uma notificação que o programa mudou de versão
fn avisar_se_atualizou(app: &AppHandle, preferencias: &mut Preferencias) {
    let atual = app.package_info().version.to_string();
    if let Some(antes) = preferencias.versao.as_deref() {
        if antes != atual {
            use tauri_plugin_notification::NotificationExt;
            let _ = app
                .notification()
                .builder()
                .title("Painel do dia atualizado")
                .body(format!("Agora na versão {atual}."))
                .show();
        }
    }
    preferencias.versao = Some(atual);
}

/// Aparência do painel no programa: igual ao sistema (o padrão), clara ou escura. Fica com o programa e é aplicada à
/// página a cada carregamento (a Central guarda a escolha no aparelho, na chave do tema).
#[derive(Clone, Copy, PartialEq)]
enum Aparencia {
    Sistema,
    Clara,
    Escura,
}

impl Aparencia {
    fn ler(preferencias: &Preferencias) -> Aparencia {
        match preferencias.aparencia.as_deref() {
            Some("clara") => Aparencia::Clara,
            Some("escura") => Aparencia::Escura,
            _ => Aparencia::Sistema,
        }
    }
    fn nome(self) -> &'static str {
        match self {
            Aparencia::Sistema => "sistema",
            Aparencia::Clara => "clara",
            Aparencia::Escura => "escura",
        }
    }
    /// Script que põe a aparência na página da Central (mesma chave e mesmos valores de src/componentes/SeletorTema.tsx)
    fn script(self) -> &'static str {
        match self {
            Aparencia::Sistema => {
                "try{localStorage.setItem('central-educacao-tema','auto')}catch(e){};document.documentElement.dataset.tema='auto'"
            }
            Aparencia::Escura => {
                "try{localStorage.setItem('central-educacao-tema','escuro')}catch(e){};document.documentElement.dataset.tema='escuro'"
            }
            Aparencia::Clara => {
                "try{localStorage.removeItem('central-educacao-tema')}catch(e){};delete document.documentElement.dataset.tema"
            }
        }
    }
}

/// Os itens marcáveis do menu, guardados para o menu do ícone e o menu ⋯ do painel (os dois são o mesmo menu)
struct ItensDoMenu {
    menu: Menu<tauri::Wry>,
    widget: CheckMenuItem<tauri::Wry>,
    por_cima: CheckMenuItem<tauri::Wry>,
    pequeno: CheckMenuItem<tauri::Wry>,
    medio: CheckMenuItem<tauri::Wry>,
    grande: CheckMenuItem<tauri::Wry>,
    sistema: CheckMenuItem<tauri::Wry>,
    clara: CheckMenuItem<tauri::Wry>,
    escura: CheckMenuItem<tauri::Wry>,
    iniciar: CheckMenuItem<tauri::Wry>,
    /// "Versão 0.4.0 · a mais nova": o suporte confere aqui se o computador está atualizado
    versao: MenuItem<tauri::Wry>,
}

/// Maior altura que a janela pode ter no tamanho escolhido (a altura segue o conteúdo até aqui)
fn altura_maxima(janela: &tauri::WebviewWindow, tamanho: Tamanho) -> f64 {
    let tela = janela
        .current_monitor()
        .ok()
        .flatten()
        .map(|m| m.size().to_logical::<f64>(m.scale_factor()).height)
        .unwrap_or(900.0);
    match tamanho {
        Tamanho::Pequeno => (tela - 160.0).clamp(420.0, 560.0),
        Tamanho::Medio => (tela - 160.0).clamp(520.0, 720.0),
        Tamanho::Grande => (tela - 96.0).clamp(600.0, 960.0),
    }
}

/// A página pede para a janela ter a altura do conteúdo (dentro do limite do tamanho)
#[tauri::command]
fn ajustar_altura(janela: tauri::WebviewWindow, altura: f64) {
    let preferencias = ler_preferencias(janela.app_handle());
    // Com a altura escolhida pela pessoa, ela manda: o painel rola o meio em vez de a janela mudar
    if !altura.is_finite() || altura_escolhida(&preferencias).is_some() {
        return;
    }
    let tamanho = Tamanho::ler(&preferencias);
    let altura = altura.clamp(240.0, altura_maxima(&janela, tamanho));
    if let Ok(atual) = janela.inner_size() {
        let escala = janela.scale_factor().unwrap_or(1.0);
        let largura = atual.to_logical::<f64>(escala).width;
        mudar_tamanho(&janela, largura, altura);
    }
}

/// O botão ⋯ do painel abre o mesmo menu do ícone (útil quando o ícone some atrás do entalhe do MacBook)
#[tauri::command]
fn abrir_menu(janela: tauri::WebviewWindow) {
    if let Some(itens) = janela.app_handle().try_state::<ItensDoMenu>() {
        let _ = janela.popup_menu(&itens.menu);
    }
}

fn montar_menu(app: &tauri::App, modo: Modo, tamanho: Tamanho, aparencia: Aparencia) -> tauri::Result<()> {
    let marcar = |id: &str, texto: &str, marcado: bool| CheckMenuItem::with_id(app, id, texto, true, marcado, None::<&str>);
    let widget = marcar("modo_widget", "Fixo na área de trabalho (widget)", modo == Modo::Widget)?;
    let por_cima = marcar("modo_por_cima", "Por cima das janelas", modo == Modo::PorCima)?;
    let pequeno = marcar("tamanho_pequeno", "Pequeno", tamanho == Tamanho::Pequeno)?;
    let medio = marcar("tamanho_medio", "Médio", tamanho == Tamanho::Medio)?;
    let grande = marcar("tamanho_grande", "Grande", tamanho == Tamanho::Grande)?;
    let sistema = marcar("aparencia_sistema", "Igual ao sistema", aparencia == Aparencia::Sistema)?;
    let clara = marcar("aparencia_clara", "Clara", aparencia == Aparencia::Clara)?;
    let escura = marcar("aparencia_escura", "Escura", aparencia == Aparencia::Escura)?;
    let iniciar = marcar("iniciar", "Abrir quando o computador ligar", app.autolaunch().is_enabled().unwrap_or(false))?;
    // Até a primeira conferência (30 s depois de abrir), só a versão
    let versao = MenuItem::with_id(app, "versao", format!("Versão {}", app.package_info().version), false, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "mostrar", "Mostrar o painel", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &widget,
            &por_cima,
            &Submenu::with_items(app, "Tamanho", true, &[&pequeno, &medio, &grande])?,
            &Submenu::with_items(app, "Aparência", true, &[&sistema, &clara, &escura])?,
            &PredefinedMenuItem::separator(app)?,
            &iniciar,
            &MenuItem::with_id(app, "central", "Abrir a Central no navegador", true, None::<&str>)?,
            &MenuItem::with_id(app, "atualizar", "Procurar atualização", true, None::<&str>)?,
            &versao,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "sair", "Sair do Painel do dia", true, None::<&str>)?,
        ],
    )?;

    let icone = app.default_window_icon().cloned().expect("ícone do programa");
    TrayIconBuilder::with_id("principal")
        .icon(icone)
        .tooltip("Painel do dia · Central da Educação")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|bandeja, evento| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = evento {
                mostrar_painel(bandeja.app_handle());
            }
        })
        .build(app)?;

    app.manage(ItensDoMenu { menu, widget, por_cima, pequeno, medio, grande, sistema, clara, escura, iniciar, versao });
    // Um tratador só para o menu do ícone e o menu ⋯ do painel
    app.on_menu_event(|app, evento| tratar_menu(app, evento.id().as_ref()));
    Ok(())
}

fn tratar_menu(app: &AppHandle, id: &str) {
    let Some(itens) = app.try_state::<ItensDoMenu>() else { return };
    let janela = app.get_webview_window(JANELA);
    let mut preferencias = ler_preferencias(app);
    match id {
        "mostrar" => mostrar_painel(app),
        "modo_widget" | "modo_por_cima" => {
            // Os dois itens funcionam como uma escolha só: marcar um desmarca o outro
            let modo = if id == "modo_widget" { Modo::Widget } else { Modo::PorCima };
            let _ = itens.widget.set_checked(modo == Modo::Widget);
            let _ = itens.por_cima.set_checked(modo == Modo::PorCima);
            if let Some(janela) = &janela {
                aplicar_modo(janela, modo, Tamanho::ler(&preferencias), true);
                let _ = janela.show();
            }
            preferencias.modo = Some(modo.nome().into());
            salvar_preferencias(app, &preferencias);
        }
        "tamanho_pequeno" | "tamanho_medio" | "tamanho_grande" => {
            let tamanho = match id {
                "tamanho_pequeno" => Tamanho::Pequeno,
                "tamanho_grande" => Tamanho::Grande,
                _ => Tamanho::Medio,
            };
            let _ = itens.pequeno.set_checked(tamanho == Tamanho::Pequeno);
            let _ = itens.medio.set_checked(tamanho == Tamanho::Medio);
            let _ = itens.grande.set_checked(tamanho == Tamanho::Grande);
            preferencias.tamanho = Some(tamanho.nome().into());
            // Escolher um tamanho volta para a altura automática (desfaz a altura puxada à mão)
            preferencias.altura = None;
            salvar_preferencias(app, &preferencias);
            if let Some(janela) = &janela {
                tamanho_e_lugar(janela, tamanho);
                if let Ok(endereco) = Url::parse(&tamanho.endereco(app)) {
                    let _ = janela.navigate(endereco);
                }
                let _ = janela.show();
            }
        }
        "aparencia_sistema" | "aparencia_clara" | "aparencia_escura" => {
            let aparencia = match id {
                "aparencia_clara" => Aparencia::Clara,
                "aparencia_escura" => Aparencia::Escura,
                _ => Aparencia::Sistema,
            };
            let _ = itens.sistema.set_checked(aparencia == Aparencia::Sistema);
            let _ = itens.clara.set_checked(aparencia == Aparencia::Clara);
            let _ = itens.escura.set_checked(aparencia == Aparencia::Escura);
            preferencias.aparencia = Some(aparencia.nome().into());
            salvar_preferencias(app, &preferencias);
            if let Some(janela) = &janela {
                let _ = janela.eval(aparencia.script());
            }
        }
        "iniciar" => {
            let gerenciador = app.autolaunch();
            let _ = if itens.iniciar.is_checked().unwrap_or(false) {
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
    }
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
        .invoke_handler(tauri::generate_handler![ajustar_altura, abrir_menu])
        .setup(|app| {
            // Os ajustes de tamanho da abertura (lugar guardado, tamanho do painel) são do programa, não da pessoa
            app.manage(UltimoAjuste(Mutex::new((Instant::now(), 0.0))));
            let handle = app.handle().clone();
            let mut preferencias = ler_preferencias(&handle);
            let modo = Modo::ler(&preferencias);
            let tamanho = Tamanho::ler(&preferencias);
            let aparencia = Aparencia::ler(&preferencias);

            let para_navegacao = handle.clone();
            let para_carga = handle.clone();
            // A tela de abertura confere a internet e abre o painel no tamanho escolhido
            // Sem identificador, ou com um fora do formato (arquivo mexido à mão): sorteia outro
            let valido = |a: &str| a.len() == 32 && a.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c));
            if !preferencias.aparelho.as_deref().is_some_and(valido) {
                preferencias.aparelho = Some(uuid::Uuid::new_v4().simple().to_string());
                salvar_preferencias(&handle, &preferencias);
            }
            let abertura = format!("index.html?tamanho={}{}", tamanho.nome(), identificacao(&handle));
            let janela = WebviewWindowBuilder::new(app, JANELA, WebviewUrl::App(abertura.into()))
                .title("Painel do dia · Central da Educação")
                .inner_size(420.0, 720.0)
                .min_inner_size(320.0, 240.0)
                // Transparente: o painel desenha o próprio cartão de cantos arredondados
                .transparent(true)
                .on_navigation(move |url| decidir_navegacao(&para_navegacao, url))
                .on_page_load(move |janela, carga| {
                    #[cfg(debug_assertions)]
                    eprintln!("[painel] página {:?}: {}", carga.event(), carga.url());
                    // Cada vez que a Central carrega, recebe a aparência escolhida no programa
                    if matches!(carga.event(), tauri::webview::PageLoadEvent::Finished) && carga.url().host_str() == Some(SITE) {
                        let _ = janela.eval(Aparencia::ler(&ler_preferencias(&para_carga)).script());
                    }
                })
                .build()?;

            let para_eventos = janela.clone();
            janela.on_window_event(move |evento| match evento {
                // Fechar a janela só esconde: o painel continua perto do relógio (para sair: menu do ícone ou ⋯)
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = para_eventos.hide();
                }
                // O lugar fica guardado assim que a pessoa solta a janela (vale mesmo se o computador desligar de repente)
                WindowEvent::Moved(_) => {
                    let _ = para_eventos.app_handle().save_window_state(StateFlags::POSITION);
                }
                WindowEvent::Resized(fisico) => guardar_altura_escolhida(&para_eventos, *fisico),
                // Mudar a escala da tela redimensiona a janela pelo sistema: não é a pessoa puxando a borda
                WindowEvent::ScaleFactorChanged { .. } => marcar_ajuste(para_eventos.app_handle(), None),
                _ => {}
            });

            // Primeira vez: tamanho e lugar conforme a tela; depois, vale o que a pessoa deixou (window-state)
            aplicar_modo(&janela, modo, tamanho, !preferencias.ja_abriu);
            if !preferencias.ja_abriu {
                // Na primeira vez, já fica para abrir quando o computador ligar (dá para desligar no menu)
                let _ = handle.autolaunch().enable();
                // Um aviso de boas-vindas logo na instalação: o Mac pede a permissão de notificação no primeiro aviso, e
                // assim o primeiro aviso de verdade (um pedido, um comunicado) não se perde no meio desse pedido
                {
                    use tauri_plugin_notification::NotificationExt;
                    let _ = handle
                        .notification()
                        .builder()
                        .title("Painel do dia instalado")
                        .body("Os avisos de pedidos, respostas e comunicados vão aparecer assim.")
                        .show();
                }
                preferencias.ja_abriu = true;
                preferencias.modo = Some(modo.nome().into());
                preferencias.tamanho = Some(tamanho.nome().into());
                preferencias.aparencia = Some(aparencia.nome().into());
                salvar_preferencias(&handle, &preferencias);
            }

            avisar_se_atualizou(&handle, &mut preferencias);
            salvar_preferencias(&handle, &preferencias);

            // No Mac, sem ícone no Dock: o painel se acessa pelo ícone na barra de cima (ou pelo ⋯ do painel)
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            montar_menu(app, modo, tamanho, aparencia)?;

            // Procura atualização pouco depois de abrir e depois de tempos em tempos
            let para_atualizar = handle.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(30));
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
