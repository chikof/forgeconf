use std::collections::BTreeMap;
use std::ffi::OsString;
use std::marker::PhantomData;
#[cfg(feature = "dotenv")]
use std::path::{Path, PathBuf};

use crate::__private::collect_deprecations;
use crate::merge::merge;
#[cfg(feature = "cli")]
use crate::source::cli::cli_node;
use crate::source::env::env_node;
use crate::{ConfigError, ConfigFile, ConfigNode, ConfigSource, FromNode, Table, Warning};

/// A type that knows how to build its own [`Loader`].
///
/// Implemented by `#[derive(Config)]`, which re-exports under the same name:
/// the derive adds the files declared with `#[config(file = ...)]` to
/// [`Config::loader`].
pub trait Config: FromNode {
    /// A loader preloaded with the files declared on the type.
    fn loader() -> Loader<Self>;
}

/// Merges configuration sources and converts the result into `T`.
///
/// Sources are merged in ascending priority order; a higher priority value
/// overrides a lower one, and equal priorities keep insertion order, with the
/// later source winning.
///
/// | Source                              | Priority |
/// |-------------------------------------|----------|
/// | declared or added files ([`ConfigFile`]) | 10  |
/// | [`source`](Loader::source) default  | 100      |
/// | environment variables, then dotenv  | 200      |
/// | command line                        | 250      |
///
/// A null value never overrides a lower source, arrays replace instead of
/// appending, and tables merge key by key.
pub struct Loader<T> {
    pub(crate) sources: Vec<Box<dyn ConfigSource>>,
    env_vars: Option<BTreeMap<String, String>>,
    on_warning: Box<dyn Fn(&Warning) + Send + Sync>,
    marker: PhantomData<fn() -> T>,
    #[cfg(feature = "dotenv")]
    dotenv: Option<PathBuf>,
    #[cfg(feature = "cli")]
    cli_args: Option<Vec<OsString>>,
}

impl<T: FromNode> Loader<T> {
    /// A loader with no sources.
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            env_vars: None,
            // No logging dependency: print like 0.5 did. Replace with
            // `on_warning`.
            on_warning: Box::new(|warning| eprintln!("warning: {warning}")),
            marker: PhantomData,
            #[cfg(feature = "dotenv")]
            dotenv: None,
            #[cfg(feature = "cli")]
            cli_args: None,
        }
    }

    /// Add a configuration file.
    #[must_use]
    pub fn file(self, file: ConfigFile) -> Self {
        self.source(file)
    }

    /// Add any [`ConfigSource`]; its [`priority`](ConfigSource::priority)
    /// decides the merge order.
    #[must_use]
    pub fn source(mut self, source: impl ConfigSource + 'static) -> Self {
        self.sources.push(Box::new(source));
        self
    }

    /// Use `vars` instead of the process environment. Meant for tests, which
    /// then never touch global state.
    #[must_use]
    pub fn env_vars<I, K, V>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.env_vars = Some(
            vars.into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        );
        self
    }

    /// Replace the default warning handler (which prints `warning: ...` to
    /// stderr). Called once per deprecated key present in the merged tree.
    #[must_use]
    pub fn on_warning(mut self, handler: impl Fn(&Warning) + Send + Sync + 'static) -> Self {
        self.on_warning = Box::new(handler);
        self
    }

    /// Load every source, merge them, report deprecations and convert.
    ///
    /// # Errors
    /// The first source error, or the conversion errors of `T` (all of them
    /// at once when several fields fail).
    pub fn load(&self) -> Result<T, ConfigError> {
        let mut layers = Vec::with_capacity(self.sources.len());
        for source in &self.sources {
            layers.push((source.priority(), source.load()?));
        }
        layers.push((ENV_PRIORITY, self.env_layer()?));
        #[cfg(feature = "cli")]
        if let Some(args) = &self.cli_args {
            layers.push((250, cli_node(T::shape(), args)?));
        }
        // Stable: equal priorities keep insertion order, later wins.
        layers.sort_by_key(|(priority, _)| *priority);

        let mut merged = ConfigNode::table(Table::new());
        for (_, node) in layers {
            merge(&mut merged, node);
        }

        let mut warnings = Vec::new();
        collect_deprecations(&merged, T::shape(), &mut warnings);
        for warning in &warnings {
            (self.on_warning)(warning);
        }

        T::from_node(&merged)
    }
}

