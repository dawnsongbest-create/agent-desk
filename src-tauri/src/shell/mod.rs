use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Manager, PhysicalPosition, WebviewWindow,
};
use tauri_plugin_window_state::{AppHandleExt, StateFlags, WindowExt};

const MIN_VISIBLE_EDGE: i32 = 64;

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
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = clamp_offscreen_geometry(&window);
        let _ = window.show();
        let _ = window.set_focus();
    }
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
