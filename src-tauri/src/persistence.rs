use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::sound::{normalize_completion_sound, NO_COMPLETION_SOUND};
use crate::timer::{
    FinishedInterval, PomodoroConfig, PomodoroPhase, TimerEngine, TimerMode, TimerStatus,
};

fn default_true() -> bool {
    true
}

fn default_completion_sound() -> String {
    "Door Bell".to_string()
}

fn default_presets() -> [Option<u64>; 3] {
    [Some(30 * 60), Some(60 * 60), Some(2 * 60 * 60)]
}

fn default_session_length_secs() -> u64 {
    PomodoroConfig::DEFAULT_SESSION_SECS
}

fn default_short_break_length_secs() -> u64 {
    PomodoroConfig::DEFAULT_SHORT_BREAK_SECS
}

fn default_long_break_length_secs() -> u64 {
    PomodoroConfig::DEFAULT_LONG_BREAK_SECS
}

fn default_sessions_until_long_break() -> u32 {
    PomodoroConfig::DEFAULT_SESSIONS_UNTIL_LONG_BREAK
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub hide_window_on_start: bool,
    pub pause_on_sleep: bool,
    pub start_at_login: bool,
    #[serde(default = "default_true")]
    pub notifications_enabled: bool,
    #[serde(default)]
    pub icon_only: bool,
    #[serde(default = "default_completion_sound")]
    pub completion_sound: String,
    #[serde(default = "default_true")]
    pub auto_check_for_updates: bool,
    #[serde(default = "default_presets")]
    pub presets: [Option<u64>; 3],
    #[serde(default = "default_session_length_secs")]
    pub session_length_secs: u64,
    #[serde(default = "default_short_break_length_secs")]
    pub short_break_length_secs: u64,
    #[serde(default = "default_long_break_length_secs")]
    pub long_break_length_secs: u64,
    #[serde(default = "default_sessions_until_long_break")]
    pub sessions_until_long_break: u32,
    #[serde(default)]
    pub auto_start_sessions: bool,
    #[serde(default)]
    pub auto_start_breaks: bool,
}

