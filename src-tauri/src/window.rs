use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, WebviewWindow};

#[cfg(target_os = "macos")]
use objc2_app_kit::NSMenu;

/// Dev-only pin for the main window.
///
/// - Release builds always return `false` (shipping apps never pin).
/// - Debug builds honor `ALWAYS_ON_TOP` from the process env / `.env` files.
pub fn always_on_top_enabled() -> bool {
    if !cfg!(debug_assertions) {
        return false;
    }
    match std::env::var("ALWAYS_ON_TOP") {
        Ok(value) => matches!(
            value.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => false,
    }
}

pub fn apply_dev_always_on_top(window: &WebviewWindow) {
    if always_on_top_enabled() {
        let _ = window.set_always_on_top(true);
    }
}

/// Hands keyboard focus to the webview.
///
/// Every window here is declared `"visible": false`, so AppKit never makes it
/// key at creation time and never assigns it a first responder — key events
/// reach the window but nothing in it, and the webview stays deaf until the
/// user clicks. A window keeps its first responder across hide/show, so this
/// only has to win on the first show.
#[cfg(target_os = "macos")]
pub fn focus_webview(window: &WebviewWindow) {
    let _ = window.with_webview(|webview| {
        use objc2_app_kit::{NSResponder, NSView, NSWindow};

        if objc2::MainThreadMarker::new().is_none() {
            return;
        }
        let ns_window = webview.ns_window() as *mut NSWindow;
        let view = webview.inner() as *mut NSView;
        if ns_window.is_null() || view.is_null() {
            return;
        }
        unsafe {
            let responder: &NSResponder = &*view;
            (*ns_window).makeFirstResponder(Some(responder));
        }
    });
}

#[cfg(not(target_os = "macos"))]
pub fn focus_webview(_window: &WebviewWindow) {}

/// Drops the ⌘X key equivalent from Edit → Cut.
///
/// Tauri's default app menu binds Cut to ⌘X. AppKit answers that in
/// `performKeyEquivalent:` before the event ever reaches the webview, so the
/// timer's cancel shortcut (also ⌘X) never sees a `keydown`. Cut still works
/// from the menu and from text fields, where the webview handles it itself
/// once the key is no longer claimed by the menu.
#[cfg(target_os = "macos")]
pub fn unbind_cut_key_equivalent() {
    use objc2_app_kit::NSApplication;

    let Some(mtm) = objc2::MainThreadMarker::new() else {
        return;
    };
    let Some(menu) = NSApplication::sharedApplication(mtm).mainMenu() else {
        return;
    };
    clear_cut_key_equivalent(&menu);
}

#[cfg(target_os = "macos")]
fn clear_cut_key_equivalent(menu: &NSMenu) {
    use objc2::sel;
    use objc2_foundation::NSString;

    let cut = sel!(cut:);
    let count = menu.numberOfItems();
    for index in 0..count {
        let Some(item) = menu.itemAtIndex(index) else {
            continue;
        };
        if item.action() == Some(cut) {
            item.setKeyEquivalent(&NSString::new());
        }
        if let Some(submenu) = item.submenu() {
            clear_cut_key_equivalent(&submenu);
        }
    }
}

/// Which content the main window is currently showing — drives the
/// animated resize between the compact Timer footprint and the larger
/// Stats view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MainView {
    Timer,
    Stats,
}

const TIMER_SIZE: (f64, f64) = (290.0, 230.0);
const STATS_SIZE: (f64, f64) = (290.0, 320.0);
const RESIZE_DURATION: f64 = 0.2;

#[tauri::command]
pub fn resize_main_window(app: AppHandle, view: MainView) -> Result<(), String> {
    let (width, height) = match view {
        MainView::Timer => TIMER_SIZE,
        MainView::Stats => STATS_SIZE,
    };
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };
    animate_main_window_resize(&window, width, height);
    Ok(())
}

/// Resizes the window natively, keeping its top-left corner fixed on
/// screen (it grows/shrinks down and to the right), using the same
/// `NSAnimationContext` grouping + `.animator()` pattern `tray.rs` uses
/// for the tray icon fade.
#[cfg(target_os = "macos")]
fn animate_main_window_resize(window: &tauri::WebviewWindow, width: f64, height: f64) {
    use objc2_app_kit::{NSAnimatablePropertyContainer, NSAnimationContext, NSWindow};
    use objc2_foundation::{NSPoint, NSRect, NSSize};

    let Ok(ns_window_ptr) = window.ns_window() else {
        return;
    };
    if objc2::MainThreadMarker::new().is_none() {
        return;
    }
    let ns_window: &NSWindow = unsafe { &*(ns_window_ptr as *mut NSWindow) };

    let current = ns_window.frame();
    let new_frame = NSRect::new(
        NSPoint::new(
            current.origin.x,
            current.origin.y + current.size.height - height,
        ),
        NSSize::new(width, height),
    );

    NSAnimationContext::beginGrouping();
    NSAnimationContext::currentContext().setDuration(RESIZE_DURATION);
    ns_window.animator().setFrame_display(new_frame, true);
    NSAnimationContext::endGrouping();
}

#[cfg(not(target_os = "macos"))]
fn animate_main_window_resize(window: &tauri::WebviewWindow, width: f64, height: f64) {
    let _ = window.set_size(tauri::LogicalSize::new(width, height));
}
