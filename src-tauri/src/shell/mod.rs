use tauri::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};
use tauri_plugin_window_state::{AppHandleExt, StateFlags, WindowExt};

use crate::domain::preferences::{
    DesktopStickyKind, DesktopStickyPosition, Preferences, StickyMode,
};

const MIN_VISIBLE_EDGE: i32 = 64;
pub const DESKTOP_STICKY_WINDOW_LABEL: &str = "desktop-sticky";
const DESKTOP_STICKY_WINDOW_WIDTH: f64 = 94.0;
const DESKTOP_STICKY_WINDOW_HEIGHT: f64 = 62.0;
const DESKTOP_MENU_OPEN: &str = "desktop-sticky-open";
const DESKTOP_MENU_TODO: &str = "desktop-sticky-kind-todo";
const DESKTOP_MENU_RECORD: &str = "desktop-sticky-kind-record";
const DESKTOP_MENU_QUOTE: &str = "desktop-sticky-kind-quote";
const DESKTOP_MENU_HIDE: &str = "desktop-sticky-hide";
const DESKTOP_MENU_QUIT: &str = "desktop-sticky-quit";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl Rect {
    fn has_visible_edge_on(self, monitor: Rect) -> bool {
        let right = i64::from(self.x) + i64::from(self.width);
        let bottom = i64::from(self.y) + i64::from(self.height);
        let monitor_right = i64::from(monitor.x) + i64::from(monitor.width);
        let monitor_bottom = i64::from(monitor.y) + i64::from(monitor.height);
        let visible_width = right.min(monitor_right) - i64::from(self.x.max(monitor.x));
        let visible_height = bottom.min(monitor_bottom) - i64::from(self.y.max(monitor.y));

        visible_width >= i64::from(MIN_VISIBLE_EDGE)
            && visible_height >= i64::from(MIN_VISIBLE_EDGE)
    }

    fn clamped_origin(self, monitor: Rect) -> PhysicalPosition<i32> {
        let max_x = i64::from(monitor.x) + i64::from(monitor.width) - i64::from(self.width);
        let max_y = i64::from(monitor.y) + i64::from(monitor.height) - i64::from(self.height);
        let x = i64::from(self.x).clamp(i64::from(monitor.x), max_x.max(i64::from(monitor.x)));
        let y = i64::from(self.y).clamp(i64::from(monitor.y), max_y.max(i64::from(monitor.y)));

        PhysicalPosition::new(x as i32, y as i32)
    }
}

