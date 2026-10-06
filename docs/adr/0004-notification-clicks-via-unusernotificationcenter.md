# Notification clicks go through UNUserNotificationCenter

`tauri-plugin-notification` can show a desktop banner but has no click callback. This app is an Accessory menu bar process, so a banner click would otherwise activate nothing visible.

Completion notifications are delivered through `UNUserNotificationCenter`. The delegate always shows and focuses the Timer window (last view kept, never a tray toggle), including a leftover banner after quit.

`UNUserNotificationCenter.currentNotificationCenter` throws `NSInternalInconsistencyException` (`bundleProxyForCurrentProcess is nil`) when the process is not inside a `.app` bundle — which is how `tauri dev` runs. A bundle identifier is not a sufficient guard: Tauri embeds `Info.plist` in the dev binary, so the identifier exists without a bundle. The UN path is used only when `NSBundle.mainBundle.bundlePath` ends in `.app`; otherwise, or when `addNotificationRequest` fails, the plugin shows an unclickable banner. Clicks are verified on a bundled build (`tauri build --debug`), not in `tauri dev`.

**Considered Options**: keep fire-and-forget plugin notifications (no click); `mac-notification-sys` `wait_for_click` on a background thread (deprecated `NSUserNotification`, running-app clicks only); treating any app activation with no visible window as a click (fragile — other activations would open the window too).
