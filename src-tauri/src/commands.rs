use std::time::SystemTime;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::app_state::AppState;
use crate::persistence::Settings;
use crate::timer::{FinishedInterval, TimerMode, TimerSnapshot, TimerStatus};

/// If the engine just finalized a run, turn it into a persisted Entry
/// (when end is after start) and notify any open Stats view.
pub fn finalize_interval(app: &AppHandle, finished: Option<FinishedInterval>) {
    let Some(interval) = finished else {
        return;
    };
    let Some(entry) = crate::entries::entry_from_interval(interval) else {
        return;
    };
    let state = app.state::<AppState>();
    let appended = state
        .entries
        .lock()
        .expect("entries lock")
        .append(entry.clone());
    if appended.is_ok() {
        let _ = app.emit("entry-recorded", &entry);
    }
}

#[tauri::command]
pub fn get_snapshot(app: AppHandle) -> TimerSnapshot {
    app.state::<AppState>().snapshot()
}

#[tauri::command]
pub fn get_totals(app: AppHandle) -> crate::entries::Totals {
    let state = app.state::<AppState>();
    let week_start = state.settings.lock().expect("settings lock").week_start;
    let totals = state
        .entries
        .lock()
        .expect("entries lock")
        .totals(SystemTime::now(), week_start);
    totals
}

#[tauri::command]
pub fn get_entries(app: AppHandle) -> Vec<crate::entries::Entry> {
    app.state::<AppState>()
        .entries
        .lock()
        .expect("entries lock")
        .load_all_newest_first()
}

#[tauri::command]
pub fn create_entry(
    app: AppHandle,
    started_at_unix: u64,
    ended_at_unix: u64,
) -> Result<crate::entries::Entry, String> {
    let created = app
        .state::<AppState>()
        .entries
        .lock()
        .expect("entries lock")
        .create_manual(started_at_unix, ended_at_unix)?;
    let _ = app.emit("entries-changed", ());
    Ok(created)
}

#[tauri::command]
pub fn update_entry(
    app: AppHandle,
    id: String,
    started_at_unix: u64,
    ended_at_unix: u64,
) -> Result<crate::entries::Entry, String> {
    let updated = app
        .state::<AppState>()
        .entries
        .lock()
        .expect("entries lock")
        .update_times(&id, started_at_unix, ended_at_unix)?;
    let _ = app.emit("entries-changed", ());
    Ok(updated)
}

#[tauri::command]
pub fn delete_entry(app: AppHandle, id: String) -> Result<(), String> {
    app.state::<AppState>()
        .entries
        .lock()
        .expect("entries lock")
        .delete(&id)?;
    let _ = app.emit("entries-changed", ());
    Ok(())
}

#[tauri::command]
pub fn reset_statistics(app: AppHandle) -> Result<(), String> {
    app.state::<AppState>()
        .entries
        .lock()
        .expect("entries lock")
        .delete_all()?;
    let _ = app.emit("entries-changed", ());
    Ok(())
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    app.state::<AppState>()
        .settings
        .lock()
        .expect("settings lock")
        .clone()
}

#[tauri::command]
pub fn update_settings(app: AppHandle, settings: Settings) -> Result<Settings, String> {
    let state = app.state::<AppState>();

    let notifications_just_enabled = {
        let mut current = state.settings.lock().expect("settings lock");
        let was_enabled = current.notifications_enabled;
        *current = Settings {
            hide_window_on_start: settings.hide_window_on_start,
            pause_on_sleep: settings.pause_on_sleep,
            start_at_login: settings.start_at_login,
            notifications_enabled: settings.notifications_enabled,
            icon_only: settings.icon_only,
            completion_sound: settings.completion_sound,
            auto_check_for_updates: settings.auto_check_for_updates,
            presets: settings.presets,
            session_length_secs: settings.session_length_secs,
            short_break_length_secs: settings.short_break_length_secs,
            long_break_length_secs: settings.long_break_length_secs,
            sessions_until_long_break: crate::timer::PomodoroConfig::sessions_until_long_break(
                settings.sessions_until_long_break,
            ),
            auto_start_sessions: settings.auto_start_sessions,
            auto_start_breaks: settings.auto_start_breaks,
            week_start: settings.week_start,
        };
        let config = current.pomodoro_config();
        drop(current);
        let mut engine = state.engine.lock().expect("engine lock");
        engine.apply_idle_pomodoro_config(&config);
        settings.notifications_enabled && !was_enabled
    };

    // Autostart
    let autostart = app.autolaunch();
    if settings.start_at_login {
        let _ = autostart.enable();
    } else {
        let _ = autostart.disable();
    }

    state.persist()?;
    if notifications_just_enabled {
        crate::notification::request_authorization();
    }
    let saved = state.settings.lock().expect("settings lock").clone();
    let _ = app.emit("settings-changed", &saved);

    let snapshot = state.snapshot();
    crate::tray::update_tray_title(&app, &snapshot.formatted);
    crate::tray::refresh_tray_menu(&app);
    crate::tray::emit_tick_if_visible(&app, &snapshot);
    let _ = app.emit("timer-tick", &snapshot);

    Ok(saved)
}