pub fn surface_main_window(app: &AppHandle) {
    let _ = crate::preferences::set_sticky_mode(app, StickyMode::Compact);
    if let Some(desktop) = app.get_webview_window(DESKTOP_STICKY_WINDOW_LABEL) {
        let _ = desktop.hide();
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = clamp_offscreen_geometry(&window);
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn desktop_window(app: &AppHandle, preferences: &Preferences) -> tauri::Result<WebviewWindow> {
    if let Some(window) = app.get_webview_window(DESKTOP_STICKY_WINDOW_LABEL) {
        return Ok(window);
    }

    let window = WebviewWindowBuilder::new(
        app,
        DESKTOP_STICKY_WINDOW_LABEL,
        WebviewUrl::App("index.html?surface=desktop-sticky".into()),
    )
    .title("Agent Desk Desktop Sticky")
    .inner_size(DESKTOP_STICKY_WINDOW_WIDTH, DESKTOP_STICKY_WINDOW_HEIGHT)
    .min_inner_size(DESKTOP_STICKY_WINDOW_WIDTH, DESKTOP_STICKY_WINDOW_HEIGHT)
    .max_inner_size(DESKTOP_STICKY_WINDOW_WIDTH, DESKTOP_STICKY_WINDOW_HEIGHT)
    .resizable(false)
    .maximizable(false)
    .minimizable(false)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .shadow(false)
    .transparent(true)
    .visible(false)
    .focused(false)
    .build()?;

    if let Some(position) = &preferences.desktop_sticky_position {
        window.set_position(PhysicalPosition::new(position.x, position.y))?;
    }
    clamp_offscreen_geometry(&window)?;
    Ok(window)
}

pub fn restore_desktop_sticky_window(
    app: &AppHandle,
    preferences: &Preferences,
) -> tauri::Result<()> {
    let window = desktop_window(app, preferences)?;
    window.show()?;
    let _ = app.emit_to(
        DESKTOP_STICKY_WINDOW_LABEL,
        "agent-desk://desktop-sticky-refresh",
        (),
    );
    Ok(())
}

#[tauri::command]
pub async fn enter_desktop_sticky(app: AppHandle) -> Result<Preferences, String> {
    let stored = crate::preferences::read(&app)?;
    let window = desktop_window(&app, &stored).map_err(|error| error.to_string())?;
    let preferences = crate::preferences::set_sticky_mode(&app, StickyMode::Desktop)?;
    window.show().map_err(|error| error.to_string())?;
    let _ = app.emit_to(
        DESKTOP_STICKY_WINDOW_LABEL,
        "agent-desk://desktop-sticky-refresh",
        (),
    );
    if let Some(main) = app.get_webview_window("main") {
        main.hide().map_err(|error| error.to_string())?;
    }
    Ok(preferences)
}

#[tauri::command]
pub async fn open_agent_desk_from_desktop(
    app: AppHandle,
    edit_kind: Option<DesktopStickyKind>,
) -> Result<Preferences, String> {
    let preferences = crate::preferences::set_sticky_mode(&app, StickyMode::Compact)?;
    if let Some(desktop) = app.get_webview_window(DESKTOP_STICKY_WINDOW_LABEL) {
        desktop.hide().map_err(|error| error.to_string())?;
    }
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "main window is unavailable".to_string())?;
    main.unminimize().map_err(|error| error.to_string())?;
    clamp_offscreen_geometry(&main).map_err(|error| error.to_string())?;
    main.show().map_err(|error| error.to_string())?;
    main.set_focus().map_err(|error| error.to_string())?;
    if let Some(kind) = edit_kind {
        let _ = app.emit_to("main", "agent-desk://open-sticky-editor", kind);
    }
    Ok(preferences)
}

#[tauri::command]
pub fn start_desktop_sticky_drag(window: WebviewWindow) -> Result<(), String> {
    if window.label() != DESKTOP_STICKY_WINDOW_LABEL {
        return Err("desktop drag is only available to the Desktop Sticky window".to_string());
    }
    window.start_dragging().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn show_desktop_sticky_menu(app: AppHandle, window: WebviewWindow) -> Result<(), String> {
    if window.label() != DESKTOP_STICKY_WINDOW_LABEL {
        return Err("Desktop Sticky menu is unavailable to this window".to_string());
    }
    let open = MenuItem::with_id(
        &app,
        DESKTOP_MENU_OPEN,
        "打开 Agent Desk",
        true,
        None::<&str>,
    )
    .map_err(|error| error.to_string())?;
    let todo = MenuItem::with_id(&app, DESKTOP_MENU_TODO, "待办", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let record = MenuItem::with_id(&app, DESKTOP_MENU_RECORD, "记录", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let quote = MenuItem::with_id(&app, DESKTOP_MENU_QUOTE, "便签一句", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let switch = Submenu::with_items(&app, "切换 Sticky 类型", true, &[&todo, &record, &quote])
        .map_err(|error| error.to_string())?;
    let separator = PredefinedMenuItem::separator(&app).map_err(|error| error.to_string())?;
    let hide = MenuItem::with_id(&app, DESKTOP_MENU_HIDE, "隐藏", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let quit = MenuItem::with_id(&app, DESKTOP_MENU_QUIT, "退出", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let menu = Menu::with_items(&app, &[&open, &switch, &separator, &hide, &quit])
        .map_err(|error| error.to_string())?;
    window.popup_menu(&menu).map_err(|error| error.to_string())
}

pub fn handle_desktop_menu_event(app: &AppHandle, event: &MenuEvent) {
    match event.id().as_ref() {
        DESKTOP_MENU_OPEN => surface_main_window(app),
        DESKTOP_MENU_TODO => {
            let _ = crate::preferences::set_desktop_sticky_kind(app, DesktopStickyKind::Todo);
        }
        DESKTOP_MENU_RECORD => {
            let _ = crate::preferences::set_desktop_sticky_kind(app, DesktopStickyKind::Record);
        }
        DESKTOP_MENU_QUOTE => {
            let _ = crate::preferences::set_desktop_sticky_kind(app, DesktopStickyKind::Quote);
        }
        DESKTOP_MENU_HIDE => {
            if let Some(window) = app.get_webview_window(DESKTOP_STICKY_WINDOW_LABEL) {
                let _ = window.hide();
            }
        }
        DESKTOP_MENU_QUIT => {
            let _ = app.save_window_state(StateFlags::POSITION | StateFlags::SIZE);
            app.exit(0);
        }
        _ => {}
    }
}

pub fn schedule_desktop_position_save(window: &tauri::Window, expected: PhysicalPosition<i32>) {
    let app = window.app_handle().clone();
    let window = window.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        if window.outer_position().ok() != Some(expected) {
            return;
        }
        let screen = window
            .current_monitor()
            .ok()
            .flatten()
            .and_then(|monitor| monitor.name().cloned());
        let _ = crate::preferences::save_desktop_sticky_position(
            &app,
            DesktopStickyPosition {
                x: expected.x,
                y: expected.y,
                screen,
            },
        );
    });
}

pub fn restore_main_window(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };

    let flags = StateFlags::POSITION | StateFlags::SIZE;
    let _ = window.restore_state(flags);
    clamp_offscreen_geometry(&window)?;
    window.show()
}

fn clamp_offscreen_geometry(window: &WebviewWindow) -> tauri::Result<()> {
    let position = window.outer_position()?;
    let size = window.outer_size()?;
    let geometry = Rect {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    };
    let monitors = window.available_monitors()?;

    if monitors.iter().any(|monitor| {
        let monitor_position = monitor.position();
        let monitor_size = monitor.size();
        geometry.has_visible_edge_on(Rect {
            x: monitor_position.x,
            y: monitor_position.y,
            width: monitor_size.width,
            height: monitor_size.height,
        })
    }) {
        return Ok(());
    }

    let target = window
        .primary_monitor()?
        .or_else(|| monitors.into_iter().next());
    if let Some(monitor) = target {
        let monitor_position = monitor.position();
        let monitor_size = monitor.size();
        let target_position = geometry.clamped_origin(Rect {
            x: monitor_position.x,
            y: monitor_position.y,
            width: monitor_size.width,
            height: monitor_size.height,
        });
        window.set_position(target_position)?;
    }

    Ok(())
}

pub fn clamp_window_to_monitor(window: &WebviewWindow) -> tauri::Result<()> {
    let position = window.outer_position()?;
    let size = window.outer_size()?;
    let target = window.current_monitor()?.or(window.primary_monitor()?);
    if let Some(monitor) = target {
        let monitor_position = monitor.position();
        let monitor_size = monitor.size();
        let target_position = Rect {
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
        }
        .clamped_origin(Rect {
            x: monitor_position.x,
            y: monitor_position.y,
            width: monitor_size.width,
            height: monitor_size.height,
        });
        window.set_position(target_position)?;
    }
    Ok(())
}

pub fn setup_tray(app: &App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show Agent Desk", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Agent Desk", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let mut builder = TrayIconBuilder::with_id("agent-desk-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("Agent Desk")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => surface_main_window(app),
            "quit" => {
                let _ = app.save_window_state(StateFlags::POSITION | StateFlags::SIZE);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                surface_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_geometry_with_a_visible_intersection() {
        let window = Rect {
            x: 950,
            y: 50,
            width: 320,
            height: 420,
        };
        let monitor = Rect {
            x: 0,
            y: 0,
            width: 1024,
            height: 768,
        };

        assert!(window.has_visible_edge_on(monitor));
    }

    #[test]
    fn rejects_fully_offscreen_geometry() {
        let window = Rect {
            x: 3000,
            y: 2000,
            width: 320,
            height: 420,
        };
        let monitor = Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };

        assert!(!window.has_visible_edge_on(monitor));
        assert_eq!(
            window.clamped_origin(monitor),
            PhysicalPosition::new(1600, 660)
        );
    }
}