#[cfg(feature = "cli")]
impl<T: FromNode> Loader<T> {
    /// Read flags from the process arguments ([`std::env::args_os`]).
    ///
    /// Flags are derived from `T`: `--port`, `--server.max-conn`, `-p` with
    /// `short`, `--verbose` / `--no-verbose` for booleans. Repeated list flags
    /// append, `-h` / `--help` yields a [`Help`](crate::ErrorKind::Help) error
    /// from [`load`](Loader::load). See [`load_or_exit`](Loader::load_or_exit).
    #[must_use]
    pub fn cli(self) -> Self {
        self.cli_args(std::env::args_os())
    }

    /// Like [`cli`](Loader::cli) with explicit arguments. `args` is the full
    /// argv, so the first item is the program name. Meant for tests.
    #[must_use]
    pub fn cli_args<I>(mut self, args: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<OsString>,
    {
        self.cli_args = Some(args.into_iter().map(Into::into).collect());
        self
    }

    /// [`load`](Loader::load), then exit the process on failure: `--help`
    /// prints the help text to stdout and exits with status 0, any other
    /// error prints `error: ...` to stderr and exits with status 2.
    pub fn load_or_exit(&self) -> T {
        match self.load() {
            Ok(value) => value,
            Err(err) if err.is_help() => {
                println!("{err}");
                std::process::exit(0);
            },
            Err(err) => {
                eprintln!("error: {err}");
                std::process::exit(2);
            },
        }
    }
}

impl<T: FromNode> Default for Loader<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Priority of the environment layer, dotenv values included.
const ENV_PRIORITY: u8 = 200;

impl<T: FromNode> Loader<T> {
    /// Variables the schema maps to: the process environment (or the
    /// `env_vars` map), then the dotenv file as a fallback.
    fn env_layer(&self) -> Result<ConfigNode, ConfigError> {
        #[cfg(feature = "dotenv")]
        let fallback = match &self.dotenv {
            Some(path) => crate::source::dotenv::load(path)?,
            None => BTreeMap::new(),
        };
        #[cfg(not(feature = "dotenv"))]
        let fallback = BTreeMap::<String, String>::new();

        env_node(T::shape(), &|name| {
            let primary = match &self.env_vars {
                Some(vars) => vars.get(name).cloned().map(Ok),
                None => std::env::var_os(name).map(OsString::into_string),
            };
            primary.or_else(|| fallback.get(name).cloned().map(Ok))
        })
    }
}

#[cfg(feature = "dotenv")]
impl<T: FromNode> Loader<T> {
    /// Read variables from the dotenv file at `path` when loading.
    ///
    /// They fill in what the real environment (or [`env_vars`](Self::env_vars))
    /// leaves unset and never override it. A missing file is ignored. The file
    /// is read on every [`load`](Self::load), so reloads see edits.
    #[must_use]
    pub fn dotenv(mut self, path: impl Into<PathBuf>) -> Self {
        self.dotenv = Some(path.into());
        self
    }

    #[cfg_attr(not(feature = "watch"), allow(dead_code, reason = "read by Loader::watch"))]
    pub(crate) fn dotenv_path(&self) -> Option<&Path> {
        self.dotenv.as_deref()
    }
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::{__private as private, FieldInfo, Origin, Shape, StructInfo};

    struct Static {
        priority: u8,
        node: ConfigNode,
    }

    impl Static {
        fn new(priority: u8, key: &str, value: &str) -> Self {
            let mut table = Table::new();
            table.insert(key.to_owned(), ConfigNode::scalar(value));
            Self { priority, node: ConfigNode::table(table) }
        }
    }

    impl ConfigSource for Static {
        fn load(&self) -> Result<ConfigNode, ConfigError> {
            Ok(self.node.clone())
        }

        fn priority(&self) -> u8 {
            self.priority
        }
    }

    struct Failing;

    impl ConfigSource for Failing {
        fn load(&self) -> Result<ConfigNode, ConfigError> {
            Err(ConfigError::invalid("boom"))
        }
    }

    #[derive(Debug, PartialEq)]
    struct Probe {
        name: String,
        legacy: Option<String>,
    }

    static PROBE: StructInfo = StructInfo::new(
        "Probe",
        &[
            FieldInfo::new("name", <String as FromNode>::shape),
            FieldInfo::new("legacy", <Option<String> as FromNode>::shape)
                .aliases(&["old"])
                .deprecated("use name"),
        ],
    );

