//! The owned conversion (`from_node_owned`, used by `from_str` and
//! `Loader::load`) must give exactly the value, error text, path and origin of
//! the borrowed one.
#![cfg(feature = "toml")]
#![allow(clippy::unwrap_used, reason = "tests fail loudly")]
#![allow(dead_code, reason = "types are only converted and compared, never read")]
#![allow(clippy::trivially_copy_pass_by_ref, reason = "validators take &T")]

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;

use forgeconf::{Config, ConfigError, ConfigNode, FileFormat, FromNode, parse_str, validators};

/// Convert `input` both ways, assert they agree, return the owned result.
fn both<T: FromNode + Debug>(input: &str) -> Result<T, ConfigError> {
    let node = parse_str(input, FileFormat::Toml).unwrap();
    same(&T::from_node(&node), T::from_node_owned(node), input)
}

fn same<T: Debug>(
    borrowed: &Result<T, ConfigError>,
    owned: Result<T, ConfigError>,
    input: &str,
) -> Result<T, ConfigError> {
    match (borrowed, &owned) {
        (Ok(a), Ok(b)) => assert_eq!(format!("{a:?}"), format!("{b:?}"), "value for {input:?}"),
        (Err(a), Err(b)) => {
            assert_eq!(a.to_string(), b.to_string(), "text for {input:?}");
            assert_eq!(a.path(), b.path(), "path for {input:?}");
            assert_eq!(a.origin(), b.origin(), "origin for {input:?}");
            assert_eq!(format!("{a:?}"), format!("{b:?}"), "debug for {input:?}");
        },
        _ => panic!("paths disagree for {input:?}: {borrowed:?} vs {owned:?}"),
    }
    owned
}

fn all_agree<T: FromNode + Debug>(inputs: &[&str]) {
    for input in inputs {
        let _ = both::<T>(input);
    }
}

fn nonzero(value: &u16) -> Result<(), ConfigError> {
    if *value == 0 { Err(ConfigError::mismatch("non-zero", "0")) } else { Ok(()) }
}

fn ordered(value: &Limits) -> Result<(), ConfigError> {
    if value.low <= value.high {
        Ok(())
    } else {
        Err(ConfigError::mismatch("low <= high", format!("{} > {}", value.low, value.high)))
    }
}

#[derive(Config, Debug)]
struct Kitchen {
    name: String,
    boxed: Box<str>,
    shared: Arc<str>,
    path: PathBuf,
    ptr: Box<u16>,
    maybe: Option<String>,
    #[config(default)]
    list: Vec<String>,
    #[config(default)]
    deque: VecDeque<u8>,
    #[config(default)]
    set: BTreeSet<String>,
    #[config(default)]
    map: BTreeMap<String, u16>,
    #[config(default)]
    hash: HashMap<String, u8>,
    #[config(default)]
    tree: BTreeMap<u8, String>,
    #[config(default)]
    pair: Option<[u8; 2]>,
    #[config(default)]
    raw: Option<ConfigNode>,
}

