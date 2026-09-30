use serde::{Deserialize, Serialize};

use super::{format_hms, PomodoroPhase, TimerEngine, TimerMode, TimerStatus};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TimerSnapshot {
    pub status: TimerStatus,
    pub mode: TimerMode,
    pub remaining_secs: u64,
    pub elapsed_secs: u64,
    pub interval_elapsed_secs: u64,
    pub duration_secs: u64,
    pub formatted: String,
    pub phase: Option<PomodoroPhase>,
    pub completed_sessions: u32,
    pub sessions_until_long_break: u32,
    pub is_break: bool,
    pub waiting: bool,
}

impl TimerSnapshot {
    pub fn from_engine(engine: &TimerEngine, now: std::time::SystemTime) -> Self {
        let remaining_secs = engine.remaining_secs(now);
        let elapsed_secs = engine.elapsed_secs(now);
        let interval_elapsed_secs = engine.current_interval_elapsed_secs(now);
        let formatted = if engine.mode().is_countdown() {
            format_hms(remaining_secs)
        } else {
            format_hms(elapsed_secs)
        };
        let is_pomodoro = engine.mode() == TimerMode::Pomodoro;
        Self {
            status: engine.status(),
            mode: engine.mode(),
            remaining_secs,
            elapsed_secs,
            interval_elapsed_secs,
            duration_secs: engine.duration_secs(),
            formatted,
            phase: is_pomodoro.then_some(engine.pomodoro_phase()),
            completed_sessions: engine.pomodoro_completed_sessions(),
            sessions_until_long_break: engine.pomodoro_sessions_until_long_break(),
            is_break: engine.is_break(),
            waiting: engine.waiting(),
        }
    }
}
