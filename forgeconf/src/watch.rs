use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, SystemTime};
use std::{fs, thread};

use crate::{ConfigError, FromNode, Loader};

/// What the watcher compares between polls: `None` when the file is absent.
type Stamp = Option<(Option<SystemTime>, u64)>;

/// Receives a freshly loaded value each time a watched file changes.
///
/// Created by [`Loader::watch`]. Dropping the watcher stops the background
/// thread (at its next poll, within one interval).
pub struct Watcher<T> {
    rx: mpsc::Receiver<Result<T, ConfigError>>,
    stop: Arc<AtomicBool>,
}

impl<T> Watcher<T> {
    /// The next reload result if one is already waiting.
    #[must_use]
    pub fn try_recv(&self) -> Option<Result<T, ConfigError>> {
        self.rx.try_recv().ok()
    }

    /// Block until the next reload. `None` once the watcher thread has ended.
    #[must_use]
    pub fn recv(&self) -> Option<Result<T, ConfigError>> {
        self.rx.recv().ok()
    }

    /// Block for at most `timeout`. `None` on timeout or once the watcher
    /// thread has ended.
    #[must_use]
    pub fn recv_timeout(&self, timeout: Duration) -> Option<Result<T, ConfigError>> {
        self.rx.recv_timeout(timeout).ok()
    }
}

impl<T> Drop for Watcher<T> {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl<T: FromNode + Send + 'static> Loader<T> {
    /// Reload when a watched file changes.
    ///
    /// Watches the path of every source that reports one (declared and added
    /// files) plus the `.env` file, if configured. A background thread polls
    /// each file's modification time and length every `interval`; on any
    /// difference, including a file appearing or disappearing, it runs
    /// [`load`](Loader::load) and sends the result. No value is sent for the
    /// state at the time of this call: load once yourself first.
    ///
    /// A failed reload arrives as an `Err` and does not stop the watcher, so
    /// a half-saved or broken file recovers on the next change. Environment
    /// variables and the command line are not watched. The warning handler
    /// runs on the watcher thread.
    ///
    /// The thread ends when the [`Watcher`] is dropped.
    ///
    /// # Example
    /// ```no_run
    /// use std::time::Duration;
    /// use forgeconf::Config;
    ///
    /// #[derive(Config)]
    /// #[config(file = "app.toml")]
    /// struct AppConfig {
    ///     port: u16,
    /// }
    ///
    /// let watcher = AppConfig::loader().watch(Duration::from_millis(500));
    /// while let Some(result) = watcher.recv() {
    ///     match result {
    ///         Ok(config) => println!("port is now {}", config.port),
    ///         Err(err) => eprintln!("reload failed: {err}"),
    ///     }
    /// }
    /// ```
    #[must_use]
    pub fn watch(self, interval: Duration) -> Watcher<T> {
        #[cfg_attr(not(feature = "dotenv"), allow(unused_mut))]
        let mut paths: Vec<PathBuf> = self
            .sources
            .iter()
            .filter_map(|source| source.watch_path())
            .map(PathBuf::from)
            .collect();
        #[cfg(feature = "dotenv")]
        paths.extend(self.dotenv_path().map(PathBuf::from));

        // Snapshot here, not in the thread: a write between this call and the
        // thread start must still count as a change.
        let mut last = snapshot(&paths);
        let (tx, rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        thread::spawn(move || {
            loop {
                thread::sleep(interval);
                if flag.load(Ordering::Relaxed) {
                    return;
                }
                let now = snapshot(&paths);
                if now != last {
                    last = now;
                    if tx.send(self.load()).is_err() {
                        return;
                    }
                }
            }
        });
        Watcher { rx, stop }
    }
}

fn snapshot(paths: &[PathBuf]) -> Vec<Stamp> {
    paths
        .iter()
        .map(|path| {
            fs::metadata(path)
                .ok()
                .map(|meta| (meta.modified().ok(), meta.len()))
        })
        .collect()
}
