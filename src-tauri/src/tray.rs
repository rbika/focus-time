use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::image::Image;
use tauri::menu::{ContextMenu, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Position, Size, WebviewWindow, Wry,
};

use crate::app_state::AppState;
use crate::persistence::WindowPosition;
use crate::timer::{TimerMode, TimerSnapshot, TimerStatus};

const TRAY_ID: &str = "main";
const SIBLING_WINDOW_LABELS: &[&str] = &[
    "settings",
    "up-to-date",
    "update-available",
    "update-progress",
];
const TRAY_ICON_STOPWATCH_BYTES: &[u8] = include_bytes!("../icons/tray-stopwatch.png");
const TRAY_ICON_TIMER_BYTES: &[u8] = include_bytes!("../icons/tray-hourglass.png");
const TRAY_ICON_POMODORO_BYTES: &[u8] = include_bytes!("../icons/tray-tomato.png");
const TRAY_INACTIVE_OPACITY: f64 = 0.4;
const TRAY_OPACITY_DURATION: f64 = 0.2;

const CLICK_GUARD: Duration = Duration::from_millis(300);

static LAST_TRAY_TOGGLE: Mutex<Option<Instant>> = Mutex::new(None);
static LAST_MAIN_WINDOW_HIDE: Mutex<Option<Instant>> = Mutex::new(None);
static LAST_TRAY_DIMMED: Mutex<Option<bool>> = Mutex::new(None);
static LAST_TRAY_MODE: Mutex<Option<TimerMode>> = Mutex::new(None);

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let mode = app
        .state::<AppState>()
        .engine
        .lock()
        .expect("engine lock")
        .mode();
    *LAST_TRAY_MODE.lock().expect("tray mode lock") = Some(mode);
    let icon = tray_icon_for_mode(mode);

    let initial_title = display_title(app);

    let tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(true)
        .title(initial_title)
        .tooltip("Focus Time")
        .on_menu_event(|app, event| {
            handle_menu_event(app, event.id().as_ref());
        })
        // Deliberately never attached via `.menu()`/`set_menu()`: once a menu is
        // attached to the NSStatusItem, macOS treats *any* click (left or right)
        // as "show the menu" (this became true for left clicks too as of macOS 27),
        // so both buttons would open the menu instead of only the right one. Instead
        // we build the menu fresh and pop it up manually on right click only.
        .on_tray_icon_event(|tray, event| {
            let TrayIconEvent::Click {
                button,
                button_state: MouseButtonState::Up,
                ..
            } = event
            else {
                return;
            };

            match button {
                MouseButton::Left => toggle_main_window_from_tray(tray.app_handle()),
                MouseButton::Right => show_tray_menu_deferred(tray.app_handle()),
                MouseButton::Middle => {}
            }
        })
        .build(app)?;

    configure_status_button(&tray, tray_is_dimmed(app));

    Ok(())
}

fn tray_icon_for_mode(mode: TimerMode) -> Image<'static> {
    let bytes = match mode {
        TimerMode::Timer => TRAY_ICON_TIMER_BYTES,
        TimerMode::Stopwatch => TRAY_ICON_STOPWATCH_BYTES,
        TimerMode::Pomodoro => TRAY_ICON_POMODORO_BYTES,
    };
    Image::from_bytes(bytes).expect("valid tray icon png")
}

pub fn update_tray_icon(app: &AppHandle) {
    let mode = app
        .state::<AppState>()
        .engine
        .lock()
        .expect("engine lock")
        .mode();

    {
        let mut last = LAST_TRAY_MODE.lock().expect("tray mode lock");
        if *last == Some(mode) {
            return;
        }
        *last = Some(mode);
    }

    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let icon = tray_icon_for_mode(mode);
        let _ = tray.set_icon(Some(icon));
        let _ = tray.set_icon_as_template(true);
    }
}

fn toggle_main_window_from_tray(app: &AppHandle) {
    if is_duplicate_toggle() {
        return;
    }

    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let visible = window.is_visible().unwrap_or(false);
    if visible {
        hide_main_window(app);
    } else if !is_recent_main_window_hide() {
        let _ = crate::commands::show_timer_window(app.clone());
    }
}

