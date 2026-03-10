use crate::state::AppState;
use tauri::{
    image::Image,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, Manager, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_positioner::{Position, WindowExt};

const WINDOW_LABEL: &str = "main";
const WINDOW_WIDTH: f64 = 360.0;
const WINDOW_HEIGHT: f64 = 500.0;

// All icons are white-on-transparent, used as NON-template (no runtime toggling).
const ICON_NORMAL: &[u8] = include_bytes!("../icons/tray-icon.png");
const ICON_DIM: &[u8] = include_bytes!("../icons/tray-icon-dim.png");
const ICON_BADGE: &[u8] = include_bytes!("../icons/tray-icon-badge.png");

fn set_icon(app_handle: &tauri::AppHandle, bytes: &[u8]) {
    if let Some(tray) = app_handle.tray_by_id("main") {
        if let Ok(icon) = Image::from_bytes(bytes) {
            let _ = tray.set_icon(Some(icon));
        }
    }
}

pub fn create_tray(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let icon = Image::from_bytes(ICON_NORMAL)?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(false)
        .tooltip("Latest - Update Checker")
        .on_tray_icon_event(|tray_handle, event| {
            tauri_plugin_positioner::on_tray_event(tray_handle.app_handle(), &event);

            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app_handle = tray_handle.app_handle();
                toggle_popup(app_handle);
            }
        })
        .build(app)?;

    Ok(())
}

/// Start or stop the blink animation during update checks.
pub fn set_tray_checking(app_handle: &tauri::AppHandle, checking: bool) {
    // Stop any existing blink task
    if let Some(state) = app_handle.try_state::<AppState>() {
        if let Ok(mut inner) = state.inner.lock() {
            if let Some(handle) = inner.blink_abort.take() {
                handle.abort();
            }
        }
    }

    if !checking {
        set_icon(app_handle, ICON_NORMAL);
        return;
    }

    // Pulse between bright and dim
    let handle = app_handle.clone();
    let task = tokio::spawn(async move {
        let mut bright = true;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(600)).await;
            bright = !bright;
            set_icon(&handle, if bright { ICON_NORMAL } else { ICON_DIM });
        }
    });

    if let Some(state) = app_handle.try_state::<AppState>() {
        if let Ok(mut inner) = state.inner.lock() {
            inner.blink_abort = Some(task.abort_handle());
        }
    }
}

/// Set the tray icon: red-dot badge when updates available, plain arrow otherwise.
pub fn set_tray_update_count(app_handle: &tauri::AppHandle, count: usize) {
    set_icon(
        app_handle,
        if count > 0 { ICON_BADGE } else { ICON_NORMAL },
    );

    if let Some(tray) = app_handle.tray_by_id("main") {
        let tooltip = if count > 0 {
            format!(
                "Latest - {} update{} available",
                count,
                if count == 1 { "" } else { "s" }
            )
        } else {
            "Latest - All up to date".to_string()
        };
        let _ = tray.set_tooltip(Some(&tooltip));
    }
}

fn toggle_popup(app_handle: &tauri::AppHandle) {
    if let Some(window) = app_handle.get_webview_window(WINDOW_LABEL) {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.move_window(Position::TrayBottomCenter);
            let _ = window.show();
            let _ = window.set_focus();
        }
    } else {
        let window = WebviewWindowBuilder::new(
            app_handle,
            WINDOW_LABEL,
            WebviewUrl::App("index.html".into()),
        )
        .title("Latest")
        .inner_size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .resizable(false)
        .visible(false)
        .decorations(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .build();

        if let Ok(window) = window {
            let _ = window.move_window(Position::TrayBottomCenter);
            let _ = window.show();
            let _ = window.set_focus();

            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::Focused(false) = event {
                    let w = window_clone.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(200));
                        if !w.is_focused().unwrap_or(true) {
                            let _ = w.hide();
                        }
                    });
                }
            });
        }
    }
}
