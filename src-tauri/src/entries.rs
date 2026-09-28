use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Datelike, Local, NaiveDate, TimeZone};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::timer::{FinishedInterval, TimerMode};

/// How an Entry was produced: copied from the engine (Timer/Stopwatch) or
/// created from Stats (Manual). Serialized as `mode` so existing files
/// keep loading.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EntryType {
    Timer,
    Stopwatch,
    Manual,
}

impl From<TimerMode> for EntryType {
    fn from(mode: TimerMode) -> Self {
        match mode {
            TimerMode::Timer => EntryType::Timer,
            TimerMode::Stopwatch => EntryType::Stopwatch,
        }
    }
}

/// A contiguous stretch of focused time, bounded by a start and an end.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    pub mode: EntryType,
    pub started_at_unix: u64,
    pub ended_at_unix: u64,
    pub duration_secs: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub today: u64,
    pub this_week: u64,
    pub this_month: u64,
}

/// A run of `MIN_ENTRY_DURATION_SECS` or less is an accidental tap (e.g.
/// Start immediately followed by Pause), not a real focus session.
const MIN_ENTRY_DURATION_SECS: u64 = 10;

/// Turns a just-finished engine interval into an Entry ready to persist,
/// or `None` if it's at or under the 10-second minimum.
pub fn entry_from_interval(interval: FinishedInterval) -> Option<Entry> {
    let started_at_unix = interval
        .started_at
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs();
    let ended_at_unix = interval.ended_at.duration_since(UNIX_EPOCH).ok()?.as_secs();
    let duration_secs = ended_at_unix.saturating_sub(started_at_unix);
    if duration_secs <= MIN_ENTRY_DURATION_SECS {
        return None;
    }
    Some(Entry {
        id: Uuid::new_v4().to_string(),
        mode: interval.mode.into(),
        started_at_unix,
        ended_at_unix,
        duration_secs,
    })
}

fn to_local(unix_secs: u64) -> DateTime<Local> {
    Local
        .timestamp_opt(unix_secs as i64, 0)
        .single()
        .unwrap_or_else(Local::now)
}

fn local_date_of(unix_secs: u64) -> Option<NaiveDate> {
    let secs = i64::try_from(unix_secs).ok()?;
    Local
        .timestamp_opt(secs, 0)
        .earliest()
        .map(|dt| dt.date_naive())
}

fn unix_at_local_hms(date: NaiveDate, hour: u32, min: u32, sec: u32) -> Option<u64> {
    let naive = date.and_hms_opt(hour, min, sec)?;
    let local = Local.from_local_datetime(&naive).single()?;
    let timestamp = local.timestamp();
    if timestamp < 0 {
        return None;
    }
    u64::try_from(timestamp).ok()
}

/// One `(start, end)` per local Calendar day. The earlier piece ends at the
/// next local 00:00:00; the next piece starts at that same instant. A range
/// that ends exactly at that midnight is one piece. If a local midnight
/// cannot be resolved, the remaining range is kept unsplit.
fn split_range_at_local_midnights(started_at_unix: u64, ended_at_unix: u64) -> Vec<(u64, u64)> {
    if ended_at_unix <= started_at_unix {
        return Vec::new();
    }
    let Some(end_date) = local_date_of(ended_at_unix) else {
        return vec![(started_at_unix, ended_at_unix)];
    };

    let mut pieces = Vec::new();
    let mut segment_start = started_at_unix;
    while let Some(segment_date) = local_date_of(segment_start) {
        if segment_date >= end_date {
            break;
        }
        let Some(next_date) = segment_date.succ_opt() else {
            break;
        };
        let Some(next_midnight) = unix_at_local_hms(next_date, 0, 0, 0) else {
            break;
        };
        if next_midnight > ended_at_unix || next_midnight <= segment_start {
            break;
        }
        pieces.push((segment_start, next_midnight));
        segment_start = next_midnight;
    }
    if ended_at_unix > segment_start {
        pieces.push((segment_start, ended_at_unix));
    }
    if pieces.is_empty() {
        vec![(started_at_unix, ended_at_unix)]
    } else {
        pieces
    }
}

fn entry_from_range(
    id: String,
    mode: EntryType,
    started_at_unix: u64,
    ended_at_unix: u64,
) -> Entry {
    Entry {
        id,
        mode,
        started_at_unix,
        ended_at_unix,
        duration_secs: ended_at_unix - started_at_unix,
    }
}