/// The popup's modal tracking loop blocks until the menu closes. Run inside
/// tao's event callback, it stalls all main-thread dispatch (tray title,
/// `timer-tick`) for that whole time, so it must run from the GCD main queue.
#[cfg(target_os = "macos")]
fn show_tray_menu_deferred(app: &AppHandle) {
    let app = app.clone();
    dispatch2::DispatchQueue::main().exec_async(move || show_tray_menu(&app));
}

#[cfg(not(target_os = "macos"))]
fn show_tray_menu_deferred(app: &AppHandle) {
    show_tray_menu(app);
}

fn show_tray_menu(app: &AppHandle) {
    let Some(window) = app.get_window("main") else {
        return;
    };
    let Ok(menu) = build_menu(app) else {
        return;
    };
    let _ = menu.popup(window);
    unhighlight_status_button(app);
}

#[cfg(target_os = "macos")]
fn unhighlight_status_button(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let _ = tray.with_inner_tray_icon(|inner| {
        let Some(status_item) = inner.ns_status_item() else {
            return;
        };
        let Some(mtm) = objc2::MainThreadMarker::new() else {
            return;
        };
        if let Some(button) = status_item.button(mtm) {
            button.highlight(false);
        }
    });
}

#[cfg(not(target_os = "macos"))]
fn unhighlight_status_button(_: &AppHandle) {}

fn is_recent_main_window_hide() -> bool {
    let Ok(last_hide) = LAST_MAIN_WINDOW_HIDE.lock() else {
        return false;
    };
    last_hide.is_some_and(|at| at.elapsed() < CLICK_GUARD)
}

fn is_duplicate_toggle() -> bool {
    let Ok(mut last_toggle) = LAST_TRAY_TOGGLE.lock() else {
        return false;
    };
    if last_toggle.is_some_and(|at| at.elapsed() < CLICK_GUARD) {
        return true;
    }
    *last_toggle = Some(Instant::now());
    false
}

pub(crate) fn tray_title(title: &str) -> &str {
    title.strip_prefix("00:").unwrap_or(title)
}

/// Computes the text that should currently be shown next to the tray icon,
/// honoring "icon only" mode by returning an empty string.
fn display_title(app: &AppHandle) -> String {
    let state = app.state::<AppState>();
    let icon_only = state.settings.lock().expect("settings lock").icon_only;
    if icon_only {
        String::new()
    } else {
        let formatted = state.snapshot().formatted;
        tray_title(&formatted).to_string()
    }
}

fn tray_is_dimmed(app: &AppHandle) -> bool {
    app.state::<AppState>()
        .engine
        .lock()
        .expect("engine lock")
        .status()
        != TimerStatus::Running
}

#[cfg(target_os = "macos")]
fn configure_status_button(tray: &tauri::tray::TrayIcon, dimmed: bool) {
    let _ = tray.with_inner_tray_icon(move |inner| {
        let Some(status_item) = inner.ns_status_item() else {
            return;
        };
        let Some(mtm) = objc2::MainThreadMarker::new() else {
            return;
        };

        if let Some(button) = status_item.button(mtm) {
            let font = objc2_app_kit::NSFont::monospacedDigitSystemFontOfSize_weight(
                objc2_app_kit::NSFont::systemFontSize(),
                0.0,
            );
            button.setFont(Some(&font));
            set_status_button_opacity(&button, dimmed, false);
        }
    });
    *LAST_TRAY_DIMMED.lock().expect("tray dimmed lock") = Some(dimmed);
}

fn update_status_item_opacity(tray: &tauri::tray::TrayIcon, dimmed: bool) {
    {
        let mut last = LAST_TRAY_DIMMED.lock().expect("tray dimmed lock");
        if *last == Some(dimmed) {
            return;
        }
        *last = Some(dimmed);
    }

    #[cfg(target_os = "macos")]
    {
        let _ = tray.with_inner_tray_icon(move |inner| {
            let Some(status_item) = inner.ns_status_item() else {
                return;
            };
            let Some(mtm) = objc2::MainThreadMarker::new() else {
                return;
            };
            if let Some(button) = status_item.button(mtm) {
                set_status_button_opacity(&button, dimmed, true);
            }
        });
    }
}

