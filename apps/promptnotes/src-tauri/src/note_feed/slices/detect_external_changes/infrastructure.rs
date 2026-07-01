use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use notify::event::EventKind;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

use super::domain::RawFileEvent;

const DEBOUNCE_WINDOW_MS: u64 = 500;

pub struct FsWatcher {
    _watcher: RecommendedWatcher,
    rx: Option<mpsc::Receiver<RawFileEvent>>,
}

impl FsWatcher {
    pub fn new(watch_dir: &Path) -> std::io::Result<Self> {
        let (tx, rx) = mpsc::channel();
        let tx_inner = tx.clone();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let path = event.paths.into_iter().next();
                    if let Some(p) = path {
                        if Self::is_tmp_file(&p) {
                            return;
                        }
                        if !Self::is_md_file(&p) {
                            return;
                        }
                        let raw = match event.kind {
                            EventKind::Create(_) => Some(RawFileEvent::Created(p)),
                            EventKind::Modify(_) => Some(RawFileEvent::Modified(p)),
                            EventKind::Remove(_) => Some(RawFileEvent::Deleted(p)),
                            _ => None,
                        };
                        if let Some(ev) = raw {
                            let _ = tx_inner.send(ev);
                        }
                    }
                }
            },
            Config::default(),
        )
        .map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("notify init: {e}"))
        })?;

        watcher
            .watch(watch_dir, RecursiveMode::NonRecursive)
            .map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::Other, format!("notify watch: {e}"))
            })?;

        Ok(Self {
            _watcher: watcher,
            rx: Some(rx),
        })
    }

    pub fn take_receiver(&mut self) -> mpsc::Receiver<RawFileEvent> {
        self.rx.take().expect("FsWatcher receiver already taken")
    }

    pub fn run_event_loop<F>(
        rx: mpsc::Receiver<RawFileEvent>,
        stop_rx: mpsc::Receiver<()>,
        mut on_event: F,
    ) where
        F: FnMut(RawFileEvent),
    {
        let debounce_window = Duration::from_millis(DEBOUNCE_WINDOW_MS);
        let mut last_seen: HashMap<PathBuf, Instant> = HashMap::new();

        loop {
            let timeout = if last_seen.is_empty() {
                Duration::from_secs(1)
            } else {
                debounce_window
            };

            match rx.recv_timeout(timeout) {
                Ok(event) => {
                    let path = match &event {
                        RawFileEvent::Created(p)
                        | RawFileEvent::Modified(p)
                        | RawFileEvent::Deleted(p) => p.clone(),
                    };

                    let now = Instant::now();
                    if let Some(&last) = last_seen.get(&path) {
                        if now.duration_since(last) < debounce_window {
                            last_seen.insert(path, now);
                            continue;
                        }
                    }
                    last_seen.insert(path, now);
                    on_event(event);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    last_seen.clear();
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }

            match stop_rx.try_recv() {
                Ok(()) | Err(mpsc::TryRecvError::Disconnected) => break,
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
    }

    pub fn debounce_window() -> Duration {
        Duration::from_millis(DEBOUNCE_WINDOW_MS)
    }

    pub fn is_md_file(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("md"))
            .unwrap_or(false)
    }

    pub fn is_tmp_file(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("tmp"))
            .unwrap_or(false)
    }
}
