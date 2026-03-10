use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, Manager, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_positioner::{Position, WindowExt};

const WINDOW_LABEL: &str = "main";
const WINDOW_WIDTH: f64 = 360.0;
const WINDOW_HEIGHT: f64 = 500.0;

pub fn create_tray(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .icon_as_template(true)
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

            // Hide when window loses focus
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::Focused(false) = event {
                    let _ = window_clone.hide();
                }
            });
        }
    }
}