#[cfg(target_os = "macos")]
fn set_status_button_opacity(
    button: &objc2_app_kit::NSStatusBarButton,
    dimmed: bool,
    animate: bool,
) {
    use objc2_app_kit::{NSAnimatablePropertyContainer, NSAnimationContext};

    let opacity = if dimmed { TRAY_INACTIVE_OPACITY } else { 1.0 };
    if animate {
        NSAnimationContext::beginGrouping();
        NSAnimationContext::currentContext().setDuration(TRAY_OPACITY_DURATION);
        button.animator().setAlphaValue(opacity);
        NSAnimationContext::endGrouping();
    } else {
        button.setAlphaValue(opacity);
    }
}

#[cfg(not(target_os = "macos"))]
fn configure_status_button(_: &tauri::tray::TrayIcon, _: bool) {}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let state = app.state::<AppState>();
    let (status, duration_secs, mode) = {
        let engine = state.engine.lock().expect("engine lock");
        (engine.status(), engine.duration_secs(), engine.mode())
    };

    let pause_label = match status {
        TimerStatus::Running => "Pause",
        TimerStatus::Paused => "Resume",
        TimerStatus::Idle | TimerStatus::Completed => "Start",
    };
    let start_enabled = match status {
        TimerStatus::Idle | TimerStatus::Completed => match mode {
            TimerMode::Stopwatch => true,
            TimerMode::Timer | TimerMode::Pomodoro => duration_secs > 0,
        },
        TimerStatus::Running | TimerStatus::Paused => true,
    };
    let pause = MenuItem::with_id(
        app,
        "toggle_pause",
        pause_label,
        start_enabled,
        None::<&str>,
    )?;
    let icon_only = state.settings.lock().expect("settings lock").icon_only;
    let icon_only_label = if icon_only {
        "Icon only: on"
    } else {
        "Icon only: off"
    };
    let icon_only_item =
        MenuItem::with_id(app, "toggle_icon_only", icon_only_label, true, None::<&str>)?;

    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Focus Time", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;

    Menu::with_items(
        app,
        &[&pause, &sep, &icon_only_item, &settings, &sep, &quit],
    )
}

pub fn refresh_tray_menu(app: &AppHandle) {
    let dimmed = tray_is_dimmed(app);
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        update_status_item_opacity(&tray, dimmed);
    }
}

pub fn update_tray_title(app: &AppHandle, title: &str) {
    let state = app.state::<AppState>();
    let icon_only = state.settings.lock().expect("settings lock").icon_only;
    let display = if icon_only {
        String::new()
    } else {
        tray_title(title).to_string()
    };

    let mut last = state.last_tray_title.lock().expect("tray title lock");
    if *last == display {
        return;
    }
    *last = display.clone();
    drop(last);
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_title(Some(display));
    }
}

fn handle_menu_event(app: &AppHandle, id: &str) {
    match id {
        "toggle_pause" => {
            let _ = crate::commands::toggle_pause(app.clone());
        }
        "toggle_icon_only" => {
            let _ = crate::commands::toggle_icon_only(app.clone());
        }
        "settings" => {
            let _ = crate::commands::open_settings(app.clone());
        }
        "quit" => {
            crate::commands::quit_app(app.clone());
        }
        _ => {}
    }
}

pub fn emit_tick_if_visible(app: &AppHandle, snapshot: &TimerSnapshot) {
    let visible = app
        .get_webview_window("main")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);
    if visible {
        let _ = app.emit("timer-tick", snapshot);
    }
}

pub fn position_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    let saved = *app
        .state::<AppState>()
        .main_window_position
        .lock()
        .expect("main window position lock");

    let position = saved
        .filter(|position| position_is_visible(&window, *position))
        .or_else(|| default_position_below_tray(app, &window));

    if let Some(position) = position {
        let _ = window.set_position(PhysicalPosition::new(position.x, position.y));
        *app.state::<AppState>()
            .main_window_position
            .lock()
            .expect("main window position lock") = Some(position);
    }
}

