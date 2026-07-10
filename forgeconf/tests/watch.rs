#![cfg(all(feature = "watch", feature = "toml"))]

use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant};

use forgeconf::{Config, ConfigFile, ErrorKind, Loader};
use tempfile::tempdir;

#[derive(Debug, PartialEq, Config)]
struct App {
    name: String,
}

const INTERVAL: Duration = Duration::from_millis(20);

#[test]
fn changed_file_yields_reloaded_value() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("app.toml");
    fs::write(&path, "name = \"one\"\n").unwrap();

    let watcher = Loader::<App>::new()
        .file(ConfigFile::new(&path))
        .watch(INTERVAL);
    assert!(watcher.try_recv().is_none());

    fs::write(&path, "name = \"second\"\n").unwrap();
    let reloaded = watcher.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(reloaded.unwrap(), App { name: "second".into() });
}

#[test]
fn deleting_a_required_file_is_an_io_error() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("app.toml");
    fs::write(&path, "name = \"one\"\n").unwrap();

    let watcher = Loader::<App>::new()
        .file(ConfigFile::new(&path))
        .watch(INTERVAL);

    fs::remove_file(&path).unwrap();
    let err = watcher
        .recv_timeout(Duration::from_secs(2))
        .unwrap()
        .unwrap_err();
    assert!(matches!(err.kind(), ErrorKind::Io { .. }));
}

#[test]
fn broken_file_does_not_stop_the_watcher() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("app.toml");
    fs::write(&path, "name = \"one\"\n").unwrap();

    let watcher = Loader::<App>::new()
        .file(ConfigFile::new(&path))
        .watch(INTERVAL);

    fs::write(&path, "name = = 1\n").unwrap();
    let err = watcher
        .recv_timeout(Duration::from_secs(2))
        .unwrap()
        .unwrap_err();
    assert!(matches!(err.kind(), ErrorKind::Syntax { .. }));

    fs::write(&path, "name = \"fixed\"\n").unwrap();
    let fixed = watcher.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(fixed.unwrap(), App { name: "fixed".into() });
}

#[test]
fn dropping_the_watcher_ends_the_thread() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("app.toml");
    fs::write(&path, "name = \"one\"\n").unwrap();

    let probe = Arc::new(());
    let held = Arc::clone(&probe);
    let watcher = Loader::<App>::new()
        .file(ConfigFile::new(&path))
        .on_warning(move |_| drop(Arc::clone(&held)))
        .watch(INTERVAL);
    assert_eq!(Arc::strong_count(&probe), 2);

    drop(watcher);
    let deadline = Instant::now() + Duration::from_secs(1);
    while Arc::strong_count(&probe) != 1 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(Arc::strong_count(&probe), 1);
}