    impl FromNode for Probe {
        fn from_node(node: &ConfigNode) -> Result<Self, ConfigError> {
            let table = private::table(node, "Probe")?;
            let mut errors = private::Collector::new();
            let name = errors.take(private::field(node, table, &["name"], false, None, |_| Ok(())));
            let legacy =
                errors
                    .take(private::field(node, table, &["legacy", "old"], false, None, |_| Ok(())));
            if let (Some(name), Some(legacy)) = (name, legacy) {
                errors.finish(Self { name, legacy })
            } else {
                Err(errors.into_error())
            }
        }

        fn shape() -> Shape {
            Shape::Struct(&PROBE)
        }
    }

    fn name_of(loader: &Loader<ConfigNode>) -> String {
        let node = loader.load().unwrap();
        node.get("k")
            .and_then(ConfigNode::as_str)
            .unwrap()
            .to_owned()
    }

    #[test]
    fn higher_priority_wins_regardless_of_insertion_order() {
        let loader = Loader::<ConfigNode>::new()
            .source(Static::new(200, "k", "high"))
            .source(Static::new(5, "k", "low"));
        assert_eq!(name_of(&loader), "high");
    }

    #[test]
    fn equal_priority_later_insertion_wins() {
        let loader = Loader::<ConfigNode>::default()
            .source(Static::new(100, "k", "first"))
            .source(Static::new(100, "k", "second"));
        assert_eq!(name_of(&loader), "second");
    }

    #[test]
    fn default_source_priority_is_above_files() {
        let loader = Loader::<ConfigNode>::new()
            .source(Static::new(100, "k", "source"))
            .file(ConfigFile::new("/definitely/not/here.toml").optional());
        assert_eq!(name_of(&loader), "source");
    }

    #[test]
    fn null_source_keeps_lower_values() {
        let loader = Loader::<ConfigNode>::new()
            .source(Static::new(5, "k", "base"))
            .source(Static { priority: 200, node: ConfigNode::null() });
        assert_eq!(name_of(&loader), "base");
    }

    #[test]
    fn source_error_aborts_load() {
        let err = Loader::<ConfigNode>::new()
            .source(Static::new(5, "k", "base"))
            .source(Failing)
            .load()
            .unwrap_err();
        assert_eq!(err.to_string(), "boom");
    }

    #[test]
    fn empty_loader_yields_empty_table() {
        let node = Loader::<ConfigNode>::new().load().unwrap();
        assert_eq!(node, ConfigNode::table(Table::new()));
    }

    #[test]
    fn conversion_errors_are_returned() {
        let err = Loader::<Probe>::new().load().unwrap_err();
        assert_eq!(err.to_string(), "name: missing value");
    }

    #[test]
    fn warning_handler_runs_once_per_deprecated_key() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&seen);
        let mut table = Table::new();
        table.insert("name".into(), ConfigNode::scalar("x"));
        table.insert(
            "old".into(),
            ConfigNode::scalar("y").with_origin(Origin::new("app.toml", 2, 7)),
        );
        let probe = Loader::<Probe>::new()
            .source(Static {
                priority: 100,
                node: ConfigNode::table(table),
            })
            .on_warning(move |warning| {
                if let Ok(mut seen) = sink.lock() {
                    seen.push(warning.to_string());
                }
            })
            .load()
            .unwrap();

        assert_eq!(
            probe,
            Probe {
                name: "x".into(),
                legacy: Some("y".into())
            }
        );
        let seen = seen.lock().unwrap();
        assert_eq!(*seen, ["old: use name (app.toml:2:7)"]);
    }

    #[test]
    fn no_warning_for_absent_deprecated_key() {
        let seen = Arc::new(Mutex::new(0usize));
        let sink = Arc::clone(&seen);
        Loader::<Probe>::new()
            .source(Static::new(100, "name", "x"))
            .on_warning(move |_| {
                if let Ok(mut count) = sink.lock() {
                    *count += 1;
                }
            })
            .load()
            .unwrap();
        assert_eq!(*seen.lock().unwrap(), 0);
    }

    #[test]
    fn env_vars_accepts_pairs() {
        let loader = Loader::<ConfigNode>::new().env_vars([("A", "1"), ("B", "2")]);
        assert_eq!(loader.env_vars.as_ref().map(BTreeMap::len), Some(2));
    }
}
