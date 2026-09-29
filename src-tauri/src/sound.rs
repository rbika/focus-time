use std::path::Path;

use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager};

/// Sentinel value for "no completion sound" (shown as "None" in settings).
pub const NO_COMPLETION_SOUND: &str = "None";

/// Bundled completion sounds shown in settings.
pub const COMPLETION_SOUNDS: &[&str] = &["Door Bell", "Hello", "Reward", "Microwave"];

fn sound_resource_path(name: &str) -> Option<&'static str> {
    match name {
        "Door Bell" => Some("sounds/door-bell.mp3"),
        "Hello" => Some("sounds/hello.mp3"),
        "Reward" => Some("sounds/reward.mp3"),
        "Microwave" => Some("sounds/microwave.mp3"),
        _ => None,
    }
}

pub fn is_completion_sound_enabled(name: &str) -> bool {
    !name.is_empty() && name != NO_COMPLETION_SOUND
}

pub fn is_valid_completion_sound(name: &str) -> bool {
    sound_resource_path(name).is_some()
}

/// Maps persisted values to a bundled sound, defaulting unknown names to the first bundled sound.
pub fn normalize_completion_sound(name: &str) -> String {
    if !is_completion_sound_enabled(name) {
        return NO_COMPLETION_SOUND.to_string();
    }
    if is_valid_completion_sound(name) {
        return name.to_string();
    }
    COMPLETION_SOUNDS[0].to_string()
}

/// Play a bundled completion sound by display name.
#[cfg(target_os = "macos")]
pub fn play_named_sound(app: &AppHandle, name: &str) {
    if !is_completion_sound_enabled(name) {
        return;
    }

    let Some(resource_path) = sound_resource_path(name) else {
        return;
    };

    let Ok(path) = app.path().resolve(resource_path, BaseDirectory::Resource) else {
        return;
    };

    if !Path::new(&path).exists() {
        return;
    }

    use objc2::AllocAnyThread;
    use objc2_app_kit::NSSound;
    use objc2_foundation::NSString;

    let path_ns = NSString::from_str(&path.to_string_lossy());
    if let Some(sound) =
        NSSound::initWithContentsOfFile_byReference(NSSound::alloc(), &path_ns, false)
    {
        let _ = sound.play();
    }
}

#[cfg(not(target_os = "macos"))]
pub fn play_named_sound(_app: &AppHandle, _name: &str) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_catalog_includes_microwave_last() {
        assert_eq!(COMPLETION_SOUNDS.last().copied(), Some("Microwave"));
        assert_eq!(normalize_completion_sound("Microwave"), "Microwave");
    }

    #[test]
    fn normalize_completion_sound_maps_unknown_to_first_bundled() {
        assert_eq!(normalize_completion_sound("Glass"), "Door Bell");
        assert_eq!(normalize_completion_sound("Bell"), "Door Bell");
        assert_eq!(normalize_completion_sound("Door Bell"), "Door Bell");
        assert_eq!(normalize_completion_sound("Hello"), "Hello");
        assert_eq!(normalize_completion_sound("Reward"), "Reward");
        assert_eq!(normalize_completion_sound("Xmas"), "Door Bell");
        assert_eq!(normalize_completion_sound("None"), NO_COMPLETION_SOUND);
    }
}
