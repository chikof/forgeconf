#![cfg(any(feature = "toml", feature = "json", feature = "yaml"))]

use forgeconf::{Config, ConfigError, FileFormat, from_str};

#[derive(Config, Debug, PartialEq)]
struct Db {
    url: String,
    #[config(default = 5)]
    pool: u8,
}

#[derive(Config, Debug, PartialEq)]
struct Doc {
    name: String,
    port: u16,
    tags: Vec<String>,
    #[config(default)]
    debug: bool,
    db: Db,
}

fn expected() -> Doc {
    Doc {
        name: "svc".into(),
        port: 8080,
        tags: vec!["a".into(), "b".into()],
        debug: true,
        db: Db { url: "u".into(), pool: 16 },
    }
}

fn err_of(input: &str, format: FileFormat) -> String {
    match from_str::<Doc>(input, format) {
        Ok(doc) => panic!("expected an error, got {doc:?}"),
        Err(err) => err.to_string(),
    }
}

#[allow(dead_code)]
fn parse_err(input: &str, format: FileFormat) -> ConfigError {
    forgeconf::parse_str(input, format).unwrap_err()
}

#[cfg(feature = "toml")]
mod toml {
    use super::*;

    const DOC: &str = "name = \"svc\"\nport = 8080\ntags = [\"a\", \"b\"]\ndebug = \
                       true\n\n[db]\nurl = \"u\"\npool = 0x10\n";

    #[test]
    fn from_str_reads_every_value_kind() {
        assert_eq!(from_str::<Doc>(DOC, FileFormat::Toml).unwrap(), expected());
    }

    #[test]
    fn extension_and_label() {
        assert_eq!(FileFormat::from_extension("TOML"), Some(FileFormat::Toml));
        assert_eq!(FileFormat::from_extension("cfg"), None);
        assert_eq!(FileFormat::Toml.label(), "toml");
    }

    #[test]
    fn syntax_error_has_exact_origin() {
        let err = parse_err("name = \"svc\"\nport = = 1\n", FileFormat::Toml);
        assert_eq!(err.to_string(), "invalid toml: invalid number \"=\" (<string>:2:8)");
    }

    #[test]
    fn duplicate_key_is_a_syntax_error() {
        let err = parse_err("a = 1\na = 2\n", FileFormat::Toml);
        assert_eq!(err.to_string(), "invalid toml: duplicate key \"a\" (<string>:2:1)");
    }

    #[test]
    fn type_error_points_at_the_value() {
        assert_eq!(
            err_of(
                "name = \"svc\"\nport = \"x\"\ntags = []\n[db]\nurl = \"u\"\n",
                FileFormat::Toml
            ),
            "port: expected u16, found \"x\" (<string>:2:8)"
        );
    }
}

#[cfg(feature = "json")]
mod json {
    use super::*;

    const DOC: &str = "{\n  \"name\": \"svc\",\n  \"port\": 8080,\n  \"tags\": [\"a\", \"b\"],\n  \
                       \"debug\": true,\n  \"db\": {\"url\": \"u\", \"pool\": 16}\n}\n";

    #[test]
    fn from_str_reads_every_value_kind() {
        assert_eq!(from_str::<Doc>(DOC, FileFormat::Json).unwrap(), expected());
    }

    #[test]
    fn extension_and_label() {
        assert_eq!(FileFormat::from_extension("json"), Some(FileFormat::Json));
        assert_eq!(FileFormat::Json.label(), "json");
    }

    #[test]
    fn syntax_error_has_exact_origin() {
        let err = parse_err("{\"a\": 1,}", FileFormat::Json);
        assert_eq!(
            err.to_string(),
            "invalid json: unexpected '}', expected a string key (<string>:1:9)"
        );
    }

    #[test]
    fn duplicate_key_is_a_syntax_error() {
        let err = parse_err("{\n  \"a\": 1,\n  \"a\": 2\n}", FileFormat::Json);
        assert_eq!(err.to_string(), "invalid json: duplicate key \"a\" (<string>:3:3)");
    }

    #[test]
    fn null_leaves_the_field_to_its_default() {
        let input = "{\"name\": \"svc\", \"port\": 1, \"tags\": null, \"debug\": null, \"db\": \
                     {\"url\": \"u\"}}";
        let doc = from_str::<Doc>(input, FileFormat::Json).unwrap();
        assert_eq!(
            doc,
            Doc {
                name: "svc".into(),
                port: 1,
                tags: Vec::new(),
                debug: false,
                db: Db { url: "u".into(), pool: 5 },
            }
        );
    }

    #[test]
    fn type_error_points_at_the_value() {
        assert_eq!(
            err_of(
                "{\"name\": \"svc\",\n \"port\": \"x\", \"tags\": [], \"db\": {\"url\": \"u\"}}",
                FileFormat::Json
            ),
            "port: expected u16, found \"x\" (<string>:2:10)"
        );
    }
}

#[cfg(feature = "yaml")]
mod yaml {
    use super::*;

    const DOC: &str =
        "name: svc\nport: 8080\ntags:\n  - a\n  - b\ndebug: true\ndb:\n  url: u\n  pool: 16\n";

    #[test]
    fn from_str_reads_every_value_kind() {
        assert_eq!(from_str::<Doc>(DOC, FileFormat::Yaml).unwrap(), expected());
    }

    #[test]
    fn flow_collections_and_comments() {
        let input =
            "name: svc # trailing\nport: 8080\ntags: [a, b]\ndebug: yes\ndb: {url: u, pool: 16}\n";
        assert_eq!(from_str::<Doc>(input, FileFormat::Yaml).unwrap(), expected());
    }

    #[test]
    fn extension_and_label() {
        assert_eq!(FileFormat::from_extension("yml"), Some(FileFormat::Yaml));
        assert_eq!(FileFormat::from_extension("yaml"), Some(FileFormat::Yaml));
        assert_eq!(FileFormat::Yaml.label(), "yaml");
    }

    #[test]
    fn syntax_error_has_exact_origin() {
        let err = parse_err("name: svc\n\tport: 1\n", FileFormat::Yaml);
        assert_eq!(
            err.to_string(),
            "invalid yaml: tabs are not allowed for indentation (<string>:2:1)"
        );
    }

    #[test]
    fn anchors_are_unsupported() {
        let err = parse_err("a: &x 1\n", FileFormat::Yaml);
        assert_eq!(
            err.to_string(),
            "unsupported: yaml anchors (&) are not supported (<string>:1:4)"
        );
    }

    #[test]
    fn type_error_points_at_the_value() {
        assert_eq!(
            err_of("name: svc\nport: x\ntags: []\ndb:\n  url: u\n", FileFormat::Yaml),
            "port: expected u16, found \"x\" (<string>:2:7)"
        );
    }
}