pub fn save_main_window_position(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let Ok(position) = window.outer_position() else {
        return;
    };

    let position = WindowPosition {
        x: position.x,
        y: position.y,
    };
    let state = app.state::<AppState>();
    let changed = {
        let mut current = state
            .main_window_position
            .lock()
            .expect("main window position lock");
        if *current == Some(position) {
            false
        } else {
            *current = Some(position);
            true
        }
    };

    if changed {
        let _ = state.persist();
    }
}

pub fn any_sibling_window_visible(app: &AppHandle) -> bool {
    SIBLING_WINDOW_LABELS.iter().any(|label| {
        app.get_webview_window(label)
            .and_then(|window| window.is_visible().ok())
            .unwrap_or(false)
    })
}

pub fn hide_main_window(app: &AppHandle) {
    save_main_window_position(app);
    let mut was_visible = false;
    if let Some(window) = app.get_webview_window("main") {
        was_visible = window.is_visible().unwrap_or(false);
        if was_visible {
            *LAST_MAIN_WINDOW_HIDE.lock().expect("main window hide lock") = Some(Instant::now());
        }
        let _ = window.hide();
    }
    if was_visible {
        let _ = app.emit("main-window-hidden", ());
    }
}

fn position_is_visible(window: &WebviewWindow, position: WindowPosition) -> bool {
    let Ok(size) = window.outer_size() else {
        return false;
    };
    let center_x = position.x as i64 + i64::from(size.width) / 2;
    let center_y = position.y as i64 + i64::from(size.height) / 2;

    window.available_monitors().is_ok_and(|monitors| {
        monitors.iter().any(|monitor| {
            let origin = monitor.position();
            let size = monitor.size();
            center_x >= i64::from(origin.x)
                && center_x < i64::from(origin.x) + i64::from(size.width)
                && center_y >= i64::from(origin.y)
                && center_y < i64::from(origin.y) + i64::from(size.height)
        })
    })
}

fn default_position_below_tray(app: &AppHandle, window: &WebviewWindow) -> Option<WindowPosition> {
    let tray_rect = app.tray_by_id(TRAY_ID)?.rect().ok()??;
    let scale = window.scale_factor().unwrap_or(1.0);
    let tray_position: PhysicalPosition<f64> = match tray_rect.position {
        Position::Physical(position) => position.cast(),
        Position::Logical(position) => position.to_physical(scale),
    };
    let tray_size: PhysicalSize<f64> = match tray_rect.size {
        Size::Physical(size) => size.cast(),
        Size::Logical(size) => size.to_physical(scale),
    };
    let window_size = window.outer_size().ok()?;
    let tray_center_x = tray_position.x + tray_size.width / 2.0;

    let monitor = window
        .available_monitors()
        .ok()?
        .into_iter()
        .find(|monitor| {
            let origin = monitor.position();
            let size = monitor.size();
            tray_center_x >= f64::from(origin.x)
                && tray_center_x < f64::from(origin.x) + f64::from(size.width)
        })?;
    let monitor_origin = monitor.position();
    let monitor_size = monitor.size();
    let min_x = monitor_origin.x;
    let max_x = monitor_origin.x + monitor_size.width as i32 - window_size.width as i32;
    let centered_x = (tray_center_x - f64::from(window_size.width) / 2.0).round() as i32;

    Some(WindowPosition {
        x: centered_x.clamp(min_x, max_x.max(min_x)),
        y: (tray_position.y + tray_size.height + 4.0).round() as i32,
    })
}

#[cfg(test)]
mod tests {
    use super::tray_title;

    #[test]
    fn hides_zero_hours() {
        assert_eq!(tray_title("00:25:00"), "25:00");
    }

    #[test]
    fn keeps_nonzero_hours() {
        assert_eq!(tray_title("01:25:00"), "01:25:00");
    }
}