#[test]
fn containers_and_scalars_agree() {
    let good = r#"
        name = "a"
        boxed = "b"
        shared = "c"
        path = "/x/y"
        ptr = 7
        maybe = "m"
        list = ["x", "y"]
        deque = [1, 2, 3]
        set = ["b", "a", "b"]
        pair = [1, 2]
        raw = { k = [1, "two"] }
        hash = { only = 1 }
        [map]
        one = 1
        two = 2
        [tree]
        1 = "a"
        2 = "b"
    "#;
    let loaded = both::<Kitchen>(good).unwrap();
    assert_eq!(loaded.name, "a");
    assert_eq!(&*loaded.boxed, "b");
    assert_eq!(loaded.list, ["x", "y"]);
    assert_eq!(loaded.map["two"], 2);
    assert_eq!(loaded.tree[&2], "b");
    assert_eq!(loaded.pair, Some([1, 2]));

    all_agree::<Kitchen>(&[
        "",
        "name = 1\nboxed = \"b\"",
        "name = [1]\nboxed = \"b\"",
        "name = \"a\"\nboxed = { x = 1 }\nshared = \"c\"\npath = \"p\"\nptr = 1",
        "name = \"a\"\nboxed = \"b\"\nshared = \"c\"\npath = \"p\"\nptr = 70000",
        "name = \"a\"\nboxed = \"b\"\nshared = \"c\"\npath = \"p\"\nptr = 1\nlist = [\"a\", 2, \
         [3]]",
        "name = \"a\"\nboxed = \"b\"\nshared = \"c\"\npath = \"p\"\nptr = 1\nlist = \"scalar\"",
        "name = \"a\"\nboxed = \"b\"\nshared = \"c\"\npath = \"p\"\nptr = 1\npair = [1, 2, 3]",
        "name = \"a\"\nboxed = \"b\"\nshared = \"c\"\npath = \"p\"\nptr = 1\npair = [1, \"x\"]",
        "name = \"a\"\nboxed = \"b\"\nshared = \"c\"\npath = \"p\"\nptr = 1\npair = 5",
        "name = \"a\"\nboxed = \"b\"\nshared = \"c\"\npath = \"p\"\nptr = 1\nmap = [1]",
        "name = \"a\"\nboxed = \"b\"\nshared = \"c\"\npath = \"p\"\nptr = 1\n[map]\na = 1\nb = \
         \"x\"",
        "name = \"a\"\nboxed = \"b\"\nshared = \"c\"\npath = \"p\"\nptr = 1\n[tree]\nx = \"y\"",
        "name = \"a\"\nboxed = \"b\"\nshared = \"c\"\npath = \"p\"\nptr = 1\n[tree]\n1 = [\"y\"]",
        "name = \"a\"\nboxed = \"b\"\nshared = \"c\"\npath = \"p\"\nptr = 1\ndeque = [256]",
    ]);
}

#[test]
fn null_and_wrong_kinds_agree_without_a_file() {
    fn nodes() -> Vec<ConfigNode> {
        vec![
            ConfigNode::null(),
            ConfigNode::scalar("x"),
            ConfigNode::scalar("12"),
            ConfigNode::array(vec![ConfigNode::scalar("1"), ConfigNode::null()]),
            ConfigNode::table(BTreeMap::from([("k".to_owned(), ConfigNode::scalar("v"))])),
        ]
    }
    fn check<T: FromNode + Debug>() {
        for node in nodes() {
            let borrowed = T::from_node(&node);
            let owned = T::from_node_owned(node.clone());
            let _ = same(&borrowed, owned, &format!("{node:?}"));
        }
    }
    check::<String>();
    check::<Box<str>>();
    check::<Arc<str>>();
    check::<PathBuf>();
    check::<u8>();
    check::<Option<String>>();
    check::<Vec<String>>();
    check::<Vec<Vec<u8>>>();
    check::<[u8; 2]>();
    check::<HashMap<String, String>>();
    check::<BTreeMap<u8, u8>>();
    check::<ConfigNode>();
    check::<Box<Vec<String>>>();
    check::<Arc<Option<String>>>();
}

#[derive(Config, Debug)]
#[config(validate = ordered)]
struct Limits {
    low: u8,
    high: u8,
}

