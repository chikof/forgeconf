use std::path::Path;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use super::ConfigSource;
use crate::{ConfigError, ConfigNode};

/// Wraps any [`ConfigSource`] and re-loads it whenever watched paths change on
/// the file system.
///
/// Use [`try_recv`](WatchSource::try_recv) to poll for updated config nodes
/// without blocking.
///
/// The underlying notify watcher is kept alive for as long as `WatchSource`
/// exists; dropping it stops watching.
pub struct WatchSource<S> {
    inner: Arc<S>,
    priority: u8,
    updates: Mutex<mpsc::Receiver<Result<ConfigNode, ConfigError>>>,
    // Held only to keep the watcher alive; never accessed after construction.
    _watcher: Mutex<RecommendedWatcher>,
}

impl<S: ConfigSource + Send + Sync + 'static> WatchSource<S> {
    /// Create a watcher that monitors `paths` and reloads `source` on any
    /// create / modify / remove event.
    pub fn new(
        source: S,
        paths: impl IntoIterator<Item = impl AsRef<Path>>,
    ) -> Result<Self, notify::Error> {
        let inner = Arc::new(source);

        let (event_tx, event_rx) = mpsc::channel();
        let (reload_tx, reload_rx) = mpsc::channel::<Result<ConfigNode, ConfigError>>();

        let mut watcher = notify::recommended_watcher(event_tx)?;
        for path in paths {
            watcher.watch(path.as_ref(), RecursiveMode::NonRecursive)?;
        }

        // No debounce: each FS event triggers one load(). Callers that write
        // files in rapid succession will queue multiple reloads; try_recv()
        // drains to the latest so only the final result is visible.
        let inner_clone = Arc::clone(&inner);
        thread::spawn(move || {
            for result in event_rx {
                let Ok(event) = result else { continue };
                if !matches!(
                    event.kind,
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                ) {
                    continue;
                }
                if reload_tx.send(inner_clone.load()).is_err() {
                    break;
                }
            }
            // event_rx closes when _watcher is dropped, which stops this
            // thread.
        });

        Ok(Self {
            inner,
            priority: 0,
            updates: Mutex::new(reload_rx),
            _watcher: Mutex::new(watcher),
        })
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Returns the most recent reload result, or `None` if no change has
    /// occurred since the last call. Drains all queued events and returns only
    /// the latest so callers never observe a stale node after a burst of
    /// writes.
    pub fn try_recv(&self) -> Option<Result<ConfigNode, ConfigError>> {
        let rx = self.updates.lock().unwrap();
        let mut last = None;
        while let Ok(v) = rx.try_recv() {
            last = Some(v);
        }
        last
    }
}

impl<S: ConfigSource + Send + Sync + 'static> ConfigSource for WatchSource<S> {
    fn priority(&self) -> u8 {
        self.priority
    }

    fn load(&self) -> Result<ConfigNode, ConfigError> {
        self.inner.load()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::io::Write;
    use std::time::Duration;

    use tempfile::NamedTempFile;

    use super::*;

    struct StaticSource(ConfigNode);

    impl ConfigSource for StaticSource {
        fn load(&self) -> Result<ConfigNode, ConfigError> {
            Ok(self.0.clone())
        }
    }

    fn node(key: &str, val: &str) -> ConfigNode {
        let mut map = BTreeMap::new();
        map.insert(key.to_string(), ConfigNode::Scalar(val.to_string()));
        ConfigNode::Table(map)
    }

    #[test]
    fn triggers_reload_on_file_modify() {
        let mut tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();

        let watch = match WatchSource::new(StaticSource(node("k", "v")), [&path]) {
            Ok(w) => w,
            // Skip only when the OS watch limit is exhausted (sandboxed CI).
            Err(e) if matches!(e.kind, notify::ErrorKind::MaxFilesWatch) => return,
            Err(e) => panic!("unexpected watcher error: {e}"),
        };

        assert!(watch.try_recv().is_none(), "no event yet");

        writeln!(tmp, "change").unwrap();
        tmp.flush().unwrap();

        // Wait for inotify event delivery.
        std::thread::sleep(Duration::from_millis(200));

        let result = watch.try_recv();
        assert!(result.is_some(), "expected reload event");
        assert!(result.unwrap().is_ok());
    }

    #[test]
    fn load_delegates_to_inner() {
        // No paths to watch. Only verifying delegation behaviour.
        let watch =
            WatchSource::new(StaticSource(node("x", "1")), std::iter::empty::<&Path>()).unwrap();
        let loaded = watch.load().unwrap();
        let table = loaded.as_table().unwrap();
        assert_eq!(table.get("x").unwrap().to_string(), "1");
    }

    #[test]
    fn with_priority_sets_value() {
        let watch = WatchSource::new(StaticSource(node("k", "v")), std::iter::empty::<&Path>())
            .unwrap()
            .with_priority(42);
        assert_eq!(watch.priority(), 42);
    }
}
