use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum PomodoroPhase {
    #[default]
    Session,
    ShortBreak,
    LongBreak,
}

impl PomodoroPhase {
    pub fn is_break(self) -> bool {
        !matches!(self, Self::Session)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PomodoroConfig {
    pub session_secs: u64,
    pub short_break_secs: u64,
    pub long_break_secs: u64,
    pub sessions_until_long_break: u32,
    pub auto_start_sessions: bool,
    pub auto_start_breaks: bool,
}

impl PomodoroConfig {
    pub const DEFAULT_SESSION_SECS: u64 = 25 * 60;
    pub const DEFAULT_SHORT_BREAK_SECS: u64 = 5 * 60;
    pub const DEFAULT_LONG_BREAK_SECS: u64 = 15 * 60;
    pub const DEFAULT_SESSIONS_UNTIL_LONG_BREAK: u32 = 4;

    pub fn sessions_until_long_break(value: u32) -> u32 {
        value.clamp(2, 10)
    }

    pub fn duration_for(&self, phase: PomodoroPhase) -> u64 {
        match phase {
            PomodoroPhase::Session => self.session_secs,
            PomodoroPhase::ShortBreak => self.short_break_secs,
            PomodoroPhase::LongBreak => self.long_break_secs,
        }
    }
}

impl Default for PomodoroConfig {
    fn default() -> Self {
        Self {
            session_secs: Self::DEFAULT_SESSION_SECS,
            short_break_secs: Self::DEFAULT_SHORT_BREAK_SECS,
            long_break_secs: Self::DEFAULT_LONG_BREAK_SECS,
            sessions_until_long_break: Self::DEFAULT_SESSIONS_UNTIL_LONG_BREAK,
            auto_start_sessions: false,
            auto_start_breaks: false,
        }
    }
}