impl Settings {
    pub fn pomodoro_config(&self) -> PomodoroConfig {
        PomodoroConfig {
            session_secs: self.session_length_secs,
            short_break_secs: self.short_break_length_secs,
            long_break_secs: self.long_break_length_secs,
            sessions_until_long_break: PomodoroConfig::sessions_until_long_break(
                self.sessions_until_long_break,
            ),
            auto_start_sessions: self.auto_start_sessions,
            auto_start_breaks: self.auto_start_breaks,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdaterMeta {
    #[serde(default)]
    pub last_auto_check_unix: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hide_window_on_start: true,
            pause_on_sleep: true,
            start_at_login: false,
            notifications_enabled: true,
            icon_only: false,
            completion_sound: default_completion_sound(),
            auto_check_for_updates: true,
            presets: default_presets(),
            session_length_secs: default_session_length_secs(),
            short_break_length_secs: default_short_break_length_secs(),
            long_break_length_secs: default_long_break_length_secs(),
            sessions_until_long_break: default_sessions_until_long_break(),
            auto_start_sessions: false,
            auto_start_breaks: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedTimer {
    #[serde(default)]
    mode: TimerMode,
    status: TimerStatus,
    duration_secs: u64,
    remaining_at_pause: u64,
    /// Unix timestamp seconds when running (timer mode).
    deadline_unix: Option<u64>,
    #[serde(default)]
    elapsed_at_pause: u64,
    /// Unix timestamp seconds when running (stopwatch mode).
    #[serde(default)]
    started_at_unix: Option<u64>,
    #[serde(default)]
    timer_duration_secs: Option<u64>,
    #[serde(default)]
    pomodoro_phase: PomodoroPhase,
    #[serde(default)]
    pomodoro_completed_sessions: u32,
    #[serde(default = "default_sessions_until_long_break")]
    pomodoro_sessions_until_long_break: u32,
    #[serde(default)]
    waiting: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedState {
    settings: Settings,
    timer: PersistedTimer,
    #[serde(default)]
    main_window_position: Option<WindowPosition>,
    #[serde(default)]
    updater: UpdaterMeta,
}

pub struct Persistence {
    path: PathBuf,
}

impl Persistence {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self {
            path: app_data_dir.join("state.json"),
        }
    }

    pub fn load(
        &self,
    ) -> (
        Settings,
        TimerEngine,
        Option<WindowPosition>,
        UpdaterMeta,
        Option<FinishedInterval>,
    ) {
        let Ok(bytes) = fs::read(&self.path) else {
            return (
                Settings::default(),
                TimerEngine::default(),
                None,
                UpdaterMeta::default(),
                None,
            );
        };
        let Ok(mut state) = serde_json::from_slice::<PersistedState>(&bytes) else {
            return (
                Settings::default(),
                TimerEngine::default(),
                None,
                UpdaterMeta::default(),
                None,
            );
        };

        // Migrate legacy `soundEnabled: false` → `completionSound: "None"`.
        if let Ok(raw) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            if raw
                .pointer("/settings/soundEnabled")
                .and_then(|v| v.as_bool())
                == Some(false)
            {
                state.settings.completion_sound = NO_COMPLETION_SOUND.to_string();
            }
        }

        state.settings.completion_sound =
            normalize_completion_sound(&state.settings.completion_sound);

        let duration = state.timer.duration_secs;
        let mut engine = TimerEngine::new(duration);
        let now = SystemTime::now();
        let mut finished = None;
        engine.restore_timer_duration(state.timer.timer_duration_secs.unwrap_or(
            if state.timer.mode == TimerMode::Timer {
                duration
            } else {
                0
            },
        ));
        engine.restore_pomodoro_cycle(
            state.timer.pomodoro_phase,
            state.timer.pomodoro_completed_sessions,
            state.timer.pomodoro_sessions_until_long_break,
        );

        match state.timer.mode {
            TimerMode::Timer => match state.timer.status {
                TimerStatus::Running => {
                    if let Some(deadline_unix) = state.timer.deadline_unix {
                        let deadline = UNIX_EPOCH + Duration::from_secs(deadline_unix);
                        engine.restore_running(deadline, now);
                    } else {
                        engine.reset(now);
                    }
                }
                TimerStatus::Paused => {
                    engine.restore_paused(state.timer.remaining_at_pause);
                }
                TimerStatus::Completed => {
                    engine.restore_completed();
                }
                TimerStatus::Idle => {
                    engine.reset(now);
                }
            },
            TimerMode::Stopwatch => match state.timer.status {
                TimerStatus::Running => {
                    if let Some(started_at_unix) = state.timer.started_at_unix {
                        let started_at = UNIX_EPOCH + Duration::from_secs(started_at_unix);
                        engine.restore_stopwatch_running(started_at, state.timer.elapsed_at_pause);
                    } else {
                        engine.set_mode(TimerMode::Stopwatch);
                        engine.reset(now);
                    }
                }
                TimerStatus::Paused => {
                    engine.set_mode(TimerMode::Stopwatch);
                    engine.restore_stopwatch_paused(state.timer.elapsed_at_pause);
                }
                TimerStatus::Idle | TimerStatus::Completed => {
                    engine.set_mode(TimerMode::Stopwatch);
                    engine.reset(now);
                }
            },
            TimerMode::Pomodoro => {
                engine.set_mode(TimerMode::Pomodoro);
                match state.timer.status {
                    TimerStatus::Running => {
                        if let Some(deadline_unix) = state.timer.deadline_unix {
                            let deadline = UNIX_EPOCH + Duration::from_secs(deadline_unix);
                            finished = engine.restore_countdown_running(deadline, now);
                            if engine.status() == TimerStatus::Completed {
                                engine.advance_after_completion(
                                    now,
                                    &state.settings.pomodoro_config(),
                                );
                            }
                        } else {
                            engine.reset(now);
                        }
                    }
                    TimerStatus::Paused => {
                        engine.restore_paused(state.timer.remaining_at_pause);
                    }
                    TimerStatus::Completed | TimerStatus::Idle => {
                        engine.reset(now);
                        engine.restore_waiting(state.timer.waiting);
                    }
                }
            }
        }

        (
            state.settings,
            engine,
            state.main_window_position,
            state.updater,
            finished,
        )
    }

    pub fn save(
        &self,
        settings: &Settings,
        engine: &TimerEngine,
        main_window_position: Option<WindowPosition>,
        updater: &UpdaterMeta,
    ) -> Result<(), String> {
        let deadline_unix = engine.deadline().and_then(|deadline| {
            deadline
                .duration_since(UNIX_EPOCH)
                .ok()
                .map(|d| d.as_secs())
        });
        let started_at_unix = engine
            .started_at()
            .and_then(|started| started.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs()));

        let state = PersistedState {
            settings: settings.clone(),
            timer: PersistedTimer {
                mode: engine.mode(),
                status: engine.status(),
                duration_secs: engine.duration_secs(),
                remaining_at_pause: engine.remaining_secs(SystemTime::now()),
                deadline_unix,
                elapsed_at_pause: engine.elapsed_at_pause(),
                started_at_unix,
                timer_duration_secs: Some(engine.timer_duration_secs()),
                pomodoro_phase: engine.pomodoro_phase(),
                pomodoro_completed_sessions: engine.pomodoro_completed_sessions(),
                pomodoro_sessions_until_long_break: engine.pomodoro_sessions_until_long_break(),
                waiting: engine.waiting(),
            },
            main_window_position,
            updater: updater.clone(),
        };

        let json = serde_json::to_vec_pretty(&state).map_err(|e| e.to_string())?;
        crate::atomic_file::write_json(&self.path, &json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timer::TimerStatus;
    use chrono::TimeZone;
    use std::time::Duration;

    #[test]
    fn settings_default_values() {
        let s = Settings::default();
        assert!(s.hide_window_on_start);
        assert!(s.pause_on_sleep);
        assert!(!s.start_at_login);
        assert!(s.notifications_enabled);
        assert!(!s.icon_only);
        assert_eq!(s.completion_sound, "Door Bell");
        assert!(s.auto_check_for_updates);
        assert_eq!(s.presets, [Some(1800), Some(3600), Some(7200)]);
        assert_eq!(s.session_length_secs, 25 * 60);
        assert_eq!(s.short_break_length_secs, 5 * 60);
        assert_eq!(s.long_break_length_secs, 15 * 60);
        assert_eq!(s.sessions_until_long_break, 4);
        assert!(!s.auto_start_sessions);
        assert!(!s.auto_start_breaks);
    }

    #[test]
    fn roundtrip_paused_state() {
        let dir = std::env::temp_dir().join(format!(
            "focus-timer-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let persistence = Persistence::new(dir.clone());

        let settings = Settings::default();
        let mut engine = TimerEngine::new(120);
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000_000);
        engine.start(now);
        engine.pause(now + Duration::from_secs(20));
        let position = WindowPosition { x: 420, y: 32 };
        let updater = UpdaterMeta {
            last_auto_check_unix: Some(1_700_000_000),
        };
        persistence
            .save(&settings, &engine, Some(position), &updater)
            .unwrap();

        let (loaded_settings, loaded_engine, loaded_position, loaded_updater, _) =
            persistence.load();
        assert_eq!(loaded_settings, settings);
        assert_eq!(loaded_engine.status(), TimerStatus::Paused);
        assert_eq!(loaded_engine.remaining_secs(SystemTime::now()), 100);
        assert_eq!(loaded_position, Some(position));
        assert_eq!(loaded_updater, updater);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn settings_serialization_uses_camel_case() {
        let json = serde_json::to_string(&Settings::default()).unwrap();
        assert!(json.contains("hideWindowOnStart"));
        assert!(json.contains("pauseOnSleep"));
        assert!(json.contains("startAtLogin"));
        assert!(json.contains("notificationsEnabled"));
        assert!(!json.contains("soundEnabled"));
        assert!(json.contains("iconOnly"));
        assert!(json.contains("completionSound"));
        assert!(json.contains("autoCheckForUpdates"));
        assert!(json.contains("presets"));
        let parsed: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, Settings::default());
    }

    #[test]
    fn legacy_state_migrates_missing_presets() {
        let json = r#"{
            "settings": {
                "hideWindowOnStart": true,
                "pauseOnSleep": true,
                "startAtLogin": false,
                "notificationsEnabled": true,
                "iconOnly": false,
                "completionSound": "Glass",
                "autoCheckForUpdates": true
            },
            "timer": {
                "status": "idle",
                "durationSecs": 1500,
                "remainingAtPause": 1500,
                "deadlineUnix": null
            }
        }"#;
        let state: PersistedState = serde_json::from_str(json).unwrap();
        assert_eq!(state.settings.presets, [Some(1800), Some(3600), Some(7200)]);
    }

    #[test]
    fn legacy_state_migrates_missing_updater_fields() {
        let json = r#"{
            "settings": {
                "hideWindowOnStart": true,
                "pauseOnSleep": true,
                "startAtLogin": false,
                "soundEnabled": true,
                "iconOnly": false,
                "completionSound": "Glass"
            },
            "timer": {
                "status": "idle",
                "durationSecs": 1500,
                "remainingAtPause": 1500,
                "deadlineUnix": null
            }
        }"#;
        let state: PersistedState = serde_json::from_str(json).unwrap();
        assert!(state.settings.auto_check_for_updates);
        assert!(state.settings.notifications_enabled);
        assert_eq!(state.updater, UpdaterMeta::default());
    }

    #[test]
    fn legacy_sound_enabled_false_migrates_to_none() {
        let dir = std::env::temp_dir().join(format!(
            "focus-timer-sound-migrate-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        fs::write(
            &path,
            r#"{
                "settings": {
                    "hideWindowOnStart": true,
                    "pauseOnSleep": true,
                    "startAtLogin": false,
                    "soundEnabled": false,
                    "iconOnly": false,
                    "completionSound": "Glass"
                },
                "timer": {
                    "status": "idle",
                    "durationSecs": 1500,
                    "remainingAtPause": 1500,
                    "deadlineUnix": null
                }
            }"#,
        )
        .unwrap();

        let persistence = Persistence::new(dir.clone());
        let (settings, _, _, _, _) = persistence.load();
        assert_eq!(settings.completion_sound, NO_COMPLETION_SOUND);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn updater_meta_roundtrip() {
        let meta = UpdaterMeta {
            last_auto_check_unix: Some(42),
        };
        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("lastAutoCheckUnix"));
        let parsed: UpdaterMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, meta);
    }

    #[test]
    fn legacy_state_without_mode_loads_as_timer() {
        let dir = std::env::temp_dir().join(format!(
            "focus-timer-mode-migrate-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        fs::write(
            &path,
            r#"{
                "settings": {
                    "hideWindowOnStart": true,
                    "pauseOnSleep": true,
                    "startAtLogin": false,
                    "notificationsEnabled": true,
                    "iconOnly": false,
                    "completionSound": "Glass",
                    "autoCheckForUpdates": true,
                    "presets": [null, null, null]
                },
                "timer": {
                    "status": "idle",
                    "durationSecs": 1500,
                    "remainingAtPause": 1500,
                    "deadlineUnix": null
                }
            }"#,
        )
        .unwrap();

        let persistence = Persistence::new(dir.clone());
        let (_, engine, _, _, _) = persistence.load();
        assert_eq!(engine.mode(), TimerMode::Timer);
        assert_eq!(engine.duration_secs(), 1500);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn stopwatch_state_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "focus-timer-stopwatch-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let persistence = Persistence::new(dir.clone());

        let settings = Settings::default();
        let mut engine = TimerEngine::new(60);
        engine.set_mode(TimerMode::Stopwatch);
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000_000);
        engine.start(now);
        engine.pause(now + Duration::from_secs(42));
        persistence
            .save(&settings, &engine, None, &UpdaterMeta::default())
            .unwrap();

        let (_, loaded_engine, _, _, _) = persistence.load();
        assert_eq!(loaded_engine.mode(), TimerMode::Stopwatch);
        assert_eq!(loaded_engine.status(), TimerStatus::Paused);
        assert_eq!(loaded_engine.elapsed_at_pause(), 42);

        let _ = fs::remove_dir_all(dir);
    }

    fn local_dt(y: i32, m: u32, d: u32, h: u32, min: u32, s: u32) -> SystemTime {
        let naive = chrono::NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, s)
            .unwrap();
        let dt = chrono::Local.from_local_datetime(&naive).single().unwrap();
        UNIX_EPOCH + Duration::from_secs(dt.timestamp() as u64)
    }

