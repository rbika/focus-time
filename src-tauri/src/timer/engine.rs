use chrono::{NaiveDate, TimeZone};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::{PomodoroConfig, PomodoroPhase};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum TimerMode {
    #[default]
    Timer,
    Stopwatch,
    Pomodoro,
}

impl TimerMode {
    pub fn is_countdown(self) -> bool {
        !matches!(self, Self::Stopwatch)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TimerStatus {
    Idle,
    Running,
    Paused,
    Completed,
}

/// A contiguous stretch of running time that just ended (pause, cancel, or
/// natural completion), ready to be turned into a persisted Entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinishedInterval {
    pub mode: TimerMode,
    pub started_at: SystemTime,
    pub ended_at: SystemTime,
    pub records_entry: bool,
}

impl FinishedInterval {
    #[allow(dead_code)]
    pub fn new(mode: TimerMode, started_at: SystemTime, ended_at: SystemTime) -> Self {
        Self {
            mode,
            started_at,
            ended_at,
            records_entry: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerEngine {
    mode: TimerMode,
    duration_secs: u64,
    remaining_at_pause: u64,
    /// Absolute deadline while running (timer mode).
    deadline: Option<SystemTime>,
    elapsed_at_pause: u64,
    /// Wall-clock anchor while running (stopwatch mode).
    started_at: Option<SystemTime>,
    status: TimerStatus,
    timer_duration_secs: u64,
    pomodoro_phase: PomodoroPhase,
    pomodoro_completed_sessions: u32,
    pomodoro_sessions_until_long_break: u32,
    #[serde(default)]
    pomodoro_waiting: bool,
}

impl Default for TimerEngine {
    fn default() -> Self {
        Self::new(0)
    }
}

impl TimerEngine {
    pub fn new(duration_secs: u64) -> Self {
        Self {
            mode: TimerMode::Timer,
            duration_secs,
            remaining_at_pause: duration_secs,
            deadline: None,
            elapsed_at_pause: 0,
            started_at: None,
            status: TimerStatus::Idle,
            timer_duration_secs: duration_secs,
            pomodoro_phase: PomodoroPhase::Session,
            pomodoro_completed_sessions: 0,
            pomodoro_sessions_until_long_break: PomodoroConfig::DEFAULT_SESSIONS_UNTIL_LONG_BREAK,
            pomodoro_waiting: false,
        }
    }

    pub fn mode(&self) -> TimerMode {
        self.mode
    }

    pub fn status(&self) -> TimerStatus {
        self.status
    }

    pub fn duration_secs(&self) -> u64 {
        self.duration_secs
    }

    pub fn timer_duration_secs(&self) -> u64 {
        self.timer_duration_secs
    }

    pub fn pomodoro_phase(&self) -> PomodoroPhase {
        self.pomodoro_phase
    }

    pub fn pomodoro_completed_sessions(&self) -> u32 {
        self.pomodoro_completed_sessions
    }

    pub fn pomodoro_sessions_until_long_break(&self) -> u32 {
        self.pomodoro_sessions_until_long_break
    }

    pub fn is_break(&self) -> bool {
        self.mode == TimerMode::Pomodoro && self.pomodoro_phase.is_break()
    }

    pub fn waiting(&self) -> bool {
        self.mode == TimerMode::Pomodoro && self.pomodoro_waiting
    }

    fn records_current_interval(&self) -> bool {
        !self.is_break()
    }

    fn finish(&self, started_at: SystemTime, ended_at: SystemTime) -> FinishedInterval {
        FinishedInterval {
            mode: self.mode,
            started_at,
            ended_at,
            records_entry: self.records_current_interval(),
        }
    }

    pub fn deadline(&self) -> Option<SystemTime> {
        self.deadline
    }

    /// The interval currently in progress, if the engine is Running — for
    /// callers that must finalize a run through a path other than
    /// `pause`/`reset`/`tick` (e.g. an auto-pause triggered by system
    /// sleep). Does not mutate engine state.
    pub fn interval_in_progress(&self, ended_at: SystemTime) -> Option<FinishedInterval> {
        self.current_interval_start()
            .map(|started_at| self.finish(started_at, ended_at))
    }

    pub fn started_at(&self) -> Option<SystemTime> {
        self.started_at
    }

    pub fn elapsed_at_pause(&self) -> u64 {
        self.elapsed_at_pause
    }

    pub fn remaining_secs(&self, now: SystemTime) -> u64 {
        match self.status {
            TimerStatus::Running => match self.deadline {
                Some(deadline) => ceil_secs(deadline.duration_since(now).unwrap_or_default()),
                None => 0,
            },
            TimerStatus::Idle | TimerStatus::Paused => self.remaining_at_pause,
            TimerStatus::Completed => 0,
        }
    }

    pub fn elapsed_secs(&self, now: SystemTime) -> u64 {
        match self.status {
            TimerStatus::Running => match self.started_at {
                Some(started) => now.duration_since(started).unwrap_or_default().as_secs(),
                None => 0,
            },
            TimerStatus::Idle | TimerStatus::Paused | TimerStatus::Completed => {
                self.elapsed_at_pause
            }
        }
    }

    /// Sleep this long before the displayed second should change.
    pub fn time_until_display_tick(&self, now: SystemTime) -> Duration {
        if self.mode.is_countdown() {
            self.time_until_display_tick_timer(now)
        } else {
            self.time_until_display_tick_stopwatch(now)
        }
    }

    fn time_until_display_tick_timer(&self, now: SystemTime) -> Duration {
        match self.status {
            TimerStatus::Running => match self.deadline {
                Some(deadline) => {
                    let remaining = deadline.duration_since(now).unwrap_or_default();
                    let nanos = remaining.subsec_nanos();
                    if remaining.is_zero() || nanos == 0 {
                        Duration::from_secs(1)
                    } else {
                        Duration::new(0, nanos)
                    }
                }
                None => Duration::from_secs(1),
            },
            TimerStatus::Idle | TimerStatus::Paused | TimerStatus::Completed => {
                Duration::from_secs(1)
            }
        }
    }

    fn time_until_display_tick_stopwatch(&self, now: SystemTime) -> Duration {
        match self.status {
            TimerStatus::Running => match self.started_at {
                Some(started) => {
                    let elapsed = now.duration_since(started).unwrap_or_default();
                    let nanos = elapsed.subsec_nanos();
                    if nanos == 0 {
                        Duration::from_secs(1)
                    } else {
                        Duration::new(0, 1_000_000_000 - nanos)
                    }
                }
                None => Duration::from_secs(1),
            },
            TimerStatus::Idle | TimerStatus::Paused | TimerStatus::Completed => {
                Duration::from_secs(1)
            }
        }
    }

    pub fn set_mode(&mut self, mode: TimerMode) {
        if !matches!(self.status, TimerStatus::Idle | TimerStatus::Completed) {
            return;
        }
        if self.waiting() {
            return;
        }
        if self.mode == TimerMode::Timer {
            self.timer_duration_secs = self.duration_secs;
        }
        self.mode = mode;
        self.deadline = None;
        self.started_at = None;
        self.status = TimerStatus::Idle;
        self.pomodoro_waiting = false;
        match mode {
            TimerMode::Timer => {
                self.duration_secs = self.timer_duration_secs;
                self.remaining_at_pause = self.duration_secs;
                self.elapsed_at_pause = 0;
            }
            TimerMode::Stopwatch => {
                self.elapsed_at_pause = 0;
                self.remaining_at_pause = self.duration_secs;
            }
            TimerMode::Pomodoro => {
                self.elapsed_at_pause = 0;
                self.remaining_at_pause = self.duration_secs;
            }
        }
    }

    pub fn set_duration(&mut self, duration_secs: u64) {
        self.duration_secs = duration_secs;
        if self.mode == TimerMode::Timer {
            self.timer_duration_secs = duration_secs;
        }
        if matches!(self.status, TimerStatus::Idle | TimerStatus::Completed) {
            self.remaining_at_pause = duration_secs;
            self.deadline = None;
            self.started_at = None;
            self.status = TimerStatus::Idle;
        }
    }

    pub fn start(&mut self, now: SystemTime) {
        if self.mode.is_countdown() {
            self.start_timer(now);
        } else {
            self.start_stopwatch(now);
        }
    }

    fn start_timer(&mut self, now: SystemTime) {
        if matches!(self.status, TimerStatus::Completed) {
            self.remaining_at_pause = self.duration_secs;
        }
        if self.remaining_at_pause == 0 {
            self.remaining_at_pause = self.duration_secs;
        }
        if self.remaining_at_pause == 0 {
            return;
        }
        self.deadline = Some(now + Duration::from_secs(self.remaining_at_pause));
        self.started_at = None;
        self.status = TimerStatus::Running;
        self.pomodoro_waiting = false;
    }

    fn start_stopwatch(&mut self, now: SystemTime) {
        if matches!(self.status, TimerStatus::Completed) {
            self.elapsed_at_pause = 0;
        }
        self.started_at = Some(now - Duration::from_secs(self.elapsed_at_pause));
        self.deadline = None;
        self.status = TimerStatus::Running;
    }

    pub fn pause(&mut self, now: SystemTime) -> Option<FinishedInterval> {
        if self.status != TimerStatus::Running {
            return None;
        }
        let started_at = self.current_interval_start();
        if self.mode.is_countdown() {
            self.pause_timer(now);
        } else {
            self.pause_stopwatch(now);
        }
        started_at.map(|started_at| self.finish(started_at, now))
    }

    /// The wall-clock instant the currently-running interval began, derived
    /// from the existing `deadline`/`remaining_at_pause` (timer mode) or
    /// `started_at`/`elapsed_at_pause` (stopwatch mode) anchors — both of
    /// which stay frozen at their pre-run values for the duration of the
    /// run, so no extra state is needed to recover it.
    ///
    /// A midnight split rewrites only the frozen offset (`remaining_at_pause`
    /// or `elapsed_at_pause`) so this instant becomes 00:00:00. The display
    /// anchors stay put: timer remaining still comes from `deadline`, and
    /// stopwatch elapsed still comes from `started_at`.
    ///
    /// Exception: a Timer restored mid-run by `restore_running` (app
    /// restart while Running) re-derives `remaining_at_pause` from the
    /// post-restart remaining time, not the pre-run value, so this
    /// resolves to the restart instant rather than the original Start —
    /// the pre-restart portion of that run is not represented in the
    /// eventual Entry. `state.json` doesn't persist the original pre-run
    /// remaining separately, so this is the best available approximation.
    fn current_interval_start(&self) -> Option<SystemTime> {
        if self.status != TimerStatus::Running {
            return None;
        }
        if self.mode.is_countdown() {
            self.deadline
                .map(|deadline| deadline - Duration::from_secs(self.remaining_at_pause))
        } else {
            self.started_at
                .map(|started_at| started_at + Duration::from_secs(self.elapsed_at_pause))
        }
    }

    fn pause_timer(&mut self, now: SystemTime) {
        self.remaining_at_pause = self.remaining_secs(now);
        self.deadline = None;
        self.status = if self.remaining_at_pause == 0 {
            TimerStatus::Completed
        } else {
            TimerStatus::Paused
        };
    }

    fn pause_stopwatch(&mut self, now: SystemTime) {
        self.elapsed_at_pause = self.elapsed_secs(now);
        self.started_at = None;
        self.status = TimerStatus::Paused;
    }

    pub fn resume(&mut self, now: SystemTime) {
        if self.status != TimerStatus::Paused {
            return;
        }
        if self.mode.is_countdown() {
            self.resume_timer(now);
        } else {
            self.resume_stopwatch(now);
        }
    }

    fn resume_timer(&mut self, now: SystemTime) {
        if self.remaining_at_pause == 0 {
            self.status = TimerStatus::Completed;
            return;
        }
        self.deadline = Some(now + Duration::from_secs(self.remaining_at_pause));
        self.started_at = None;
        self.status = TimerStatus::Running;
    }

    fn resume_stopwatch(&mut self, now: SystemTime) {
        self.started_at = Some(now - Duration::from_secs(self.elapsed_at_pause));
        self.deadline = None;
        self.status = TimerStatus::Running;
    }

    pub fn toggle_pause(&mut self, now: SystemTime) -> Option<FinishedInterval> {
        match self.status {
            TimerStatus::Running => self.pause(now),
            TimerStatus::Paused => {
                self.resume(now);
                None
            }
            TimerStatus::Idle | TimerStatus::Completed => {
                self.start(now);
                None
            }
        }
    }

    /// Elapsed seconds of the in-flight Interval, or 0 if not Running.
    pub fn current_interval_elapsed_secs(&self, now: SystemTime) -> u64 {
        self.current_interval_start()
            .map(|started_at| now.duration_since(started_at).unwrap_or_default().as_secs())
            .unwrap_or(0)
    }

    /// Resets to Idle. If a run was in progress, finalizes it into an Entry
    /// first (a run already paused was finalized when it was paused, so
    /// resetting from Paused doesn't produce a second Entry).
    pub fn reset(&mut self, now: SystemTime) -> Option<FinishedInterval> {
        let finished = self
            .current_interval_start()
            .map(|started_at| self.finish(started_at, now));
        self.return_to_idle();
        finished
    }

    /// Returns to Idle without producing a finished Interval.
    pub fn discard(&mut self) {
        self.return_to_idle();
    }

    fn return_to_idle(&mut self) {
        self.deadline = None;
        self.started_at = None;
        self.status = TimerStatus::Idle;
        if self.mode.is_countdown() {
            self.remaining_at_pause = self.duration_secs;
        } else {
            self.elapsed_at_pause = 0;
        }
    }

    /// When a running interval's start and `now` fall on different local
    /// dates, close each finished day at the next local 00:00:00 and retarget
    /// the in-flight interval to that same instant. The engine stays
    /// `Running`. Countdown remaining and total stopwatch elapsed are
    /// unchanged.
    ///
    /// Same-second portions are still returned; withholding them is
    /// `entries::entry_from_interval`'s job. Call this before `tick` so a
    /// timer that also reached its deadline doesn't record one entry that
    /// straddles midnight.
    pub fn split_crossed_midnight(&mut self, now: SystemTime) -> Vec<FinishedInterval> {
        if self.status != TimerStatus::Running {
            return Vec::new();
        }
        let Some(mut segment_start) = self.current_interval_start() else {
            return Vec::new();
        };
        let horizon = self.split_horizon(now);
        let Some(horizon_date) = local_date(horizon) else {
            return Vec::new();
        };

        let mut finished = Vec::new();
        let mut retarget_to = None;
        while let Some(segment_date) = local_date(segment_start) {
            if segment_date >= horizon_date {
                break;
            }
            let Some(next_date) = segment_date.succ_opt() else {
                break;
            };
            let Some(next_midnight) = at_local(next_date, 0, 0, 0) else {
                break;
            };
            if next_midnight > horizon {
                break;
            }
            if next_midnight > segment_start {
                finished.push(self.finish(segment_start, next_midnight));
            }
            segment_start = next_midnight;
            retarget_to = Some(next_midnight);
        }

        if let Some(new_start) = retarget_to {
            self.retarget_interval_start(new_start);
        }
        finished
    }

    /// Latest instant that still belongs to this run. A timer that already
    /// hit its deadline must not close days past that deadline.
    fn split_horizon(&self, now: SystemTime) -> SystemTime {
        if self.mode.is_countdown() {
            self.deadline
                .filter(|deadline| *deadline <= now)
                .unwrap_or(now)
        } else {
            now
        }
    }

    fn retarget_interval_start(&mut self, new_start: SystemTime) {
        if self.mode.is_countdown() {
            let Some(deadline) = self.deadline else {
                return;
            };
            let Ok(until_deadline) = deadline.duration_since(new_start) else {
                return;
            };
            // Floor so `deadline - remaining_at_pause` lands on or after
            // midnight when `deadline` has a fractional second.
            self.remaining_at_pause = until_deadline.as_secs();
        } else {
            let Some(anchor) = self.started_at else {
                return;
            };
            let Ok(offset) = new_start.duration_since(anchor) else {
                return;
            };
            let secs = offset.as_secs();
            // Ceil so `started_at + elapsed_at_pause` lands on or after
            // midnight. Display elapsed still reads `started_at` alone.
            self.elapsed_at_pause = if offset.subsec_nanos() == 0 {
                secs
            } else {
                secs.saturating_add(1)
            };
        }
    }

    /// Advance wall-clock state. Returns the finished interval if the timer
    /// just completed naturally. Does not split at midnight; the tick loop
    /// calls `split_crossed_midnight` first.
    pub fn tick(&mut self, now: SystemTime) -> Option<FinishedInterval> {
        if self.mode == TimerMode::Stopwatch {
            return None;
        }
        if self.status != TimerStatus::Running {
            return None;
        }
        let remaining = self.remaining_secs(now);
        if remaining == 0 {
            let started_at = self.current_interval_start();
            let ended_at = self.deadline.unwrap_or(now);
            self.deadline = None;
            self.remaining_at_pause = 0;
            self.status = TimerStatus::Completed;
            return started_at.map(|started_at| self.finish(started_at, ended_at));
        }
        None
    }

    /// Restore a previously running timer after process restart.
    pub fn restore_running(&mut self, deadline: SystemTime, now: SystemTime) {
        self.mode = TimerMode::Timer;
        self.deadline = Some(deadline);
        self.started_at = None;
        self.status = TimerStatus::Running;
        if self.tick(now).is_some() {
            // completed during downtime; not retroactively recorded as an Entry
        } else {
            self.remaining_at_pause = self.remaining_secs(now);
        }
    }

    pub fn restore_stopwatch_running(&mut self, started_at: SystemTime, elapsed_at_pause: u64) {
        self.mode = TimerMode::Stopwatch;
        self.started_at = Some(started_at);
        self.elapsed_at_pause = elapsed_at_pause;
        self.deadline = None;
        self.status = TimerStatus::Running;
    }

    pub fn restore_paused(&mut self, remaining_secs: u64) {
        if self.mode == TimerMode::Stopwatch {
            self.mode = TimerMode::Timer;
        }
        self.deadline = None;
        self.started_at = None;
        self.remaining_at_pause = remaining_secs.min(self.duration_secs);
        self.status = if self.remaining_at_pause == 0 {
            TimerStatus::Completed
        } else {
            TimerStatus::Paused
        };
    }

    pub fn restore_stopwatch_paused(&mut self, elapsed_secs: u64) {
        self.mode = TimerMode::Stopwatch;
        self.deadline = None;
        self.started_at = None;
        self.elapsed_at_pause = elapsed_secs;
        self.status = TimerStatus::Paused;
    }

    pub fn restore_completed(&mut self) {
        self.mode = TimerMode::Timer;
        self.deadline = None;
        self.started_at = None;
        self.remaining_at_pause = 0;
        self.status = TimerStatus::Completed;
    }

    pub fn restore_pomodoro_cycle(
        &mut self,
        phase: PomodoroPhase,
        completed_sessions: u32,
        sessions_until_long_break: u32,
    ) {
        self.pomodoro_phase = phase;
        self.pomodoro_completed_sessions = completed_sessions;
        self.pomodoro_sessions_until_long_break =
            PomodoroConfig::sessions_until_long_break(sessions_until_long_break);
    }

    pub fn restore_timer_duration(&mut self, duration_secs: u64) {
        self.timer_duration_secs = duration_secs;
    }

    pub fn restore_countdown_running(
        &mut self,
        deadline: SystemTime,
        now: SystemTime,
    ) -> Option<FinishedInterval> {
        self.deadline = Some(deadline);
        self.started_at = None;
        self.status = TimerStatus::Running;
        if let Some(finished) = self.tick(now) {
            Some(finished)
        } else {
            self.remaining_at_pause = self.remaining_secs(now);
            None
        }
    }

    pub fn apply_idle_pomodoro_config(&mut self, config: &PomodoroConfig) {
        if self.mode != TimerMode::Pomodoro {
            return;
        }
        if !matches!(self.status, TimerStatus::Idle | TimerStatus::Completed) {
            return;
        }
        if self.pomodoro_phase == PomodoroPhase::Session && self.pomodoro_completed_sessions == 0 {
            self.pomodoro_sessions_until_long_break =
                PomodoroConfig::sessions_until_long_break(config.sessions_until_long_break);
        }
        self.set_duration(config.duration_for(self.pomodoro_phase));
    }

    pub fn reset_cycle(&mut self, config: &PomodoroConfig) {
        self.pomodoro_phase = PomodoroPhase::Session;
        self.pomodoro_completed_sessions = 0;
        self.pomodoro_sessions_until_long_break =
            PomodoroConfig::sessions_until_long_break(config.sessions_until_long_break);
        self.set_duration(config.session_secs);
        self.return_to_idle();
        self.pomodoro_waiting = false;
    }

    pub fn advance_after_completion(&mut self, now: SystemTime, config: &PomodoroConfig) {
        if self.mode != TimerMode::Pomodoro {
            return;
        }
        match self.pomodoro_phase {
            PomodoroPhase::Session => {
                self.pomodoro_completed_sessions =
                    self.pomodoro_completed_sessions.saturating_add(1);
                self.pomodoro_phase = if self.pomodoro_completed_sessions
                    >= self.pomodoro_sessions_until_long_break
                {
                    PomodoroPhase::LongBreak
                } else {
                    PomodoroPhase::ShortBreak
                };
                self.set_duration(config.duration_for(self.pomodoro_phase));
                if config.auto_start_breaks {
                    self.start(now);
                } else {
                    self.pomodoro_waiting = true;
                }
            }
            PomodoroPhase::ShortBreak => {
                self.pomodoro_phase = PomodoroPhase::Session;
                self.set_duration(config.session_secs);
                if config.auto_start_sessions {
                    self.start(now);
                } else {
                    self.pomodoro_waiting = true;
                }
            }
            PomodoroPhase::LongBreak => {
                self.reset_cycle(config);
                if config.auto_start_sessions {
                    self.start(now);
                } else {
                    self.pomodoro_waiting = true;
                }
            }
        }
    }

    pub fn skip_break(&mut self, config: &PomodoroConfig) -> bool {
        if self.mode != TimerMode::Pomodoro || !self.pomodoro_phase.is_break() {
            return false;
        }
        let in_flight = matches!(self.status, TimerStatus::Running | TimerStatus::Paused);
        if !in_flight && !(self.status == TimerStatus::Idle && self.pomodoro_waiting) {
            return false;
        }
        let long_break = self.pomodoro_phase == PomodoroPhase::LongBreak;
        self.discard();
        if long_break {
            self.reset_cycle(config);
        } else {
            self.pomodoro_phase = PomodoroPhase::Session;
            self.set_duration(config.session_secs);
        }
        self.pomodoro_waiting = true;
        true
    }

    pub fn restore_waiting(&mut self, waiting: bool) {
        self.pomodoro_waiting = waiting;
    }
}

/// Kitchen-timer rounding: keep showing N until that second has fully elapsed.
fn local_date(time: SystemTime) -> Option<NaiveDate> {
    let secs = i64::try_from(time.duration_since(UNIX_EPOCH).ok()?.as_secs()).ok()?;
    // `earliest` still resolves a date during a DST fold, when `single`
    // would refuse the repeated hour and skip the split.
    chrono::Local
        .timestamp_opt(secs, 0)
        .earliest()
        .map(|dt| dt.date_naive())
}

fn at_local(date: NaiveDate, hour: u32, min: u32, sec: u32) -> Option<SystemTime> {
    let naive = date.and_hms_opt(hour, min, sec)?;
    let local = chrono::Local.from_local_datetime(&naive).single()?;
    let timestamp = local.timestamp();
    if timestamp < 0 {
        return None;
    }
    u64::try_from(timestamp)
        .ok()
        .map(|secs| UNIX_EPOCH + Duration::from_secs(secs))
}

/// Kitchen-timer rounding: keep showing N until that second has fully elapsed.
fn ceil_secs(duration: Duration) -> u64 {
    let secs = duration.as_secs();
    if duration.subsec_nanos() == 0 {
        secs
    } else {
        secs.saturating_add(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::time::UNIX_EPOCH;

    fn t0() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000)
    }

    #[test]
    fn start_pause_resume_preserves_remaining() {
        let mut engine = TimerEngine::new(100);
        let now = t0();
        engine.start(now);
        assert_eq!(engine.status(), TimerStatus::Running);
        assert_eq!(engine.remaining_secs(now), 100);

        let later = now + Duration::from_secs(40);
        engine.pause(later);
        assert_eq!(engine.status(), TimerStatus::Paused);
        assert_eq!(engine.remaining_secs(later), 60);

        let resume_at = later + Duration::from_secs(1_000);
        engine.resume(resume_at);
        assert_eq!(engine.status(), TimerStatus::Running);
        assert_eq!(engine.remaining_secs(resume_at), 60);
        assert_eq!(
            engine.remaining_secs(resume_at + Duration::from_secs(10)),
            50
        );
    }

    #[test]
    fn remaining_holds_full_second_until_it_elapses() {
        let mut engine = TimerEngine::new(60);
        let now = t0();
        engine.start(now);
        assert_eq!(engine.remaining_secs(now), 60);
        assert_eq!(engine.remaining_secs(now + Duration::from_millis(1)), 60);
        assert_eq!(engine.remaining_secs(now + Duration::from_millis(999)), 60);
        assert_eq!(engine.remaining_secs(now + Duration::from_secs(1)), 59);
        assert_eq!(
            engine.remaining_secs(now + Duration::from_millis(1_001)),
            59
        );
    }

    #[test]
    fn time_until_display_tick_is_the_fractional_remainder() {
        let mut engine = TimerEngine::new(60);
        let now = t0();
        engine.start(now);
        assert_eq!(engine.time_until_display_tick(now), Duration::from_secs(1));
        assert_eq!(
            engine.time_until_display_tick(now + Duration::from_millis(250)),
            Duration::from_millis(750)
        );
        engine.pause(now);
        assert_eq!(engine.time_until_display_tick(now), Duration::from_secs(1));
    }

    #[test]
    fn tick_completes_at_deadline() {
        let mut engine = TimerEngine::new(20);
        let now = t0();
        engine.start(now);
        assert!(engine.tick(now + Duration::from_secs(19)).is_none());
        assert!(engine.tick(now + Duration::from_secs(20)).is_some());
        assert_eq!(engine.status(), TimerStatus::Completed);
        assert_eq!(engine.remaining_secs(now + Duration::from_secs(20)), 0);
    }

    #[test]
    fn reset_returns_to_idle_full_duration() {
        let mut engine = TimerEngine::new(90);
        engine.start(t0());
        engine.pause(t0() + Duration::from_secs(30));
        engine.reset(t0() + Duration::from_secs(30));
        assert_eq!(engine.status(), TimerStatus::Idle);
        assert_eq!(engine.remaining_secs(t0()), 90);
    }

    #[test]
    fn restore_running_after_crash_completes_if_past_deadline() {
        let mut engine = TimerEngine::new(30);
        let started = t0();
        let deadline = started + Duration::from_secs(30);
        engine.restore_running(deadline, started + Duration::from_secs(45));
        assert_eq!(engine.status(), TimerStatus::Completed);
    }

    #[test]
    fn restore_running_keeps_remaining() {
        let mut engine = TimerEngine::new(30);
        let started = t0();
        let deadline = started + Duration::from_secs(30);
        engine.restore_running(deadline, started + Duration::from_secs(10));
        assert_eq!(engine.status(), TimerStatus::Running);
        assert_eq!(engine.remaining_secs(started + Duration::from_secs(10)), 20);
    }

    #[test]
    fn set_duration_updates_idle_timer() {
        let mut engine = TimerEngine::new(60);
        engine.set_duration(120);
        assert_eq!(engine.duration_secs(), 120);
        assert_eq!(engine.remaining_secs(t0()), 120);
    }

    #[test]
    fn set_duration_allows_zero() {
        let mut engine = TimerEngine::new(60);
        engine.set_duration(0);
        assert_eq!(engine.duration_secs(), 0);
        assert_eq!(engine.remaining_secs(t0()), 0);
        engine.start(t0());
        assert_eq!(engine.status(), TimerStatus::Idle);
    }

    #[test]
    fn toggle_pause_cycles_states() {
        let mut engine = TimerEngine::new(10);
        let now = t0();
        engine.toggle_pause(now);
        assert_eq!(engine.status(), TimerStatus::Running);
        engine.toggle_pause(now + Duration::from_secs(2));
        assert_eq!(engine.status(), TimerStatus::Paused);
        engine.toggle_pause(now + Duration::from_secs(3));
        assert_eq!(engine.status(), TimerStatus::Running);
    }

    #[test]
    fn stopwatch_start_pause_resume_preserves_elapsed() {
        let mut engine = TimerEngine::new(60);
        engine.set_mode(TimerMode::Stopwatch);
        let now = t0();
        engine.start(now);
        assert_eq!(engine.status(), TimerStatus::Running);
        assert_eq!(engine.elapsed_secs(now), 0);

        let later = now + Duration::from_secs(45);
        engine.pause(later);
        assert_eq!(engine.status(), TimerStatus::Paused);
        assert_eq!(engine.elapsed_secs(later), 45);

        let resume_at = later + Duration::from_secs(100);
        engine.resume(resume_at);
        assert_eq!(engine.status(), TimerStatus::Running);
        assert_eq!(engine.elapsed_secs(resume_at), 45);
        assert_eq!(engine.elapsed_secs(resume_at + Duration::from_secs(10)), 55);
    }

    #[test]
    fn stopwatch_tick_never_completes() {
        let mut engine = TimerEngine::new(60);
        engine.set_mode(TimerMode::Stopwatch);
        let now = t0();
        engine.start(now);
        assert!(engine.tick(now + Duration::from_secs(3600)).is_none());
        assert_eq!(engine.status(), TimerStatus::Running);
    }

    #[test]
    fn stopwatch_reset_clears_elapsed() {
        let mut engine = TimerEngine::new(60);
        engine.set_mode(TimerMode::Stopwatch);
        let now = t0();
        engine.start(now);
        engine.pause(now + Duration::from_secs(30));
        engine.reset(now + Duration::from_secs(30));
        assert_eq!(engine.status(), TimerStatus::Idle);
        assert_eq!(engine.elapsed_secs(now), 0);
    }

    #[test]
    fn set_mode_only_when_idle_or_completed() {
        let mut engine = TimerEngine::new(60);
        let now = t0();
        engine.start(now);
        engine.set_mode(TimerMode::Stopwatch);
        assert_eq!(engine.mode(), TimerMode::Timer);
    }

    #[test]
    fn set_mode_does_not_leave_waiting() {
        let mut engine = TimerEngine::new(25);
        let config = pomodoro_config();
        engine.set_mode(TimerMode::Pomodoro);
        engine.apply_idle_pomodoro_config(&config);
        let now = t0();
        engine.start(now);
        engine.tick(now + Duration::from_secs(25));
        engine.advance_after_completion(now + Duration::from_secs(25), &config);
        assert!(engine.waiting());
        engine.set_mode(TimerMode::Timer);
        assert_eq!(engine.mode(), TimerMode::Pomodoro);
        assert!(engine.waiting());
    }

    #[test]
    fn set_mode_resets_to_idle() {
        let mut engine = TimerEngine::new(60);
        engine.set_mode(TimerMode::Stopwatch);
        assert_eq!(engine.mode(), TimerMode::Stopwatch);
        assert_eq!(engine.status(), TimerStatus::Idle);
        assert_eq!(engine.elapsed_secs(t0()), 0);
    }

    #[test]
    fn pause_finalizes_a_finished_interval() {
        let mut engine = TimerEngine::new(120);
        let now = t0();
        engine.start(now);
        let finished = engine.pause(now + Duration::from_secs(30)).unwrap();
        assert_eq!(finished.mode, TimerMode::Timer);
        assert_eq!(finished.started_at, now);
        assert_eq!(finished.ended_at, now + Duration::from_secs(30));
    }

    #[test]
    fn pause_reports_a_finished_interval_even_for_a_short_tap() {
        // The engine itself always reports the interval that just ended;
        // withholding a same-second tap is `entries`' job (see
        // `entries::entry_from_interval`), not the engine's.
        let mut engine = TimerEngine::new(120);
        let now = t0();
        engine.start(now);
        let finished = engine.pause(now + Duration::from_secs(3)).unwrap();
        assert_eq!(finished.started_at, now);
        assert_eq!(finished.ended_at, now + Duration::from_secs(3));
    }

    #[test]
    fn entry_from_interval_records_when_end_is_after_start() {
        let mut engine = TimerEngine::new(120);
        let now = t0();

        engine.start(now);
        let same_second = engine.pause(now).unwrap();
        assert!(crate::entries::entry_from_interval(same_second).is_none());

        engine.resume(now);
        let one_second = engine.pause(now + Duration::from_secs(1)).unwrap();
        assert_eq!(
            crate::entries::entry_from_interval(one_second)
                .unwrap()
                .duration_secs,
            1
        );
    }

    #[test]
    fn resume_starts_a_fresh_interval_not_extending_the_previous_one() {
        let mut engine = TimerEngine::new(120);
        let now = t0();
        engine.start(now);
        let first = engine.pause(now + Duration::from_secs(20)).unwrap();
        assert_eq!(first.started_at, now);
        assert_eq!(first.ended_at, now + Duration::from_secs(20));

        let resume_at = now + Duration::from_secs(100);
        engine.resume(resume_at);
        let second = engine.pause(resume_at + Duration::from_secs(15)).unwrap();
        assert_eq!(second.started_at, resume_at);
        assert_eq!(second.ended_at, resume_at + Duration::from_secs(15));
    }

    #[test]
    fn cancel_while_running_finalizes_the_in_progress_interval() {
        let mut engine = TimerEngine::new(120);
        let now = t0();
        engine.start(now);
        let finished = engine.reset(now + Duration::from_secs(40)).unwrap();
        assert_eq!(finished.started_at, now);
        assert_eq!(finished.ended_at, now + Duration::from_secs(40));
        assert_eq!(engine.status(), TimerStatus::Idle);
    }

    #[test]
    fn cancel_while_paused_does_not_produce_a_second_entry() {
        let mut engine = TimerEngine::new(120);
        let now = t0();
        engine.start(now);
        engine.pause(now + Duration::from_secs(30));
        assert!(engine.reset(now + Duration::from_secs(9_000)).is_none());
    }

    #[test]
    fn cancel_while_idle_produces_no_entry() {
        let mut engine = TimerEngine::new(120);
        assert!(engine.reset(t0()).is_none());
    }

    #[test]
    fn discard_while_running_returns_to_idle_without_a_finished_interval() {
        let mut engine = TimerEngine::new(120);
        let now = t0();
        engine.start(now);
        engine.discard();
        assert_eq!(engine.status(), TimerStatus::Idle);
        assert_eq!(engine.remaining_secs(now), 120);
    }

    #[test]
    fn current_interval_elapsed_is_only_the_in_flight_stretch() {
        let mut engine = TimerEngine::new(120);
        let now = t0();
        engine.start(now);
        assert_eq!(
            engine.current_interval_elapsed_secs(now + Duration::from_secs(40)),
            40
        );

        engine.pause(now + Duration::from_secs(40));
        let resume_at = now + Duration::from_secs(100);
        engine.resume(resume_at);
        assert_eq!(
            engine.current_interval_elapsed_secs(resume_at + Duration::from_secs(5)),
            5
        );
    }

    #[test]
    fn natural_completion_finalizes_the_interval_at_the_deadline() {
        let mut engine = TimerEngine::new(20);
        let now = t0();
        engine.start(now);
        let finished = engine.tick(now + Duration::from_secs(20)).unwrap();
        assert_eq!(finished.mode, TimerMode::Timer);
        assert_eq!(finished.started_at, now);
        assert_eq!(finished.ended_at, now + Duration::from_secs(20));
    }

    #[test]
    fn natural_completion_of_a_short_timer_still_reports_completed() {
        // A 10-second Timer still transitions to Completed and reports its
        // finished interval — sound/notification must still fire — and the
        // Interval records as an Entry.
        let mut engine = TimerEngine::new(10);
        let now = t0();
        engine.start(now);
        let finished = engine.tick(now + Duration::from_secs(10)).unwrap();
        assert_eq!(engine.status(), TimerStatus::Completed);
        assert_eq!(
            crate::entries::entry_from_interval(finished)
                .unwrap()
                .duration_secs,
            10
        );
    }

    #[test]
    fn stopwatch_pause_finalizes_interval_from_actual_resume_time() {
        let mut engine = TimerEngine::new(60);
        engine.set_mode(TimerMode::Stopwatch);
        let now = t0();
        engine.start(now);
        let first = engine.pause(now + Duration::from_secs(45)).unwrap();
        assert_eq!(first.mode, TimerMode::Stopwatch);
        assert_eq!(first.started_at, now);
        assert_eq!(first.ended_at, now + Duration::from_secs(45));

        let resume_at = now + Duration::from_secs(500);
        engine.resume(resume_at);
        let second = engine.pause(resume_at + Duration::from_secs(15)).unwrap();
        assert_eq!(second.started_at, resume_at);
        assert_eq!(second.ended_at, resume_at + Duration::from_secs(15));
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
    fn timer_split_at_midnight_keeps_running_and_remaining() {
        let mut engine = TimerEngine::new(4 * 60 * 60);
        let start = local_dt(2026, 6, 15, 23, 0, 0);
        let after = local_dt(2026, 6, 16, 0, 30, 0);
        let midnight = local_dt(2026, 6, 16, 0, 0, 0);
        engine.start(start);

        let remaining_before = engine.remaining_secs(after);
        let finished = engine.split_crossed_midnight(after);
        assert_eq!(finished.len(), 1);
        assert_eq!(finished[0].mode, TimerMode::Timer);
        assert_eq!(finished[0].started_at, start);
        assert_eq!(finished[0].ended_at, midnight);

        assert_eq!(engine.status(), TimerStatus::Running);
        assert_eq!(engine.remaining_secs(after), remaining_before);
        assert_eq!(engine.current_interval_elapsed_secs(after), 30 * 60);

        let post = engine.pause(after).unwrap();
        assert_eq!(post.started_at, midnight);
        assert_eq!(post.ended_at, after);
    }

    #[test]
    fn stopwatch_split_at_midnight_keeps_total_elapsed() {
        let mut engine = TimerEngine::new(60);
        engine.set_mode(TimerMode::Stopwatch);
        let start = local_dt(2026, 6, 15, 23, 0, 0);
        let after = local_dt(2026, 6, 16, 0, 30, 0);
        let midnight = local_dt(2026, 6, 16, 0, 0, 0);
        engine.start(start);

        let elapsed_before = engine.elapsed_secs(after);
        let finished = engine.split_crossed_midnight(after);
        assert_eq!(finished.len(), 1);
        assert_eq!(finished[0].mode, TimerMode::Stopwatch);
        assert_eq!(finished[0].started_at, start);
        assert_eq!(finished[0].ended_at, midnight);

        assert_eq!(engine.status(), TimerStatus::Running);
        assert_eq!(engine.elapsed_secs(after), elapsed_before);
        assert_eq!(elapsed_before, 90 * 60);
        assert_eq!(engine.current_interval_elapsed_secs(after), 30 * 60);

        // Restart must keep the post-midnight boundary, or the next tick
        // would record yesterday a second time.
        let mut restored = TimerEngine::new(60);
        restored.restore_stopwatch_running(engine.started_at().unwrap(), engine.elapsed_at_pause());
        assert_eq!(restored.elapsed_secs(after), elapsed_before);
        assert_eq!(restored.current_interval_elapsed_secs(after), 30 * 60);
        assert!(restored.split_crossed_midnight(after).is_empty());

        let post = engine.pause(after).unwrap();
        assert_eq!(post.started_at, midnight);
        assert_eq!(post.ended_at, after);
    }

    #[test]
    fn stopwatch_split_after_resume_keeps_total_elapsed() {
        let mut engine = TimerEngine::new(60);
        engine.set_mode(TimerMode::Stopwatch);
        engine.start(local_dt(2026, 6, 15, 22, 0, 0));
        engine.pause(local_dt(2026, 6, 15, 22, 30, 0));
        let resume_at = local_dt(2026, 6, 15, 23, 30, 0);
        engine.resume(resume_at);
        let after = local_dt(2026, 6, 16, 0, 10, 0);

        let elapsed_before = engine.elapsed_secs(after);
        let finished = engine.split_crossed_midnight(after);
        assert_eq!(finished.len(), 1);
        assert_eq!(finished[0].started_at, resume_at);
        assert_eq!(finished[0].ended_at, local_dt(2026, 6, 16, 0, 0, 0));
        assert_eq!(engine.status(), TimerStatus::Running);
        assert_eq!(engine.elapsed_secs(after), elapsed_before);
        assert_eq!(elapsed_before, 70 * 60);
        assert_eq!(engine.current_interval_elapsed_secs(after), 10 * 60);
    }

    #[test]
    fn subsecond_start_splits_once_and_lands_on_the_new_day() {
        let mut engine = TimerEngine::new(60);
        engine.set_mode(TimerMode::Stopwatch);
        let start = local_dt(2026, 6, 15, 23, 30, 0) + Duration::from_millis(400);
        let after = local_dt(2026, 6, 16, 0, 0, 2);
        let midnight = local_dt(2026, 6, 16, 0, 0, 0);
        engine.start(start);

        let elapsed_before = engine.elapsed_secs(after);
        assert_eq!(engine.split_crossed_midnight(after).len(), 1);
        assert_eq!(engine.elapsed_secs(after), elapsed_before);
        assert!(engine.split_crossed_midnight(after).is_empty());
        let interval_start = engine.interval_in_progress(after).unwrap().started_at;
        assert!(interval_start >= midnight);
        assert!(interval_start < midnight + Duration::from_secs(1));
    }

    #[test]
    fn short_pre_midnight_portion_records_and_new_interval_starts() {
        let mut engine = TimerEngine::new(60 * 60);
        let start = local_dt(2026, 6, 15, 23, 59, 50);
        let after = local_dt(2026, 6, 16, 0, 0, 5);
        let midnight = local_dt(2026, 6, 16, 0, 0, 0);
        engine.start(start);

        let finished = engine.split_crossed_midnight(after);
        assert_eq!(finished.len(), 1);
        assert_eq!(
            crate::entries::entry_from_interval(finished[0])
                .unwrap()
                .duration_secs,
            10
        );
        assert_eq!(engine.status(), TimerStatus::Running);
        assert_eq!(engine.current_interval_elapsed_secs(after), 5);

        let post = engine.pause(after).unwrap();
        assert_eq!(post.started_at, midnight);
    }

    #[test]
    fn sleep_through_midnight_splits_retroactively() {
        let mut engine = TimerEngine::new(8 * 60 * 60);
        let start = local_dt(2026, 6, 15, 22, 0, 0);
        let after = local_dt(2026, 6, 16, 2, 30, 0);
        let midnight = local_dt(2026, 6, 16, 0, 0, 0);
        engine.start(start);

        let remaining_before = engine.remaining_secs(after);
        let finished = engine.split_crossed_midnight(after);
        assert_eq!(finished.len(), 1);
        assert_eq!(finished[0].started_at, start);
        assert_eq!(finished[0].ended_at, midnight);
        assert!(crate::entries::entry_from_interval(finished[0]).is_some());

        assert_eq!(engine.status(), TimerStatus::Running);
        assert_eq!(engine.remaining_secs(after), remaining_before);
        assert_eq!(engine.current_interval_elapsed_secs(midnight), 0);
        assert_eq!(
            engine.current_interval_elapsed_secs(after),
            2 * 60 * 60 + 30 * 60
        );
        assert!(engine.split_crossed_midnight(after).is_empty());
    }

    #[test]
    fn sleep_across_two_midnights_records_each_calendar_day() {
        let mut engine = TimerEngine::new(60);
        engine.set_mode(TimerMode::Stopwatch);
        let start = local_dt(2026, 6, 15, 22, 0, 0);
        let after = local_dt(2026, 6, 17, 1, 0, 0);
        engine.start(start);

        let elapsed_before = engine.elapsed_secs(after);
        let finished = engine.split_crossed_midnight(after);
        assert_eq!(finished.len(), 2);
        assert_eq!(finished[0].started_at, start);
        assert_eq!(finished[0].ended_at, local_dt(2026, 6, 16, 0, 0, 0));
        assert_eq!(finished[1].started_at, local_dt(2026, 6, 16, 0, 0, 0));
        assert_eq!(finished[1].ended_at, local_dt(2026, 6, 17, 0, 0, 0));
        assert_eq!(engine.status(), TimerStatus::Running);
        assert_eq!(engine.elapsed_secs(after), elapsed_before);
        assert_eq!(engine.current_interval_elapsed_secs(after), 60 * 60);
        assert_eq!(
            engine.interval_in_progress(after).unwrap().started_at,
            local_dt(2026, 6, 17, 0, 0, 0)
        );
    }

    #[test]
    fn split_does_nothing_when_not_running_or_still_the_same_day() {
        let mut engine = TimerEngine::new(60 * 60);
        let start = local_dt(2026, 6, 15, 10, 0, 0);
        engine.start(start);
        assert!(engine
            .split_crossed_midnight(local_dt(2026, 6, 15, 12, 0, 0))
            .is_empty());

        engine.pause(local_dt(2026, 6, 15, 12, 0, 0));
        assert!(engine
            .split_crossed_midnight(local_dt(2026, 6, 16, 1, 0, 0))
            .is_empty());
    }

    #[test]
    fn split_before_completion_does_not_record_past_the_deadline() {
        let mut engine = TimerEngine::new(30 * 60);
        let start = local_dt(2026, 6, 15, 23, 50, 0);
        let deadline = start + Duration::from_secs(30 * 60);
        let now = local_dt(2026, 6, 16, 0, 40, 0);
        engine.start(start);

        let finished = engine.split_crossed_midnight(now);
        let completed = engine.tick(now).unwrap();
        assert_eq!(finished.len(), 1);
        assert_eq!(finished[0].ended_at, local_dt(2026, 6, 16, 0, 0, 0));
        assert_eq!(completed.started_at, local_dt(2026, 6, 16, 0, 0, 0));
        assert_eq!(completed.ended_at, deadline);
        assert_eq!(engine.status(), TimerStatus::Completed);
    }

    #[test]
    fn pre_midnight_piece_of_eleven_seconds_is_recorded() {
        let mut engine = TimerEngine::new(60 * 60);
        let start = local_dt(2026, 6, 15, 23, 59, 49);
        let midnight = local_dt(2026, 6, 16, 0, 0, 0);
        let after = local_dt(2026, 6, 16, 0, 0, 5);
        engine.start(start);

        let finished = engine.split_crossed_midnight(after);
        assert_eq!(finished.len(), 1);
        assert_eq!(finished[0].started_at, start);
        assert_eq!(finished[0].ended_at, midnight);
        let entry = crate::entries::entry_from_interval(finished[0]).unwrap();
        assert_eq!(entry.duration_secs, 11);
        assert_eq!(engine.status(), TimerStatus::Running);
        assert_eq!(engine.current_interval_elapsed_secs(after), 5);
    }

    #[test]
    fn timer_that_completes_at_midnight_records_through_that_midnight() {
        let mut engine = TimerEngine::new(60 * 60);
        let start = local_dt(2026, 6, 15, 23, 0, 0);
        let midnight = local_dt(2026, 6, 16, 0, 0, 0);
        engine.start(start);

        let finished = engine.split_crossed_midnight(midnight);
        assert_eq!(finished.len(), 1);
        assert_eq!(finished[0].started_at, start);
        assert_eq!(finished[0].ended_at, midnight);
        let entry = crate::entries::entry_from_interval(finished[0]).unwrap();
        assert_eq!(entry.duration_secs, 3600);

        let completed = engine.tick(midnight).unwrap();
        assert_eq!(completed.started_at, midnight);
        assert_eq!(completed.ended_at, midnight);
        assert!(crate::entries::entry_from_interval(completed).is_none());
        assert_eq!(engine.status(), TimerStatus::Completed);
    }

    fn pomodoro_config() -> PomodoroConfig {
        PomodoroConfig {
            session_secs: 25,
            short_break_secs: 5,
            long_break_secs: 15,
            sessions_until_long_break: 4,
            auto_start_sessions: false,
            auto_start_breaks: false,
        }
    }

    #[test]
    fn pomodoro_session_counts_down_and_records() {
        let mut engine = TimerEngine::new(25);
        engine.set_mode(TimerMode::Pomodoro);
        engine.apply_idle_pomodoro_config(&pomodoro_config());
        let now = t0();
        engine.start(now);
        let finished = engine.tick(now + Duration::from_secs(25)).unwrap();
        assert_eq!(finished.mode, TimerMode::Pomodoro);
        assert!(finished.records_entry);
        assert_eq!(engine.status(), TimerStatus::Completed);
        assert!(crate::entries::entry_from_interval(finished).is_some());
    }

    #[test]
    fn pomodoro_session_completion_advances_to_short_break() {
        let mut engine = TimerEngine::new(25);
        let config = pomodoro_config();
        engine.set_mode(TimerMode::Pomodoro);
        engine.apply_idle_pomodoro_config(&config);
        let now = t0();
        engine.start(now);
        engine.tick(now + Duration::from_secs(25));
        engine.advance_after_completion(now + Duration::from_secs(25), &config);
        assert_eq!(engine.pomodoro_phase(), PomodoroPhase::ShortBreak);
        assert_eq!(engine.pomodoro_completed_sessions(), 1);
        assert_eq!(engine.duration_secs(), 5);
        assert_eq!(engine.status(), TimerStatus::Idle);
        assert!(engine.waiting());
    }

    #[test]
    fn pomodoro_fourth_session_advances_to_long_break() {
        let mut engine = TimerEngine::new(25);
        let config = pomodoro_config();
        engine.set_mode(TimerMode::Pomodoro);
        engine.apply_idle_pomodoro_config(&config);
        engine.restore_pomodoro_cycle(PomodoroPhase::Session, 3, 4);
        let now = t0();
        engine.start(now);
        engine.tick(now + Duration::from_secs(25));
        engine.advance_after_completion(now + Duration::from_secs(25), &config);
        assert_eq!(engine.pomodoro_phase(), PomodoroPhase::LongBreak);
        assert_eq!(engine.pomodoro_completed_sessions(), 4);
        assert_eq!(engine.duration_secs(), 15);
        assert!(engine.waiting());
    }

    #[test]
    fn pomodoro_break_does_not_record_and_skip_keeps_cycle() {
        let mut engine = TimerEngine::new(25);
        let config = pomodoro_config();
        engine.set_mode(TimerMode::Pomodoro);
        engine.restore_pomodoro_cycle(PomodoroPhase::ShortBreak, 2, 4);
        engine.apply_idle_pomodoro_config(&config);
        let now = t0();
        engine.start(now);
        let finished = engine.pause(now + Duration::from_secs(3)).unwrap();
        assert!(!finished.records_entry);
        assert!(crate::entries::entry_from_interval(finished).is_none());

        engine.resume(now + Duration::from_secs(3));
        assert!(engine.skip_break(&config));
        assert_eq!(engine.pomodoro_phase(), PomodoroPhase::Session);
        assert_eq!(engine.pomodoro_completed_sessions(), 2);
        assert_eq!(engine.duration_secs(), 25);
        assert_eq!(engine.status(), TimerStatus::Idle);
        assert!(engine.waiting());
    }

    #[test]
    fn pomodoro_skip_while_waiting_on_a_break_jumps_to_the_next_session() {
        let mut engine = TimerEngine::new(25);
        let config = pomodoro_config();
        engine.set_mode(TimerMode::Pomodoro);
        engine.apply_idle_pomodoro_config(&config);
        let now = t0();
        engine.start(now);
        engine.tick(now + Duration::from_secs(25));
        engine.advance_after_completion(now + Duration::from_secs(25), &config);
        assert_eq!(engine.pomodoro_phase(), PomodoroPhase::ShortBreak);
        assert!(engine.waiting());
        assert!(engine.skip_break(&config));
        assert_eq!(engine.pomodoro_phase(), PomodoroPhase::Session);
        assert_eq!(engine.pomodoro_completed_sessions(), 1);
        assert_eq!(engine.duration_secs(), 25);
        assert_eq!(engine.status(), TimerStatus::Idle);
        assert!(engine.waiting());
    }

    #[test]
    fn pomodoro_skip_of_a_long_break_while_waiting_is_session_one_waiting() {
        let mut engine = TimerEngine::new(25);
        let config = pomodoro_config();
        engine.set_mode(TimerMode::Pomodoro);
        engine.restore_pomodoro_cycle(PomodoroPhase::LongBreak, 4, 4);
        engine.apply_idle_pomodoro_config(&config);
        engine.restore_waiting(true);
        assert!(engine.skip_break(&config));
        assert_eq!(engine.pomodoro_phase(), PomodoroPhase::Session);
        assert_eq!(engine.pomodoro_completed_sessions(), 0);
        assert_eq!(engine.status(), TimerStatus::Idle);
        assert!(engine.waiting());
    }

    #[test]
    fn pomodoro_skip_does_not_auto_start_the_next_session() {
        let mut engine = TimerEngine::new(25);
        let config = PomodoroConfig {
            auto_start_sessions: true,
            ..pomodoro_config()
        };
        engine.set_mode(TimerMode::Pomodoro);
        engine.restore_pomodoro_cycle(PomodoroPhase::ShortBreak, 1, 4);
        engine.apply_idle_pomodoro_config(&config);
        let now = t0();
        engine.start(now);
        assert!(engine.skip_break(&config));
        assert_eq!(engine.pomodoro_phase(), PomodoroPhase::Session);
        assert_eq!(engine.status(), TimerStatus::Idle);
        assert!(engine.waiting());
    }

    #[test]
    fn pomodoro_cancel_leaves_waiting() {
        let mut engine = TimerEngine::new(25);
        let config = pomodoro_config();
        engine.set_mode(TimerMode::Pomodoro);
        engine.apply_idle_pomodoro_config(&config);
        let now = t0();
        engine.start(now);
        engine.tick(now + Duration::from_secs(25));
        engine.advance_after_completion(now + Duration::from_secs(25), &config);
        assert!(engine.waiting());
        engine.reset_cycle(&config);
        assert_eq!(engine.pomodoro_phase(), PomodoroPhase::Session);
        assert_eq!(engine.pomodoro_completed_sessions(), 0);
        assert_eq!(engine.status(), TimerStatus::Idle);
        assert!(!engine.waiting());
    }

    #[test]
    fn pomodoro_start_from_waiting_clears_waiting() {
        let mut engine = TimerEngine::new(25);
        let config = pomodoro_config();
        engine.set_mode(TimerMode::Pomodoro);
        engine.apply_idle_pomodoro_config(&config);
        let now = t0();
        engine.start(now);
        engine.tick(now + Duration::from_secs(25));
        engine.advance_after_completion(now + Duration::from_secs(25), &config);
        assert!(engine.waiting());
        engine.start(now + Duration::from_secs(25));
        assert_eq!(engine.status(), TimerStatus::Running);
        assert!(!engine.waiting());
    }

    #[test]
    fn pomodoro_cancel_resets_cycle_to_session_one() {
        let mut engine = TimerEngine::new(25);
        let config = pomodoro_config();
        engine.set_mode(TimerMode::Pomodoro);
        engine.restore_pomodoro_cycle(PomodoroPhase::ShortBreak, 2, 4);
        engine.apply_idle_pomodoro_config(&config);
        engine.reset_cycle(&config);
        assert_eq!(engine.pomodoro_phase(), PomodoroPhase::Session);
        assert_eq!(engine.pomodoro_completed_sessions(), 0);
        assert_eq!(engine.duration_secs(), 25);
        assert_eq!(engine.status(), TimerStatus::Idle);
        assert!(!engine.waiting());
    }

    #[test]
    fn pomodoro_short_session_records_and_advances_the_cycle() {
        let mut engine = TimerEngine::new(5);
        let config = PomodoroConfig {
            session_secs: 5,
            ..pomodoro_config()
        };
        engine.set_mode(TimerMode::Pomodoro);
        engine.apply_idle_pomodoro_config(&config);
        let now = t0();
        engine.start(now);
        let finished = engine.tick(now + Duration::from_secs(5)).unwrap();
        let entry = crate::entries::entry_from_interval(finished).unwrap();
        assert_eq!(entry.duration_secs, 5);
        assert_eq!(entry.mode, crate::entries::EntryType::Pomodoro);
        engine.advance_after_completion(now + Duration::from_secs(5), &config);
        assert_eq!(engine.pomodoro_completed_sessions(), 1);
        assert_eq!(engine.pomodoro_phase(), PomodoroPhase::ShortBreak);
    }

    #[test]
    fn pomodoro_set_mode_to_timer_preserves_cycle() {
        let mut engine = TimerEngine::new(1500);
        let config = pomodoro_config();
        engine.set_mode(TimerMode::Pomodoro);
        engine.restore_pomodoro_cycle(PomodoroPhase::ShortBreak, 2, 4);
        engine.apply_idle_pomodoro_config(&config);
        engine.set_mode(TimerMode::Timer);
        assert_eq!(engine.mode(), TimerMode::Timer);
        assert_eq!(engine.duration_secs(), 1500);
        engine.set_mode(TimerMode::Pomodoro);
        engine.apply_idle_pomodoro_config(&config);
        assert_eq!(engine.pomodoro_phase(), PomodoroPhase::ShortBreak);
        assert_eq!(engine.pomodoro_completed_sessions(), 2);
        assert_eq!(engine.duration_secs(), 5);
    }

    #[test]
    fn pomodoro_auto_start_break_starts_the_next_phase() {
        let mut engine = TimerEngine::new(25);
        let config = PomodoroConfig {
            auto_start_breaks: true,
            ..pomodoro_config()
        };
        engine.set_mode(TimerMode::Pomodoro);
        engine.apply_idle_pomodoro_config(&config);
        let now = t0();
        engine.start(now);
        engine.tick(now + Duration::from_secs(25));
        engine.advance_after_completion(now + Duration::from_secs(25), &config);
        assert_eq!(engine.pomodoro_phase(), PomodoroPhase::ShortBreak);
        assert_eq!(engine.status(), TimerStatus::Running);
        assert!(!engine.waiting());
    }

    #[test]
    fn pomodoro_long_break_after_applies_on_a_fresh_cycle() {
        let mut engine = TimerEngine::new(25);
        let mut config = pomodoro_config();
        engine.set_mode(TimerMode::Pomodoro);
        engine.apply_idle_pomodoro_config(&config);
        assert_eq!(engine.pomodoro_sessions_until_long_break(), 4);
        config.sessions_until_long_break = 6;
        engine.apply_idle_pomodoro_config(&config);
        assert_eq!(engine.pomodoro_sessions_until_long_break(), 6);

        engine.restore_pomodoro_cycle(PomodoroPhase::Session, 2, 4);
        config.sessions_until_long_break = 8;
        engine.apply_idle_pomodoro_config(&config);
        assert_eq!(engine.pomodoro_sessions_until_long_break(), 4);
    }

    #[test]
    fn pomodoro_long_break_completion_resets_cycle() {
        let mut engine = TimerEngine::new(15);
        let config = pomodoro_config();
        engine.set_mode(TimerMode::Pomodoro);
        engine.restore_pomodoro_cycle(PomodoroPhase::LongBreak, 4, 4);
        engine.apply_idle_pomodoro_config(&config);
        let now = t0();
        engine.start(now);
        let finished = engine.tick(now + Duration::from_secs(15)).unwrap();
        assert!(!finished.records_entry);
        engine.advance_after_completion(now + Duration::from_secs(15), &config);
        assert_eq!(engine.pomodoro_phase(), PomodoroPhase::Session);
        assert_eq!(engine.pomodoro_completed_sessions(), 0);
        assert_eq!(engine.duration_secs(), 25);
        assert_eq!(engine.status(), TimerStatus::Idle);
        assert!(engine.waiting());
    }

    #[test]
    fn pomodoro_break_midnight_split_does_not_record() {
        let mut engine = TimerEngine::new(4 * 60 * 60);
        engine.set_mode(TimerMode::Pomodoro);
        engine.restore_pomodoro_cycle(PomodoroPhase::ShortBreak, 1, 4);
        engine.set_duration(4 * 60 * 60);
        let start = local_dt(2026, 6, 15, 23, 0, 0);
        let after = local_dt(2026, 6, 16, 0, 30, 0);
        engine.start(start);
        let finished = engine.split_crossed_midnight(after);
        assert_eq!(finished.len(), 1);
        assert!(!finished[0].records_entry);
        assert!(crate::entries::entry_from_interval(finished[0]).is_none());
        assert_eq!(engine.status(), TimerStatus::Running);
    }

    #[test]
    fn pomodoro_short_break_completion_is_waiting_on_the_next_session() {
        let mut engine = TimerEngine::new(5);
        let config = pomodoro_config();
        engine.set_mode(TimerMode::Pomodoro);
        engine.restore_pomodoro_cycle(PomodoroPhase::ShortBreak, 1, 4);
        engine.apply_idle_pomodoro_config(&config);
        let now = t0();
        engine.start(now);
        engine.tick(now + Duration::from_secs(5));
        engine.advance_after_completion(now + Duration::from_secs(5), &config);
        assert_eq!(engine.pomodoro_phase(), PomodoroPhase::Session);
        assert_eq!(engine.pomodoro_completed_sessions(), 1);
        assert_eq!(engine.duration_secs(), 25);
        assert_eq!(engine.status(), TimerStatus::Idle);
        assert!(engine.waiting());
    }
}
