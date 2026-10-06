mod app_state;
mod atomic_file;
mod commands;
mod entries;
mod notification;
mod persistence;
mod sleep;
mod sound;
mod timer;
mod tray;
mod updater;
mod window;

use std::path::PathBuf;
use std::time::SystemTime;

use tauri::{Emitter, Manager};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

use app_state::AppState;
use persistence::Persistence;
use sleep::SleepDetector;
use timer::TimerStatus;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(debug_assertions)]
    load_dev_dotenv();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::get_totals,
            commands::get_entries,
            commands::create_entry,
            commands::update_entry,
            commands::delete_entry,
            commands::reset_statistics,
            commands::get_settings,
            commands::update_settings,
            commands::set_mode,
            commands::set_duration,
            commands::start,
            commands::pause,
            commands::resume,
            commands::toggle_pause,
            commands::reset,
            commands::discard,
            commands::skip,
            commands::toggle_icon_only,
            commands::show_timer_window,
            commands::hide_timer_window,
            commands::toggle_timer_window,
            commands::open_settings,
            commands::get_completion_sounds,
            commands::preview_sound,
            commands::quit_app,
            commands::check_for_updates,
            commands::get_update_status,
            commands::install_available_update,
            commands::dismiss_available_update,
            commands::cancel_update_download,
            commands::dismiss_update_progress,
            commands::install_and_restart,
            window::resize_main_window,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                window::unbind_cut_key_equivalent();
            }

            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from(".").join("focus-time-data"));
            let persistence = Persistence::new(data_dir.clone());
            let entries = entries::EntriesStore::new(data_dir);
            let (settings, engine, main_window_position, updater_meta, finished) =
                persistence.load();
            let state = AppState::new(
                persistence,
                entries,
                settings,
                engine,
                main_window_position,
                updater_meta,
            );
            // Ensure a state file exists immediately for crash recovery.
            let _ = state.persist();
            app.manage(state);
            commands::finalize_interval(app.handle(), finished);

            sync_autostart(app.handle());
            tray::create_tray(app.handle())?;
            tray::position_main_window(app.handle());
            if let Some(window) = app.get_webview_window("main") {
                window::apply_dev_always_on_top(&window);
            }
            notification::install(app.handle());
            start_tick_loop(app.handle().clone());
            updater::start_background_checks(app.handle().clone());

            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                if window.label() == "main" {
                    tray::hide_main_window(window.app_handle());
                } else {
                    let _ = window.hide();
                }
            }
            tauri::WindowEvent::Focused(false)
                if window.label() == "main" && !window::always_on_top_enabled() =>
            {
                let app = window.app_handle();
                if !tray::any_sibling_window_visible(app) {
                    tray::hide_main_window(app);
                }
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running Focus Time");
}

/// Load `.env.local` then `.env` from the project root (debug only).
/// Existing process env wins; `.env.local` wins over `.env` for unset keys.
#[cfg(debug_assertions)]
fn load_dev_dotenv() {
    let _ = dotenvy::from_filename(".env.local");
    let _ = dotenvy::dotenv();
}

fn sync_autostart(app: &tauri::AppHandle) {
    let enabled = app
        .state::<AppState>()
        .settings
        .lock()
        .expect("settings")
        .start_at_login;
    let autostart = app.autolaunch();
    if enabled {
        let _ = autostart.enable();
    } else {
        let _ = autostart.disable();
    }
}

fn start_tick_loop(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut detector = {
            let remaining = app.state::<AppState>().snapshot().remaining_secs;
            SleepDetector::new(remaining)
        };

        loop {
            let sleep_for = {
                let state = app.state::<AppState>();
                let engine = state.engine.lock().expect("engine");
                engine.time_until_display_tick(SystemTime::now())
            };
            std::thread::sleep(sleep_for);

            let state = app.state::<AppState>();

            if let Some((remaining_before_sleep, sleep_onset)) = detector.poll_sleep() {
                let pause_on_sleep = state.settings.lock().expect("settings").pause_on_sleep;
                let mut engine = state.engine.lock().expect("engine");
                if pause_on_sleep && engine.status() == TimerStatus::Running {
                    let finished = engine.interval_in_progress(sleep_onset);
                    engine.restore_paused(remaining_before_sleep);
                    drop(engine);
                    commands::finalize_interval(&app, finished);
                    let _ = state.persist();
                    let snapshot = state.snapshot();
                    tray::update_tray_title(&app, &snapshot.formatted);
                    tray::refresh_tray_menu(&app);
                    tray::emit_tick_if_visible(&app, &snapshot);
                    detector.note_remaining(snapshot.remaining_secs);
                    continue;
                }
            }

            // Sleep that jumps the wall clock over midnight still reaches
            // here when pause-on-sleep is off, so the split is retroactive.
            let now = SystemTime::now();
            let (splits, finished_interval) = {
                let mut engine = state.engine.lock().expect("engine");
                let splits = engine.split_crossed_midnight(now);
                let finished_interval = engine.tick(now);
                (splits, finished_interval)
            };
            let completed = finished_interval.is_some();
            let completed_break = finished_interval
                .as_ref()
                .is_some_and(|interval| !interval.records_entry);
            let split = !splits.is_empty();
            if split {
                // Anchor first, entry second. A crash in between drops the
                // pre-midnight entry once; the reverse order would record it
                // again on the next launch.
                let _ = state.persist();
                let mut ticks = state.ticks_since_persist.lock().expect("ticks");
                *ticks = 0;
            }
            for interval in splits {
                commands::finalize_interval(&app, Some(interval));
            }
            commands::finalize_interval(&app, finished_interval);

            if completed {
                commands::maybe_advance_pomodoro(&app);
            }

            let snapshot = state.snapshot();
            detector.note_remaining(snapshot.remaining_secs);

            tray::update_tray_title(&app, &snapshot.formatted);
            tray::emit_tick_if_visible(&app, &snapshot);

            if completed {
                let (notifications_enabled, completion_sound) = {
                    let settings = state.settings.lock().expect("settings");
                    (
                        settings.notifications_enabled,
                        settings.completion_sound.clone(),
                    )
                };
                sound::play_named_sound(&app, &completion_sound);
                if notifications_enabled {
                    if completed_break {
                        notification::show_break_finished(&app);
                    } else {
                        notification::show_timer_finished(&app);
                    }
                }
                let _ = state.persist();
                tray::refresh_tray_menu(&app);
                let _ = app.emit("timer-completed", &snapshot);
                let _ = app.emit("timer-tick", &snapshot);
            } else if snapshot.status == TimerStatus::Running {
                let mut ticks = state.ticks_since_persist.lock().expect("ticks");
                *ticks += 1;
                if *ticks >= 5 {
                    *ticks = 0;
                    drop(ticks);
                    let _ = state.persist();
                }
            }
        }
    });
}