#[tauri::command]
pub fn set_mode(app: AppHandle, mode: TimerMode) -> Result<TimerSnapshot, String> {
    let state = app.state::<AppState>();

    let config = state
        .settings
        .lock()
        .expect("settings lock")
        .pomodoro_config();
    {
        let mut engine = state.engine.lock().expect("engine lock");
        if !matches!(engine.status(), TimerStatus::Idle | TimerStatus::Completed) {
            return Err("Timer must be idle to change mode".into());
        }
        if engine.waiting() {
            return Err("Cannot change mode while Waiting".into());
        }
        if engine.mode() == mode {
            return Ok(TimerSnapshot::from_engine(&engine, SystemTime::now()));
        }
        engine.set_mode(mode);
        if mode == TimerMode::Pomodoro {
            engine.apply_idle_pomodoro_config(&config);
        }
    }

    state.persist()?;
    let snapshot = state.snapshot();
    crate::tray::update_tray_title(&app, &snapshot.formatted);
    crate::tray::update_tray_icon(&app);
    crate::tray::refresh_tray_menu(&app);
    let _ = app.emit("timer-tick", &snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub fn set_duration(app: AppHandle, duration_secs: u64) -> Result<TimerSnapshot, String> {
    let state = app.state::<AppState>();

    let (unchanged, prev_duration) = {
        let mut engine = state.engine.lock().expect("engine lock");
        if !matches!(engine.status(), TimerStatus::Idle | TimerStatus::Completed) {
            return Err("Timer must be idle to change duration".into());
        }
        let prev = engine.duration_secs();
        if prev == duration_secs {
            (true, prev)
        } else {
            engine.set_duration(duration_secs);
            (false, prev)
        }
    };

    if unchanged {
        return Ok(state.snapshot());
    }

    state.persist()?;
    let snapshot = state.snapshot();
    crate::tray::update_tray_title(&app, &snapshot.formatted);
    // Start is disabled in the tray at 0s, so rebuild when that flips.
    if (prev_duration == 0) != (duration_secs == 0) {
        crate::tray::refresh_tray_menu(&app);
    }
    Ok(snapshot)
}

#[tauri::command]
pub fn start(app: AppHandle) -> Result<TimerSnapshot, String> {
    let state = app.state::<AppState>();
    let hide = state
        .settings
        .lock()
        .expect("settings lock")
        .hide_window_on_start;

    {
        let mut engine = state.engine.lock().expect("engine lock");
        engine.start(SystemTime::now());
    }

    state.persist()?;
    let snapshot = state.snapshot();
    crate::tray::update_tray_title(&app, &snapshot.formatted);
    crate::tray::refresh_tray_menu(&app);
    let _ = app.emit("timer-tick", &snapshot);

    if hide {
        crate::tray::hide_main_window(&app);
    }

    Ok(snapshot)
}

#[tauri::command]
pub fn pause(app: AppHandle) -> Result<TimerSnapshot, String> {
    let state = app.state::<AppState>();
    let finished = {
        let mut engine = state.engine.lock().expect("engine lock");
        engine.pause(SystemTime::now())
    };
    finalize_interval(&app, finished);
    maybe_advance_pomodoro(&app);
    after_control(&app)
}

#[tauri::command]
pub fn resume(app: AppHandle) -> Result<TimerSnapshot, String> {
    let state = app.state::<AppState>();
    {
        let mut engine = state.engine.lock().expect("engine lock");
        engine.resume(SystemTime::now());
    }
    after_control(&app)
}

#[tauri::command]
pub fn toggle_pause(app: AppHandle) -> Result<TimerSnapshot, String> {
    let state = app.state::<AppState>();
    let hide = {
        let settings = state.settings.lock().expect("settings lock");
        settings.hide_window_on_start
    };

    let (became_running, finished) = {
        let mut engine = state.engine.lock().expect("engine lock");
        let before = engine.status();
        let finished = engine.toggle_pause(SystemTime::now());
        let became_running = matches!(
            (before, engine.status()),
            (
                TimerStatus::Idle | TimerStatus::Paused | TimerStatus::Completed,
                TimerStatus::Running
            )
        );
        (became_running, finished)
    };
    finalize_interval(&app, finished);
    maybe_advance_pomodoro(&app);

    state.persist()?;
    let snapshot = state.snapshot();
    crate::tray::update_tray_title(&app, &snapshot.formatted);
    crate::tray::refresh_tray_menu(&app);
    let _ = app.emit("timer-tick", &snapshot);

    if became_running && hide {
        crate::tray::hide_main_window(&app);
    }

    Ok(snapshot)
}

#[tauri::command]
pub fn reset(app: AppHandle) -> Result<TimerSnapshot, String> {
    let state = app.state::<AppState>();
    let finished = {
        let mut engine = state.engine.lock().expect("engine lock");
        engine.reset(SystemTime::now())
    };
    finalize_interval(&app, finished);
    reset_pomodoro_cycle(&app);
    after_control(&app)
}

#[tauri::command]
pub fn discard(app: AppHandle) -> Result<TimerSnapshot, String> {
    let state = app.state::<AppState>();
    {
        let mut engine = state.engine.lock().expect("engine lock");
        engine.discard();
    }
    reset_pomodoro_cycle(&app);
    after_control(&app)
}

#[tauri::command]
pub fn skip(app: AppHandle) -> Result<TimerSnapshot, String> {
    let state = app.state::<AppState>();
    let config = state
        .settings
        .lock()
        .expect("settings lock")
        .pomodoro_config();
    {
        let mut engine = state.engine.lock().expect("engine lock");
        engine.skip_break(&config);
    }
    after_control(&app)
}

#[tauri::command]
pub fn toggle_icon_only(app: AppHandle) -> Result<Settings, String> {
    let state = app.state::<AppState>();
    let updated = {
        let mut settings = state.settings.lock().expect("settings lock");
        settings.icon_only = !settings.icon_only;
        settings.clone()
    };

    state.persist()?;
    let _ = app.emit("settings-changed", &updated);

    let snapshot = state.snapshot();
    crate::tray::update_tray_title(&app, &snapshot.formatted);
    crate::tray::refresh_tray_menu(&app);

    Ok(updated)
}

#[tauri::command]
pub fn show_timer_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        crate::window::apply_dev_always_on_top(&window);
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        crate::window::focus_webview(&window);
        // Push current snapshot when shown
        let snapshot = app.state::<AppState>().snapshot();
        let _ = app.emit("timer-tick", &snapshot);
    }
    Ok(())
}