    #[test]
    fn running_stopwatch_roundtrip_keeps_post_midnight_interval() {
        let dir = std::env::temp_dir().join(format!(
            "focus-timer-stopwatch-midnight-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let persistence = Persistence::new(dir.clone());

        let mut engine = TimerEngine::new(60);
        engine.set_mode(TimerMode::Stopwatch);
        let start = local_dt(2026, 6, 15, 23, 0, 0);
        let after = local_dt(2026, 6, 16, 0, 30, 0);
        engine.start(start);
        assert_eq!(engine.split_crossed_midnight(after).len(), 1);
        persistence
            .save(&Settings::default(), &engine, None, &UpdaterMeta::default())
            .unwrap();

        let (_, mut loaded_engine, _, _, _) = persistence.load();
        assert_eq!(loaded_engine.mode(), TimerMode::Stopwatch);
        assert_eq!(loaded_engine.status(), TimerStatus::Running);
        assert_eq!(loaded_engine.elapsed_secs(after), 90 * 60);
        assert_eq!(loaded_engine.current_interval_elapsed_secs(after), 30 * 60);
        assert!(loaded_engine.split_crossed_midnight(after).is_empty());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn pomodoro_cycle_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "focus-timer-pomodoro-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let persistence = Persistence::new(dir.clone());

        let settings = Settings::default();
        let mut engine = TimerEngine::new(1500);
        engine.set_mode(TimerMode::Pomodoro);
        engine.restore_pomodoro_cycle(crate::timer::PomodoroPhase::ShortBreak, 2, 4);
        engine.apply_idle_pomodoro_config(&settings.pomodoro_config());
        persistence
            .save(&settings, &engine, None, &UpdaterMeta::default())
            .unwrap();

        let (loaded_settings, loaded_engine, _, _, _) = persistence.load();
        assert_eq!(loaded_settings.session_length_secs, 25 * 60);
        assert_eq!(loaded_engine.mode(), TimerMode::Pomodoro);
        assert_eq!(
            loaded_engine.pomodoro_phase(),
            crate::timer::PomodoroPhase::ShortBreak
        );
        assert_eq!(loaded_engine.pomodoro_completed_sessions(), 2);
        assert_eq!(loaded_engine.duration_secs(), 5 * 60);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn pomodoro_waiting_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "focus-timer-waiting-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let persistence = Persistence::new(dir.clone());

        let settings = Settings::default();
        let mut engine = TimerEngine::new(1500);
        engine.set_mode(TimerMode::Pomodoro);
        engine.restore_pomodoro_cycle(crate::timer::PomodoroPhase::ShortBreak, 1, 4);
        engine.apply_idle_pomodoro_config(&settings.pomodoro_config());
        engine.restore_waiting(true);
        persistence
            .save(&settings, &engine, None, &UpdaterMeta::default())
            .unwrap();

        let (_, loaded_engine, _, _, finished) = persistence.load();
        assert!(finished.is_none());
        assert!(loaded_engine.waiting());
        assert_eq!(
            loaded_engine.pomodoro_phase(),
            crate::timer::PomodoroPhase::ShortBreak
        );
        assert_eq!(loaded_engine.pomodoro_completed_sessions(), 1);
        assert_eq!(loaded_engine.status(), TimerStatus::Idle);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn pomodoro_session_completed_while_quit_records_and_is_waiting() {
        let dir = std::env::temp_dir().join(format!(
            "focus-timer-quit-complete-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let persistence = Persistence::new(dir.clone());

        let mut settings = Settings::default();
        settings.session_length_secs = 25;
        settings.short_break_length_secs = 5;
        let mut engine = TimerEngine::new(25);
        engine.set_mode(TimerMode::Pomodoro);
        engine.apply_idle_pomodoro_config(&settings.pomodoro_config());
        let start = UNIX_EPOCH + Duration::from_secs(1_000_000);
        engine.start(start);
        persistence
            .save(&settings, &engine, None, &UpdaterMeta::default())
            .unwrap();

        let (_, loaded_engine, _, _, finished) = persistence.load();
        let finished = finished.expect("session completed while quit should finish");
        assert!(finished.records_entry);
        assert_eq!(loaded_engine.pomodoro_phase(), PomodoroPhase::ShortBreak);
        assert_eq!(loaded_engine.status(), TimerStatus::Idle);
        assert!(loaded_engine.waiting());
        assert!(crate::entries::entry_from_interval(finished).is_some());

        let _ = fs::remove_dir_all(dir);
    }
}
