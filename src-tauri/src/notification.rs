use std::path::Path;
use std::sync::OnceLock;

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

/// Install the click delegate after AppState and window position exist.
/// `APP_HANDLE` is set first so a leftover banner that launched the app —
/// delivered when the delegate is attached — can open the Timer window.
pub fn install(app: &AppHandle) {
    let _ = APP_HANDLE.set(app.clone());

    #[cfg(target_os = "macos")]
    macos::install_delegate();
}

pub fn request_authorization() {
    #[cfg(target_os = "macos")]
    on_main(macos::request_authorization);
}

/// Show a native macOS notification when a focus session ends.
pub fn show_timer_finished(app: &AppHandle) {
    show(app, "Your focus session has finished.");
}

/// Show a native macOS notification when a Break ends.
pub fn show_break_finished(app: &AppHandle) {
    show(app, "Your break has finished.");
}

fn show(app: &AppHandle, body: &str) {
    request_authorization();

    #[cfg(target_os = "macos")]
    {
        let app = app.clone();
        let body = body.to_string();
        on_main(move || {
            if !macos::show(&app, "Focus Time", &body) {
                plugin_show(&app, &body);
            }
        });
    }

    #[cfg(not(target_os = "macos"))]
    plugin_show(app, body);
}

fn plugin_show(app: &AppHandle, body: &str) {
    let _ = app
        .notification()
        .builder()
        .title("Focus Time")
        .body(body)
        .show();
}

fn open_timer_window() {
    let Some(app) = APP_HANDLE.get().cloned() else {
        return;
    };
    on_main(move || {
        let _ = crate::commands::show_timer_window(app);
    });
}

fn on_main<F>(work: F)
where
    F: FnOnce() + Send + 'static,
{
    #[cfg(target_os = "macos")]
    dispatch2::DispatchQueue::main().exec_async(work);
    #[cfg(not(target_os = "macos"))]
    work();
}

/// `UNUserNotificationCenter` throws (and aborts the process) unless the main
/// bundle is a `.app`. `tauri dev` runs the bare binary but still embeds an
/// `Info.plist`, so a bundle identifier alone is not proof of a bundle.
fn is_app_bundle_path(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "app")
}

#[cfg(target_os = "macos")]
mod macos {
    use std::path::PathBuf;

    use block2::{DynBlock, RcBlock};
    use objc2::rc::Retained;
    use objc2::runtime::{NSObject, NSObjectProtocol, ProtocolObject};
    use objc2::{define_class, msg_send, AllocAnyThread};
    use objc2_foundation::{NSBundle, NSError, NSString};
    use objc2_user_notifications::{
        UNAuthorizationOptions, UNMutableNotificationContent, UNNotification,
        UNNotificationPresentationOptions, UNNotificationRequest, UNNotificationResponse,
        UNUserNotificationCenter, UNUserNotificationCenterDelegate,
    };

    const DEFAULT_ACTION: &str = "com.apple.UNNotificationDefaultActionIdentifier";

    define_class!(
        #[unsafe(super = NSObject)]
        #[name = "FTNotificationDelegate"]
        struct NotificationDelegate;

        unsafe impl NSObjectProtocol for NotificationDelegate {}

        unsafe impl UNUserNotificationCenterDelegate for NotificationDelegate {
            #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
            unsafe fn will_present(
                &self,
                _center: &UNUserNotificationCenter,
                _notification: &UNNotification,
                completion_handler: &DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
            ) {
                let options = UNNotificationPresentationOptions::List
                    | UNNotificationPresentationOptions::Banner;
                completion_handler.call((options,));
            }

            #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
            unsafe fn did_receive(
                &self,
                _center: &UNUserNotificationCenter,
                response: &UNNotificationResponse,
                completion_handler: &DynBlock<dyn Fn()>,
            ) {
                if response.actionIdentifier().to_string() == DEFAULT_ACTION {
                    super::open_timer_window();
                }
                completion_handler.call(());
            }
        }
    );

    impl NotificationDelegate {
        fn new() -> Retained<Self> {
            let this = Self::alloc().set_ivars(());
            unsafe { msg_send![super(this), init] }
        }
    }

    fn center() -> Option<Retained<UNUserNotificationCenter>> {
        let path = PathBuf::from(NSBundle::mainBundle().bundlePath().to_string());
        super::is_app_bundle_path(&path).then(UNUserNotificationCenter::currentNotificationCenter)
    }

    pub(super) fn install_delegate() {
        let Some(center) = center() else {
            return;
        };
        let delegate = NotificationDelegate::new();
        let proto = ProtocolObject::from_retained(delegate.clone());
        center.setDelegate(Some(&proto));
        // The center keeps only a weak reference.
        let _ = Retained::into_raw(delegate);
    }

    pub(super) fn request_authorization() {
        let Some(center) = center() else {
            return;
        };
        let handler = RcBlock::new(|_granted: objc2::runtime::Bool, _error: *mut NSError| {});
        center.requestAuthorizationWithOptions_completionHandler(
            UNAuthorizationOptions::Alert,
            &handler,
        );
    }

    /// Returns whether the request was handed to `UNUserNotificationCenter`;
    /// a later delivery failure falls back to the plugin banner.
    pub(super) fn show(app: &super::AppHandle, title: &str, body: &str) -> bool {
        let Some(center) = center() else {
            return false;
        };

        let content = UNMutableNotificationContent::new();
        content.setTitle(&NSString::from_str(title));
        content.setBody(&NSString::from_str(body));

        let identifier = uuid::Uuid::new_v4().to_string();
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &NSString::from_str(&identifier),
            &content,
            None,
        );

        let app = app.clone();
        let body = body.to_string();
        let handler = RcBlock::new(move |error: *mut NSError| {
            if error.is_null() {
                return;
            }
            super::plugin_show(&app, &body);
        });
        center.addNotificationRequest_withCompletionHandler(&request, Some(&handler));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_bundle_path_is_detected() {
        assert!(is_app_bundle_path(Path::new(
            "/Applications/Focus Time.app"
        )));
    }

    #[test]
    fn dev_binary_directory_is_not_an_app_bundle() {
        assert!(!is_app_bundle_path(Path::new(
            "/Users/me/focus-timer/src-tauri/target/debug"
        )));
    }

    #[test]
    fn app_suffix_in_name_without_extension_is_not_a_bundle() {
        assert!(!is_app_bundle_path(Path::new("/Users/me/myapp")));
    }
}