#[derive(Config, Debug)]
struct Port(#[config(validate = nonzero)] u16);

#[derive(Config, Debug)]
struct Plain(String);

#[derive(Config)]
struct Checked {
    #[config(alias = "old", default = 1)]
    id: u8,
    #[config(insensitive, alias = "Level", default)]
    level: u8,
    #[config(sensitive, validate = validators::non_empty(), default = "tok")]
    token: String,
    #[config(validate = nonzero, default = 9)]
    port: u16,
    #[config(skip)]
    cache: Vec<u8>,
    nested: Option<Limits>,
    many: Vec<Limits>,
    by_name: BTreeMap<String, Limits>,
    wrapped: Port,
    plain: Plain,
}

#[test]
fn struct_fields_agree() {
    let base = "wrapped = 5\nplain = \"p\"\n";
    let loaded = both::<Checked>(&format!(
        "{base}old = 3\nLEVEL = 4\ntoken = \"s\"\nport = 80\n[nested]\nlow = 1\nhigh = \
         2\n[[many]]\nlow = 1\nhigh = 3\n[by_name.x]\nlow = 0\nhigh = 9\n"
    ))
    .unwrap();
    assert_eq!((loaded.id, loaded.level, loaded.port), (3, 4, 80));
    assert_eq!(loaded.many.len(), 1);
    assert_eq!(loaded.by_name["x"].high, 9);

    all_agree::<Checked>(&[
        "",
        base,
        &format!("{base}id = 2\nold = 3"),
        &format!("{base}Level = 5"),
        &format!("{base}level = \"x\""),
        &format!("{base}token = \"\""),
        &format!("{base}token = [1]"),
        &format!("{base}port = 0"),
        &format!("{base}port = \"x\""),
        &format!("{base}[nested]\nlow = 1"),
        &format!("{base}[nested]\nlow = \"x\"\nhigh = \"y\""),
        &format!("{base}[[many]]\nlow = 1\n[[many]]\nlow = 1\nhigh = 2"),
        &format!("{base}[by_name.a]\nlow = 5\nhigh = 1\n[by_name.b]\nlow = 1"),
        "wrapped = 0\nplain = \"p\"",
        "wrapped = \"x\"\nplain = \"p\"",
        "wrapped = 1\nplain = [1]",
        "wrapped = { a = 1 }\nplain = \"p\"",
        "nested = 5\nwrapped = 1\nplain = \"p\"",
        "many = 5\nwrapped = 1\nplain = \"p\"",
        "many = [5]\nwrapped = 1\nplain = \"p\"",
    ]);
    assert_eq!(
        both::<Checked>(&format!("{base}token = \"\""))
            .unwrap_err()
            .to_string(),
        "token: must not be empty (<string>:3:9)"
    );
}

#[derive(Config, Debug)]
#[config(deny_unknown_fields)]
struct Strict {
    host: String,
    #[config(insensitive)]
    port: u16,
    #[config(alias = "tmo", default)]
    timeout: u8,
}

#[test]
fn deny_unknown_fields_agrees() {
    all_agree::<Strict>(&[
        "host = \"h\"\nPORT = 1",
        "host = \"h\"\nport = 1\ntmo = 3",
        "host = \"h\"\nport = 1\nhots = \"x\"\nextra = 2",
        "hots = \"x\"\nPoRt = 1",
        "host = 1\nporrt = 2",
        "",
    ]);
    assert!(both::<Strict>("host = \"h\"\nPORT = 1\ntimeout = 2").is_ok());
}

#[derive(Config, Debug)]
struct Common {
    #[config(default = 1)]
    retries: u8,
}

#[derive(Config, Debug)]
struct Flat {
    name: String,
    #[config(flatten)]
    common: Common,
    inner: Option<Flat2>,
}

#[derive(Config, Debug)]
struct Flat2 {
    #[config(flatten)]
    common: Common,
    name: String,
}

#[test]
fn flatten_structs_agree() {
    all_agree::<Flat>(&[
        "name = \"a\"",
        "name = \"a\"\nretries = 5",
        "name = \"a\"\nretries = \"x\"",
        "retries = \"x\"",
        "name = \"a\"\n[inner]\nname = \"b\"\nretries = 9",
        "name = \"a\"\n[inner]\nretries = \"z\"",
    ]);
}

#[derive(Config, Debug)]
struct Collide {
    #[config(insensitive)]
    port: u16,
    #[config(rename = "PORT", default)]
    shadow: u16,
}

#[test]
fn colliding_insensitive_fields_agree() {
    let loaded = both::<Collide>("PORT = 3").unwrap();
    assert_eq!((loaded.port, loaded.shadow), (3, 3));
    all_agree::<Collide>(&["port = 3", "PORT = \"x\"", ""]);
}

#[derive(Config, Debug)]
#[config(rename_all = "lowercase")]
enum Sink {
    Stdout,
    File {
        path: String,
        #[config(default = false)]
        append: bool,
    },
    Tcp(u16),
    Custom(Limits),
    #[config(alias = "dup")]
    Pair {
        #[config(validate = nonzero)]
        a: u16,
        #[config(insensitive)]
        b: Vec<String>,
    },
}

#[derive(Config, Debug)]
struct Output {
    sink: Sink,
}

#[test]
fn externally_tagged_enums_agree() {
    all_agree::<Output>(&[
        "sink = \"stdout\"",
        "sink = \"nope\"",
        "sink = [1]",
        "sink = 4",
        "sink = {}",
        "sink = { tcp = 1, file = { path = \"x\" } }",
        "sink = { tcp = 9000 }",
        "sink = { tcp = \"x\" }",
        "sink = { tcp = 70000 }",
        "sink = { udp = 1 }",
        "sink = { file = { path = \"x\" } }",
        "sink = { file = { path = \"x\", append = true } }",
        "sink = { file = { append = \"maybe\" } }",
        "sink = { file = 5 }",
        "sink = { custom = { low = 1, high = 2 } }",
        "sink = { custom = { low = 1 } }",
        "sink = { pair = { a = 1, B = [\"x\"] } }",
        "sink = { dup = { a = 0, b = [\"x\"] } }",
        "sink = { pair = { a = 1 } }",
        "",
    ]);
    let loaded = both::<Output>("sink = { file = { path = \"x\" } }").unwrap();
    assert!(matches!(loaded.sink, Sink::File { ref path, append: false } if path == "x"));
}

#[derive(Config, Debug)]
enum OnlyData {
    A(u8),
    B { x: String },
}

#[test]
fn enum_without_unit_variants_agrees() {
    let node = parse_str("a = 1", FileFormat::Toml).unwrap();
    let _ = same(&OnlyData::from_node(&node), OnlyData::from_node_owned(node.clone()), "a = 1");
    for input in ["b = { x = \"s\" }", "b = { x = 1 }", "a = \"x\"", "a = 1\nb = 2", "c = 1", ""] {
        let node = parse_str(input, FileFormat::Toml).unwrap();
        let _ = same(&OnlyData::from_node(&node), OnlyData::from_node_owned(node), input);
    }
    for node in [ConfigNode::scalar("a"), ConfigNode::null(), ConfigNode::array(Vec::new())] {
        let _ = same(&OnlyData::from_node(&node), OnlyData::from_node_owned(node.clone()), "node");
    }
}

#[derive(Config, Debug)]
#[config(tag = "type", rename_all = "lowercase")]
enum Shape {
    Empty,
    Square(Limits),
    Circle {
        radius: u8,
        #[config(default = 1)]
        scale: u8,
    },
    #[config(alias = "ngon")]
    Poly {
        #[config(validate = nonzero)]
        sides: u16,
    },
}

#[derive(Config, Debug)]
struct Canvas {
    shapes: Vec<Shape>,
}

#[test]
fn internally_tagged_enums_agree() {
    all_agree::<Canvas>(&[
        "shapes = []",
        "shapes = [{ type = \"empty\" }]",
        "shapes = [{ type = \"square\", low = 1, high = 2 }]",
        "shapes = [{ type = \"square\", low = 1 }]",
        "shapes = [{ type = \"circle\", radius = 3 }]",
        "shapes = [{ type = \"circle\" }]",
        "shapes = [{ type = \"circle\", radius = \"r\", scale = \"s\" }]",
        "shapes = [{ type = \"poly\", sides = 3 }, { type = \"ngon\", sides = 0 }]",
        "shapes = [{ type = \"hexagon\" }]",
        "shapes = [{ radius = 3 }]",
        "shapes = [{ type = 5 }]",
        "shapes = [{ type = [1] }]",
        "shapes = [4]",
        "shapes = [{ type = \"empty\" }, { type = \"nope\" }]",
    ]);
    let loaded = both::<Canvas>("shapes = [{ type = \"circle\", radius = 3 }]").unwrap();
    assert!(matches!(loaded.shapes[0], Shape::Circle { radius: 3, scale: 1 }));
}
