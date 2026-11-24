use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use std::time::SystemTime;

use crate::entries::EntriesStore;
use crate::persistence::{Persistence, Settings, UpdaterMeta, WindowPosition};
use crate::timer::{TimerEngine, TimerSnapshot};
use crate::updater::{DownloadedUpdate, PendingUpdate, UpdateStatus};

pub struct AppState {
    pub engine: Mutex<TimerEngine>,
    pub settings: Mutex<Settings>,
    pub persistence: Persistence,
    pub entries: Mutex<EntriesStore>,
    pub main_window_position: Mutex<Option<WindowPosition>>,
    pub last_tray_title: Mutex<String>,
    pub ticks_since_persist: Mutex<u32>,
    pub updater_meta: Mutex<UpdaterMeta>,
    pub update_status: Mutex<UpdateStatus>,
    pub update_in_flight: AtomicBool,
    pub pending_update: Mutex<Option<PendingUpdate>>,
    pub downloaded_update: Mutex<Option<DownloadedUpdate>>,
    pub download_cancelled: AtomicBool,
}

impl AppState {
    pub fn new(
        persistence: Persistence,
        entries: EntriesStore,
        settings: Settings,
        engine: TimerEngine,
        main_window_position: Option<WindowPosition>,
        updater_meta: UpdaterMeta,
    ) -> Self {
        let formatted = TimerSnapshot::from_engine(&engine, SystemTime::now()).formatted;
        let title = if settings.icon_only {
            String::new()
        } else {
            crate::tray::tray_title(&formatted).to_string()
        };
        Self {
            engine: Mutex::new(engine),
            settings: Mutex::new(settings),
            persistence,
            entries: Mutex::new(entries),
            main_window_position: Mutex::new(main_window_position),
            last_tray_title: Mutex::new(title),
            ticks_since_persist: Mutex::new(0),
            updater_meta: Mutex::new(updater_meta),
            update_status: Mutex::new(UpdateStatus::Idle),
            update_in_flight: AtomicBool::new(false),
            pending_update: Mutex::new(None),
            downloaded_update: Mutex::new(None),
            download_cancelled: AtomicBool::new(false),
        }
    }

    pub fn snapshot(&self) -> TimerSnapshot {
        let engine = self.engine.lock().expect("engine lock");
        TimerSnapshot::from_engine(&engine, SystemTime::now())
    }

    pub fn persist(&self) -> Result<(), String> {
        let settings = self.settings.lock().expect("settings lock");
        let engine = self.engine.lock().expect("engine lock");
        let main_window_position = *self
            .main_window_position
            .lock()
            .expect("main window position lock");
        let updater = self.updater_meta.lock().expect("updater meta lock");
        self.persistence.save(
            &settings,
            &engine,
            main_window_position,
            &updater,
            SystemTime::now(),
        )
    }
}