#[tauri::command]
pub fn hide_timer_window(app: AppHandle) -> Result<(), String> {
    crate::tray::hide_main_window(&app);
    Ok(())
}

#[tauri::command]
pub fn toggle_timer_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        let visible = window.is_visible().unwrap_or(false);
        if visible {
            crate::tray::hide_main_window(&app);
        } else {
            show_timer_window(app)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn open_settings(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        crate::window::focus_webview(&window);
    }
    Ok(())
}

#[tauri::command]
pub fn get_completion_sounds() -> Vec<&'static str> {
    crate::sound::COMPLETION_SOUNDS.to_vec()
}

#[tauri::command]
pub fn preview_sound(app: AppHandle, name: String) {
    crate::sound::play_named_sound(&app, &name);
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    crate::tray::save_main_window_position(&app);
    let state = app.state::<AppState>();
    let finished = {
        let mut engine = state.engine.lock().expect("engine lock");
        engine.pause(SystemTime::now())
    };
    finalize_interval(&app, finished);
    let _ = state.persist();
    app.exit(0);
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<crate::updater::UpdateStatus, String> {
    crate::updater::run_check(app, true).await
}

#[tauri::command]
pub fn get_update_status(app: AppHandle) -> crate::updater::UpdateStatus {
    crate::updater::current_status(&app)
}

#[tauri::command]
pub async fn install_available_update(
    app: AppHandle,
) -> Result<crate::updater::UpdateStatus, String> {
    crate::updater::install_available_update(app).await
}

#[tauri::command]
pub fn dismiss_available_update(app: AppHandle) {
    crate::updater::dismiss_available_update(&app);
}

#[tauri::command]
pub fn cancel_update_download(app: AppHandle) {
    crate::updater::cancel_update_download(&app);
}

#[tauri::command]
pub fn dismiss_update_progress(app: AppHandle) {
    crate::updater::dismiss_update_progress(&app);
}

#[tauri::command]
pub async fn install_and_restart(app: AppHandle) -> Result<(), String> {
    crate::updater::install_and_restart(app).await
}

fn pomodoro_config(state: &AppState) -> crate::timer::PomodoroConfig {
    state
        .settings
        .lock()
        .expect("settings lock")
        .pomodoro_config()
}

pub fn maybe_advance_pomodoro(app: &AppHandle) {
    let state = app.state::<AppState>();
    let config = pomodoro_config(&state);
    let mut engine = state.engine.lock().expect("engine lock");
    if engine.mode() == TimerMode::Pomodoro && engine.status() == TimerStatus::Completed {
        engine.advance_after_completion(SystemTime::now(), &config);
    }
}

fn reset_pomodoro_cycle(app: &AppHandle) {
    let state = app.state::<AppState>();
    let config = pomodoro_config(&state);
    let mut engine = state.engine.lock().expect("engine lock");
    if engine.mode() == TimerMode::Pomodoro {
        engine.reset_cycle(&config);
    }
}

fn after_control(app: &AppHandle) -> Result<TimerSnapshot, String> {
    let state = app.state::<AppState>();
    state.persist()?;
    let snapshot = state.snapshot();
    crate::tray::update_tray_title(app, &snapshot.formatted);
    crate::tray::refresh_tray_menu(app);
    let _ = app.emit("timer-tick", &snapshot);
    Ok(snapshot)
}