/// Sums Entry durations into Today/This-Week/This-Month buckets, bucketed
/// by each Entry's start time using local-timezone calendar boundaries
/// (day is midnight-to-midnight, week starts Monday, month is calendar
/// month), for the given instant.
pub fn compute_totals(entries: &[Entry], now: SystemTime) -> Totals {
    let now_unix = now.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let now_local = to_local(now_unix).naive_local();

    let today_start = now_local.date().and_hms_opt(0, 0, 0).unwrap();
    let days_since_monday = now_local.date().weekday().num_days_from_monday();
    let week_start = today_start - chrono::Duration::days(days_since_monday as i64);
    let month_start = now_local
        .date()
        .with_day(1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    let mut totals = Totals::default();
    for entry in entries {
        let started_local = to_local(entry.started_at_unix).naive_local();
        if started_local >= today_start {
            totals.today += entry.duration_secs;
        }
        if started_local >= week_start {
            totals.this_week += entry.duration_secs;
        }
        if started_local >= month_start {
            totals.this_month += entry.duration_secs;
        }
    }
    totals
}

pub struct EntriesStore {
    path: PathBuf,
}

impl EntriesStore {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self {
            path: app_data_dir.join("entries.json"),
        }
    }

    pub fn load_all(&self) -> Vec<Entry> {
        let Ok(bytes) = fs::read(&self.path) else {
            return Vec::new();
        };
        serde_json::from_slice(&bytes).unwrap_or_default()
    }

    pub fn append(&self, entry: Entry) -> Result<(), String> {
        let mut entries = self.load_all();
        entries.push(entry);
        self.write_all(&entries)
    }

    pub fn totals(&self, now: SystemTime) -> Totals {
        compute_totals(&self.load_all(), now)
    }

    /// All Entries, newest-first.
    pub fn load_all_newest_first(&self) -> Vec<Entry> {
        let mut entries = self.load_all();
        entries.reverse();
        entries
    }

    /// Corrects an Entry's start and end. Duration is derived. Mode is
    /// unchanged. Any duration is allowed as long as end is after start.
    /// A range that crosses local midnight is split: this Entry becomes
    /// the earliest piece and later pieces are appended.
    pub fn update_times(
        &self,
        id: &str,
        started_at_unix: u64,
        ended_at_unix: u64,
    ) -> Result<Entry, String> {
        if ended_at_unix <= started_at_unix {
            return Err("invalid range".into());
        }
        let pieces = split_range_at_local_midnights(started_at_unix, ended_at_unix);
        let mut entries = self.load_all();
        let Some(index) = entries.iter().position(|entry| entry.id == id) else {
            return Err("not found".into());
        };
        let mode = entries[index].mode;
        let Some(&(first_start, first_end)) = pieces.first() else {
            return Err("invalid range".into());
        };
        entries[index] = entry_from_range(id.to_string(), mode, first_start, first_end);
        let updated = entries[index].clone();
        for &(started, ended) in pieces.iter().skip(1) {
            entries.push(entry_from_range(
                Uuid::new_v4().to_string(),
                mode,
                started,
                ended,
            ));
        }
        self.write_all(&entries)?;
        Ok(updated)
    }

    /// Persists a Manual entry. Any duration is allowed as long as end is
    /// after start; the 10-second recording skip does not apply. A range
    /// that crosses local midnight is split into one Entry per Calendar
    /// day, appended earliest-first.
    pub fn create_manual(&self, started_at_unix: u64, ended_at_unix: u64) -> Result<Entry, String> {
        if ended_at_unix <= started_at_unix {
            return Err("invalid range".into());
        }
        let pieces = split_range_at_local_midnights(started_at_unix, ended_at_unix);
        let created: Vec<Entry> = pieces
            .into_iter()
            .map(|(started, ended)| {
                entry_from_range(
                    Uuid::new_v4().to_string(),
                    EntryType::Manual,
                    started,
                    ended,
                )
            })
            .collect();
        let Some(first) = created.first().cloned() else {
            return Err("invalid range".into());
        };
        let mut entries = self.load_all();
        entries.extend(created);
        self.write_all(&entries)?;
        Ok(first)
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let mut entries = self.load_all();
        let before = entries.len();
        entries.retain(|entry| entry.id != id);
        if entries.len() == before {
            return Err("not found".into());
        }
        self.write_all(&entries)
    }

    pub fn delete_all(&self) -> Result<(), String> {
        self.write_all(&[])
    }

    fn write_all(&self, entries: &[Entry]) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(entries).map_err(|e| e.to_string())?;
        crate::atomic_file::write_json(&self.path, &json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "focus-timer-entries-test-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn entry_from_interval_computes_duration_and_a_fresh_id() {
        let started = SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000_000);
        let interval = FinishedInterval {
            mode: TimerMode::Timer,
            started_at: started,
            ended_at: started + Duration::from_secs(90),
        };
        let entry = entry_from_interval(interval).unwrap();
        assert_eq!(entry.mode, EntryType::Timer);
        assert_eq!(entry.started_at_unix, 2_000_000_000);
        assert_eq!(entry.ended_at_unix, 2_000_000_090);
        assert_eq!(entry.duration_secs, 90);
        assert!(Uuid::parse_str(&entry.id).is_ok());
    }

    #[test]
    fn entry_from_interval_withholds_at_or_under_ten_seconds() {
        let started = SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000_000);
        let ten_seconds = FinishedInterval {
            mode: TimerMode::Timer,
            started_at: started,
            ended_at: started + Duration::from_secs(10),
        };
        assert!(entry_from_interval(ten_seconds).is_none());

        let eleven_seconds = FinishedInterval {
            mode: TimerMode::Timer,
            started_at: started,
            ended_at: started + Duration::from_secs(11),
        };
        assert!(entry_from_interval(eleven_seconds).is_some());
    }

    #[test]
    fn roundtrip_save_and_load() {
        let dir = temp_dir("roundtrip");
        fs::create_dir_all(&dir).unwrap();
        let store = EntriesStore::new(dir.clone());

        let entry = Entry {
            id: "entry-1".into(),
            mode: EntryType::Stopwatch,
            started_at_unix: 1_700_000_000,
            ended_at_unix: 1_700_000_060,
            duration_secs: 60,
        };
        store.append(entry.clone()).unwrap();

        let loaded = store.load_all();
        assert_eq!(loaded, vec![entry]);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn append_preserves_existing_entries_and_appends_atomically() {
        let dir = temp_dir("append");
        fs::create_dir_all(&dir).unwrap();
        let store = EntriesStore::new(dir.clone());

        let first = Entry {
            id: "first".into(),
            mode: EntryType::Timer,
            started_at_unix: 100,
            ended_at_unix: 200,
            duration_secs: 100,
        };
        let second = Entry {
            id: "second".into(),
            mode: EntryType::Timer,
            started_at_unix: 300,
            ended_at_unix: 400,
            duration_secs: 100,
        };
        store.append(first.clone()).unwrap();
        store.append(second.clone()).unwrap();

        assert_eq!(store.load_all(), vec![first, second]);
        assert!(!dir.join("entries.json.tmp").exists());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn load_all_newest_first_reverses_append_order() {
        let dir = temp_dir("newest-first");
        fs::create_dir_all(&dir).unwrap();
        let store = EntriesStore::new(dir.clone());

        let first = Entry {
            id: "first".into(),
            mode: EntryType::Timer,
            started_at_unix: 100,
            ended_at_unix: 200,
            duration_secs: 100,
        };
        let second = Entry {
            id: "second".into(),
            mode: EntryType::Timer,
            started_at_unix: 300,
            ended_at_unix: 400,
            duration_secs: 100,
        };
        store.append(first.clone()).unwrap();
        store.append(second.clone()).unwrap();

        assert_eq!(store.load_all_newest_first(), vec![second, first]);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn load_all_on_missing_file_returns_empty() {
        let dir = temp_dir("missing");
        let store = EntriesStore::new(dir);
        assert_eq!(store.load_all(), Vec::new());
    }

    fn unix_at_local(y: i32, m: u32, d: u32, h: u32, min: u32, s: u32) -> u64 {
        Local
            .with_ymd_and_hms(y, m, d, h, min, s)
            .unwrap()
            .timestamp() as u64
    }

    #[test]
    fn totals_bucket_by_calendar_day_week_and_month() {
        // "Now" is Wednesday 2024-01-17, 12:00 local.
        let now =
            SystemTime::UNIX_EPOCH + Duration::from_secs(unix_at_local(2024, 1, 17, 12, 0, 0));

        let entries = vec![
            // Today, 09:00 -> counts toward all three buckets.
            Entry {
                id: "today".into(),
                mode: EntryType::Timer,
                started_at_unix: unix_at_local(2024, 1, 17, 9, 0, 0),
                ended_at_unix: unix_at_local(2024, 1, 17, 9, 10, 0),
                duration_secs: 600,
            },
            // Monday this week (week start), before today -> week + month only.
            Entry {
                id: "this-week".into(),
                mode: EntryType::Timer,
                started_at_unix: unix_at_local(2024, 1, 15, 8, 0, 0),
                ended_at_unix: unix_at_local(2024, 1, 15, 8, 5, 0),
                duration_secs: 300,
            },
            // Earlier this month, before this week -> month only.
            Entry {
                id: "this-month".into(),
                mode: EntryType::Timer,
                started_at_unix: unix_at_local(2024, 1, 3, 8, 0, 0),
                ended_at_unix: unix_at_local(2024, 1, 3, 8, 5, 0),
                duration_secs: 300,
            },
            // Last month -> none of the buckets.
            Entry {
                id: "last-month".into(),
                mode: EntryType::Timer,
                started_at_unix: unix_at_local(2023, 12, 20, 8, 0, 0),
                ended_at_unix: unix_at_local(2023, 12, 20, 8, 5, 0),
                duration_secs: 300,
            },
        ];

        let totals = compute_totals(&entries, now);
        assert_eq!(totals.today, 600);
        assert_eq!(totals.this_week, 900);
        assert_eq!(totals.this_month, 1200);
    }

    #[test]
    fn entry_exactly_at_day_boundary_counts_toward_the_new_day() {
        let now =
            SystemTime::UNIX_EPOCH + Duration::from_secs(unix_at_local(2024, 1, 17, 23, 59, 0));
        let entries = vec![Entry {
            id: "midnight".into(),
            mode: EntryType::Timer,
            started_at_unix: unix_at_local(2024, 1, 17, 0, 0, 0),
            ended_at_unix: unix_at_local(2024, 1, 17, 0, 1, 0),
            duration_secs: 60,
        }];
        assert_eq!(compute_totals(&entries, now).today, 60);

        let entries_before_midnight = vec![Entry {
            id: "before-midnight".into(),
            mode: EntryType::Timer,
            started_at_unix: unix_at_local(2024, 1, 16, 23, 59, 59),
            ended_at_unix: unix_at_local(2024, 1, 17, 0, 0, 30),
            duration_secs: 31,
        }];
        assert_eq!(compute_totals(&entries_before_midnight, now).today, 0);
    }

    #[test]
    fn entry_exactly_at_week_boundary_counts_toward_the_new_week() {
        // Week starts Monday 2024-01-15.
        let now =
            SystemTime::UNIX_EPOCH + Duration::from_secs(unix_at_local(2024, 1, 17, 12, 0, 0));
        let on_monday = vec![Entry {
            id: "monday".into(),
            mode: EntryType::Timer,
            started_at_unix: unix_at_local(2024, 1, 15, 0, 0, 0),
            ended_at_unix: unix_at_local(2024, 1, 15, 0, 1, 0),
            duration_secs: 60,
        }];
        assert_eq!(compute_totals(&on_monday, now).this_week, 60);

        let before_monday = vec![Entry {
            id: "sunday".into(),
            mode: EntryType::Timer,
            started_at_unix: unix_at_local(2024, 1, 14, 23, 59, 0),
            ended_at_unix: unix_at_local(2024, 1, 14, 23, 59, 30),
            duration_secs: 30,
        }];
        assert_eq!(compute_totals(&before_monday, now).this_week, 0);
    }

    #[test]
    fn entry_exactly_at_month_boundary_counts_toward_the_new_month() {
        let now =
            SystemTime::UNIX_EPOCH + Duration::from_secs(unix_at_local(2024, 1, 17, 12, 0, 0));
        let on_first = vec![Entry {
            id: "first".into(),
            mode: EntryType::Timer,
            started_at_unix: unix_at_local(2024, 1, 1, 0, 0, 0),
            ended_at_unix: unix_at_local(2024, 1, 1, 0, 1, 0),
            duration_secs: 60,
        }];
        assert_eq!(compute_totals(&on_first, now).this_month, 60);

        let before_first = vec![Entry {
            id: "last-day-of-december".into(),
            mode: EntryType::Timer,
            started_at_unix: unix_at_local(2023, 12, 31, 23, 59, 0),
            ended_at_unix: unix_at_local(2023, 12, 31, 23, 59, 30),
            duration_secs: 30,
        }];
        assert_eq!(compute_totals(&before_first, now).this_month, 0);
    }

    #[test]
    fn both_modes_contribute_to_totals() {
        let now =
            SystemTime::UNIX_EPOCH + Duration::from_secs(unix_at_local(2024, 1, 17, 12, 0, 0));
        let entries = vec![
            Entry {
                id: "timer".into(),
                mode: EntryType::Timer,
                started_at_unix: unix_at_local(2024, 1, 17, 9, 0, 0),
                ended_at_unix: unix_at_local(2024, 1, 17, 9, 5, 0),
                duration_secs: 300,
            },
            Entry {
                id: "stopwatch".into(),
                mode: EntryType::Stopwatch,
                started_at_unix: unix_at_local(2024, 1, 17, 10, 0, 0),
                ended_at_unix: unix_at_local(2024, 1, 17, 10, 5, 0),
                duration_secs: 300,
            },
        ];
        assert_eq!(compute_totals(&entries, now).today, 600);
    }

    #[test]
    fn update_times_rewrites_start_end_and_derives_duration_keeping_mode() {
        let dir = temp_dir("update-times");
        fs::create_dir_all(&dir).unwrap();
        let store = EntriesStore::new(dir.clone());
        store
            .append(Entry {
                id: "entry-1".into(),
                mode: EntryType::Stopwatch,
                started_at_unix: 1_000,
                ended_at_unix: 1_100,
                duration_secs: 100,
            })
            .unwrap();

        let updated = store.update_times("entry-1", 2_000, 2_005).unwrap();
        assert_eq!(updated.mode, EntryType::Stopwatch);
        assert_eq!(updated.started_at_unix, 2_000);
        assert_eq!(updated.ended_at_unix, 2_005);
        assert_eq!(updated.duration_secs, 5);

        assert_eq!(store.load_all(), vec![updated]);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn update_times_allows_one_second_duration() {
        let dir = temp_dir("update-one-sec");
        fs::create_dir_all(&dir).unwrap();
        let store = EntriesStore::new(dir.clone());
        store
            .append(Entry {
                id: "entry-1".into(),
                mode: EntryType::Timer,
                started_at_unix: 1_000,
                ended_at_unix: 1_100,
                duration_secs: 100,
            })
            .unwrap();

        let updated = store.update_times("entry-1", 50, 51).unwrap();
        assert_eq!(updated.duration_secs, 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn update_times_rejects_ended_not_after_started() {
        let dir = temp_dir("update-invalid");
        fs::create_dir_all(&dir).unwrap();
        let store = EntriesStore::new(dir.clone());
        let original = Entry {
            id: "entry-1".into(),
            mode: EntryType::Timer,
            started_at_unix: 1_000,
            ended_at_unix: 1_100,
            duration_secs: 100,
        };
        store.append(original.clone()).unwrap();

        assert!(store.update_times("entry-1", 100, 100).is_err());
        assert!(store.update_times("entry-1", 200, 100).is_err());
        assert_eq!(store.load_all(), vec![original]);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn update_times_rejects_unknown_id() {
        let dir = temp_dir("update-missing");
        fs::create_dir_all(&dir).unwrap();
        let store = EntriesStore::new(dir);
        assert!(store.update_times("nope", 1, 2).is_err());
    }

    #[test]
    fn delete_removes_only_the_matching_entry() {
        let dir = temp_dir("delete");
        fs::create_dir_all(&dir).unwrap();
        let store = EntriesStore::new(dir.clone());
        let first = Entry {
            id: "first".into(),
            mode: EntryType::Timer,
            started_at_unix: 100,
            ended_at_unix: 200,
            duration_secs: 100,
        };
        let second = Entry {
            id: "second".into(),
            mode: EntryType::Timer,
            started_at_unix: 300,
            ended_at_unix: 400,
            duration_secs: 100,
        };
        store.append(first.clone()).unwrap();
        store.append(second.clone()).unwrap();

        store.delete("first").unwrap();
        assert_eq!(store.load_all(), vec![second]);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn delete_rejects_unknown_id() {
        let dir = temp_dir("delete-missing");
        fs::create_dir_all(&dir).unwrap();
        let store = EntriesStore::new(dir);
        assert!(store.delete("nope").is_err());
    }

    #[test]
    fn delete_all_removes_every_entry() {
        let dir = temp_dir("delete-all");
        fs::create_dir_all(&dir).unwrap();
        let store = EntriesStore::new(dir.clone());
        store
            .append(Entry {
                id: "first".into(),
                mode: EntryType::Timer,
                started_at_unix: 100,
                ended_at_unix: 200,
                duration_secs: 100,
            })
            .unwrap();
        store
            .append(Entry {
                id: "second".into(),
                mode: EntryType::Manual,
                started_at_unix: 300,
                ended_at_unix: 400,
                duration_secs: 100,
            })
            .unwrap();

        store.delete_all().unwrap();
        assert_eq!(store.load_all(), Vec::new());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn delete_all_succeeds_when_there_are_no_entries() {
        let dir = temp_dir("delete-all-empty");
        fs::create_dir_all(&dir).unwrap();
        let store = EntriesStore::new(dir.clone());

        store.delete_all().unwrap();
        assert_eq!(store.load_all(), Vec::new());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn create_manual_persists_manual_type_without_ten_second_skip() {
        let dir = temp_dir("create-manual");
        fs::create_dir_all(&dir).unwrap();
        let store = EntriesStore::new(dir.clone());

        let created = store.create_manual(1_000, 1_005).unwrap();
        assert_eq!(created.mode, EntryType::Manual);
        assert_eq!(created.started_at_unix, 1_000);
        assert_eq!(created.ended_at_unix, 1_005);
        assert_eq!(created.duration_secs, 5);
        assert!(Uuid::parse_str(&created.id).is_ok());
        assert_eq!(store.load_all(), vec![created]);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn create_manual_rejects_ended_not_after_started() {
        let dir = temp_dir("create-manual-invalid");
        fs::create_dir_all(&dir).unwrap();
        let store = EntriesStore::new(dir.clone());

        assert!(store.create_manual(100, 100).is_err());
        assert!(store.create_manual(200, 100).is_err());
        assert_eq!(store.load_all(), Vec::new());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn manual_entries_contribute_to_totals() {
        let now =
            SystemTime::UNIX_EPOCH + Duration::from_secs(unix_at_local(2024, 1, 17, 12, 0, 0));
        let entries = vec![Entry {
            id: "manual".into(),
            mode: EntryType::Manual,
            started_at_unix: unix_at_local(2024, 1, 17, 9, 0, 0),
            ended_at_unix: unix_at_local(2024, 1, 17, 9, 5, 0),
            duration_secs: 300,
        }];
        assert_eq!(compute_totals(&entries, now).today, 300);
    }

    #[test]
    fn create_manual_splits_at_local_midnight() {
        let dir = temp_dir("create-split-midnight");
        fs::create_dir_all(&dir).unwrap();
        let store = EntriesStore::new(dir.clone());

        let started = unix_at_local(2024, 1, 16, 23, 0, 0);
        let ended = unix_at_local(2024, 1, 17, 1, 0, 0);
        let first = store.create_manual(started, ended).unwrap();

        assert_eq!(first.mode, EntryType::Manual);
        assert_eq!(first.started_at_unix, started);
        assert_eq!(first.ended_at_unix, unix_at_local(2024, 1, 17, 0, 0, 0));
        assert_eq!(first.duration_secs, 3600);

        let loaded = store.load_all();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0], first);
        assert_eq!(loaded[1].mode, EntryType::Manual);
        assert_eq!(
            loaded[1].started_at_unix,
            unix_at_local(2024, 1, 17, 0, 0, 0)
        );
        assert_eq!(loaded[1].ended_at_unix, ended);
        assert_eq!(loaded[1].duration_secs, 3600);
        assert_ne!(loaded[1].id, first.id);
        assert!(Uuid::parse_str(&loaded[1].id).is_ok());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn create_manual_one_second_range_ending_at_midnight_is_one_piece() {
        let dir = temp_dir("create-midnight-gap");
        fs::create_dir_all(&dir).unwrap();
        let store = EntriesStore::new(dir.clone());

        let started = unix_at_local(2024, 1, 16, 23, 59, 59);
        let ended = unix_at_local(2024, 1, 17, 0, 0, 0);
        let created = store.create_manual(started, ended).unwrap();

        assert_eq!(created.started_at_unix, started);
        assert_eq!(created.ended_at_unix, ended);
        assert_eq!(created.duration_secs, 1);
        assert_eq!(store.load_all(), vec![created]);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn create_manual_ending_exactly_at_next_midnight_is_one_piece() {
        let dir = temp_dir("create-end-midnight");
        fs::create_dir_all(&dir).unwrap();
        let store = EntriesStore::new(dir.clone());

        let started = unix_at_local(2024, 1, 16, 23, 0, 0);
        let ended = unix_at_local(2024, 1, 17, 0, 0, 0);
        let created = store.create_manual(started, ended).unwrap();

        assert_eq!(created.ended_at_unix, ended);
        assert_eq!(created.duration_secs, 3600);
        assert_eq!(store.load_all(), vec![created]);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn create_manual_splits_each_calendar_day_across_two_midnights() {
        let dir = temp_dir("create-split-two-days");
        fs::create_dir_all(&dir).unwrap();
        let store = EntriesStore::new(dir.clone());

        let started = unix_at_local(2024, 1, 15, 22, 0, 0);
        let ended = unix_at_local(2024, 1, 18, 8, 0, 0);
        let first = store.create_manual(started, ended).unwrap();
        let loaded = store.load_all();

        assert_eq!(loaded.len(), 4);
        assert_eq!(loaded[0].id, first.id);
        assert_eq!(loaded[0].started_at_unix, started);
        assert_eq!(loaded[0].ended_at_unix, unix_at_local(2024, 1, 16, 0, 0, 0));
        assert_eq!(
            loaded[1].started_at_unix,
            unix_at_local(2024, 1, 16, 0, 0, 0)
        );
        assert_eq!(loaded[1].ended_at_unix, unix_at_local(2024, 1, 17, 0, 0, 0));
        assert_eq!(loaded[1].duration_secs, 24 * 3600);
        assert_eq!(
            loaded[2].started_at_unix,
            unix_at_local(2024, 1, 17, 0, 0, 0)
        );
        assert_eq!(loaded[2].ended_at_unix, unix_at_local(2024, 1, 18, 0, 0, 0));
        assert_eq!(loaded[2].duration_secs, 24 * 3600);
        assert_eq!(
            loaded[3].started_at_unix,
            unix_at_local(2024, 1, 18, 0, 0, 0)
        );
        assert_eq!(loaded[3].ended_at_unix, ended);
        assert!(loaded.iter().all(|entry| entry.mode == EntryType::Manual));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn create_manual_keeps_sub_ten_second_split_pieces() {
        let dir = temp_dir("create-split-short");
        fs::create_dir_all(&dir).unwrap();
        let store = EntriesStore::new(dir.clone());

        let started = unix_at_local(2024, 1, 16, 23, 59, 55);
        let ended = unix_at_local(2024, 1, 17, 0, 0, 3);
        store.create_manual(started, ended).unwrap();
        let loaded = store.load_all();

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].duration_secs, 5);
        assert_eq!(loaded[1].duration_secs, 3);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn load_all_leaves_an_existing_straddling_entry_intact() {
        let dir = temp_dir("leave-straddler");
        fs::create_dir_all(&dir).unwrap();
        let store = EntriesStore::new(dir.clone());
        let straddler = Entry {
            id: "old-straddle".into(),
            mode: EntryType::Manual,
            started_at_unix: unix_at_local(2024, 1, 16, 22, 0, 0),
            ended_at_unix: unix_at_local(2024, 1, 17, 2, 0, 0),
            duration_secs: 4 * 3600,
        };
        store.append(straddler.clone()).unwrap();

        assert_eq!(store.load_all(), vec![straddler]);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn load_all_leaves_an_entry_ended_at_235959_unchanged() {
        let dir = temp_dir("leave-235959");
        fs::create_dir_all(&dir).unwrap();
        let store = EntriesStore::new(dir.clone());
        let ended_at_235959 = Entry {
            id: "old-cut".into(),
            mode: EntryType::Timer,
            started_at_unix: unix_at_local(2024, 1, 16, 23, 0, 0),
            ended_at_unix: unix_at_local(2024, 1, 16, 23, 59, 59),
            duration_secs: 3599,
        };
        store.append(ended_at_235959.clone()).unwrap();

        assert_eq!(store.load_all(), vec![ended_at_235959]);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn update_times_splits_keeping_id_mode_and_list_position() {
        let dir = temp_dir("update-split-midnight");
        fs::create_dir_all(&dir).unwrap();
        let store = EntriesStore::new(dir.clone());
        let earlier = Entry {
            id: "earlier".into(),
            mode: EntryType::Timer,
            started_at_unix: unix_at_local(2024, 1, 16, 8, 0, 0),
            ended_at_unix: unix_at_local(2024, 1, 16, 8, 10, 0),
            duration_secs: 600,
        };
        let target = Entry {
            id: "target".into(),
            mode: EntryType::Stopwatch,
            started_at_unix: unix_at_local(2024, 1, 16, 10, 0, 0),
            ended_at_unix: unix_at_local(2024, 1, 16, 11, 0, 0),
            duration_secs: 3600,
        };
        store.append(earlier.clone()).unwrap();
        store.append(target.clone()).unwrap();

        let started = unix_at_local(2024, 1, 16, 22, 0, 0);
        let ended = unix_at_local(2024, 1, 18, 8, 0, 0);
        let updated = store.update_times("target", started, ended).unwrap();

        assert_eq!(updated.id, "target");
        assert_eq!(updated.mode, EntryType::Stopwatch);
        assert_eq!(updated.started_at_unix, started);
        assert_eq!(updated.ended_at_unix, unix_at_local(2024, 1, 17, 0, 0, 0));

        let loaded = store.load_all();
        assert_eq!(loaded.len(), 4);
        assert_eq!(loaded[0], earlier);
        assert_eq!(loaded[1], updated);
        assert_eq!(loaded[2].mode, EntryType::Stopwatch);
        assert_eq!(
            loaded[2].started_at_unix,
            unix_at_local(2024, 1, 17, 0, 0, 0)
        );
        assert_eq!(loaded[2].ended_at_unix, unix_at_local(2024, 1, 18, 0, 0, 0));
        assert_eq!(loaded[2].duration_secs, 24 * 3600);
        assert_eq!(
            loaded[3].started_at_unix,
            unix_at_local(2024, 1, 18, 0, 0, 0)
        );
        assert_eq!(loaded[3].ended_at_unix, ended);
        assert_ne!(loaded[2].id, "target");
        assert_ne!(loaded[3].id, "target");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn update_times_moves_identity_to_the_earliest_piece() {
        let dir = temp_dir("update-identity-moves");
        fs::create_dir_all(&dir).unwrap();
        let store = EntriesStore::new(dir.clone());
        store
            .append(Entry {
                id: "monday".into(),
                mode: EntryType::Timer,
                started_at_unix: unix_at_local(2024, 1, 15, 10, 0, 0),
                ended_at_unix: unix_at_local(2024, 1, 15, 11, 0, 0),
                duration_secs: 3600,
            })
            .unwrap();

        let updated = store
            .update_times(
                "monday",
                unix_at_local(2024, 1, 14, 22, 0, 0),
                unix_at_local(2024, 1, 15, 11, 0, 0),
            )
            .unwrap();

        assert_eq!(updated.id, "monday");
        assert_eq!(
            updated.started_at_unix,
            unix_at_local(2024, 1, 14, 22, 0, 0)
        );
        assert_eq!(updated.ended_at_unix, unix_at_local(2024, 1, 15, 0, 0, 0));

        let loaded = store.load_all();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0], updated);
        assert_eq!(
            loaded[1].started_at_unix,
            unix_at_local(2024, 1, 15, 0, 0, 0)
        );
        assert_eq!(
            loaded[1].ended_at_unix,
            unix_at_local(2024, 1, 15, 11, 0, 0)
        );
        assert_eq!(loaded[1].mode, EntryType::Timer);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn update_times_same_day_does_not_add_entries() {
        let dir = temp_dir("update-same-day");
        fs::create_dir_all(&dir).unwrap();
        let store = EntriesStore::new(dir.clone());
        store
            .append(Entry {
                id: "entry-1".into(),
                mode: EntryType::Manual,
                started_at_unix: unix_at_local(2024, 1, 16, 10, 0, 0),
                ended_at_unix: unix_at_local(2024, 1, 16, 11, 0, 0),
                duration_secs: 3600,
            })
            .unwrap();

        let updated = store
            .update_times(
                "entry-1",
                unix_at_local(2024, 1, 16, 9, 0, 0),
                unix_at_local(2024, 1, 16, 10, 0, 0),
            )
            .unwrap();
        assert_eq!(store.load_all(), vec![updated]);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn update_times_does_not_split_an_untouched_straddler() {
        let dir = temp_dir("update-other-leaves-straddler");
        fs::create_dir_all(&dir).unwrap();
        let store = EntriesStore::new(dir.clone());
        let straddler = Entry {
            id: "straddle".into(),
            mode: EntryType::Manual,
            started_at_unix: unix_at_local(2024, 1, 16, 22, 0, 0),
            ended_at_unix: unix_at_local(2024, 1, 17, 2, 0, 0),
            duration_secs: 4 * 3600,
        };
        let other = Entry {
            id: "other".into(),
            mode: EntryType::Timer,
            started_at_unix: unix_at_local(2024, 1, 16, 8, 0, 0),
            ended_at_unix: unix_at_local(2024, 1, 16, 8, 5, 0),
            duration_secs: 300,
        };
        store.append(straddler.clone()).unwrap();
        store.append(other).unwrap();

        store
            .update_times(
                "other",
                unix_at_local(2024, 1, 16, 8, 0, 0),
                unix_at_local(2024, 1, 16, 8, 10, 0),
            )
            .unwrap();

        assert_eq!(store.load_all()[0], straddler);
        assert_eq!(store.load_all().len(), 2);

        let _ = fs::remove_dir_all(dir);
    }
}
