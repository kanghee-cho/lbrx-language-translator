mod config;
mod engine;
mod hotkey;
mod platform;

use config::Config;
use engine::Translation;
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc, Arc, Mutex, RwLock,
    },
    time::Duration,
};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};
use tokio::sync::oneshot;

struct AppState {
    config: Arc<RwLock<Config>>,
    config_path: PathBuf,
    config_error: Mutex<Option<String>>,
    listener_error: Mutex<Option<String>>,
    busy: AtomicBool,
    next_id: AtomicU64,
    popup: Mutex<Option<Popup>>,
    cancel: Mutex<Option<oneshot::Sender<()>>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Popup {
    id: u64,
    #[serde(skip)]
    anchor: tauri::PhysicalPosition<f64>,
    status: String,
    source: String,
    source_language: Option<String>,
    skipped_languages: Vec<String>,
    translations: Vec<Translation>,
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    language_codes: Vec<&'static str>,
    config: Config,
    shortcut: &'static str,
    platform: &'static str,
    permission_granted: bool,
    config_path: String,
    config_error: Option<String>,
    listener_error: Option<String>,
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> String {
    format!("앱 상태에 접근하지 못했습니다. 앱을 다시 시작해주세요: {error}")
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Result<Settings, String> {
    let state = app.state::<AppState>();
    let config = state.config.read().map_err(lock_error)?.clone();
    let config_error = state.config_error.lock().map_err(lock_error)?.clone();
    let listener_error = state.listener_error.lock().map_err(lock_error)?.clone();
    Ok(Settings {
        language_codes: config::LANGUAGE_CODES.split_ascii_whitespace().collect(),
        config,
        shortcut: if cfg!(target_os = "macos") {
            "⌘ C C"
        } else {
            "Ctrl C C"
        },
        platform: std::env::consts::OS,
        permission_granted: platform::input_permission(),
        config_path: state.config_path.display().to_string(),
        config_error,
        listener_error,
    })
}

#[tauri::command]
fn save_settings(
    app: AppHandle,
    window: WebviewWindow,
    config: Config,
) -> Result<Settings, String> {
    if window.label() != "settings" {
        return Err("설정 창에서만 변경할 수 있습니다.".into());
    }
    let config = config.validated()?;
    let state = app.state::<AppState>();
    config::save(&state.config_path, &config)?;
    *state.config.write().map_err(lock_error)? = config;
    *state.config_error.lock().map_err(lock_error)? = None;
    get_settings(app)
}

#[tauri::command]
async fn check_cli(config: Config) -> Result<String, String> {
    engine::version(&config.validated()?).await
}

#[tauri::command]
fn open_permissions() -> Result<(), String> {
    platform::open_permissions()
}

#[tauri::command]
fn get_popup(app: AppHandle) -> Result<Option<Popup>, String> {
    app.state::<AppState>()
        .popup
        .lock()
        .map_err(lock_error)
        .map(|value| value.clone())
}

fn clear_popup(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    *state.popup.lock().map_err(lock_error)? = None;
    if let Some(cancel) = state.cancel.lock().map_err(lock_error)?.take() {
        let _ = cancel.send(());
    }
    Ok(())
}

fn dismiss(app: &AppHandle) -> Result<(), String> {
    clear_popup(app)?;
    if let Some(window) = app.get_webview_window("tooltip") {
        window.destroy().map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn close_window(app: AppHandle, window: WebviewWindow) -> Result<(), String> {
    if window.label() == "tooltip" {
        dismiss(&app)
    } else {
        window.close().map_err(|error| error.to_string())
    }
}

fn show_settings(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        window.show().map_err(|error| error.to_string())?;
        window.unminimize().map_err(|error| error.to_string())?;
        return window.set_focus().map_err(|error| error.to_string());
    }
    WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("index.html".into()))
        .title("LBRX Translator")
        .inner_size(640.0, 760.0)
        .min_inner_size(500.0, 500.0)
        .center()
        .build()
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn open_settings(app: AppHandle) {
    // WebView2 window creation must not block a Windows event-handler thread.
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(error) = show_settings(&app) {
            eprintln!("Cannot open settings: {error}");
        }
    });
}

fn show_tooltip(app: &AppHandle) -> Result<(), String> {
    if app
        .state::<AppState>()
        .popup
        .lock()
        .map_err(lock_error)?
        .is_none()
    {
        return Ok(());
    }
    if app.get_webview_window("tooltip").is_none() {
        WebviewWindowBuilder::new(app, "tooltip", WebviewUrl::App("index.html".into()))
            .initialization_script("window.__LBRX_VIEW__ = 'tooltip';")
            .title("Translation")
            .inner_size(440.0, 240.0)
            .decorations(false)
            .resizable(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(false)
            .focusable(false)
            .visible(false)
            .build()
            .map_err(|error| error.to_string())?;
    }
    app.emit_to("tooltip", "translation-updated", ())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn present_popup(app: AppHandle, window: WebviewWindow, height: f64) -> Result<(), String> {
    if window.label() != "tooltip" || !height.is_finite() {
        return Err("툴팁 크기 요청이 올바르지 않습니다.".into());
    }
    let cursor = match app
        .state::<AppState>()
        .popup
        .lock()
        .map_err(lock_error)?
        .as_ref()
    {
        Some(popup) => popup.anchor,
        None => return Ok(()),
    };
    let monitor = app
        .monitor_from_point(cursor.x, cursor.y)
        .map_err(|error| error.to_string())?
        .or(app.primary_monitor().map_err(|error| error.to_string())?)
        .ok_or("모니터 정보를 가져오지 못했습니다.")?;
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let width = 440.0_f64
        .min(f64::from(area.size.width) / scale - 16.0)
        .max(100.0);
    let height = height
        .clamp(160.0, 620.0)
        .min(f64::from(area.size.height) / scale - 16.0)
        .max(100.0);
    let physical_width = width * scale;
    let physical_height = height * scale;
    let left = f64::from(area.position.x);
    let top = f64::from(area.position.y);
    let right = left + f64::from(area.size.width);
    let bottom = top + f64::from(area.size.height);
    let x = if cursor.x + 16.0 + physical_width > right {
        cursor.x - physical_width - 16.0
    } else {
        cursor.x + 16.0
    }
    .clamp(left, (right - physical_width).max(left));
    let y = (cursor.y + 20.0).clamp(top, (bottom - physical_height).max(top));
    window
        .set_size(tauri::LogicalSize::new(width, height))
        .map_err(|error| error.to_string())?;
    window
        .set_position(tauri::PhysicalPosition::new(x as i32, y as i32))
        .map_err(|error| error.to_string())?;
    window.show().map_err(|error| error.to_string())
}

fn fresh_clipboard(baseline: u64) -> Result<String, String> {
    let mut last_error =
        "새로 복사한 텍스트가 없습니다. 문장을 선택한 뒤 단축키를 눌러주세요.".to_owned();
    std::thread::sleep(Duration::from_millis(100));
    for _ in 0..8 {
        let revision = platform::clipboard_revision();
        if revision != baseline {
            match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get_text()) {
                Ok(text) if platform::clipboard_revision() == revision => {
                    if text.trim().is_empty() {
                        return Err("선택한 텍스트가 비어 있습니다.".into());
                    }
                    if text.chars().count() > config::MAX_SOURCE_CHARS {
                        return Err(format!(
                            "한 번에 {}자까지 번역할 수 있습니다.",
                            config::MAX_SOURCE_CHARS
                        ));
                    }
                    return Ok(text);
                }
                Ok(_) => {
                    last_error = "클립보드가 변경되었습니다. 다시 선택하고 복사해주세요.".into()
                }
                Err(error) => last_error = format!("복사한 텍스트를 읽지 못했습니다: {error}"),
            }
        }
        std::thread::sleep(Duration::from_millis(75));
    }
    Err(last_error)
}

async fn translate_selection(app: AppHandle, baseline: u64) -> Result<(), String> {
    let state = app.state::<AppState>();
    if state
        .busy
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Ok(());
    }
    let result = translation_request(&app, baseline).await;
    state.busy.store(false, Ordering::SeqCst);
    result
}

async fn translation_request(app: &AppHandle, baseline: u64) -> Result<(), String> {
    let state = app.state::<AppState>();
    let config = state.config.read().map_err(lock_error)?.clone();
    if !config.consent {
        return Err("설정에서 번역 전송에 동의해주세요.".into());
    }
    let id = state.next_id.fetch_add(1, Ordering::SeqCst);
    let (cancel, cancelled) = oneshot::channel();
    *state.cancel.lock().map_err(lock_error)? = Some(cancel);
    *state.popup.lock().map_err(lock_error)? = Some(Popup {
        id,
        anchor: app.cursor_position().map_err(|error| error.to_string())?,
        status: "loading".into(),
        source: String::new(),
        source_language: None,
        skipped_languages: Vec::new(),
        translations: Vec::new(),
        error: None,
    });
    show_tooltip(app)?;
    let work = async {
        let source = tauri::async_runtime::spawn_blocking(move || fresh_clipboard(baseline))
            .await
            .map_err(|error| error.to_string())??;
        {
            let mut popup = state.popup.lock().map_err(lock_error)?;
            if let Some(popup) = popup.as_mut().filter(|popup| popup.id == id) {
                popup.source = source.clone();
            }
        }
        app.emit_to("tooltip", "translation-updated", ())
            .map_err(|error| error.to_string())?;
        engine::translate(&config, &source).await
    };
    let result = tokio::select! {
        result = work => result,
        _ = cancelled => return Ok(()),
    };
    {
        let mut popup = state.popup.lock().map_err(lock_error)?;
        if let Some(popup) = popup.as_mut().filter(|popup| popup.id == id) {
            match result {
                Ok(result) => {
                    popup.status = "complete".into();
                    popup.source_language = Some(result.source_language);
                    popup.skipped_languages = result.skipped_languages;
                    popup.translations = result.translations;
                }
                Err(error) => {
                    popup.status = "error".into();
                    popup.error = Some(error);
                }
            }
        }
    }
    *state.cancel.lock().map_err(lock_error)? = None;
    app.emit_to("tooltip", "translation-updated", ())
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn report_error(app: &AppHandle, error: String) {
    eprintln!("LBRX Translator: {error}");
    match app.state::<AppState>().listener_error.lock() {
        Ok(mut value) => *value = Some(error),
        Err(error) => eprintln!("Cannot report application error: {error}"),
    }
    open_settings(app.clone());
    if let Err(error) = app.emit_to("settings", "runtime-updated", ()) {
        eprintln!("Cannot notify settings: {error}");
    }
}

fn tray_icon() -> Image<'static> {
    let mut pixels = vec![0_u8; 24 * 24 * 4];
    for y in 4..20 {
        for x in 4..20 {
            let l = (x <= 6 || y >= 17) && x <= 13;
            let arrow = (15..=17).contains(&x) && y <= 13
                || (10..=14).contains(&y) && x >= 12 && (x as i32 - 16).abs() <= 14 - y as i32;
            if l || arrow {
                let offset = (y * 24 + x) * 4;
                pixels[offset..offset + 4].copy_from_slice(&[42, 91, 73, 255]);
            }
        }
    }
    Image::new_owned(pixels, 24, 24)
}

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            open_settings(app.clone());
        }))
        .invoke_handler(tauri::generate_handler![
            get_settings, save_settings, check_cli, open_permissions,
            get_popup, present_popup, close_window
        ])
        .on_window_event(|window, event| {
            if window.label() == "tooltip" && matches!(event, tauri::WindowEvent::Destroyed) {
                if let Err(error) = clear_popup(window.app_handle()) {
                    eprintln!("Cannot clear translation popup: {error}");
                }
            }
        })
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let config_path = app.path().app_config_dir()?.join("config.json");
            let (config, config_error) = match config::load(&config_path) {
                Ok(config) => (config, None),
                Err(error) => (Config::default(), Some(error)),
            };
            let permission = platform::input_permission();
            let needs_setup = !config.consent || config_error.is_some() || !permission;
            let config = Arc::new(RwLock::new(config));
            app.manage(AppState {
                config: config.clone(),
                config_path,
                config_error: Mutex::new(config_error),
                listener_error: Mutex::new(if permission { None } else {
                    Some("macOS 손쉬운 사용 권한을 허용한 뒤 앱을 다시 시작해주세요. 입력 모니터링을 요청하면 함께 허용해주세요.".into())
                }),
                busy: AtomicBool::new(false),
                next_id: AtomicU64::new(1),
                popup: Mutex::new(None),
                cancel: Mutex::new(None),
            });
            let settings = MenuItem::with_id(app, "settings", "설정…", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "종료", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings, &quit])?;
            TrayIconBuilder::with_id("translator")
                .icon(tray_icon())
                .icon_as_template(cfg!(target_os = "macos"))
                .tooltip("LBRX Translator")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "settings" => {
                        open_settings(app.clone());
                    }
                    "quit" => {
                        if let Err(error) = clear_popup(app) {
                            eprintln!("Cannot cancel translation: {error}");
                        }
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            if permission {
                let (sender, receiver) = mpsc::sync_channel(4);
                hotkey::start(config, sender);
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    for shortcut in receiver {
                        let app = handle.clone();
                        match shortcut {
                            hotkey::Shortcut::Translate { clipboard_revision } => {
                                tauri::async_runtime::spawn(async move {
                                    if let Err(error) = translate_selection(app.clone(), clipboard_revision).await {
                                        report_error(&app, error);
                                    }
                                });
                            }
                            hotkey::Shortcut::Dismiss => {
                                let handle = app.clone();
                                if let Err(error) = app.run_on_main_thread(move || {
                                    if let Err(error) = dismiss(&handle) {
                                        report_error(&handle, error);
                                    }
                                }) {
                                    eprintln!("Cannot dismiss translation: {error}");
                                }
                            }
                            hotkey::Shortcut::Error(error) => {
                                let handle = app.clone();
                                if let Err(error) = app.run_on_main_thread(move || report_error(&handle, error)) {
                                    eprintln!("Cannot report shortcut failure: {error}");
                                }
                            }
                        }
                    }
                });
            }
            if needs_setup {
                show_settings(app.handle()).map_err(std::io::Error::other)?;
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("LBRX Translator could not start");
    app.run(|_, event| {
        if let tauri::RunEvent::ExitRequested {
            api, code: None, ..
        } = event
        {
            api.prevent_exit();
        }
    });
}
